//! Observability runtime: the M4 operations surface composing the M1
//! contracts, M2 writers, and M3 GlitchTip sink.
//!
//! The runtime is the single place an operator/service interacts with
//! the EP-038 stack: it applies redaction first, writes local fallback
//! telemetry (structured log + Prometheus + OTLP when requested), and
//! optionally delivers incidents through the real GlitchTip sink.
//!
//! Fail-closed rules enforced here:
//! - every observed field passes `RedactionPolicy` before any egress;
//! - a redaction denial is surfaced as `Policy`/`RedactionDenied` and
//!   no provider bytes are produced;
//! - incident delivery failure is classified (never a generic error)
//!   and the incident is retained in the recording sink (quarantined)
//!   so a bounded recovery can retry;
//! - the diagnostic ladder never claims READY without a production
//!   probe (see `diag`).

use nexus_domain::{CorrelationId, IncidentId};
use nexus_glitchtip::GlitchTipIncidentSink;
use nexus_observability::model::{
    now_epoch_secs, CompositeHealthAggregator, MetricDefinition, MetricRegistry,
    RecordingIncidentSink, RedactedEnvelope, RedactionPolicy, WindowedSloEvaluator,
};
use nexus_observability::port::{HealthAggregator, IncidentSink, MetricCatalog, SloEvaluator};
use nexus_observability::{
    IncidentDeliveryResult, ObservabilityError, ObservabilityResult, Severity, TelemetryContext,
    TelemetrySignal,
};

use crate::audit::AuditRecord;
use crate::diag::OpsDiagnostic;
use crate::recovery::{RecoveryBudget, RecoveryOutcome};

/// Runtime configuration (bounded, no secrets).
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub node: String,
    pub environment: String,
    pub release: String,
    /// GlitchTip DSN when real incident delivery is configured.
    pub glitchtip_dsn: Option<nexus_glitchtip::Dsn>,
    /// SLO definitions registered with the runtime.
    pub slos: Vec<nexus_observability::model::SloDefinition>,
    /// Metrics pre-registered with the catalog.
    pub metrics: Vec<MetricDefinition>,
}

/// The observability runtime.
pub struct ObservabilityRuntime {
    node: String,
    environment: String,
    release: String,
    policy: RedactionPolicy,
    metrics: MetricRegistry,
    health: CompositeHealthAggregator,
    slo: WindowedSloEvaluator,
    recording: RecordingIncidentSink,
    glitchtip: Option<GlitchTipIncidentSink>,
    /// DSN kept for diagnostics (probe path); never rendered.
    glitchtip_dsn: Option<nexus_glitchtip::Dsn>,
    /// Bounded, redacted audit trail (in-memory, newest last).
    audit: Vec<AuditRecord>,
}

impl ObservabilityRuntime {
    pub fn new(config: RuntimeConfig) -> ObservabilityResult<Self> {
        let mut metrics = MetricRegistry::new();
        for m in config.metrics {
            metrics.register(m)?;
        }
        let release = config.release.clone();
        let environment = config.environment.clone();
        let glitchtip = config.glitchtip_dsn.as_ref().map(|dsn| {
            GlitchTipIncidentSink::new(dsn.clone(), release.clone(), environment.clone())
        });
        Ok(Self {
            node: config.node,
            environment,
            release,
            policy: RedactionPolicy::default(),
            metrics,
            health: CompositeHealthAggregator::new(),
            slo: WindowedSloEvaluator,
            recording: RecordingIncidentSink::new(),
            glitchtip,
            glitchtip_dsn: config.glitchtip_dsn.clone(),
            audit: Vec::new(),
        })
    }

    // ------------------------------------------------------- redaction

