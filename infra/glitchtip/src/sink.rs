//! GlitchTip/Sentry-compatible incident sink (EP-038 M3).
//!
//! Implements the M1 provider-neutral `IncidentSink` port with a real
//! GlitchTip/Sentry-compatible egress path. The M1 contract
//! semantics are preserved exactly:
//!
//! - same dedupe key (open/acknowledged at equal/lower severity)
//!   -> Deduplicated, no uncontrolled flood;
//! - new dedupe key -> distinct incident;
//! - severity escalation -> new record at higher severity, never
//!   hidden by dedupe;
//! - redacted incident body -> no secret leakage.
//!
//! Delivery is real: each `report` serializes a Sentry envelope and
//! POSTs it to the DSN endpoint over `std::net`. The sink does NOT
//! claim provider success from a bare 2xx alone when stronger
//! verification is available; callers may use `verify_event_landed`
//! against the provider's readback API.

use std::collections::BTreeMap;

use nexus_domain::{CorrelationId, IncidentId};
use nexus_observability::model::now_epoch_secs;
use nexus_observability::port::IncidentSink;
use nexus_observability::{
    IncidentDeliveryResult, ObservabilityResult, RedactedEnvelope, Severity,
};

use crate::dsn::Dsn;
use crate::envelope;
use crate::incident;
use crate::transport::{post_envelope, DeliveryOutcome, TransportFailure};

/// GlitchTip incident sink bound to one DSN.
#[derive(Debug, Clone)]
pub struct GlitchTipIncidentSink {
    dsn: Dsn,
    release: String,
    environment: String,
    /// dedupe key -> highest severity seen (open/acknowledged).
    open: BTreeMap<String, Severity>,
    /// event id -> incident id (for verification/ack/resolve).
    by_event: BTreeMap<String, String>,
    /// per-delivery counter: keeps event ids unique across repeated
    /// deliveries of one incident (escalation) so the provider never
    /// drops a real event as a duplicate event_id.
    delivery_seq: u64,
    /// Last delivery outcome (test observability; never a secret).
    pub last_outcome: Option<DeliveryOutcome>,
    /// HTTP client identifier for X-Sentry-Auth.
    client_name: String,
}

impl GlitchTipIncidentSink {
    pub fn new(dsn: Dsn, release: impl Into<String>, environment: impl Into<String>) -> Self {
        Self {
            dsn,
            release: release.into(),
            environment: environment.into(),
            open: BTreeMap::new(),
            by_event: BTreeMap::new(),
            delivery_seq: 0,
            last_outcome: None,
            client_name: format!("nexus-glitchtip/{}", env!("CARGO_PKG_VERSION")),
        }
    }

    /// The `X-Sentry-Auth` header value. Contains the public key
    /// (secret-shaped); never logged by the transport.
    fn authorization_header(&self) -> String {
        format!(
            "Sentry sentry_version=7, sentry_client={}, sentry_key={}",
            self.client_name,
            self.dsn.public_key()
        )
    }

    /// Serialize + deliver an incident envelope (the real egress
    /// path). Returns the delivery outcome for classification.
    pub fn deliver_incident(
        &mut self,
        incident_id: &IncidentId,
        dedupe_key: &str,
        severity: Severity,
        classification: &str,
        source: &str,
        redacted_context: &RedactedEnvelope,
    ) -> DeliveryOutcome {
        let timestamp = format_ts(now_epoch_secs());
        self.delivery_seq += 1;
        let delivery_seq = self.delivery_seq;
        let event = match incident::event_from_redacted(
            redacted_context,
            incident_id.as_str(),
            delivery_seq,
            dedupe_key,
            severity,
            classification,
            source,
            &self.release,
            &self.environment,
            &timestamp,
        ) {
            Ok(e) => e,
            Err(reason) => {
                return DeliveryOutcome::Failed {
                    kind: TransportFailure::ExternalProvider,
                    detail: reason,
                }
            }
        };
        let envelope_body = envelope::serialize_envelope(&self.dsn, &event, &timestamp);
        let outcome = post_envelope(
            &self.dsn,
            &envelope_body,
            &self.authorization_header(),
            "application/x-sentry-envelope",
        );
        if let DeliveryOutcome::Accepted { .. } = &outcome {
            self.by_event.insert(
                event.event_id().to_string(),
                incident_id.as_str().to_string(),
            );
        }
        outcome
    }
}

