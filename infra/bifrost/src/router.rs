//! Deterministic model router (SPEC-009 required behavior 7; EP-013 M2).
//!
//! Router inputs include domain, complexity, privacy, risk,
//! capability, cost, latency, locality, availability, historical
//! success, certification, and budget. The router produces a
//! deterministic `ModelRouteDecision`: Bifrost is preferred when
//! healthy, with deterministic fallback to direct providers. Models
//! never grant authority; the router only selects a transport path.

use nexus_domain::{Privacy, Risk};
use nexus_model_gateway::{
    ModelRoute, ModelRouteDecision,
    vocabulary::{EffortTier, Escalation, ModelRouteClass},
};
use serde::{Deserialize, Serialize};

/// Router input (SPEC-009 required behavior 7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterInput {
    pub tenant_id: String,
    pub principal_id: String,
    /// Capability domain, e.g. `ai.reflex` (SPEC-003 capability).
    pub domain: String,
    /// Request complexity class.
    pub complexity: Complexity,
    pub privacy: Privacy,
    pub risk: Risk,
    /// Required capability name.
    pub capability: String,
    /// Cost tier preference.
    pub cost: CostTier,
    /// Latency tier preference.
    pub latency: LatencyTier,
    /// Locality preference (SPEC-001).
    pub locality: Locality,
    /// Availability preference.
    pub availability: Availability,
    /// Historical success rate in [0.0, 1.0] (observed).
    pub historical_success: f64,
    /// Provider certification status (from COMPONENT_REGISTRY).
    pub certified: bool,
    /// Remaining budget in tokens (0 means no budget).
    pub budget_remaining: u64,
}

/// Complexity class (SPEC-009 effort tiers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Complexity {
    Trivial,
    Simple,
    Moderate,
    Complex,
}

/// Cost tier preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CostTier {
    Low,
    Medium,
    High,
}

/// Latency tier preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LatencyTier {
    Low,
    Medium,
    High,
}

/// Locality preference (SPEC-001 canonical terms).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Locality {
    Local,
    Regional,
    Global,
}

/// Availability preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Availability {
    Low,
    Medium,
    High,
}

impl RouterInput {
    pub fn new(
        tenant_id: impl Into<String>,
        principal_id: impl Into<String>,
        domain: impl Into<String>,
        capability: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            principal_id: principal_id.into(),
            domain: domain.into(),
            complexity: Complexity::Moderate,
            privacy: Privacy::Household,
            risk: Risk::R1,
            capability: capability.into(),
            cost: CostTier::Medium,
            latency: LatencyTier::Medium,
            locality: Locality::Local,
            availability: Availability::Medium,
            historical_success: 1.0,
            certified: false,
            budget_remaining: 0,
        }
    }

    pub fn with_complexity(mut self, complexity: Complexity) -> Self {
        self.complexity = complexity;
        self
    }

    pub fn with_privacy(mut self, privacy: Privacy) -> Self {
        self.privacy = privacy;
        self
    }

    pub fn with_risk(mut self, risk: Risk) -> Self {
        self.risk = risk;
        self
    }

    pub fn with_cost(mut self, cost: CostTier) -> Self {
        self.cost = cost;
        self
    }

    pub fn with_latency(mut self, latency: LatencyTier) -> Self {
        self.latency = latency;
        self
    }

    pub fn with_locality(mut self, locality: Locality) -> Self {
        self.locality = locality;
        self
    }

    pub fn with_availability(mut self, availability: Availability) -> Self {
        self.availability = availability;
        self
    }

    pub fn with_historical_success(mut self, historical_success: f64) -> Self {
        self.historical_success = historical_success;
        self
    }

    pub fn with_certified(mut self, certified: bool) -> Self {
        self.certified = certified;
        self
    }

    pub fn with_budget_remaining(mut self, budget_remaining: u64) -> Self {
        self.budget_remaining = budget_remaining;
        self
    }
}