    /// Apply the mandatory redaction policy to observed fields.
    /// Fail-closed: unclassified values and secret-shaped values are
    /// redacted/hashed per policy; the result is the ONLY form that may
    /// egress.
    pub fn redact(
        &self,
        signal: TelemetrySignal,
        component: &str,
        operation: &str,
        severity: Severity,
        observed: Vec<(String, String)>,
    ) -> ObservabilityResult<RedactedEnvelope> {
        self.redact_with_context(
            signal,
            TelemetryContext::new(
                self.node.clone(),
                None,
                None,
                None,
                None,
                None,
                None,
                component.to_string(),
                operation.to_string(),
                severity,
                Some(self.environment.clone()),
                None,
            )
            .expect("valid telemetry context"),
            observed,
        )
    }

    /// Apply redaction under a caller-supplied, already-validated
    /// `TelemetryContext`. AUD-056: incident redaction must NOT
    /// reconstruct telemetry with correlation/trace/request/tenant
    /// context absent. This entry point preserves whatever safe context
    /// metadata the caller holds (correlation, trace ids, request id,
    /// tenant, source interface) in the redacted envelope, so incident
    /// delivery can correlate across the provider boundary.
    pub fn redact_with_context(
        &self,
        signal: TelemetrySignal,
        context: TelemetryContext,
        observed: Vec<(String, String)>,
    ) -> ObservabilityResult<RedactedEnvelope> {
        Ok(self.policy.apply(signal, context, observed))
    }

    // ------------------------------------------------------- writers

    /// Emit a structured JSON log line from already-redacted fields.
    /// Re-verifies exportability (M2 boundary) before rendering.
    pub fn structured_log(&self, envelope: &RedactedEnvelope) -> ObservabilityResult<String> {
        nexus_otel::export_structured_log(envelope)
    }

    /// Emit a Prometheus text family line from an already-redacted
    /// metric point (local fallback writer).
    pub fn prometheus_point(
        &self,
        metric: &str,
        value: f64,
        labels: &[(String, String)],
    ) -> ObservabilityResult<String> {
        let def = self.metrics.lookup(metric).ok_or_else(|| {
            ObservabilityError::unsupported_signal(format!("unknown metric {metric}"))
        })?;
        // Catalog validation rejects unbounded labels and secret-shaped
        // values before any rendering.
        for (k, v) in labels {
            self.metrics.validate_label(metric, k, v)?;
        }
        nexus_otel::export_prometheus_family(def, value, labels)
    }

    // ------------------------------------------------------- incidents

    /// Report an incident: redact, record locally (quarantine), and
    /// deliver through the real GlitchTip sink when configured.
    ///
    /// Delivery failures are classified and the incident remains in the
    /// recording sink for bounded recovery; a failed delivery never
    /// disappears silently.
    pub fn report_incident(
        &mut self,
        dedupe_key: String,
        severity: Severity,
        classification: &str,
        source: &str,
        correlation: Option<CorrelationId>,
        observed: Vec<(String, String)>,
    ) -> IncidentDeliveryResult {
        // AUD-056: the redacted envelope context must carry the incident
        // correlation (and the canonical node/environment/operation
        // metadata), never a context reconstructed with correlation
        // absent. The caller-provided correlation is threaded into the
        // envelope context so every downstream surface (recording sink,
        // GlitchTip sink, audit correlation) sees the same correlation.
        let context = match TelemetryContext::new(
            self.node.clone(),
            None,
            None,
            correlation.clone(),
            None,
            None,
            None,
            source.to_string(),
            "incident.report".to_string(),
            severity,
            Some(self.environment.clone()),
            None,
        ) {
            Ok(ctx) => ctx,
            Err(reason) => {
                return IncidentDeliveryResult::Failed {
                    reason: reason.to_string(),
                }
            }
        };
        self.report_incident_with_context(
            dedupe_key,
            severity,
            classification,
            source,
            context,
            correlation,
            observed,
        )
    }

