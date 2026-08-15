//! NexusControlObject deterministic validator (SPEC-009 behavior 3/10;
//! ADR-021).
//!
//! Every provider returns the same `NexusControlObject` schema;
//! deterministic validation rejects extra or invalid fields. Only
//! validated control objects continue (SPEC-009 acceptance obligation
//! 5). The validator is schema-version aware and fails closed on
//! unknown fields, wrong types, invalid vocabulary, and out-of-range
//! numeric fields.

use crate::error::ReflexError;
use nexus_model_gateway::model::NexusControlObject;
use serde_json::Value;

/// Allowed risk values (SPEC-006; schema nexus-control-object).
const ALLOWED_RISK: [&str; 5] = ["R0", "R1", "R2", "R3", "R4"];

/// Allowed route values (SPEC-009; schema nexus-control-object).
const ALLOWED_ROUTE: [&str; 7] = [
    "DETERMINISTIC",
    "REFLEX",
    "CHEAP_API",
    "FRONTIER_API",
    "SPECIALIST_AGENT",
    "CLARIFY",
    "REJECT",
];

/// Allowed privacy values (SPEC-001; schema nexus-control-object).
const ALLOWED_PRIVACY: [&str; 7] = [
    "PUBLIC",
    "HOUSEHOLD",
    "PERSONAL",
    "SENSITIVE",
    "BUSINESS_CONFIDENTIAL",
    "SECURITY",
    "SECRET",
];

/// The canonical allowed keys inside `control`.
const ALLOWED_CONTROL_KEYS: [&str; 13] = [
    "schema_version",
    "intent",
    "route",
    "risk",
    "privacy",
    "ambiguity",
    "approval_required",
    "executable_instruction",
    "confidence",
    "required_capabilities",
    "entities",
    "escalation_reason",
    "workflow",
];

/// Deterministic validator for `NexusControlObject`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NexusControlObjectValidator {
    schema_version: String,
}

impl NexusControlObjectValidator {
    pub fn new(schema_version: impl Into<String>) -> Self {
        Self {
            schema_version: schema_version.into(),
        }
    }