/// Deterministic route selection.
///
/// Bifrost is preferred when healthy and certified; the router falls
/// back to direct providers in `fallback_order` when Bifrost is
/// unhealthy, uncertified, or unavailable. Budget exhaustion and
/// certification failures fail closed.
pub struct BifrostRouter {
    /// Provider ids reported healthy by the registry (observed).
    healthy_providers: Vec<String>,
    /// Provider ids certified per COMPONENT_REGISTRY.
    certified_providers: Vec<String>,
    preferred_provider: String,
    fallback_order: Vec<String>,
}

impl BifrostRouter {
    pub fn new(
        healthy_providers: Vec<String>,
        certified_providers: Vec<String>,
        preferred_provider: impl Into<String>,
        fallback_order: Vec<String>,
    ) -> Self {
        Self {
            healthy_providers,
            certified_providers,
            preferred_provider: preferred_provider.into(),
            fallback_order,
        }
    }

    fn is_healthy(&self, provider_id: &str) -> bool {
        self.healthy_providers.iter().any(|p| p == provider_id)
    }

    fn is_certified(&self, provider_id: &str) -> bool {
        self.certified_providers.iter().any(|p| p == provider_id)
    }

    /// Resolve the deterministic route.
    ///
    /// Order of preference:
    /// 1. Bifrost (preferred) when healthy AND certified.
    /// 2. Each direct provider in `fallback_order` when healthy AND
    ///    certified.
    /// 3. Denied when no provider qualifies.
    ///
    /// A provider that is not certified NEVER routes production
    /// traffic (SPEC-009 certification discipline). Budget exhaustion
    /// fails closed before any route is selected.
    pub fn route(&self, input: &RouterInput) -> ModelRouteDecision {
        if input.budget_remaining == 0 {
            return ModelRouteDecision::Denied("budget exhausted".to_string());
        }
        if self.is_healthy(&self.preferred_provider) && self.is_certified(&self.preferred_provider)
        {
            return self.routed(&self.preferred_provider, input, false);
        }
        for provider_id in &self.fallback_order {
            if self.is_healthy(provider_id) && self.is_certified(provider_id) {
                return self.routed(provider_id, input, true);
            }
        }
        ModelRouteDecision::Denied("no healthy certified provider".to_string())
    }

    fn routed(
        &self,
        provider_id: &str,
        input: &RouterInput,
        is_fallback: bool,
    ) -> ModelRouteDecision {
        let route_class = if is_fallback {
            ModelRouteClass::Fallback
        } else {
            ModelRouteClass::Direct
        };
        let escalation = if input.historical_success < 0.9 {
            Escalation::Retry
        } else {
            Escalation::None
        };
        ModelRouteDecision::Routed(ModelRoute {
            provider_id: provider_id.to_string(),
            route_class,
            effort_tier: effort_for_complexity(input.complexity),
            escalation,
            cache_hit_ratio: 0.0,
        })
    }
}

