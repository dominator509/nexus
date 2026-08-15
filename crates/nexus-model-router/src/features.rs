//! Routing features (SPEC-009 required behavior 7; ADR-022).
//!
//! The full canonical router input set: domain, complexity, privacy,
//! risk, capability, cost, latency, locality, availability, historical
//! success, certification, and budget. Typed and deterministic; the
//! RoutePolicy consumes these features and nothing else.

use crate::error::RouterError;
use nexus_domain::vocabulary::{Privacy, Risk};
use serde::{Deserialize, Serialize};

/// Router input features (SPEC-009 behavior 7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingFeatures {
    /// Task domain (e.g. "contacts.query"); stable canonical string.
    pub domain: String,
    /// Task complexity in 0..=1 (0 trivial, 1 maximal).
    pub complexity: f64,
    /// Privacy class (nexus-domain vocabulary).
    pub privacy: Privacy,
    /// Risk class (nexus-domain vocabulary).
    pub risk: Risk,
    /// Required capability when the task targets one (canonical id).
    pub capability: Option<String>,
    /// Relative cost weight in 0..=1 (higher = more expensive provider).
    pub cost: f64,
    /// Latency budget in milliseconds.
    pub latency_ms: u64,
    /// True when the task must be served locally (data locality).
    pub local_only: bool,
    /// Provider availability requirement in 0..=1 (1 = must be highly available).
    pub availability: f64,
    /// Historical success rate in 0..=1 for the selected route/provider.
    pub historical_success: f64,
    /// True when the route requires a certified provider.
    pub requires_certified: bool,
    /// Optional token budget cap.
    pub budget: Option<u64>,
}

impl RoutingFeatures {
    pub fn new(
        domain: impl Into<String>,
        complexity: f64,
        privacy: Privacy,
        risk: Risk,
        capability: Option<String>,
        cost: f64,
        latency_ms: u64,
        local_only: bool,
        availability: f64,
        historical_success: f64,
        requires_certified: bool,
        budget: Option<u64>,
    ) -> Self {
        Self {
            domain: domain.into(),
            complexity,
            privacy,
            risk,
            capability,
            cost,
            latency_ms,
            local_only,
            availability,
            historical_success,
            requires_certified,
            budget,
        }
    }

    /// Validate feature bounds. Fails closed on out-of-range values.
    pub fn validate(&self) -> Result<(), RouterError> {
        for (name, value) in [
            ("complexity", self.complexity),
            ("cost", self.cost),
            ("availability", self.availability),
            ("historical_success", self.historical_success),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(RouterError::validation(
                    format!("routing feature {name} out of range"),
                    Some("routing-features".into()),
                ));
            }
        }
        if self.domain.is_empty() || self.domain.len() > 128 {
            return Err(RouterError::validation(
                "routing feature domain length out of range",
                Some("routing-features".into()),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> RoutingFeatures {
        RoutingFeatures::new(
            "contacts.query",
            0.2,
            Privacy::Personal,
            Risk::R1,
            Some("contacts.query".into()),
            0.3,
            500,
            false,
            0.99,
            0.95,
            true,
            Some(1000),
        )
    }

    #[test]
    fn ep015_unit_features_validate_accepts_canonical() {
        assert!(valid().validate().is_ok());
    }

    #[test]
    fn ep015_unit_features_reject_out_of_range() {
        let mut f = valid();
        f.complexity = 1.5;
        assert!(f.validate().is_err());
        let mut f = valid();
        f.historical_success = -0.1;
        assert!(f.validate().is_err());
    }

    #[test]
    fn ep015_unit_features_reject_empty_domain() {
        let mut f = valid();
        f.domain = String::new();
        assert!(f.validate().is_err());
    }

    #[test]
    fn ep015_unit_features_serde_round_trip() {
        let f = valid();
        let v = serde_json::to_value(&f).unwrap();
        let back: RoutingFeatures = serde_json::from_value(v).unwrap();
        assert_eq!(back, f);
    }
}