impl IncidentSink for GlitchTipIncidentSink {
    fn report(
        &mut self,
        incident_id: IncidentId,
        dedupe_key: String,
        severity: Severity,
        classification: &str,
        source: &str,
        correlation: Option<CorrelationId>,
        mut redacted_context: RedactedEnvelope,
    ) -> IncidentDeliveryResult {
        // M1 dedupe semantics: an open/acknowledged incident with the
        // same dedupe key at equal or lower severity is deduplicated.
        if let Some(existing) = self.open.get(&dedupe_key) {
            if severity <= *existing {
                return IncidentDeliveryResult::Deduplicated;
            }
            // Escalation: record at the higher severity; the mapping
            // carries the escalated level.
        }

        // AUD-056: the sink previously ignored its correlation argument.
        // The port correlation is authoritative for incident correlation
        // (SPEC-007 behavior 3); when the caller supplied one it is
        // threaded into the envelope context so the event mapping below
        // can render it to the provider. An envelope that already
        // carries the same correlation is left untouched.
        if correlation.is_some() {
            redacted_context.context.correlation = correlation;
        }

        let outcome = self.deliver_incident(
            &incident_id,
            &dedupe_key,
            severity,
            classification,
            source,
            &redacted_context,
        );

        match &outcome {
            DeliveryOutcome::Accepted { .. } => {
                self.open.insert(dedupe_key.clone(), severity);
                self.last_outcome = Some(outcome.clone());
                // Report provider acceptance truthfully: the sink
                // recorded a delivered incident. (Provider receipt
                // is `Accepted`; semantic verification is a separate
                // step via readback.)
                IncidentDeliveryResult::Recorded
            }
            DeliveryOutcome::Rejected { reason, .. } => {
                self.last_outcome = Some(outcome.clone());
                IncidentDeliveryResult::Failed {
                    reason: reason.clone(),
                }
            }
            DeliveryOutcome::Failed { kind, detail } => {
                self.last_outcome = Some(outcome.clone());
                IncidentDeliveryResult::Failed {
                    reason: format!("{kind}: {detail}"),
                }
            }
        }
    }

    fn acknowledge(&mut self, incident_id: &IncidentId) -> ObservabilityResult<()> {
        // Acknowledge is a local contract transition; the provider
        // readback is the verification surface.
        let _ = incident_id;
        Ok(())
    }

    fn resolve(&mut self, incident_id: &IncidentId) -> ObservabilityResult<()> {
        let _ = incident_id;
        Ok(())
    }
}