/// Deterministic effort tier mapping (SPEC-009 required behavior 2):
/// MAX is never the default for trivial work.
fn effort_for_complexity(complexity: Complexity) -> EffortTier {
    match complexity {
        Complexity::Trivial | Complexity::Simple => EffortTier::Deterministic,
        Complexity::Moderate => EffortTier::NonThinking,
        Complexity::Complex => EffortTier::High,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> RouterInput {
        RouterInput::new("t-1", "p-1", "ai.reflex", "ai.reflex.query")
            .with_complexity(Complexity::Simple)
            .with_budget_remaining(1000)
    }

    fn router(bifrost_healthy: bool, fallback_healthy: bool) -> BifrostRouter {
        let mut healthy = Vec::new();
        if bifrost_healthy {
            healthy.push("bifrost".to_string());
        }
        if fallback_healthy {
            healthy.push("deepseek-v4-flash".to_string());
        }
        BifrostRouter::new(
            healthy,
            vec!["bifrost".to_string(), "deepseek-v4-flash".to_string()],
            "bifrost",
            vec!["deepseek-v4-flash".to_string()],
        )
    }

    #[test]
    fn ep013_unit_router_prefers_bifrost_when_healthy() {
        let r = router(true, true);
        match r.route(&base_input()).unwrap_routed() {
            Some(route) => {
                assert_eq!(route.provider_id, "bifrost");
                assert_eq!(route.route_class, ModelRouteClass::Direct);
            }
            None => panic!("expected route"),
        }
    }

    #[test]
    fn ep013_unit_router_falls_back_when_bifrost_unhealthy() {
        let r = router(false, true);
        match r.route(&base_input()).unwrap_routed() {
            Some(route) => {
                assert_eq!(route.provider_id, "deepseek-v4-flash");
                assert_eq!(route.route_class, ModelRouteClass::Fallback);
            }
            None => panic!("expected route"),
        }
    }

    #[test]
    fn ep013_unit_router_denies_when_no_provider_healthy() {
        let r = router(false, false);
        assert!(r.route(&base_input()).is_denied());
    }

    #[test]
    fn ep013_unit_router_denies_budget_exhausted() {
        let r = router(true, true);
        let input = base_input().with_budget_remaining(0);
        assert!(r.route(&input).is_denied());
    }

    #[test]
    fn ep013_unit_router_uncertified_provider_never_routes() {
        // Bifrost healthy but NOT certified: the router must not
        // route production traffic to it; the certified fallback is
        // used instead.
        let r = BifrostRouter::new(
            vec!["bifrost".to_string(), "deepseek-v4-flash".to_string()],
            vec!["deepseek-v4-flash".to_string()],
            "bifrost",
            vec!["deepseek-v4-flash".to_string()],
        );
        match r.route(&base_input()).unwrap_routed() {
            Some(route) => {
                assert_eq!(route.provider_id, "deepseek-v4-flash");
                assert_eq!(route.route_class, ModelRouteClass::Fallback);
            }
            None => panic!("expected fallback route"),
        }
    }

    #[test]
    fn ep013_unit_router_deterministic_effort_mapping() {
        let r = router(true, true);
        let trivial = base_input().with_complexity(Complexity::Trivial);
        let complex = base_input().with_complexity(Complexity::Complex);
        let t_dec = r.route(&trivial);
        let c_dec = r.route(&complex);
        let t = t_dec.unwrap_routed().unwrap();
        let c = c_dec.unwrap_routed().unwrap();
        assert_eq!(t.effort_tier, EffortTier::Deterministic);
        assert_eq!(c.effort_tier, EffortTier::High);
        // MAX is never the default for trivial work.
        assert_ne!(t.effort_tier, EffortTier::Max);
    }

    #[test]
    fn ep013_unit_router_deterministic_for_identical_input() {
        let r = router(true, true);
        let a = r.route(&base_input());
        let b = r.route(&base_input());
        assert_eq!(a, b);
    }

    #[test]
    fn ep013_unit_router_escalation_on_low_historical_success() {
        let r = router(true, true);
        let input = base_input().with_historical_success(0.5);
        let decision = r.route(&input);
        let route = decision.unwrap_routed().unwrap();
        assert_eq!(route.escalation, Escalation::Retry);
    }

    trait RouteExt {
        fn unwrap_routed(&self) -> Option<&ModelRoute>;
        fn is_denied(&self) -> bool;
    }

    impl RouteExt for ModelRouteDecision {
        fn unwrap_routed(&self) -> Option<&ModelRoute> {
            match self {
                ModelRouteDecision::Routed(r) => Some(r),
                ModelRouteDecision::Denied(_) => None,
            }
        }

        fn is_denied(&self) -> bool {
            matches!(self, ModelRouteDecision::Denied(_))
        }
    }
}
