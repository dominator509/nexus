//! Provider registry contract (EP-013 node contract `ProviderRegistry`).

use crate::error::ModelGatewayError;
use crate::health::ProviderHealth;
use crate::provider::ModelProvider;

/// Provider registry port.
///
/// Bifrost is preferred but hidden behind the `ModelGateway`; direct
/// providers remain available for replacement and diagnostics. The
/// registry holds provider-neutral entries by id; credentials are
/// referenced, never stored here.
pub trait ProviderRegistry {
    /// Register a provider; duplicate id is a conflict.
    fn register(&mut self, provider: Box<dyn ModelProvider>) -> Result<(), ModelGatewayError>;

    /// Fetch a provider by id.
    fn provider(&self, provider_id: &str) -> Result<&dyn ModelProvider, ModelGatewayError>;

    /// Fetch a provider mutably by id (for health probes and calls).
    /// Providers are stored as `'static` boxes, so the mutable borrow
    /// returns the `'static` trait object reborrowed for `'a`.
    fn provider_mut<'a>(
        &'a mut self,
        provider_id: &str,
    ) -> Result<&'a mut (dyn ModelProvider + 'static), ModelGatewayError>;

    /// List registered provider ids (deterministic order).
    fn provider_ids(&self) -> Vec<String>;

    /// Health snapshot for a provider.
    fn health(&self, provider_id: &str) -> Result<ProviderHealth, ModelGatewayError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ModelRequest, ModelResponse, NexusControlObject, UsageReport};
    use crate::vocabulary::ProviderHealthState;

    struct Probe;
    impl ModelProvider for Probe {
        fn generate(&mut self, request: &ModelRequest) -> Result<ModelResponse, ModelGatewayError> {
            Ok(ModelResponse {
                request_id: request.request_id.clone(),
                correlation_id: request.correlation_id.clone(),
                control_object: NexusControlObject {
                    schema_version: "1.0".into(),
                    control: serde_json::json!({"ok": true}),
                    provider: "probe".into(),
                    model: "probe".into(),
                    usage: UsageReport {
                        prompt_tokens: 1,
                        completion_tokens: 1,
                        cache_hit_prompt_tokens: 0,
                    },
                },
            })
        }

        fn health(&self) -> ProviderHealth {
            ProviderHealth::healthy("probe")
        }

        fn provider_id(&self) -> &str {
            "probe"
        }
    }

    #[test]
    fn ep013_unit_provider_registry_trait_object_usable() {
        struct Registry {
            inner: Vec<Box<dyn ModelProvider>>,
        }
        impl ProviderRegistry for Registry {
            fn register(
                &mut self,
                provider: Box<dyn ModelProvider>,
            ) -> Result<(), ModelGatewayError> {
                let id = provider.provider_id().to_string();
                if self.inner.iter().any(|p| p.provider_id() == id) {
                    return Err(ModelGatewayError::conflict(
                        "provider already registered",
                        Some("registry".into()),
                    ));
                }
                self.inner.push(provider);
                Ok(())
            }

            fn provider(&self, provider_id: &str) -> Result<&dyn ModelProvider, ModelGatewayError> {
                self.inner
                    .iter()
                    .find(|p| p.provider_id() == provider_id)
                    .map(|p| p.as_ref())
                    .ok_or_else(|| {
                        ModelGatewayError::not_found("provider not found", Some("registry".into()))
                    })
            }

            fn provider_mut<'a>(
                &'a mut self,
                provider_id: &str,
            ) -> Result<&'a mut (dyn ModelProvider + 'static), ModelGatewayError> {
                self.inner
                    .iter_mut()
                    .find(|p| p.provider_id() == provider_id)
                    .map(|p| p.as_mut())
                    .ok_or_else(|| {
                        ModelGatewayError::not_found("provider not found", Some("registry".into()))
                    })
            }

            fn provider_ids(&self) -> Vec<String> {
                let mut ids: Vec<String> = self
                    .inner
                    .iter()
                    .map(|p| p.provider_id().to_string())
                    .collect();
                ids.sort();
                ids
            }

            fn health(&self, provider_id: &str) -> Result<ProviderHealth, ModelGatewayError> {
                Ok(self.provider(provider_id)?.health())
            }
        }

        let mut registry: Box<dyn ProviderRegistry> = Box::new(Registry { inner: Vec::new() });
        registry.register(Box::new(Probe)).unwrap();
        assert_eq!(registry.provider_ids(), vec!["probe".to_string()]);
        assert_eq!(
            registry.health("probe").unwrap().state,
            ProviderHealthState::Healthy
        );
        assert!(registry.provider("missing").is_err());
        // Duplicate registration is a conflict.
        assert!(registry.register(Box::new(Probe)).is_err());
    }
}
