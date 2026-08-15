//! Canonical router policy configuration (SPEC-009 required test
//! "Router policy table"; EP-015 M2; ADR-022).
//!
//! The policy table lives at `config/models/router/policy.json` and is
//! the canonical source for deterministic route selection thresholds
//! and safety floors. `RouterPolicyConfig::default()` carries the same
//! values in code; the canonical loader reads the artifact and M2 tests
//! prove the artifact and the code agree (config-as-source-of-truth).

use crate::error::RouterError;
use nexus_domain::vocabulary::Route;
use serde::{Deserialize, Serialize};

/// Canonical router policy table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterPolicyConfig {
    pub schema_version: String,
    pub routes: RouterRouteTable,
    pub thresholds: RouterThresholds,
    pub failover: RouterFailoverConfig,
}

/// Canonical route assignments in the policy table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouterRouteTable {
    pub deterministic_route: String,
    pub local_route: String,
    pub default_route: String,
    pub r3_route: String,
    pub secret_route: String,
    pub r4_route: String,
    pub specialist_prefix: String,
}

/// Canonical thresholds in the policy table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterThresholds {
    pub cheap_max_risk: String,
    pub cheap_max_complexity: f64,
    pub cheap_max_cost: f64,
    pub frontier_min_complexity: f64,
    pub frontier_min_availability: f64,
    pub availability_floor: f64,
    pub ambiguity_threshold: f64,
    pub certification_min_success: f64,
    pub ambiguity_min_success: f64,
}

/// Canonical provider failover bounds in the policy table (EP-015 M5;
/// ADR-022; LF-021). The failed primary attempt consumes the configured
/// per-attempt cost and latency budgets; the secondary attempt receives
/// the remaining budgets (never a fresh cap) and the total provider
/// attempts stay within `max_provider_attempts`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterFailoverConfig {
    /// Maximum provider attempts per request (primary + failover).
    pub max_provider_attempts: usize,
    /// Cost-budget units (0..=1) consumed by each provider attempt.
    pub attempt_cost: f64,
    /// Latency-budget milliseconds consumed by each provider attempt.
    pub attempt_latency_ms: u64,
}

impl Default for RouterPolicyConfig {
    /// The canonical defaults (code side). Must equal policy.json.
    fn default() -> Self {
        Self {
            schema_version: "1.0.0".into(),
            routes: RouterRouteTable {
                deterministic_route: "DETERMINISTIC".into(),
                local_route: "REFLEX".into(),
                default_route: "REFLEX".into(),
                r3_route: "FRONTIER_API".into(),
                secret_route: "FRONTIER_API".into(),
                r4_route: "REJECT".into(),
                specialist_prefix: "specialist.".into(),
            },
            thresholds: RouterThresholds {
                cheap_max_risk: "R1".into(),
                cheap_max_complexity: 0.4,
                cheap_max_cost: 0.5,
                frontier_min_complexity: 0.7,
                frontier_min_availability: 0.95,
                availability_floor: 0.5,
                ambiguity_threshold: 0.6,
                certification_min_success: 0.8,
                ambiguity_min_success: 0.5,
            },
            failover: RouterFailoverConfig {
                max_provider_attempts: 2,
                attempt_cost: 0.1,
                attempt_latency_ms: 100,
            },
        }
    }
}

impl RouterPolicyConfig {
    /// Load the canonical policy table from the repository artifact.
    pub fn from_canonical_file() -> Result<Self, RouterError> {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../config/models/router/policy.json");
        let text = std::fs::read_to_string(&path).map_err(|e| {
            RouterError::validation(
                format!("cannot read canonical router policy: {e}"),
                Some("router-policy".into()),
            )
        })?;
        Self::from_json(&text)
    }

