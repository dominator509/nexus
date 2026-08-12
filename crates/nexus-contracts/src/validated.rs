//! Typed validation layer over generated wire contracts (EP-002 M2).
//!
//! `generated.rs` remains the canonical wire DTO (stringly-typed, generated
//! from `schemas/`). This module converts generated contracts into
//! domain-typed values: typed IDs (nexus-domain) and canonical vocabulary
//! enums. It enforces EP-002 acceptance obligations 1 and 3 at the domain
//! boundary:
//!   1. IDs are typed and non-interchangeable in Rust.
//!   3. Vocabulary tables reject unknown risk, privacy, route, principal,
//!      and capability classes.
//!
//! Only unambiguous mappings are typed. `action_id` and `principal_id`
//! remain opaque strings because a principal may be a person, service,
//! agent, device, or system (PrincipalType), and no canonical ActionId kind
//! exists in SPEC-001's locked ID list.

use std::fmt;
use std::str::FromStr;

use nexus_domain::{
    ApprovalClass, Availability, CapabilityClass, CapabilityId, CorrelationId, EventId,
    Idempotency, Locality, Privacy, Reversal, Risk, Route, TenantId,
};

use crate::generated::{
    ActionRequest, CapabilityDescriptor, EventEnvelope, InvocationContext, NexusControlObject,
};

/// Error from validating a generated contract into typed domain values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A canonical class string was not a known vocabulary value.
    Vocabulary { field: &'static str, value: String },
    /// An identifier string was not a canonical UUIDv7.
    Id { field: &'static str, value: String },
    /// A schema `const` was not the required constant value.
    Const {
        field: &'static str,
        expected: String,
        actual: String,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Vocabulary { field, value } => {
                write!(f, "field {field}: unknown canonical class {value:?}")
            }
            Self::Id { field, value } => {
                write!(f, "field {field}: invalid canonical ID {value:?}")
            }
            Self::Const {
                field,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "field {field}: expected const {expected:?}, got {actual:?}"
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Parse a vocabulary class with a field name for error reporting.
fn vocab<T: FromStr>(field: &'static str, value: &str) -> Result<T, ValidationError> {
    T::from_str(value).map_err(|_| ValidationError::Vocabulary {
        field,
        value: value.to_string(),
    })
}

/// Parse a typed ID with a field name for error reporting.
fn id<'a, T: TryFrom<&'a str, Error = nexus_domain::IdError>>(
    field: &'static str,
    value: &'a str,
) -> Result<T, ValidationError> {
    T::try_from(value).map_err(|_| ValidationError::Id {
        field,
        value: value.to_string(),
    })
}

/// Typed, validated view of an `ActionRequest`.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedActionRequest {
    pub action_id: String,
    pub tenant_id: TenantId,
    pub principal_id: String,
    pub capability_id: CapabilityId,
    pub risk: Risk,
    pub approval_class: ApprovalClass,
    pub reversal: Reversal,
    pub idempotency_key: String,
    pub arguments: serde_json::Value,
    pub expected_state: serde_json::Value,
    pub invocation: InvocationContext,
}

impl TryFrom<&ActionRequest> for ValidatedActionRequest {
    type Error = ValidationError;

    fn try_from(raw: &ActionRequest) -> Result<Self, Self::Error> {
        Ok(Self {
            action_id: raw.action_id.clone(),
            tenant_id: id::<TenantId>("tenantId", &raw.tenant_id)?,
            principal_id: raw.principal_id.clone(),
            capability_id: id::<CapabilityId>("capabilityId", &raw.capability_id)?,
            risk: vocab("risk", &raw.risk)?,
            approval_class: vocab("approvalClass", &raw.approval_class)?,
            reversal: vocab("reversal", &raw.reversal)?,
            idempotency_key: raw.idempotency_key.clone(),
            arguments: raw.arguments.clone(),
            expected_state: raw.expected_state.clone(),
            invocation: raw.invocation.clone(),
        })
    }
}

/// Typed, validated view of a `NexusControlObject`.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedNexusControlObject {
    pub schema_version: String,
    pub intent: String,
    pub route: Route,
    pub risk: Risk,
    pub privacy: Privacy,
    pub ambiguity: f64,
    pub approval_required: bool,
    pub executable_instruction: bool,
    pub confidence: f64,
    pub required_capabilities: Vec<String>,
    pub entities: serde_json::Value,
}

impl TryFrom<&NexusControlObject> for ValidatedNexusControlObject {
    type Error = ValidationError;

    fn try_from(raw: &NexusControlObject) -> Result<Self, Self::Error> {
        // The canonical schema pins schema_version to the const "1.0.0".
        if raw.schema_version != "1.0.0" {
            return Err(ValidationError::Const {
                field: "schema_version",
                expected: "1.0.0".into(),
                actual: raw.schema_version.clone(),
            });
        }
        Ok(Self {
            schema_version: raw.schema_version.clone(),
            intent: raw.intent.clone(),
            route: vocab("route", &raw.route)?,
            risk: vocab("risk", &raw.risk)?,
            privacy: vocab("privacy", &raw.privacy)?,
            ambiguity: raw.ambiguity,
            approval_required: raw.approval_required,
            executable_instruction: raw.executable_instruction,
            confidence: raw.confidence,
            required_capabilities: raw.required_capabilities.clone(),
            entities: raw.entities.clone(),
        })
    }
}

/// Typed, validated view of a `CapabilityDescriptor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedCapabilityDescriptor {
    /// Canonical slug identifier (`^[a-z][a-z0-9_.-]+$` per schema). The
    /// descriptor's own id is NOT a UUID; references to a capability from an
    /// ActionRequest use `CapabilityId` (UUIDv7) instead.
    pub id: String,
    pub class: CapabilityClass,
    pub risk: Risk,
    pub reversal: Reversal,
    pub approval: ApprovalClass,
    pub idempotency: Idempotency,
    pub availability: Availability,
    pub locality: Option<Locality>,
    pub version: String,
    pub input_schema: String,
    pub output_schema: String,
    pub required_scopes: Vec<String>,
}

impl TryFrom<&CapabilityDescriptor> for ValidatedCapabilityDescriptor {
    type Error = ValidationError;

    fn try_from(raw: &CapabilityDescriptor) -> Result<Self, Self::Error> {
        Ok(Self {
            id: raw.id.clone(),
            class: vocab("class", &raw.class)?,
            risk: vocab("risk", &raw.risk)?,
            reversal: vocab("reversal", &raw.reversal)?,
            approval: vocab("approval", &raw.approval)?,
            idempotency: vocab("idempotency", &raw.idempotency)?,
            availability: vocab("availability", &raw.availability)?,
            locality: match &raw.locality {
                Some(l) => Some(vocab("locality", l)?),
                None => None,
            },
            version: raw.version.clone(),
            input_schema: raw.input_schema.clone(),
            output_schema: raw.output_schema.clone(),
            required_scopes: raw.required_scopes.clone(),
        })
    }
}

/// Typed, validated view of an `EventEnvelope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedEventEnvelope {
    pub event_id: EventId,
    pub correlation_id: CorrelationId,
    pub tenant_id: TenantId,
    pub event_type: String,
    pub schema_version: String,
    pub occurred_at: String,
    pub source: String,
    pub subject: String,
    pub payload: serde_json::Value,
}

