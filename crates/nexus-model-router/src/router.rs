//! NexusModelRouter port and deterministic implementation (SPEC-009;
//! ADR-022).
//!
//! The router composes the deterministic `RoutePolicy` and
//! `EscalationPolicy` with optional advisory strategies (learned
//! adapter, Microbrain shadow). Learned routing can never override
//! security policy. The default implementation is pure policy routing;
//! a learned router must beat the frozen benchmark before it can
//! replace policy (node contract fallback).

use crate::decision::RoutingDecision;
use crate::error::RouterError;
use crate::escalation::{EscalationOutcome, EscalationPolicy};
use crate::features::RoutingFeatures;
use crate::learned::LearnedRouterAdapter;
use crate::microbrain::MicrobrainProvider;
use crate::policy::RoutePolicy;
use crate::vocabulary::{
    EscalationReason, MicrobrainState, RouterStrategyClass, RoutingDecisionClass,
};
use nexus_domain::vocabulary::Route;

/// Provider-neutral model router port.
pub trait NexusModelRouter {
    /// Route a request deterministically. Returns a validated
    /// `RoutingDecision`; never an unsafe route.
    fn route(
        &mut self,
        request_id: &str,
        correlation_id: &str,
        features: &RoutingFeatures,
    ) -> Result<RoutingDecision, RouterError>;
}

/// Deterministic policy router (V1).
///
/// Strategy precedence: deterministic policy first; a learned adapter
/// may propose, but the policy overrides it for security; the Microbrain
/// is advisory shadow only and is ignored unless it is past promotion
/// gates (never in V1 default). Providers are recorded by health so the
/// router can fail over and escalate on unavailability.
#[derive(Debug)]
pub struct DeterministicModelRouter {
    policy: RoutePolicy,
    escalation: EscalationPolicy,
    learned: Option<Box<dyn LearnedRouterAdapter>>,
    microbrain: Option<Box<dyn MicrobrainProvider>>,
    /// provider_id -> availability (0..=1); empty registry = policy only.
    provider_availability: std::collections::HashMap<String, f64>,
}

impl DeterministicModelRouter {
    pub fn new() -> Self {
        Self {
            policy: RoutePolicy::new(),
            escalation: EscalationPolicy::new(),
            learned: None,
            microbrain: None,
            provider_availability: std::collections::HashMap::new(),
        }
    }

    pub fn with_escalation_policy(mut self, escalation: EscalationPolicy) -> Self {
        self.escalation = escalation;
        self
    }

    pub fn with_learned_adapter(mut self, adapter: Box<dyn LearnedRouterAdapter>) -> Self {
        self.learned = Some(adapter);
        self
    }

    pub fn with_microbrain(mut self, microbrain: Box<dyn MicrobrainProvider>) -> Self {
        self.microbrain = Some(microbrain);
        self
    }

    pub fn with_provider_availability(
        mut self,
        provider_id: impl Into<String>,
        availability: f64,
    ) -> Self {
        self.provider_availability
            .insert(provider_id.into(), availability);
        self
    }
}