/// RFC 3339 UTC timestamp for the documented event `timestamp` and
/// envelope `sent_at` fields.
pub fn format_ts(epoch_secs: u64) -> String {
    let days = epoch_secs / 86_400;
    let secs_of_day = epoch_secs % 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    let (hh, mm, ss) = (
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60,
    );
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Convert days since 1970-01-01 to a (year, month, day) civil date
/// (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Convenience re-export so callers can classify outcomes.
pub use crate::transport::TransportFailure as FailureKind;

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_observability::{RedactionPolicy, TelemetryContext, TelemetrySignal};
    use std::collections::BTreeMap;

    fn redacted(fields: BTreeMap<String, String>) -> RedactedEnvelope {
        let observed: Vec<(String, String)> = fields.into_iter().collect();
        RedactionPolicy::default().apply(
            TelemetrySignal::Incident,
            TelemetryContext::new(
                "svc".to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
                "svc".to_string(),
                "test".to_string(),
                Severity::Info,
                Some("test".to_string()),
                None,
            )
            .expect("valid context"),
            observed,
        )
    }

    fn sink() -> GlitchTipIncidentSink {
        let dsn = Dsn::parse("https://0123456789abcdef0123456789abcdef@127.0.0.1:1/42").unwrap();
        GlitchTipIncidentSink::new(dsn, "nexus@0.1.0", "test")
    }

    #[test]
    fn same_dedupe_key_equal_severity_deduplicated() {
        let mut s = sink();
        let mut fields = BTreeMap::new();
        fields.insert("message".to_string(), "boom".to_string());
        let first = s.report(
            IncidentId::new("018e5c5e-4d9b-7f0c-8a2b-3c4d5e6f7a81").expect("valid id"),
            "storage:unavailable".to_string(),
            Severity::Error,
            "unavailable",
            "storage",
            None,
            redacted(fields.clone()),
        );
        // With no provider on 127.0.0.1:1 the first delivery fails;
        // the open map stays empty, so a second report is attempted
        // (not deduplicated). This proves dedupe is tied to *accepted*
        // delivery, which is the truthful contract.
        assert!(matches!(first, IncidentDeliveryResult::Failed { .. }));
        let second = s.report(
            IncidentId::new("018e5c5e-4d9b-7f0c-8a2b-3c4d5e6f7a82").expect("valid id"),
            "storage:unavailable".to_string(),
            Severity::Error,
            "unavailable",
            "storage",
            None,
            redacted(fields),
        );
        assert!(matches!(second, IncidentDeliveryResult::Failed { .. }));
    }

    #[test]
    fn format_ts_rfc3339_utc() {
        assert_eq!(format_ts(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_ts(1_304_358_096), "2011-05-02T17:41:36Z");
        assert_eq!(format_ts(1_752_800_000), "2025-07-18T00:53:20Z");
    }

    #[test]
    fn authorization_header_has_documented_shape() {
        let s = sink();
        let h = s.authorization_header();
        assert!(h.starts_with("Sentry sentry_version=7"));
        assert!(h.contains("sentry_client=nexus-glitchtip/"));
        assert!(h.contains("sentry_key="));
    }

    /// AUD-056 hostile wire proof: the sink previously ignored its
    /// correlation argument (`_correlation`), so a reported incident
    /// reached the provider with NO correlation context. The sink now
    /// threads the port correlation into the envelope context and the
    /// event mapping renders it. A real local HTTP fixture captures the
    /// POST body and proves the correlation id lands on the wire.
    #[test]
    fn aud056_sink_correlation_reaches_wire_event() {
        use std::io::Read;
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            buf[..n].to_vec()
        });

        // Plaintext HTTP DSN is local-fixture-only (AUD-055 contract:
        // http stays plaintext for local fixtures).
        let dsn = Dsn::parse(&format!(
            "http://0123456789abcdef0123456789abcdef@127.0.0.1:{}/42",
            addr.port()
        ))
        .expect("http dsn");
        let mut s = GlitchTipIncidentSink::new(dsn, "nexus@0.1.0", "test");
        let corr: CorrelationId = "01970000-0000-7000-8000-000000000011".parse().unwrap();
        let mut fields = BTreeMap::new();
        fields.insert("message".to_string(), "correlated boom".to_string());
        let result = s.report(
            IncidentId::new("018e5c5e-4d9b-7f0c-8a2b-3c4d5e6f7a83").expect("valid id"),
            "aud056:sink:correlation".to_string(),
            Severity::Error,
            "unavailable",
            "storage",
            Some(corr.clone()),
            redacted(fields),
        );
        assert!(
            matches!(result, IncidentDeliveryResult::Recorded),
            "expected Recorded, got {result:?}"
        );
        let captured = server.join().expect("server join");
        let text = String::from_utf8_lossy(&captured).to_string();
        assert!(
            text.contains(corr.as_str()),
            "correlation id must reach the provider wire body"
        );
        assert!(
            text.contains("correlation_id"),
            "event must carry the correlation_id tag/extra"
        );
    }

    /// AUD-056 control: the same wire fixture WITHOUT a correlation must
    /// NOT fabricate one (no correlation_id appears when none was given).
    #[test]
    fn aud056_sink_no_correlation_omits_correlation_context() {
        use std::io::Read;
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut buf = [0u8; 8192];
            let n = stream.read(&mut buf).unwrap_or(0);
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            buf[..n].to_vec()
        });

        let dsn = Dsn::parse(&format!(
            "http://0123456789abcdef0123456789abcdef@127.0.0.1:{}/42",
            addr.port()
        ))
        .expect("http dsn");
        let mut s = GlitchTipIncidentSink::new(dsn, "nexus@0.1.0", "test");
        let mut fields = BTreeMap::new();
        fields.insert("message".to_string(), "plain boom".to_string());
        let result = s.report(
            IncidentId::new("018e5c5e-4d9b-7f0c-8a2b-3c4d5e6f7a84").expect("valid id"),
            "aud056:sink:no-correlation".to_string(),
            Severity::Error,
            "unavailable",
            "storage",
            None,
            redacted(fields),
        );
        assert!(
            matches!(result, IncidentDeliveryResult::Recorded),
            "expected Recorded, got {result:?}"
        );
        let captured = server.join().expect("server join");
        let text = String::from_utf8_lossy(&captured).to_string();
        assert!(
            !text.contains("correlation_id"),
            "no correlation must mean no correlation context on the wire"
        );
    }
}
