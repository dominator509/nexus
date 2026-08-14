//! Model provider contract (SPEC-009 canonical term ReflexProvider;
//! EP-013 node contract `ModelProvider`).

use crate::error::ModelGatewayError;
use crate::health::ProviderHealth;
use crate::model::{ModelRequest, ModelResponse};

/// Provider-neutral model provider port.
///
/// An implementation calls a real provider (Bifrost, DeepSeek, or an
/// OpenAI-compatible endpoint) and returns the canonical
/// `ModelResponse` envelope. Provider credentials never leave the
/// gateway: adapters resolve credentials by reference and never
/// serialize them into requests or telemetry.
pub trait ModelProvider {
    /// Generate a model response for the request.
    fn generate(&mut self, request: &ModelRequest) -> Result<ModelResponse, ModelGatewayError>;

    /// Current provider health (observed, never assumed).
    fn health(&self) -> ProviderHealth;

    /// Stable provider id (registry key).
    fn provider_id(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{PromptSegment, PromptSegmentPart, UsageReport};
    use crate::vocabulary::EffortTier;
    use nexus_domain::{NexusId, PrincipalType, TenantId};

    #[test]
    fn ep013_unit_model_provider_trait_object_is_usable() {
        // The trait is dyn-compatible: a provider-neutral adapter can
        // be stored behind the port.
        struct StubProvider;
        impl ModelProvider for StubProvider {
            fn generate(
                &mut self,
                _request: &ModelRequest,
            ) -> Result<ModelResponse, ModelGatewayError> {
                Ok(ModelResponse {
                    request_id: _request.request_id.clone(),
                    correlation_id: _request.correlation_id.clone(),
                    control_object: crate::model::NexusControlObject {
                        schema_version: "1.0".into(),
                        control: serde_json::json!({"action": "query"}),
                        provider: "stub".into(),
                        model: "stub".into(),
                        usage: UsageReport {
                            prompt_tokens: 1,
                            completion_tokens: 1,
                            cache_hit_prompt_tokens: 0,
                        },
                    },
                })
            }

            fn health(&self) -> ProviderHealth {
                ProviderHealth::healthy("stub")
            }

            fn provider_id(&self) -> &str {
                "stub"
            }
        }

        let tenant = TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap();
        let principal = NexusId::new("018f0f6f-9c1e-7b6e-8000-00000000000a").unwrap();
        let _ = (
            tenant.as_str(),
            principal.as_str(),
            PrincipalType::Human.as_str(),
        );

        let mut provider: Box<dyn ModelProvider> = Box::new(StubProvider);
        let req = ModelRequest {
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
        };
        let resp = provider.generate(&req).unwrap();
        assert_eq!(resp.request_id, "r-1");
        assert_eq!(
            provider.health().state,
            crate::vocabulary::ProviderHealthState::Healthy
        );
        assert_eq!(provider.provider_id(), "stub");
    }
}