impl Default for DeterministicModelRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl NexusModelRouter for DeterministicModelRouter {
    fn route(
        &mut self,
        request_id: &str,
        correlation_id: &str,
        features: &RoutingFeatures,
    ) -> Result<RoutingDecision, RouterError> {
        // 1. Deterministic policy selects the route (safety floors).
        let policy_route = self.policy.select(features)?;

        // 2. Availability floor: if the selected provider is unavailable,
        //    fall back or escalate (never silently route to a dead provider).
        let unavailable = self.provider_availability.values().any(|a| *a < 0.5);
        if unavailable && policy_route != Route::Deterministic {
            return Ok(RoutingDecision::new(
                request_id,
                correlation_id,
                RoutingDecisionClass::Fallback,
                Route::Reflex,
                RouterStrategyClass::Policy,
                None,
                Some(EscalationReason::Unavailable),
                0.5,
                "primary provider unavailable; deterministic fallback",
            ));
        }

        // 3. Escalation check on the policy route.
        match self.escalation.escalate(features, policy_route) {
            EscalationOutcome::Reject(reason) => {
                return Ok(RoutingDecision::new(
                    request_id,
                    correlation_id,
                    RoutingDecisionClass::Rejected,
                    Route::Reject,
                    RouterStrategyClass::Policy,
                    None,
                    Some(reason),
                    0.0,
                    "escalation policy rejected the request",
                ));
            }
            EscalationOutcome::Escalate(reason) => {
                return Ok(RoutingDecision::new(
                    request_id,
                    correlation_id,
                    RoutingDecisionClass::Escalated,
                    policy_route,
                    RouterStrategyClass::Policy,
                    None,
                    Some(reason),
                    0.5,
                    "escalation policy applied",
                ));
            }
            EscalationOutcome::None => {}
        }

        // 4. Learned adapter is advisory: propose, then let policy
        //    override for security.
        if let Some(learned) = self.learned.as_mut() {
            let scores = learned.score(features)?;
            if scores.out_of_distribution {
                return Ok(RoutingDecision::new(
                    request_id,
                    correlation_id,
                    RoutingDecisionClass::Escalated,
                    policy_route,
                    learned.strategy(),
                    None,
                    Some(EscalationReason::OutOfDistribution),
                    0.4,
                    "learned scorer out of distribution; policy route retained",
                ));
            }
            if let Some(best) = scores.best() {
                if let Ok(proposed) = best.route.parse::<Route>() {
                    let final_route = self.policy.override_security(features, proposed)?;
                    if final_route != proposed {
                        return Ok(RoutingDecision::new(
                            request_id,
                            correlation_id,
                            RoutingDecisionClass::Routed,
                            final_route,
                            RouterStrategyClass::Policy,
                            None,
                            Some(EscalationReason::Security),
                            0.6,
                            "learned proposal overridden by security policy",
                        ));
                    }
                    return Ok(RoutingDecision::new(
                        request_id,
                        correlation_id,
                        RoutingDecisionClass::Routed,
                        final_route,
                        learned.strategy(),
                        None,
                        None,
                        scores.best().map(|s| s.score as f64 / 100.0).unwrap_or(0.5),
                        "learned routing accepted",
                    ));
                }
            }
        }

        // 5. Microbrain shadow is advisory and only affects decisions
        //    after promotion gates (never in the V1 default). The
        //    disabled default returns None and never changes routing.
        if let Some(microbrain) = self.microbrain.as_mut() {
            if microbrain.state() == MicrobrainState::Active {
                return Ok(RoutingDecision::new(
                    request_id,
                    correlation_id,
                    RoutingDecisionClass::Shadow,
                    policy_route,
                    RouterStrategyClass::Microbrain,
                    None,
                    None,
                    0.7,
                    "microbrain active; shadow decision recorded",
                ));
            }
        }

        // 6. Default: policy route stands.
        Ok(RoutingDecision::new(
            request_id,
            correlation_id,
            RoutingDecisionClass::Routed,
            policy_route,
            RouterStrategyClass::Policy,
            None,
            None,
            0.8,
            "deterministic policy routing",
        ))
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
    fn ep015_unit_router_routes_deterministically() {
        let mut router = DeterministicModelRouter::new();
        let d = router.route("r-1", "c-1", &features()).unwrap();
        assert_eq!(d.class, RoutingDecisionClass::Routed);
        assert_eq!(d.strategy, RouterStrategyClass::Policy);
        assert_eq!(d.route, Route::CheapApi);
    }

    #[test]
    fn ep015_unit_router_rejects_r4() {
        let mut f = features();
        f.risk = Risk::R4;
        let mut router = DeterministicModelRouter::new();
        let d = router.route("r-1", "c-1", &f).unwrap();
        assert_eq!(d.class, RoutingDecisionClass::Rejected);
        assert_eq!(d.route, Route::Reject);
    }

    #[test]
    fn ep015_unit_router_escalates_on_budget_cap() {
        let mut f = features();
        f.budget = Some(50);
        let mut router = DeterministicModelRouter::new();
        let d = router.route("r-1", "c-1", &f).unwrap();
        assert_eq!(d.class, RoutingDecisionClass::Escalated);
        assert_eq!(d.escalation_reason, Some(EscalationReason::Budget));
    }

    #[test]
    fn ep015_unit_router_falls_back_when_provider_unavailable() {
        let mut f = features();
        f.complexity = 0.2;
        let mut router =
            DeterministicModelRouter::new().with_provider_availability("deepseek-v4-flash", 0.1);
        let d = router.route("r-1", "c-1", &f).unwrap();
        assert_eq!(d.class, RoutingDecisionClass::Fallback);
        assert_eq!(d.escalation_reason, Some(EscalationReason::Unavailable));
    }

    #[test]
    fn ep015_unit_router_accepts_learned_proposal_within_policy() {
        let mut f = features();
        f.privacy = Privacy::Personal;
        f.risk = Risk::R1;
        let adapter = crate::learned::tests_probe::OkAdapter;
        let mut router = DeterministicModelRouter::new().with_learned_adapter(Box::new(adapter));
        let d = router.route("r-1", "c-1", &f).unwrap();
        assert_eq!(d.class, RoutingDecisionClass::Routed);
        assert_eq!(d.strategy, RouterStrategyClass::RouteLlm);
    }

    #[test]
    fn ep015_unit_router_security_override_beats_learned() {
        // Learned proposes CHEAP_API for SECRET privacy; policy overrides.
        let mut f = features();
        f.privacy = Privacy::Secret;
        let adapter = crate::learned::tests_probe::CheapProposingAdapter;
        let mut router = DeterministicModelRouter::new().with_learned_adapter(Box::new(adapter));
        let d = router.route("r-1", "c-1", &f).unwrap();
        assert_eq!(d.strategy, RouterStrategyClass::Policy);
        assert_eq!(d.escalation_reason, Some(EscalationReason::Security));
        assert_eq!(d.route, Route::FrontierApi);
    }

    #[test]
    fn ep015_unit_router_serializes_decision() {
        let mut router = DeterministicModelRouter::new();
        let d = router.route("r-1", "c-1", &features()).unwrap();
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["route"], "CHEAP_API");
        assert_eq!(v["class"], "ROUTED");
    }
}
