//! Reflex decision (SPEC-009; ADR-021).
//!
//! A `ReflexDecision` is the provider-neutral outcome of a reflex
//! request. It records whether the decision was produced deterministically
//! (model bypassed) or by a real model, and always carries a validated
//! `NexusControlObject`.

pub use crate::vocabulary::ReflexDecisionClass;
use nexus_model_gateway::model::NexusControlObject;
use serde::{Deserialize, Serialize};

/// Provider-neutral reflex decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflexDecision {
    pub request_id: String,
    pub correlation_id: String,
    pub class: ReflexDecisionClass,
    pub control_object: NexusControlObject,
}

impl ReflexDecision {
    pub fn deterministic(
        request_id: impl Into<String>,
        correlation_id: impl Into<String>,
        control_object: NexusControlObject,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            correlation_id: correlation_id.into(),
            class: ReflexDecisionClass::Deterministic,
            control_object,
        }
    }

    pub fn model(
        request_id: impl Into<String>,
        correlation_id: impl Into<String>,
        control_object: NexusControlObject,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            correlation_id: correlation_id.into(),
            class: ReflexDecisionClass::Model,
            control_object,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_model_gateway::model::UsageReport;

    fn object() -> NexusControlObject {
        NexusControlObject {
            schema_version: "1.0.0".into(),
            control: serde_json::json!({
                "schema_version": "1.0.0",
                "intent": "contacts.query",
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
            provider: "deterministic".into(),
            model: "deterministic".into(),
            usage: UsageReport {
                prompt_tokens: 0,
                completion_tokens: 0,
                cache_hit_prompt_tokens: 0,
            },
        }
    }

    #[test]
    fn ep014_unit_decision_constructors() {
        let d = ReflexDecision::deterministic("r-1", "c-1", object());
        assert_eq!(d.class, ReflexDecisionClass::Deterministic);
        assert_eq!(d.request_id, "r-1");
        let m = ReflexDecision::model("r-2", "c-2", object());
        assert_eq!(m.class, ReflexDecisionClass::Model);
    }

    #[test]
    fn ep014_unit_decision_serde_round_trip() {
        let d = ReflexDecision::model("r-1", "c-1", object());
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["class"], "MODEL");
        assert_eq!(v["control_object"]["provider"], "deterministic");
        let back: ReflexDecision = serde_json::from_value(v).unwrap();
        assert_eq!(back, d);
    }
}