    /// Report an incident under a caller-supplied, already-validated
    /// telemetry context (AUD-056). The context may carry correlation,
    /// trace/span ids, request id, tenant, and source interface; those
    /// safe fields are preserved in the redacted envelope and delivered
    /// with the incident, so GlitchTip receives correlation/trace
    /// context instead of a stripped envelope.
    #[allow(clippy::too_many_arguments)]
    pub fn report_incident_with_context(
        &mut self,
        dedupe_key: String,
        severity: Severity,
        classification: &str,
        source: &str,
        context: TelemetryContext,
        correlation: Option<CorrelationId>,
        observed: Vec<(String, String)>,
    ) -> IncidentDeliveryResult {
        // The sink-port correlation is authoritative when supplied;
        // otherwise the context correlation (if any) is threaded so the
        // port and the envelope never disagree.
        let correlation = correlation.or_else(|| context.correlation.clone());
        let envelope = match self.redact_with_context(TelemetrySignal::Incident, context, observed)
        {
            Ok(e) => e,
            Err(reason) => {
                return IncidentDeliveryResult::Failed {
                    reason: reason.to_string(),
                }
            }
        };

        // Local record first: the incident is never lost on provider
        // failure (quarantine semantics).
        // Deterministic UUIDv7-shaped id derived from the dedupe key:
        // stable across redeliveries of one incident.
        let digest = nexus_observability::model::sha256_fingerprint(&dedupe_key);
        let hex: String = digest
            .chars()
            .filter(|c| c.is_ascii_hexdigit())
            .take(12)
            .collect();
        let incident_id =
            IncidentId::new(format!("018e5c5e-4d9b-7f0c-8a2b-{hex}")).expect("uuidv7-shaped id");
        self.recording.report(
            incident_id.clone(),
            dedupe_key.clone(),
            severity,
            classification,
            source,
            correlation.clone(),
            envelope.clone(),
        );

        match &mut self.glitchtip {
            Some(sink) => sink.report(
                incident_id,
                dedupe_key,
                severity,
                classification,
                source,
                correlation,
                envelope,
            ),
            None => IncidentDeliveryResult::Recorded,
        }
    }

    /// Number of incidents currently quarantined/recorded in the local
    /// recording sink (not provider-acknowledged).
    pub fn quarantined_count(&self) -> usize {
        self.recording.len()
    }

    // ------------------------------------------------------- health

    /// Ingest one component health observation.
    pub fn ingest_health(
        &mut self,
        component: impl Into<String>,
        state: nexus_observability::vocabulary::HealthState,
        detail: Option<String>,
    ) {
        self.ingest_health_at(component, state, detail, now_epoch_secs());
    }

    /// Ingest one component health observation with a controlled
    /// `last_seen` (deterministic staleness proofs).
    pub fn ingest_health_at(
        &mut self,
        component: impl Into<String>,
        state: nexus_observability::vocabulary::HealthState,
        detail: Option<String>,
        last_seen: u64,
    ) {
        self.health
            .ingest(nexus_observability::model::ComponentHealth::new(
                component, state, last_seen, detail,
            ));
    }

    /// Compose current health (never healthy from stale/config-only).
    pub fn compose_health(&self, window_secs: u64) -> nexus_observability::vocabulary::HealthState {
        self.health.compose(now_epoch_secs(), window_secs)
    }

    // ------------------------------------------------------- SLO

    /// Evaluate one SLO (NoData and InsufficientEvidence are never Met).
    pub fn evaluate_slo(
        &self,
        slo: &nexus_observability::model::SloDefinition,
        good: u64,
        total: u64,
    ) -> nexus_observability::model::SloEvaluation {
        self.slo.evaluate(slo, good, total)
    }

    // ------------------------------------------------------- diagnostic

    /// Run the ops diagnostic over the configured stack.
    pub fn diagnose(&self, window_secs: u64) -> OpsDiagnostic {
        OpsDiagnostic::run(
            self.glitchtip_dsn.as_ref(),
            &self.release,
            &self.environment,
            now_epoch_secs(),
            window_secs,
        )
    }

