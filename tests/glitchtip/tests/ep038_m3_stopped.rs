//! EP-038 M3 stopped-provider proof.
//!
//! The M3 gate runs this as a SEPARATE cargo invocation AFTER the real
//! GlitchTip fixture has been stopped, exporting
//! `NEXUS_GLITCHTIP_STOPPED_DSN` (the DSN of the now-dead fixture
//! endpoint). The production adapter must classify the refused
//! connection as `Unavailable`.
//!
//! There is NO silent skip in this file: if the stopped-phase env is
//! missing, the test panics loudly. The gate asserts this target ran
//! and passed, so a missing phase invocation fails the gate.

use nexus_glitchtip::{Dsn, GlitchTipIncidentSink};
use nexus_observability::{
    IncidentDeliveryResult, IncidentSink, RedactionPolicy, Severity, TelemetryContext,
    TelemetrySignal,
};

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

fn redacted(fields: Vec<(&str, &str)>) -> nexus_observability::RedactedEnvelope {
    let observed: Vec<(String, String)> = fields
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    RedactionPolicy::default().apply(
        TelemetrySignal::Incident,
        TelemetryContext::new(
            "nexus-glitchtip-stopped".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            "nexus-glitchtip-stopped".to_string(),
            "integration".to_string(),
            Severity::Error,
            Some("test".to_string()),
            None,
        )
        .expect("valid context"),
        observed,
    )
}

fn uuid7() -> String {
    "018e5c5e-4d9b-7f0c-8a2b-0000000f0a01".to_string()
}

/// Stop/unavailable: with the real fixture stopped, the production
/// transport must classify the refused connection as `Unavailable`.
#[test]
fn ep038_integration_stopped_provider_unavailable() {
    let stopped = env("NEXUS_GLITCHTIP_STOPPED_DSN");
    assert!(
        !stopped.is_empty(),
        "stopped-provider phase requires NEXUS_GLITCHTIP_STOPPED_DSN; \
         the gate must export it before invoking this target -- refusing to silently skip"
    );
    let d = Dsn::parse(&stopped).expect("valid stopped DSN");
    let mut sink = GlitchTipIncidentSink::new(d, "nexus@0.1.0", "test");
    let result = sink.report(
        nexus_domain::IncidentId::new(uuid7()).expect("valid id"),
        "it:stopped".to_string(),
        Severity::Error,
        "unavailable",
        "storage",
        None,
        redacted(vec![("message", "stopped provider")]),
    );
    match result {
        IncidentDeliveryResult::Failed { reason } => {
            assert!(
                reason.contains("Unavailable") || reason.contains("refused"),
                "expected Unavailable classification, got: {reason}"
            );
        }
        other => panic!("expected Failed, got {other:?}"),
    }
}
