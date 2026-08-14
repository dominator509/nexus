//! Model gateway contract (SPEC-009 canonical term ModelGateway;
//! EP-013 node contract `ModelGateway`, `ModelRoute`).

use crate::budget::ModelBudget;
use crate::error::ModelGatewayError;
use crate::model::{ModelRequest, ModelResponse};
use crate::vocabulary::{Escalation, ModelRouteClass};
use serde::{Deserialize, Serialize};

/// A resolved route decision (SPEC-009 canonical term ModelRoute).
///
/// The route is a pure decision record: provider id, route class,
/// effort tier, escalation, and cache hit ratio. It never carries
/// credentials and never grants authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRoute {
    pub provider_id: String,
    pub route_class: ModelRouteClass,
    pub effort_tier: crate::vocabulary::EffortTier,
    pub escalation: Escalation,
    pub cache_hit_ratio: f64,
}

/// Route decision result: either a concrete route or a denied route.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelRouteDecision {
    Routed(ModelRoute),
    Denied(String),
}

/// Model gateway port.
///
/// The gateway is the ONLY composition path for model traffic:
/// Bifrost is preferred but hidden behind this contract; direct
/// providers remain available for replacement and diagnostics.
/// Budgets, retries, rate limits, fallbacks, and usage accounting are
/// consistent across every route. Models never grant authority
/// (SPEC-009 behavior 10).
pub trait ModelGateway {
    /// Route a request and generate a response.
    ///
    /// The gateway checks the budget BEFORE routing, selects a route
    /// (preferring Bifrost when healthy), calls the provider, records
    /// usage, and returns the canonical response. Provider credentials
    /// never leave the gateway.
    fn generate(&mut self, request: &ModelRequest) -> Result<ModelResponse, ModelGatewayError>;

    /// Resolve a route for a request without calling a provider
    /// (used for diagnostics and admission).
    fn route(&self, request: &ModelRequest) -> Result<ModelRouteDecision, ModelGatewayError>;

    /// The budget view for the gateway.
    fn budget(&self) -> &dyn ModelBudget;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::BudgetDecision;
    use crate::budget::BudgetLedger;
    use crate::model::{PromptSegment, PromptSegmentPart, UsageReport};
    use crate::provider::ModelProvider;
    use crate::vocabulary::{EffortTier, ProviderHealthState};

    struct StubProvider;
    impl ModelProvider for StubProvider {
        fn generate(&mut self, request: &ModelRequest) -> Result<ModelResponse, ModelGatewayError> {
            Ok(ModelResponse {
                request_id: request.request_id.clone(),
                correlation_id: request.correlation_id.clone(),
                control_object: crate::model::NexusControlObject {
                    schema_version: "1.0".into(),
                    control: serde_json::json!({"ok": true}),
                    provider: "stub".into(),
                    model: "stub".into(),
                    usage: UsageReport {
                        prompt_tokens: 2,
                        completion_tokens: 1,
                        cache_hit_prompt_tokens: 0,
                    },
                },
            })
        }

        fn health(&self) -> crate::health::ProviderHealth {
            crate::health::ProviderHealth::healthy("stub")
        }

        fn provider_id(&self) -> &str {
            "stub"
        }
    }

    struct GatewayWithLedger {
        ledger: BudgetLedger,
    }
    impl ModelBudget for GatewayWithLedger {
        fn check(&self, request: &ModelRequest) -> Result<BudgetDecision, ModelGatewayError> {
            let _ = request;
            Ok(self.ledger.check(3))
        }

        fn record(
            &mut self,
            request: &ModelRequest,
            usage: &UsageReport,
        ) -> Result<(), ModelGatewayError> {
            let _ = request;
            self.ledger.record(usage.total_tokens())
        }
    }

    struct StubGateway;
    impl ModelGateway for StubGateway {
        fn generate(&mut self, request: &ModelRequest) -> Result<ModelResponse, ModelGatewayError> {
            let mut provider = StubProvider;
            provider.generate(request)
        }

        fn route(&self, request: &ModelRequest) -> Result<ModelRouteDecision, ModelGatewayError> {
            let _ = request;
            Ok(ModelRouteDecision::Routed(ModelRoute {
                provider_id: "stub".into(),
                route_class: ModelRouteClass::Direct,
                effort_tier: EffortTier::Deterministic,
                escalation: Escalation::None,
                cache_hit_ratio: 0.0,
            }))
        }

        fn budget(&self) -> &dyn ModelBudget {
            // The budget view is advisory; this stub returns an empty
            // ledger-backed view for interface completeness.
            struct Empty;
            impl ModelBudget for Empty {
                fn check(
                    &self,
                    _request: &ModelRequest,
                ) -> Result<BudgetDecision, ModelGatewayError> {
                    Ok(BudgetDecision::Allowed)
                }

                fn record(
                    &mut self,
                    _request: &ModelRequest,
                    _usage: &UsageReport,
                ) -> Result<(), ModelGatewayError> {
                    Ok(())
                }
            }
            static EMPTY: Empty = Empty;
            &EMPTY
        }
    }

    fn request() -> ModelRequest {
        ModelRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            causation_id: None,
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
            effort_tier: EffortTier::Deterministic,
            segments: vec![PromptSegmentPart {
                segment: PromptSegment::Constitution,
                content: "constitution".into(),
            }],
            budget_ref: None,
            schema_version: "1.0".into(),
        }
    }

    #[test]
    fn ep013_unit_model_gateway_trait_usable() {
        let mut gateway: Box<dyn ModelGateway> = Box::new(StubGateway);
        let resp = gateway.generate(&request()).unwrap();
        assert_eq!(resp.control_object.provider, "stub");
        match gateway.route(&request()).unwrap() {
            ModelRouteDecision::Routed(route) => {
                assert_eq!(route.route_class, ModelRouteClass::Direct);
                assert_eq!(route.escalation, Escalation::None);
            }
            ModelRouteDecision::Denied(_) => panic!("route must resolve"),
        }
    }

    #[test]
    fn ep013_unit_route_round_trip() {
        let route = ModelRoute {
            provider_id: "bifrost".into(),
            route_class: ModelRouteClass::Fallback,
            effort_tier: EffortTier::High,
            escalation: Escalation::Retry,
            cache_hit_ratio: 0.97,
        };
        let v = serde_json::to_value(&route).unwrap();
        let back: ModelRoute = serde_json::from_value(v).unwrap();
        assert_eq!(back.provider_id, "bifrost");
        assert_eq!(back.route_class, ModelRouteClass::Fallback);
        assert_eq!(back.escalation, Escalation::Retry);
    }

    #[test]
    fn ep013_unit_route_decision_denied() {
        let denied = ModelRouteDecision::Denied("budget exhausted".into());
        let v = serde_json::to_value(&denied).unwrap();
        let back: ModelRouteDecision = serde_json::from_value(v).unwrap();
        assert!(matches!(back, ModelRouteDecision::Denied(_)));
    }

    #[test]
    fn ep013_unit_gateway_ledger_budget_enforced() {
        // The ledger-backed budget denies once the budget is exhausted.
        let mut g = GatewayWithLedger {
            ledger: BudgetLedger::new("b", 3),
        };
        assert_eq!(g.check(&request()).unwrap(), BudgetDecision::Allowed);
        g.record(
            &request(),
            &UsageReport {
                prompt_tokens: 2,
                completion_tokens: 1,
                cache_hit_prompt_tokens: 0,
            },
        )
        .unwrap();
        assert_eq!(g.check(&request()).unwrap(), BudgetDecision::Denied);
        let _ = ProviderHealthState::Healthy;
    }
}