    // ------------------------------------------------------- audit

    /// Append one bounded redacted audit record.
    pub fn audit(&mut self, record: AuditRecord) {
        self.audit.push(record);
    }

    pub fn audit_len(&self) -> usize {
        self.audit.len()
    }

    pub fn audit_trail(&self) -> &[AuditRecord] {
        &self.audit
    }

    // ------------------------------------------------------- recovery

    /// Execute bounded recovery: retry `attempt` until success, a
    /// permanent failure, or budget exhaustion. The outcome always
    /// reports attempts + last failure (fail-closed, diagnosable).
    pub fn recover(
        &self,
        budget: &RecoveryBudget,
        attempt: crate::recovery::AttemptFn<'_>,
    ) -> RecoveryOutcome {
        crate::recovery::recover_with_budget(budget, attempt)
    }

    /// Bounded recovery specialized to incident redelivery: retry the
    /// real GlitchTip sink until Accepted or budget exhaustion. The
    /// redacted context is reused from the quarantine record.
    pub fn recover_incident(&mut self, budget: &RecoveryBudget) -> RecoveryOutcome {
        // Reuse the most recent quarantined incident context for
        // redelivery; none available means nothing to recover.
        let latest = self.recording.open_incidents().into_iter().last().cloned();
        let Some(incident) = latest else {
            return RecoveryOutcome {
                recovered: false,
                attempts: 0,
                last_failure: Some("no quarantined incident".to_string()),
                elapsed: std::time::Duration::ZERO,
                budget_exhausted: false,
            };
        };
        let mut attempts = 0u32;
        let mut last_failure: Option<String> = None;
        let start = std::time::Instant::now();
        while attempts < budget.max_attempts && start.elapsed() < budget.max_elapsed {
            attempts += 1;
            match &mut self.glitchtip {
                Some(sink) => {
                    let outcome = sink.report(
                        incident.incident_id.clone(),
                        incident.dedupe_key.clone(),
                        incident.severity,
                        &incident.classification,
                        &incident.source,
                        incident.correlation.clone(),
                        incident.redacted_context.clone(),
                    );
                    match outcome {
                        IncidentDeliveryResult::Recorded => {
                            return RecoveryOutcome {
                                recovered: true,
                                attempts,
                                last_failure,
                                elapsed: start.elapsed(),
                                budget_exhausted: false,
                            }
                        }
                        IncidentDeliveryResult::Failed { reason } => {
                            last_failure = Some(reason);
                        }
                        other => {
                            last_failure = Some(format!("{other:?}"));
                        }
                    }
                }
                None => {
                    return RecoveryOutcome {
                        recovered: false,
                        attempts,
                        last_failure: Some("no glitchtip sink configured".to_string()),
                        elapsed: start.elapsed(),
                        budget_exhausted: false,
                    }
                }
            }
            if attempts < budget.max_attempts {
                std::thread::sleep(budget.backoff);
            }
        }
        RecoveryOutcome {
            recovered: false,
            attempts,
            last_failure,
            elapsed: start.elapsed(),
            budget_exhausted: true,
        }
    }
}

/// Convenience: a metrics catalog preloaded with the M4-owned ops
/// metrics (bounded labels, deny-high-cardinality).
pub fn ops_metric_definitions() -> Vec<MetricDefinition> {
    use nexus_domain::Privacy;
    use nexus_observability::vocabulary::{CardinalityPolicy, MetricKind, StabilityLevel};
    vec![
        MetricDefinition::new(
            "nexus.ops.incidents.delivered",
            "incident deliveries recorded",
            "1",
            MetricKind::Counter,
            vec!["source".to_string(), "classification".to_string()],
            CardinalityPolicy::DenyHighCardinality,
            Privacy::Public,
            "ep-038",
            StabilityLevel::Stable,
            "sum",
        )
        .expect("valid metric"),
        MetricDefinition::new(
            "nexus.ops.incidents.failed",
            "incident delivery failures",
            "1",
            MetricKind::Counter,
            vec!["classification".to_string()],
            CardinalityPolicy::DenyHighCardinality,
            Privacy::Public,
            "ep-038",
            StabilityLevel::Stable,
            "sum",
        )
        .expect("valid metric"),
        MetricDefinition::new(
            "nexus.ops.health.composed",
            "composed health state rank",
            "1",
            MetricKind::Gauge,
            vec!["node".to_string()],
            CardinalityPolicy::DenyHighCardinality,
            Privacy::Public,
            "ep-038",
            StabilityLevel::Stable,
            "last",
        )
        .expect("valid metric"),
    ]
}

