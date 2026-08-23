//! GlitchTip connectivity/verification ladder (EP-038 M3; the
//! SPEC-007 ladder: CONFIGURED != REACHABLE != RESPONDING !=
//! ACCEPTED != VERIFIED).
//!
//! The probe never reports healthy from configuration alone and
//! never prints secrets. It uses the same production transport as
//! the sink so the ladder reflects real provider state.

use nexus_observability::model::now_epoch_secs;

use crate::dsn::Dsn;
use crate::incident;
use crate::transport::{post_envelope, DeliveryOutcome, TransportFailure};

/// Probe ladder result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeState {
    Configured,
    Reachable,
    Responding,
    Accepted,
    Verified,
    Failed {
        kind: TransportFailure,
        detail: String,
    },
}

impl ProbeState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Configured => "CONFIGURED",
            Self::Reachable => "REACHABLE",
            Self::Responding => "RESPONDING",
            Self::Accepted => "ACCEPTED",
            Self::Verified => "VERIFIED",
            Self::Failed { .. } => "FAILED",
        }
    }
}

/// Probe the DSN end-to-end using a synthetic redacted incident.
///
/// The probe envelope carries no raw data (a fixed safe body) and a
/// unique event id, so it cannot be confused with a real incident.
/// `verify_readback` performs an additional provider readback when
/// the readback API is reachable; otherwise the strongest observable
/// state is returned truthfully.
pub fn probe(dsn: &Dsn, release: &str, environment: &str, verify_readback: bool) -> ProbeState {
    if !verify_readback {
        return ProbeState::Configured;
    }

    // Build a safe synthetic envelope. The probe deliberately uses
    // the same low-level transport as the sink (never a fake path).
    let event = incident::event_from_redacted(
        &safe_envelope(),
        "probe",
        0,
        "probe:health",
        nexus_observability::Severity::Info,
        "health",
        "probe",
        release,
        environment,
        &format_ts_probe(),
    );
    let event = match event {
        Ok(e) => e,
        Err(reason) => {
            return ProbeState::Failed {
                kind: TransportFailure::ExternalProvider,
                detail: reason,
            }
        }
    };

    let timestamp = format_ts_probe();
    let body = crate::envelope::serialize_envelope(dsn, &event, &timestamp);
    // The X-Sentry-Auth header carries the DSN public key: GlitchTip
    // 6.1.8 authenticates envelope ingestion from this header (the
    // envelope-body `dsn` field alone is ignored for auth -- verified
    // against the real provider).
    let auth = format!(
        "Sentry sentry_version=7, sentry_client=nexus-glitchtip/{}, sentry_key={}",
        env!("CARGO_PKG_VERSION"),
        dsn.public_key()
    );

    match post_envelope(dsn, &body, &auth, "application/x-sentry-envelope") {
        DeliveryOutcome::Accepted { .. } => {
            if verify_readback {
                // Readback verification is the strongest provider
                // proof; when unavailable we stay at Accepted and
                // never claim Verified.
                match crate::diag::readback_probe(dsn) {
                    Ok(()) => ProbeState::Verified,
                    Err(_) => ProbeState::Accepted,
                }
            } else {
                ProbeState::Accepted
            }
        }
        DeliveryOutcome::Rejected { status, reason } => ProbeState::Failed {
            kind: TransportFailure::ExternalProvider,
            detail: format!("provider rejected probe with HTTP {status}: {reason}"),
        },
        DeliveryOutcome::Failed { kind, detail } => ProbeState::Failed { kind, detail },
    }
}

/// A fixed safe envelope for probes: no raw data, no secrets.
fn safe_envelope() -> nexus_observability::RedactedEnvelope {
    use nexus_observability::{RedactionPolicy, TelemetryContext, TelemetrySignal};
    let observed = vec![
        ("message".to_string(), "probe".to_string()),
        ("component".to_string(), "glitchtip".to_string()),
    ];
    RedactionPolicy::default().apply(
        TelemetrySignal::Incident,
        TelemetryContext::new(
            "nexus-probe".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            "nexus-probe".to_string(),
            "probe".to_string(),
            nexus_observability::Severity::Info,
            Some("probe".to_string()),
            None,
        )
        .expect("valid probe context"),
        observed,
    )
}

/// Probe readback against the provider's issue API.
///
/// This is intentionally conservative: the function does not invent a
/// provider API surface. Callers that can provide a real readback
/// (integration tests with a live GlitchTip) implement it; this
/// default returns an error so `probe` never overclaims.
pub fn readback_probe(_dsn: &Dsn) -> Result<(), String> {
    Err("readback not configured".to_string())
}

fn format_ts_probe() -> String {
    crate::sink::format_ts(now_epoch_secs())
}

/// Secret-safe description of a probe failure.
pub fn describe_failure(state: &ProbeState) -> String {
    match state {
        ProbeState::Failed { kind, detail } => format!("probe failed: {kind}: {detail}"),
        other => format!("probe state: {}", other.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_configured_without_readback() {
        let dsn = Dsn::parse("https://0123456789abcdef0123456789abcdef@127.0.0.1:1/42").unwrap();
        let state = probe(&dsn, "nexus@0.1.0", "test", false);
        assert_eq!(state, ProbeState::Configured);
    }

    #[test]
    fn probe_fails_cleanly_when_unreachable() {
        let dsn = Dsn::parse("https://0123456789abcdef0123456789abcdef@127.0.0.1:1/42").unwrap();
        let state = probe(&dsn, "nexus@0.1.0", "test", true);
        assert!(matches!(
            state,
            ProbeState::Failed {
                kind: TransportFailure::Unavailable,
                ..
            }
        ));
    }

    #[test]
    fn ladder_strings_distinct() {
        assert_ne!(
            ProbeState::Configured.as_str(),
            ProbeState::Reachable.as_str()
        );
        assert_ne!(
            ProbeState::Reachable.as_str(),
            ProbeState::Responding.as_str()
        );
        assert_ne!(
            ProbeState::Responding.as_str(),
            ProbeState::Accepted.as_str()
        );
        assert_ne!(ProbeState::Accepted.as_str(), ProbeState::Verified.as_str());
    }

    #[test]
    fn describe_failure_never_contains_key() {
        let state = ProbeState::Failed {
            kind: TransportFailure::Unavailable,
            detail: "connect refused".to_string(),
        };
        let d = describe_failure(&state);
        assert!(!d.contains("0123456789abcdef0123456789abcdef"));
        assert!(d.contains("Unavailable"));
    }
}
