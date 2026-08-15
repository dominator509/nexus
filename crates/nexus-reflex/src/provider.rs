//! Reflex provider port and request contract (SPEC-009 canonical term
//! ReflexProvider; ADR-021).
//!
//! `ReflexProvider` is the provider-neutral port for the reflex plane.
//! The `DeepSeekFlashProvider` is the V1 primary implementation; Bifrost
//! remains the preferred ModelGateway but this port is replaceable.

use crate::decision::{ReflexDecision, ReflexDecisionClass};
use crate::effort::{EffortInput, EffortPolicy};
use crate::error::ReflexError;
use crate::validator::NexusControlObjectValidator;
use nexus_model_gateway::health::ProviderHealth;
use nexus_model_gateway::model::{NexusControlObject, PromptSegmentPart, UsageReport};
use nexus_model_gateway::vocabulary::EffortTier;

/// A provider-neutral reflex request.
///
/// Carries the AUTHENTICATED tenant and principal context, the effort
/// input used by the deterministic `EffortPolicy`, the ordered prompt
/// segments, and correlation ids. Provider credentials are never part
/// of a request; adapters resolve credentials by reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflexRequest {
    pub request_id: String,
    pub correlation_id: String,
    pub causation_id: Option<String>,
    pub tenant_id: String,
    pub principal_id: String,
    /// Effort inputs consumed by the deterministic `EffortPolicy`.
    pub effort_input: EffortInput,
    /// Ordered prompt segments; the provider assembles them canonically.
    pub segments: Vec<PromptSegmentPart>,
    /// Whether this request belongs to the cacheable corpus (affects
    /// the rolling cache-hit ratio ledger).
    pub cacheable: bool,
    pub budget_ref: Option<String>,
    pub schema_version: String,
}

/// Provider-neutral reflex provider port.
///
/// An implementation resolves a request to a validated
/// `ReflexDecision`. Deterministic tasks (EffortTier::Deterministic)
/// bypass the model entirely. Non-deterministic tasks must go through a
/// real provider and the returned `NexusControlObject` must pass
/// validation before it continues (SPEC-009 behavior 10).
pub trait ReflexProvider {
    /// Resolve a reflex request to a validated decision.
    fn reflex(&mut self, request: &ReflexRequest) -> Result<ReflexDecision, ReflexError>;

    /// Current provider health (observed, never assumed).
    fn health(&self) -> ProviderHealth;

    /// Stable provider id (registry key, e.g. `deepseek-v4-flash`).
    fn provider_id(&self) -> &str;
}

/// DeepSeek V4 Flash ReflexProvider (SPEC-009: V1 primary).
///
/// Deterministic tasks bypass the model: when the effort policy selects
/// `EffortTier::Deterministic`, the provider returns a deterministic
/// `ReflexDecision` without calling any model. Non-deterministic tasks
/// route through a real transport (wired by M3) and every returned
/// control object is validated by `NexusControlObjectValidator`.
///
/// Provider credentials never leave the adapter: the transport is
/// configured with a credential reference, never a value.
#[derive(Debug)]
pub struct DeepSeekFlashProvider {
    provider_id: String,
    effort_policy: EffortPolicy,
    validator: NexusControlObjectValidator,
    health: ProviderHealth,
    /// Real model transport behind the provider-neutral port.
    transport: Option<Box<dyn ReflexTransport>>,
}

/// Transport port for the DeepSeek adapter.
///
/// The transport is the only component that talks to a provider
/// endpoint. It is injected so tests and M3 integration can use the
/// real HTTP path without coupling the provider logic to a vendor SDK.
pub trait ReflexTransport: std::fmt::Debug {
    fn generate(&mut self, request: &ReflexRequest) -> Result<NexusControlObject, ReflexError>;

    fn provider_id(&self) -> &str;
}

