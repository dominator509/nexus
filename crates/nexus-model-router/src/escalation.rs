//! Deterministic escalation policy (SPEC-009 canonical term Escalation;
//! ADR-022).
//!
//! Escalation is a pure, deterministic decision over the routing
//! features and the selected route. It never fabricates a route and it
//! fails closed (REJECT/CLARIFY rather than an unsafe route).

use crate::config::RouterPolicyConfig;
use crate::features::RoutingFeatures;
use crate::policy::risk_rank;
use crate::vocabulary::EscalationReason;
use nexus_domain::vocabulary::{Privacy, Route};

/// Outcome of the escalation check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationOutcome {
    /// No escalation; the selected route stands.
    None,
    /// Escalate (e.g. CLARIFY or a higher tier); carry the reason.
    Escalate(EscalationReason),
    /// Reject the request; carry the reason.
    Reject(EscalationReason),
}

/// Deterministic escalation policy.
///
/// Thresholds come from the canonical `RouterPolicyConfig`
/// (`config/models/router/policy.json`; `new()` uses the code defaults
/// that M2 proves equal the artifact).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EscalationPolicy {
    /// Ambiguity threshold above which CLARIFY escalation fires (0..=1).
    pub ambiguity_threshold: f64,
    /// Certification floor: below this historical success, a certified
    /// requirement escalates (0..=1).
    pub certification_min_success: f64,
    /// Ambiguity floor: below this historical success, escalation fires
    /// (0..=1).
    pub ambiguity_min_success: f64,
}

impl EscalationPolicy {
    pub fn new() -> Self {
        Self::from_config(&RouterPolicyConfig::default())
    }

    pub fn from_config(config: &RouterPolicyConfig) -> Self {
        Self {
            ambiguity_threshold: config.thresholds.ambiguity_threshold,
            certification_min_success: config.thresholds.certification_min_success,
            ambiguity_min_success: config.thresholds.ambiguity_min_success,
        }
    }

    pub fn with_ambiguity_threshold(threshold: f64) -> Self {
        let mut policy = Self::new();
        policy.ambiguity_threshold = threshold;
        policy
    }

    /// Deterministic escalation decision for the selected route.
    pub fn escalate(&self, features: &RoutingFeatures, selected: Route) -> EscalationOutcome {
        // R4 is always rejected; no model route can carry it.
        if features.risk == nexus_domain::vocabulary::Risk::R4 {
            return EscalationOutcome::Reject(EscalationReason::Risk);
        }

        // SECRET privacy never travels on a cheap or unverified route.
        if features.privacy == Privacy::Secret && selected == Route::CheapApi {
            return EscalationOutcome::Escalate(EscalationReason::Privacy);
        }

        // High risk on a cheap route escalates.
        if risk_rank(features.risk) >= 3 && selected == Route::CheapApi {
            return EscalationOutcome::Escalate(EscalationReason::Risk);
        }

        // Local-only work must never be routed remotely.
        if features.local_only && matches!(selected, Route::CheapApi | Route::FrontierApi) {
            return EscalationOutcome::Escalate(EscalationReason::Security);
        }

        // Budget cap: a cost-heavy route over the cap escalates.
        if let Some(budget) = features.budget {
            if features.cost * 1000.0 > budget as f64 {
                return EscalationOutcome::Escalate(EscalationReason::Budget);
            }
        }

        // Certification requirement with no certified provider is a
        // hard unavailable/certification escalation.
        if features.requires_certified
            && features.historical_success < self.certification_min_success
        {
            return EscalationOutcome::Escalate(EscalationReason::Certification);
        }

        // Low historical success on the selected route escalates
        // (ambiguity/verification signal).
        if features.historical_success < self.ambiguity_min_success {
            return EscalationOutcome::Escalate(EscalationReason::Ambiguity);
        }

        // High complexity with a weak route escalates.
        if features.complexity >= self.ambiguity_threshold && selected == Route::CheapApi {
            return EscalationOutcome::Escalate(EscalationReason::Ambiguity);
        }

        EscalationOutcome::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::RoutingFeatures;
    use nexus_domain::vocabulary::{Privacy, Risk};

    fn features() -> RoutingFeatures {
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
    fn ep015_unit_escalation_none_for_safe_route() {
        assert_eq!(
            EscalationPolicy::new().escalate(&features(), Route::CheapApi),
            EscalationOutcome::None
        );
    }

    #[test]
    fn ep015_unit_escalation_rejects_r4() {
        let mut f = features();
        f.risk = Risk::R4;
        assert_eq!(
            EscalationPolicy::new().escalate(&f, Route::FrontierApi),
            EscalationOutcome::Reject(EscalationReason::Risk)
        );
    }

    #[test]
    fn ep015_unit_escalation_privacy_on_cheap() {
        let mut f = features();
        f.privacy = Privacy::Secret;
        assert_eq!(
            EscalationPolicy::new().escalate(&f, Route::CheapApi),
            EscalationOutcome::Escalate(EscalationReason::Privacy)
        );
    }

    #[test]
    fn ep015_unit_escalation_ambiguity_on_high_complexity_cheap() {
        let mut f = features();
        f.complexity = 0.8;
        assert_eq!(
            EscalationPolicy::new().escalate(&f, Route::CheapApi),
            EscalationOutcome::Escalate(EscalationReason::Ambiguity)
        );
    }

    #[test]
    fn ep015_unit_escalation_budget_cap() {
        let mut f = features();
        f.budget = Some(50);
        assert_eq!(
            EscalationPolicy::new().escalate(&f, Route::CheapApi),
            EscalationOutcome::Escalate(EscalationReason::Budget)
        );
    }

    #[test]
    fn ep015_unit_escalation_certification_floor() {
        let mut f = features();
        f.historical_success = 0.5;
        assert_eq!(
            EscalationPolicy::new().escalate(&f, Route::CheapApi),
            EscalationOutcome::Escalate(EscalationReason::Certification)
        );
    }

    #[test]
    fn ep015_unit_escalation_local_only_security() {
        let mut f = features();
        f.local_only = true;
        assert_eq!(
            EscalationPolicy::new().escalate(&f, Route::FrontierApi),
            EscalationOutcome::Escalate(EscalationReason::Security)
        );
    }
}