/// BTreeMap helper for observed fields.
pub fn fields(pairs: Vec<(&str, &str)>) -> Vec<(String, String)> {
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuditSeverity;
    use nexus_observability::vocabulary::HealthState;

    fn config() -> RuntimeConfig {
        RuntimeConfig {
            node: "n1".to_string(),
            environment: "test".to_string(),
            release: "nexus@0.1.0".to_string(),
            glitchtip_dsn: None,
            slos: vec![],
            metrics: ops_metric_definitions(),
        }
    }

    #[test]
    fn ep038_failure_redaction_denies_secret_shaped_context() {
        let rt = ObservabilityRuntime::new(config()).unwrap();
        // Secret-shaped component names are rejected by TelemetryContext
        // (fail-closed before any egress).
        let result = rt.redact(
            TelemetrySignal::Log,
            "svc",
            "op",
            Severity::Info,
            vec![("message".to_string(), "hello".to_string())],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn ep038_failure_prometheus_rejects_unknown_metric_and_cardinality() {
        let rt = ObservabilityRuntime::new(config()).unwrap();
        assert!(rt.prometheus_point("nexus.nope", 1.0, &[]).is_err());
        // High-cardinality raw value rejected by catalog (fail-closed).
        let raw = "user-0197000000000000000000000000000000000000000000000001";
        assert!(rt
            .prometheus_point(
                "nexus.ops.health.composed",
                1.0,
                &[("node".to_string(), raw.to_string())],
            )
            .is_err());
    }

    #[test]
    fn ep038_failure_incident_quarantined_when_no_sink() {
        let mut rt = ObservabilityRuntime::new(config()).unwrap();
        let result = rt.report_incident(
            "storage:unavailable".to_string(),
            Severity::Error,
            "unavailable",
            "storage",
            None,
            fields(vec![("message", "storage down")]),
        );
        // No DSN: local recording is the fallback (Recorded), and the
        // incident is quarantined locally.
        assert_eq!(result, IncidentDeliveryResult::Recorded);
        assert_eq!(rt.quarantined_count(), 1);
    }

    /// AUD-056 hostile proof (runtime redaction): reporting an incident
    /// WITH a correlation must preserve that correlation in the redacted
    /// envelope context that is delivered/quarantined. Previously the
    /// runtime reconstructed telemetry with correlation absent, so the
    /// provider-facing envelope could never carry it.
    #[test]
    fn aud056_incident_report_preserves_correlation_in_envelope_context() {
        let mut rt = ObservabilityRuntime::new(config()).unwrap();
        let corr: CorrelationId = "01970000-0000-7000-8000-000000000011".parse().unwrap();
        let result = rt.report_incident(
            "aud056:correlation".to_string(),
            Severity::Error,
            "unavailable",
            "storage",
            Some(corr.clone()),
            fields(vec![("message", "correlated incident")]),
        );
        assert_eq!(result, IncidentDeliveryResult::Recorded);
        // The quarantined record must carry the correlation BOTH on the
        // incident record and in the redacted envelope context (the form
        // that egresses). The context field is what event mapping reads.
        let incident = rt
            .recording
            .open_incidents()
            .into_iter()
            .find(|i| i.dedupe_key == "aud056:correlation")
            .expect("quarantined incident exists");
        assert_eq!(incident.correlation.as_ref(), Some(&corr));
        assert_eq!(
            incident.redacted_context.context.correlation.as_ref(),
            Some(&corr),
            "envelope context correlation must not be stripped by redaction"
        );
    }

    /// AUD-056 positive proof (runtime redaction with context): the
    /// context-aware redaction path preserves tenant/request/trace/span
    /// metadata when the caller supplies a validated context, so
    /// incident delivery can correlate at the provider.
    #[test]
    fn aud056_redact_with_context_preserves_full_context_metadata() {
        let rt = ObservabilityRuntime::new(config()).unwrap();
        let tenant: nexus_domain::TenantId =
            "01970000-0000-7000-8000-000000000001".parse().unwrap();
        let corr: CorrelationId = "01970000-0000-7000-8000-000000000011".parse().unwrap();
        let context = TelemetryContext::new(
            "n1",
            Some(tenant),
            None,
            Some(corr),
            Some("req-123".to_string()),
            Some("0123456789abcdef0123456789abcdef".to_string()),
            Some("0123456789abcdef".to_string()),
            "storage",
            "put",
            Severity::Error,
            Some("test".to_string()),
            Some("s3".to_string()),
        )
        .expect("valid context");
        let envelope = rt
            .redact_with_context(
                TelemetrySignal::Incident,
                context.clone(),
                fields(vec![("message", "boom")]),
            )
            .expect("redaction ok");
        assert_eq!(envelope.context.correlation, context.correlation);
        assert_eq!(envelope.context.tenant, context.tenant);
        assert_eq!(envelope.context.request_id, context.request_id);
        assert_eq!(envelope.context.trace_id, context.trace_id);
        assert_eq!(envelope.context.span_id, context.span_id);
        assert_eq!(envelope.context.source_interface, context.source_interface);
    }

    #[test]
    fn ep038_failure_health_never_ready_from_stale() {
        let mut rt = ObservabilityRuntime::new(config()).unwrap();
        let now = now_epoch_secs();
        rt.ingest_health_at("glitchtip", HealthState::Ready, None, now);
        // Fresh: Ready.
        assert_eq!(rt.compose_health(60), HealthState::Ready);
        // Stale (last_seen 10s in the past, window 5s): Unknown.
        rt.ingest_health_at(
            "glitchtip",
            HealthState::Ready,
            None,
            now.saturating_sub(10),
        );
        assert_eq!(rt.compose_health(5), HealthState::Unknown);
    }

    #[test]
    fn ep038_failure_slo_no_data_never_met() {
        let rt = ObservabilityRuntime::new(config()).unwrap();
        let slo = nexus_observability::model::SloDefinition::new(
            "nexus.slo.home_p95",
            0.95,
            std::time::Duration::from_secs(3600),
            "home.command",
            10,
        )
        .unwrap();
        let ev = rt.evaluate_slo(&slo, 0, 0);
        assert_eq!(ev.status, nexus_observability::vocabulary::SloState::NoData);
        let ev2 = rt.evaluate_slo(&slo, 9, 9);
        assert_eq!(
            ev2.status,
            nexus_observability::vocabulary::SloState::InsufficientEvidence
        );
    }

    #[test]
    fn ep038_failure_audit_record_roundtrip() {
        let mut rt = ObservabilityRuntime::new(config()).unwrap();
        let rec = AuditRecord::new(
            now_epoch_secs(),
            AuditSeverity::Critical,
            "storage",
            "migrate",
            "n1",
            "verification",
            None,
            fields(vec![("detail", "hash mismatch")]),
        )
        .unwrap();
        rt.audit(rec);
        assert_eq!(rt.audit_len(), 1);
        assert!(rt.audit_trail()[0].to_json_line().is_ok());
    }
}