impl TryFrom<&EventEnvelope> for ValidatedEventEnvelope {
    type Error = ValidationError;

    fn try_from(raw: &EventEnvelope) -> Result<Self, Self::Error> {
        Ok(Self {
            event_id: id("eventId", &raw.event_id)?,
            correlation_id: id("correlationId", &raw.correlation_id)?,
            tenant_id: id("tenantId", &raw.tenant_id)?,
            event_type: raw.event_type.clone(),
            schema_version: raw.schema_version.clone(),
            occurred_at: raw.occurred_at.clone(),
            source: raw.source.clone(),
            subject: raw.subject.clone(),
            payload: raw.payload.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::ActionRequest;

    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6071";
    const CAP: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072";

    fn sample_request() -> ActionRequest {
        ActionRequest {
            action_id: "act_1".into(),
            tenant_id: TENANT.into(),
            principal_id: "user_1".into(),
            capability_id: CAP.into(),
            idempotency_key: "key_1".into(),
            risk: "R3".into(),
            approval_class: "HUMAN".into(),
            reversal: "COMPENSATING".into(),
            arguments: serde_json::json!({"door": "front"}),
            expected_state: serde_json::json!({"locked": true}),
            invocation: sample_invocation(),
        }
    }

    fn sample_invocation() -> InvocationContext {
        InvocationContext {
            request_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073".into(),
            correlation_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6074".into(),
            origin_system: "voice".into(),
            external_actor_id: "user_1".into(),
            external_actor_type: "PERSON".into(),
            channel: Some(Some("voice".into())),
            causation_id: None,
            approval_id: None,
            device_id: None,
            objective_id: None,
            room_id: None,
            task_id: None,
        }
    }

    #[test]
    fn ep002_unit_validated_action_request_parses() {
        let v = ValidatedActionRequest::try_from(&sample_request()).expect("valid request");
        assert_eq!(v.risk, Risk::R3);
        assert_eq!(v.approval_class, ApprovalClass::Human);
        assert_eq!(v.reversal, Reversal::Compensating);
        assert_eq!(v.tenant_id.as_str(), TENANT);
        assert_eq!(v.capability_id.as_str(), CAP);
    }

    #[test]
    fn ep002_unit_validated_rejects_unknown_risk() {
        let mut req = sample_request();
        req.risk = "R9".into();
        assert_eq!(
            ValidatedActionRequest::try_from(&req),
            Err(ValidationError::Vocabulary {
                field: "risk",
                value: "R9".into()
            })
        );
    }

    #[test]
    fn ep002_unit_validated_rejects_unknown_approval() {
        let mut req = sample_request();
        req.approval_class = "MAYBE".into();
        assert!(matches!(
            ValidatedActionRequest::try_from(&req),
            Err(ValidationError::Vocabulary {
                field: "approvalClass",
                ..
            })
        ));
    }

    #[test]
    fn ep002_unit_validated_rejects_bad_tenant_id() {
        let mut req = sample_request();
        req.tenant_id = "tenant-1".into();
        assert_eq!(
            ValidatedActionRequest::try_from(&req),
            Err(ValidationError::Id {
                field: "tenantId",
                value: "tenant-1".into()
            })
        );
    }

    #[test]
    fn ep002_unit_validated_rejects_non_uuidv7_capability() {
        let mut req = sample_request();
        req.capability_id = "0190e1c4-5c8a-1f40-8a1b-2c3d4e5f6072".into(); // version 1
        assert!(matches!(
            ValidatedActionRequest::try_from(&req),
            Err(ValidationError::Id {
                field: "capabilityId",
                ..
            })
        ));
    }

    #[test]
    fn ep002_unit_validated_ids_are_typed_and_distinct() {
        // Compile-time: tenant_id is TenantId, capability_id is CapabilityId.
        let v = ValidatedActionRequest::try_from(&sample_request()).unwrap();
        fn take_tenant(_t: TenantId) -> bool {
            true
        }
        assert!(take_tenant(v.tenant_id.clone()));
        // Same textual value in different kinds must not be comparable:
        // the following line would not compile:
        // assert_eq!(v.tenant_id, v.capability_id);
        assert_ne!(v.tenant_id.as_str(), v.capability_id.as_str());
    }

    #[test]
    fn ep002_unit_validated_control_object_enforces_schema_version_const() {
        use crate::generated::NexusControlObject;

        let base = NexusControlObject {
            schema_version: "1.0.0".into(),
            intent: "home.lights.set".into(),
            route: "DETERMINISTIC".into(),
            risk: "R0".into(),
            privacy: "HOUSEHOLD".into(),
            ambiguity: 0.0,
            approval_required: false,
            executable_instruction: true,
            confidence: 0.99,
            required_capabilities: vec!["home.lights.set".into()],
            entities: serde_json::json!({}),
            escalation_reason: None,
            workflow: None,
        };
        let ok = ValidatedNexusControlObject::try_from(&base);
        assert!(ok.is_ok(), "canonical const must validate");
        assert_eq!(ok.unwrap().schema_version, "1.0.0");

        let mut bad = base;
        bad.schema_version = "2.0.0".into();
        assert_eq!(
            ValidatedNexusControlObject::try_from(&bad),
            Err(ValidationError::Const {
                field: "schema_version",
                expected: "1.0.0".into(),
                actual: "2.0.0".into(),
            })
        );
    }

    #[test]
    fn ep002_unit_validated_capability_descriptor_preserves_class_alias() {
        use crate::generated::CapabilityDescriptor;

        let raw = CapabilityDescriptor {
            id: "cap.lights.set".into(),
            version: "1.0.0".into(),
            class: "COMMAND".into(),
            description: "Set a light's power state".into(),
            input_schema: "/schemas/light-set.json".into(),
            output_schema: "/schemas/light-state.json".into(),
            required_scopes: vec!["home.lights.write".into()],
            risk: "R1".into(),
            approval: "NONE".into(),
            reversal: "COMPENSATING".into(),
            idempotency: "REQUIRED".into(),
            availability: "AVAILABLE".into(),
            data_classes: None,
            event_types: None,
            locality: None,
            provider_id: None,
        };
        let v = ValidatedCapabilityDescriptor::try_from(&raw).expect("valid descriptor");
        assert_eq!(v.class, CapabilityClass::Command);
        // Serialized wire name must be the canonical snake_case `class`,
        // identical to the schema property name.
        let json = serde_json::to_string(&raw).unwrap();
        assert!(json.contains("\"class\":\"COMMAND\""));
        assert!(!json.contains("class_"));
    }
}