    /// Validate a control object. Returns Ok only when every field is
    /// canonical; otherwise a typed VALIDATION error.
    pub fn validate(&self, object: &NexusControlObject) -> Result<(), ReflexError> {
        // 1. Envelope schema version must match the validator.
        if object.schema_version != self.schema_version {
            return Err(ReflexError::validation(
                "control object schema_version mismatch",
                Some("nexus-control-object".into()),
            ));
        }

        // 2. Provider and model must be present and non-empty.
        if object.provider.is_empty() || object.model.is_empty() {
            return Err(ReflexError::validation(
                "control object provider/model must be non-empty",
                Some("nexus-control-object".into()),
            ));
        }

        // 3. Control payload must be an object.
        let Value::Object(control) = &object.control else {
            return Err(ReflexError::validation(
                "control must be a JSON object",
                Some("nexus-control-object".into()),
            ));
        };

        // 4. No extra fields (additionalProperties false).
        for key in control.keys() {
            if !ALLOWED_CONTROL_KEYS.contains(&key.as_str()) {
                return Err(ReflexError::validation(
                    format!("control contains unknown field: {key}"),
                    Some("nexus-control-object".into()),
                ));
            }
        }

        // 5. Required fields present and typed.
        let intent = control
            .get("intent")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ReflexError::validation(
                    "control.intent must be a string",
                    Some("nexus-control-object".into()),
                )
            })?;
        if intent.len() < 3 || intent.len() > 128 {
            return Err(ReflexError::validation(
                "control.intent length out of range",
                Some("nexus-control-object".into()),
            ));
        }

        let route = control
            .get("route")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ReflexError::validation(
                    "control.route must be a string",
                    Some("nexus-control-object".into()),
                )
            })?;
        if !ALLOWED_ROUTE.contains(&route) {
            return Err(ReflexError::validation(
                format!("control.route unknown: {route}"),
                Some("nexus-control-object".into()),
            ));
        }

        let risk = control.get("risk").and_then(Value::as_str).ok_or_else(|| {
            ReflexError::validation(
                "control.risk must be a string",
                Some("nexus-control-object".into()),
            )
        })?;
        if !ALLOWED_RISK.contains(&risk) {
            return Err(ReflexError::validation(
                format!("control.risk unknown: {risk}"),
                Some("nexus-control-object".into()),
            ));
        }

        let privacy = control
            .get("privacy")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ReflexError::validation(
                    "control.privacy must be a string",
                    Some("nexus-control-object".into()),
                )
            })?;
        if !ALLOWED_PRIVACY.contains(&privacy) {
            return Err(ReflexError::validation(
                format!("control.privacy unknown: {privacy}"),
                Some("nexus-control-object".into()),
            ));
        }

        // Numeric bounds (schema: 0..=1).
        for field in ["ambiguity", "confidence"] {
            let value = control.get(field).and_then(Value::as_f64).ok_or_else(|| {
                ReflexError::validation(
                    format!("control.{field} must be a number"),
                    Some("nexus-control-object".into()),
                )
            })?;
            if !(0.0..=1.0).contains(&value) {
                return Err(ReflexError::validation(
                    format!("control.{field} out of range"),
                    Some("nexus-control-object".into()),
                ));
            }
        }

        // Booleans.
        for field in ["approval_required", "executable_instruction"] {
            if !control.get(field).and_then(Value::as_bool).is_some() {
                return Err(ReflexError::validation(
                    format!("control.{field} must be a boolean"),
                    Some("nexus-control-object".into()),
                ));
            }
        }

        // required_capabilities must be a non-empty unique string array.
        let caps = control
            .get("required_capabilities")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                ReflexError::validation(
                    "control.required_capabilities must be an array",
                    Some("nexus-control-object".into()),
                )
            })?;
        if caps.len() > 32 {
            return Err(ReflexError::validation(
                "control.required_capabilities exceeds 32 entries",
                Some("nexus-control-object".into()),
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for cap in caps {
            let name = cap.as_str().ok_or_else(|| {
                ReflexError::validation(
                    "control.required_capabilities entries must be strings",
                    Some("nexus-control-object".into()),
                )
            })?;
            if !seen.insert(name.to_string()) {
                return Err(ReflexError::validation(
                    "control.required_capabilities must be unique",
                    Some("nexus-control-object".into()),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_model_gateway::model::UsageReport;

    fn valid_object() -> NexusControlObject {
        NexusControlObject {
            schema_version: "1.0.0".into(),
            control: serde_json::json!({
                "schema_version": "1.0.0",
                "intent": "contacts.query",
                "route": "REFLEX",
                "risk": "R1",
                "privacy": "PERSONAL",
                "ambiguity": 0.2,
                "approval_required": false,
                "executable_instruction": true,
                "confidence": 0.9,
                "required_capabilities": ["contacts.query"],
                "entities": {},
            }),
            provider: "deepseek-v4-flash".into(),
            model: "deepseek-v4-flash".into(),
            usage: UsageReport {
                prompt_tokens: 10,
                completion_tokens: 5,
                cache_hit_prompt_tokens: 0,
            },
        }
    }

    #[test]
    fn ep014_unit_validator_accepts_canonical_object() {
        let validator = NexusControlObjectValidator::new("1.0.0");
        assert!(validator.validate(&valid_object()).is_ok());
    }

    #[test]
    fn ep014_unit_validator_rejects_extra_field() {
        let mut object = valid_object();
        object
            .control
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), Value::Bool(true));
        let validator = NexusControlObjectValidator::new("1.0.0");
        let err = validator.validate(&object).unwrap_err();
        assert_eq!(err.code, crate::error::ReflexErrorCode::Validation);
        assert!(err.message.contains("unknown field"));
    }

    #[test]
    fn ep014_unit_validator_rejects_unknown_risk() {
        let mut object = valid_object();
        object
            .control
            .as_object_mut()
            .unwrap()
            .insert("risk".into(), Value::String("R9".into()));
        let validator = NexusControlObjectValidator::new("1.0.0");
        assert!(validator.validate(&object).is_err());
    }

    #[test]
    fn ep014_unit_validator_rejects_unknown_route() {
        let mut object = valid_object();
        object
            .control
            .as_object_mut()
            .unwrap()
            .insert("route".into(), Value::String("AUTOPILOT".into()));
        let validator = NexusControlObjectValidator::new("1.0.0");
        assert!(validator.validate(&object).is_err());
    }

    #[test]
    fn ep014_unit_validator_rejects_wrong_schema_version() {
        let mut object = valid_object();
        object.schema_version = "2.0.0".into();
        let validator = NexusControlObjectValidator::new("1.0.0");
        assert!(validator.validate(&object).is_err());
    }

    #[test]
    fn ep014_unit_validator_rejects_missing_required_field() {
        let mut object = valid_object();
        object.control.as_object_mut().unwrap().remove("intent");
        let validator = NexusControlObjectValidator::new("1.0.0");
        assert!(validator.validate(&object).is_err());
    }

    #[test]
    fn ep014_unit_validator_rejects_duplicate_capabilities() {
        let mut object = valid_object();
        object.control.as_object_mut().unwrap().insert(
            "required_capabilities".into(),
            Value::Array(vec![
                Value::String("contacts.query".into()),
                Value::String("contacts.query".into()),
            ]),
        );
        let validator = NexusControlObjectValidator::new("1.0.0");
        assert!(validator.validate(&object).is_err());
    }

    #[test]
    fn ep014_unit_validator_rejects_out_of_range_confidence() {
        let mut object = valid_object();
        object
            .control
            .as_object_mut()
            .unwrap()
            .insert("confidence".into(), Value::from(1.5));
        let validator = NexusControlObjectValidator::new("1.0.0");
        assert!(validator.validate(&object).is_err());
    }

    #[test]
    fn ep014_unit_validator_rejects_missing_boolean() {
        let mut object = valid_object();
        object
            .control
            .as_object_mut()
            .unwrap()
            .remove("approval_required");
        let validator = NexusControlObjectValidator::new("1.0.0");
        assert!(validator.validate(&object).is_err());
    }
}
