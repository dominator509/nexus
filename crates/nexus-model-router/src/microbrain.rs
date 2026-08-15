//! Microbrain seam (SPEC-009 behavior 9; SPEC-025; ADR-022).
//!
//! The Microbrain uses the SAME ReflexProvider contract as DeepSeek and
//! can remain disabled. It begins in shadow, passes frozen and
//! adversarial evals, then canaries low-risk traffic with DeepSeek
//! fallback (SPEC-025). This module owns the provider-neutral port and
//! the shadow/promotion vocabulary; the training factory and promotion
//! gates are later nodes (SPEC-025 pipeline), never a runtime
//! dependency of Nexus V1.

use crate::error::RouterError;
use crate::features::RoutingFeatures;
use crate::vocabulary::{MicrobrainState, ShadowDecisionClass};
use nexus_reflex::{ReflexProvider, ReflexRequest};
use serde::{Deserialize, Serialize};

/// A shadow comparison between the Microbrain and the primary
/// ReflexProvider (SPEC-025 canonical term ShadowDecision).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShadowDecision {
    pub request_id: String,
    pub class: ShadowDecisionClass,
    /// True when the shadow result exactly matches the primary decision.
    pub exact_match: bool,
    /// True when the shadow provider failed (never trusted on failure).
    pub failed: bool,
}

impl ShadowDecision {
    pub fn match_result(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            class: ShadowDecisionClass::Match,
            exact_match: true,
            failed: false,
        }
    }

    pub fn diverge(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            class: ShadowDecisionClass::Diverge,
            exact_match: false,
            failed: false,
        }
    }

    pub fn failed(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            class: ShadowDecisionClass::Failed,
            exact_match: false,
            failed: true,
        }
    }
}

/// Microbrain provider port (SPEC-025).
///
/// The Microbrain uses the SAME ReflexProvider contract (SPEC-009
/// behavior 9) and can remain disabled. A disabled Microbrain never
/// produces a decision and never affects routing.
pub trait MicrobrainProvider: std::fmt::Debug + Send {
    /// The ReflexProvider backing this Microbrain.
    fn provider(&self) -> &dyn ReflexProvider;

    /// Current promotion state.
    fn state(&self) -> MicrobrainState;

    /// Run the Microbrain in shadow against a request; returns None when
    /// disabled or when shadow is not permitted for the features.
    fn shadow(
        &mut self,
        request: &ReflexRequest,
        features: &RoutingFeatures,
    ) -> Result<Option<ShadowDecision>, RouterError>;
}

/// A Microbrain that is always disabled (the safe default). It holds no
/// provider, never runs, and never affects routing.
#[derive(Debug, Default)]
pub struct DisabledMicrobrain;

impl MicrobrainProvider for DisabledMicrobrain {
    fn provider(&self) -> &dyn ReflexProvider {
        unreachable!("disabled microbrain has no provider")
    }

    fn state(&self) -> MicrobrainState {
        MicrobrainState::Disabled
    }

    fn shadow(
        &mut self,
        _request: &ReflexRequest,
        _features: &RoutingFeatures,
    ) -> Result<Option<ShadowDecision>, RouterError> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep015_unit_disabled_microbrain_never_runs() {
        let mut mb = DisabledMicrobrain;
        assert_eq!(mb.state(), MicrobrainState::Disabled);
        let req = ReflexRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            causation_id: None,
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
            effort_input: nexus_reflex::EffortInput::deterministic(),
            segments: vec![],
            cacheable: false,
            budget_ref: None,
            schema_version: "1.0.0".into(),
        };
        let features = RoutingFeatures::new(
            "contacts.query",
            0.2,
            nexus_domain::vocabulary::Privacy::Personal,
            nexus_domain::vocabulary::Risk::R1,
            None,
            0.3,
            500,
            false,
            0.99,
            0.95,
            true,
            None,
        );
        assert_eq!(mb.shadow(&req, &features).unwrap(), None);
    }

    #[test]
    fn ep015_unit_shadow_decision_classes() {
        assert_eq!(
            ShadowDecision::match_result("r-1").class,
            ShadowDecisionClass::Match
        );
        assert!(ShadowDecision::match_result("r-1").exact_match);
        assert_eq!(
            ShadowDecision::diverge("r-1").class,
            ShadowDecisionClass::Diverge
        );
        assert!(ShadowDecision::failed("r-1").failed);
        assert_eq!(
            serde_json::to_value(ShadowDecision::failed("r-1")).unwrap()["class"],
            "FAILED"
        );
    }
}