impl DeepSeekFlashProvider {
    pub fn new(
        provider_id: impl Into<String>,
        effort_policy: EffortPolicy,
        validator: NexusControlObjectValidator,
        health: ProviderHealth,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            effort_policy,
            validator,
            health,
            transport: None,
        }
    }

    pub fn with_transport(mut self, transport: Box<dyn ReflexTransport>) -> Self {
        self.transport = Some(transport);
        self
    }

    /// The deterministic effort policy owned by this provider.
    pub fn effort_policy(&self) -> &EffortPolicy {
        &self.effort_policy
    }

    /// The control-object validator owned by this provider.
    pub fn validator(&self) -> &NexusControlObjectValidator {
        &self.validator
    }

    fn deterministic_decision(&self, request: &ReflexRequest) -> Result<ReflexDecision, ReflexError> {
        // Deterministic tasks bypass the model (SPEC-009 behavior 1).
        // The decision is built from the request's own fields; no model
        // call occurs. The control object still passes the validator.
        let control = NexusControlObject {
            schema_version: request.schema_version.clone(),
            control: serde_json::json!({
                "schema_version": request.schema_version,
                "intent": "reflex.deterministic",
                "route": "DETERMINISTIC",
                "risk": "R0",
                "privacy": "PUBLIC",
                "ambiguity": 0.0,
                "approval_required": false,
                "executable_instruction": true,
                "confidence": 1.0,
                "required_capabilities": [],
                "entities": {},
            }),
            provider: self.provider_id.clone(),
            model: "deterministic".into(),
            usage: UsageReport {
                prompt_tokens: 0,
                completion_tokens: 0,
                cache_hit_prompt_tokens: 0,
            },
        };
        self.validator.validate(&control)?;
        Ok(ReflexDecision {
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            class: ReflexDecisionClass::Deterministic,
            control_object: control,
        })
    }
}

impl ReflexProvider for DeepSeekFlashProvider {
    fn reflex(&mut self, request: &ReflexRequest) -> Result<ReflexDecision, ReflexError> {
        // Deterministic effort tier bypasses the model entirely.
        if request.effort_input.tier() == EffortTier::Deterministic {
            return self.deterministic_decision(request);
        }

        // Non-deterministic tasks require a real transport.
        let transport = self.transport.as_mut().ok_or_else(|| {
            ReflexError::unavailable(
                "reflex transport not configured",
                Some(self.provider_id.clone()),
            )
        })?;

        let control_object = transport.generate(request)?;

        // Only validated NexusControlObject output continues.
        self.validator.validate(&control_object)?;

        Ok(ReflexDecision {
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            class: ReflexDecisionClass::Model,
            control_object,
        })
    }

    fn health(&self) -> ProviderHealth {
        self.health.clone()
    }

    fn provider_id(&self) -> &str {
        &self.provider_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effort::EffortInput;
    use nexus_model_gateway::vocabulary::ProviderHealthState;

    fn test_request(tier: EffortTier, cacheable: bool) -> ReflexRequest {
        ReflexRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            causation_id: None,
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
            effort_input: EffortInput::new(tier),
            segments: vec![],
            cacheable,
            budget_ref: None,
            schema_version: "1.0.0".into(),
        }
    }

    fn test_provider() -> DeepSeekFlashProvider {
        DeepSeekFlashProvider::new(
            "deepseek-v4-flash",
            EffortPolicy::new(),
            NexusControlObjectValidator::new("1.0.0"),
            ProviderHealth::new(
                "deepseek-v4-flash",
                ProviderHealthState::Unknown,
                None,
                "not yet certified",
                "probe",
            ),
        )
    }

    #[test]
    fn ep014_unit_deepseek_provider_id_is_canonical() {
        let p = test_provider();
        assert_eq!(p.provider_id(), "deepseek-v4-flash");
        assert_eq!(
            p.health().state,
            ProviderHealthState::Unknown
        );
    }

    #[test]
    fn ep014_unit_deterministic_task_bypasses_model_without_transport() {
        // A deterministic request must succeed even though no transport
        // is configured: the model is bypassed entirely.
        let mut p = test_provider();
        let req = test_request(EffortTier::Deterministic, true);
        let decision = p.reflex(&req).unwrap();
        assert_eq!(decision.class, ReflexDecisionClass::Deterministic);
        assert_eq!(decision.control_object.control["route"], "DETERMINISTIC");
        assert_eq!(decision.control_object.usage.prompt_tokens, 0);
    }

    #[test]
    fn ep014_unit_non_deterministic_without_transport_fails_closed() {
        let mut p = test_provider();
        let req = test_request(EffortTier::NonThinking, true);
        let err = p.reflex(&req).unwrap_err();
        assert_eq!(err.code, crate::error::ReflexErrorCode::Unavailable);
    }

    #[test]
    fn ep014_unit_provider_trait_object_is_usable() {
        // The trait is dyn-compatible: a provider-neutral adapter can be
        // stored behind the port.
        let mut provider: Box<dyn ReflexProvider> = Box::new(test_provider());
        let req = test_request(EffortTier::Deterministic, true);
        let decision = provider.reflex(&req).unwrap();
        assert_eq!(decision.class, ReflexDecisionClass::Deterministic);
        assert_eq!(provider.provider_id(), "deepseek-v4-flash");
    }
}
