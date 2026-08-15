//! Routing decision (SPEC-009 canonical term ModelRoute; ADR-022).
//!
//! The provider-neutral outcome of a routing request. Records the
//! selected canonical `Route`, the strategy that produced it, optional
//! escalation, and the deterministic reason. The decision is
//! serializable and versioned; it is advisory control-routing input for
//! the deterministic authority layers, never an authorization.

use crate::vocabulary::{EscalationReason, RouterStrategyClass, RoutingDecisionClass};
use nexus_domain::vocabulary::Route;
use serde::{Deserialize, Serialize};

/// Provider-neutral routing decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub request_id: String,
    pub correlation_id: String,
    pub class: RoutingDecisionClass,
    pub route: Route,
    pub strategy: RouterStrategyClass,
    pub provider_id: Option<String>,
    pub escalation_reason: Option<EscalationReason>,
    /// Deterministic confidence in 0..=1 (policy score, not model confidence).
    pub confidence: f64,
    /// Human-readable deterministic reason.
    pub reason: String,
}

impl RoutingDecision {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: impl Into<String>,
        correlation_id: impl Into<String>,
        class: RoutingDecisionClass,
        route: Route,
        strategy: RouterStrategyClass,
        provider_id: Option<String>,
        escalation_reason: Option<EscalationReason>,
        confidence: f64,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            correlation_id: correlation_id.into(),
            class,
            route,
            strategy,
            provider_id,
            escalation_reason,
            confidence,
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep015_unit_decision_constructors() {
        let d = RoutingDecision::new(
            "r-1",
            "c-1",
            RoutingDecisionClass::Routed,
            Route::CheapApi,
            RouterStrategyClass::Policy,
            Some("deepseek-v4-flash".into()),
            None,
            0.9,
            "low risk low complexity",
        );
        assert_eq!(d.class, RoutingDecisionClass::Routed);
        assert_eq!(d.route, Route::CheapApi);
        assert_eq!(d.strategy, RouterStrategyClass::Policy);
    }

    #[test]
    fn ep015_unit_decision_serde_round_trip() {
        let d = RoutingDecision::new(
            "r-1",
            "c-1",
            RoutingDecisionClass::Escalated,
            Route::Clarify,
            RouterStrategyClass::Policy,
            None,
            Some(EscalationReason::Ambiguity),
            0.4,
            "ambiguous",
        );
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["class"], "ESCALATED");
        assert_eq!(v["route"], "CLARIFY");
        assert_eq!(v["escalation_reason"], "AMBIGUITY");
        let back: RoutingDecision = serde_json::from_value(v).unwrap();
        assert_eq!(back, d);
    }

    #[test]
    fn ep015_unit_decision_rejects_unknown_route() {
        // The canonical Route enum rejects unknown values at parse time.
        assert!("AUTOPILOT".parse::<Route>().is_err());
    }
}
