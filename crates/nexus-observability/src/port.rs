//! Provider-neutral observability port traits (SPEC-007; EP-038 M1).
//!
//! These traits are the stable interface boundary between the
//! observability domain and any future exporter/collector/incident
//! backend. M1 owns only the contracts; no provider adapter exists yet
//! (certification boundary: Prometheus/Grafana/OTel/GlitchTip/incident
//! delivery are NOT asserted until later milestones).
//!
//! Dependency direction: domain model types (this crate) define the
//! ports; infrastructure adapters in later milestones import these
//! ports, never the reverse.

use nexus_domain::{CorrelationId, IncidentId};

use crate::error::ObservabilityResult;
use crate::model::{ComponentHealth, HealthReport, RedactedEnvelope, SloDefinition, SloEvaluation};
use crate::vocabulary::Severity;

/// Canonical metric catalog interface (SPEC-007).
pub trait MetricCatalog {
    fn register(&mut self, definition: crate::model::MetricDefinition) -> ObservabilityResult<()>;
    fn lookup(&self, id: &str) -> Option<&crate::model::MetricDefinition>;
    /// Validate a label value against the definition; rejects unknown
    /// metrics, unknown labels, and high-cardinality values.
    fn validate_label(&self, metric: &str, label: &str, value: &str) -> ObservabilityResult<()>;
}

/// Canonical health aggregation interface (SPEC-007 behavior 4).
pub trait HealthAggregator {
    fn ingest(&mut self, check: ComponentHealth);
    /// Compose health at `now` from observed checks only. Config
    /// existence never produces Ready.
    fn compose(&self, now: u64, window_secs: u64) -> crate::vocabulary::HealthState;
    fn report(&self, subject: &str, now: u64) -> HealthReport;
}

/// Canonical incident sink interface (SPEC-007; provider-neutral).
pub trait IncidentSink {
    #[allow(clippy::too_many_arguments)]
    fn report(
        &mut self,
        incident_id: IncidentId,
        dedupe_key: String,
        severity: Severity,
        classification: &str,
        source: &str,
        correlation: Option<CorrelationId>,
        redacted_context: RedactedEnvelope,
    ) -> crate::model::IncidentDeliveryResult;
    fn acknowledge(&mut self, incident_id: &IncidentId) -> ObservabilityResult<()>;
    fn resolve(&mut self, incident_id: &IncidentId) -> ObservabilityResult<()>;
}

/// Canonical SLO evaluation interface (SPEC-007).
pub trait SloEvaluator {
    fn evaluate(&self, slo: &SloDefinition, good: u64, total: u64) -> SloEvaluation;
}

/// Convenience re-export of the full port surface for adapters.
pub trait PortSurface: MetricCatalog + HealthAggregator + IncidentSink + SloEvaluator {}

impl<T> PortSurface for T where T: MetricCatalog + HealthAggregator + IncidentSink + SloEvaluator {}