    /// Parse and validate a policy table document. Fails closed on
    /// unknown routes or out-of-range thresholds.
    pub fn from_json(text: &str) -> Result<Self, RouterError> {
        let config: RouterPolicyConfig = serde_json::from_str(text).map_err(|e| {
            RouterError::validation(
                format!("router policy invalid JSON: {e}"),
                Some("router-policy".into()),
            )
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Validate every route name against the canonical Route enum and
    /// every threshold to its 0..=1 range.
    pub fn validate(&self) -> Result<(), RouterError> {
        for (name, value) in [
            ("deterministic_route", &self.routes.deterministic_route),
            ("local_route", &self.routes.local_route),
            ("default_route", &self.routes.default_route),
            ("r3_route", &self.routes.r3_route),
            ("secret_route", &self.routes.secret_route),
            ("r4_route", &self.routes.r4_route),
        ] {
            value.parse::<Route>().map_err(|_| {
                RouterError::validation(
                    format!("router policy route {name} unknown: {value}"),
                    Some("router-policy".into()),
                )
            })?;
        }
        for (name, value) in [
            ("cheap_max_complexity", self.thresholds.cheap_max_complexity),
            ("cheap_max_cost", self.thresholds.cheap_max_cost),
            (
                "frontier_min_complexity",
                self.thresholds.frontier_min_complexity,
            ),
            (
                "frontier_min_availability",
                self.thresholds.frontier_min_availability,
            ),
            ("availability_floor", self.thresholds.availability_floor),
            ("ambiguity_threshold", self.thresholds.ambiguity_threshold),
            (
                "certification_min_success",
                self.thresholds.certification_min_success,
            ),
            (
                "ambiguity_min_success",
                self.thresholds.ambiguity_min_success,
            ),
        ] {
            if !(0.0..=1.0).contains(&value) {
                return Err(RouterError::validation(
                    format!("router policy threshold {name} out of range"),
                    Some("router-policy".into()),
                ));
            }
        }
        if self.failover.max_provider_attempts == 0 {
            return Err(RouterError::validation(
                "router policy failover max_provider_attempts must be >= 1",
                Some("router-policy".into()),
            ));
        }
        if !(0.0..=1.0).contains(&self.failover.attempt_cost) {
            return Err(RouterError::validation(
                "router policy failover attempt_cost out of range",
                Some("router-policy".into()),
            ));
        }
        if self.failover.attempt_latency_ms == 0 {
            return Err(RouterError::validation(
                "router policy failover attempt_latency_ms must be >= 1",
                Some("router-policy".into()),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::RouterErrorCode;

    #[test]
    fn ep015_unit_canonical_policy_config_loads() {
        let config = RouterPolicyConfig::from_canonical_file().unwrap();
        assert_eq!(config.schema_version, "1.0.0");
        assert_eq!(config.routes.r4_route, "REJECT");
        assert_eq!(config.routes.secret_route, "FRONTIER_API");
        assert_eq!(config.thresholds.cheap_max_risk, "R1");
    }

    #[test]
    fn ep015_unit_canonical_policy_config_matches_defaults() {
        // Config-as-source-of-truth: the repository artifact and the
        // code defaults must agree.
        let file = RouterPolicyConfig::from_canonical_file().unwrap();
        assert_eq!(file, RouterPolicyConfig::default());
    }

    #[test]
    fn ep015_unit_canonical_policy_config_rejects_unknown_route() {
        let bad = r#"{"schema_version":"1.0.0","routes":{"deterministic_route":"AUTOPILOT","local_route":"REFLEX","default_route":"REFLEX","r3_route":"FRONTIER_API","secret_route":"FRONTIER_API","r4_route":"REJECT","specialist_prefix":"specialist."},"thresholds":{"cheap_max_risk":"R1","cheap_max_complexity":0.4,"cheap_max_cost":0.5,"frontier_min_complexity":0.7,"frontier_min_availability":0.95,"availability_floor":0.5,"ambiguity_threshold":0.6,"certification_min_success":0.8,"ambiguity_min_success":0.5,"failover":{"max_provider_attempts":2,"attempt_cost":0.1,"attempt_latency_ms":100}}"#;
        let err = RouterPolicyConfig::from_json(bad).unwrap_err();
        assert_eq!(err.code, RouterErrorCode::Validation);
    }

    #[test]
    fn ep015_unit_canonical_policy_config_rejects_out_of_range_threshold() {
        let bad = r#"{"schema_version":"1.0.0","routes":{"deterministic_route":"DETERMINISTIC","local_route":"REFLEX","default_route":"REFLEX","r3_route":"FRONTIER_API","secret_route":"FRONTIER_API","r4_route":"REJECT","specialist_prefix":"specialist."},"thresholds":{"cheap_max_risk":"R1","cheap_max_complexity":1.7,"cheap_max_cost":0.5,"frontier_min_complexity":0.7,"frontier_min_availability":0.95,"availability_floor":0.5,"ambiguity_threshold":0.6,"certification_min_success":0.8,"ambiguity_min_success":0.5,"failover":{"max_provider_attempts":2,"attempt_cost":0.1,"attempt_latency_ms":100}}"#;
        assert!(RouterPolicyConfig::from_json(bad).is_err());
    }

    #[test]
    fn ep015_unit_policy_config_serde_round_trip() {
        let config = RouterPolicyConfig::default();
        let v = serde_json::to_value(&config).unwrap();
        let back: RouterPolicyConfig = serde_json::from_value(v).unwrap();
        assert_eq!(back, config);
    }

    #[test]
    fn ep015_unit_failover_config_defaults_match_artifact() {
        // EP-015 M5: the failover bounds come from the canonical policy
        // artifact (config-as-source-of-truth), exactly like the route
        // and threshold tables.
        let file = RouterPolicyConfig::from_canonical_file().unwrap();
        assert_eq!(file.failover, RouterPolicyConfig::default().failover);
        assert_eq!(file.failover.max_provider_attempts, 2);
        assert!((file.failover.attempt_cost - 0.1).abs() < 1e-9);
        assert_eq!(file.failover.attempt_latency_ms, 100);
    }

    #[test]
    fn ep015_unit_failover_config_rejects_zero_attempts() {
        let mut config = RouterPolicyConfig::default();
        config.failover.max_provider_attempts = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn ep015_unit_failover_config_rejects_out_of_range_cost() {
        let mut config = RouterPolicyConfig::default();
        config.failover.attempt_cost = 1.5;
        assert!(config.validate().is_err());
    }
}
