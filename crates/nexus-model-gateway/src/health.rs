//! Provider health contract (SPEC-009 canonical term ProviderHealth).

use crate::vocabulary::ProviderHealthState;
use serde::{Deserialize, Serialize};

/// Provider health snapshot (SPEC-009 canonical term ProviderHealth).
///
/// Health is an observed state with a fingerprint; it never carries
/// credentials. `Healthy`/`Degraded`/`Unhealthy`/`Unknown` are the
/// canonical states (fail closed on unknown).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderHealth {
    pub provider_id: String,
    pub state: ProviderHealthState,
    pub latency_ms: Option<u64>,
    pub message: String,
    /// Fingerprint of the health probe (never the raw probe payload).
    pub probe_fingerprint: String,
}

impl ProviderHealth {
    pub fn new(
        provider_id: impl Into<String>,
        state: ProviderHealthState,
        latency_ms: Option<u64>,
        message: impl Into<String>,
        probe_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            state,
            latency_ms,
            message: message.into(),
            probe_fingerprint: probe_fingerprint.into(),
        }
    }

    pub fn healthy(provider_id: impl Into<String>) -> Self {
        Self::new(
            provider_id,
            ProviderHealthState::Healthy,
            None,
            "ok",
            "probe",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep013_unit_provider_health_round_trip() {
        let h = ProviderHealth::new(
            "bifrost",
            ProviderHealthState::Healthy,
            Some(12),
            "ok",
            "fp-1",
        );
        let v = serde_json::to_value(&h).unwrap();
        let back: ProviderHealth = serde_json::from_value(v).unwrap();
        assert_eq!(back.provider_id, "bifrost");
        assert_eq!(back.state, ProviderHealthState::Healthy);
        assert_eq!(back.latency_ms, Some(12));
        assert_eq!(back.probe_fingerprint, "fp-1");
    }

    #[test]
    fn ep013_unit_provider_health_rejects_unknown_state() {
        assert!(
            serde_json::from_value::<ProviderHealth>(serde_json::json!({
                "provider_id": "x",
                "state": "ON_FIRE",
                "latency_ms": null,
                "message": "boom",
                "probe_fingerprint": "fp"
            }))
            .is_err()
        );
    }

    #[test]
    fn ep013_unit_provider_health_helper() {
        let h = ProviderHealth::healthy("deepseek");
        assert_eq!(h.state, ProviderHealthState::Healthy);
        assert_eq!(h.message, "ok");
    }
}
