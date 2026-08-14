//! Canonical request envelope (directive C/E).
//!
//! One closed envelope schema for every sidecar request. Every field
//! is validated before provider invocation; unknown top-level fields
//! are rejected; duplicate security-sensitive keys are rejected with
//! one documented deterministic behavior across all SDK languages.
//!
//! Duplicate handling policy (directive E): any duplicated top-level
//! key is REJECTED. serde_json alone silently keeps the last value,
//! so the envelope is deserialized with a custom `MapAccess` visitor
//! that records every key it has seen and fails on repeats. This is
//! the same behavior regardless of which language produced the body.

use serde::de::{Error as _, MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

use nexus_connector_sdk::vocabulary::SidecarTransport;

use crate::error::{SidecarError, SidecarErrorKind};
use crate::limits::{validate_correlation_id, validate_idempotency_key, validate_request_id};

/// Envelope schema version (canonical).
pub const ENVELOPE_SCHEMA_VERSION: &str = "1.0";

/// Canonical request operation classes (directive A: class-specific
/// dispatch routing). Wire values are canonical SCREAMING_SNAKE.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RequestOperation {
    Discover,
    Query,
    Command,
    Workflow,
    Health,
    Changefeed,
    Poll,
    Webhook,
}

impl RequestOperation {
    /// Canonical wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discover => "DISCOVER",
            Self::Query => "QUERY",
            Self::Command => "COMMAND",
            Self::Workflow => "WORKFLOW",
            Self::Health => "HEALTH",
            Self::Changefeed => "CHANGEFEED",
            Self::Poll => "POLL",
            Self::Webhook => "WEBHOOK",
        }
    }

    /// Parse a canonical operation; unknown values are rejected.
    pub fn parse(value: &str) -> Result<Self, SidecarError> {
        match value {
            "DISCOVER" => Ok(Self::Discover),
            "QUERY" => Ok(Self::Query),
            "COMMAND" => Ok(Self::Command),
            "WORKFLOW" => Ok(Self::Workflow),
            "HEALTH" => Ok(Self::Health),
            "CHANGEFEED" => Ok(Self::Changefeed),
            "POLL" => Ok(Self::Poll),
            "WEBHOOK" => Ok(Self::Webhook),
            other => Err(SidecarError::validation(
                format!("unknown request operation: {other:?}"),
                None,
            )),
        }
    }
}

/// Strict canonical request envelope (directive C).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestEnvelope {
    /// Protocol version (must equal the canonical version).
    pub protocol_version: String,
    /// Correlation id (validated against log injection, directive V).
    pub correlation_id: String,
    /// Request id (validated).
    pub request_id: String,
    /// Tenant id (must match the bound tenant, directive F).
    pub tenant_id: String,
    /// Connector id (must match the sidecar's connector, directive G).
    pub connector_id: String,
    /// Capability id (must belong to the connector, directive G).
    pub capability_id: String,
    /// Canonical operation.
    pub operation: RequestOperation,
    /// Sidecar transport family (canonical vocabulary).
    pub transport: SidecarTransport,
    /// Idempotency key for commands (directive C).
    pub idempotency_key: Option<String>,
    /// Schema version (must equal the canonical envelope schema).
    pub schema_version: String,
    /// Canonical input payload.
    pub input: serde_json::Value,
}

/// Required envelope fields for validation.
const REQUIRED_FIELDS: [&str; 9] = [
    "protocol_version",
    "correlation_id",
    "request_id",
    "tenant_id",
    "connector_id",
    "capability_id",
    "operation",
    "transport",
    "schema_version",
];

impl RequestEnvelope {
    /// Parse a strict envelope from raw JSON bytes (directive C/E).
    ///
    /// The whole body is parsed with a duplicate-detecting visitor;
    /// missing required fields, unknown top-level fields, duplicate
    /// keys, invalid ids, and unsupported versions are all rejected
    /// before any provider invocation.
    pub fn parse(raw: &[u8]) -> Result<Self, SidecarError> {
        let mut de = serde_json::Deserializer::from_slice(raw);
        let envelope = RequestEnvelope::deserialize(&mut de)
            .map_err(|e| SidecarError::validation(e.to_string(), None))?;
        envelope.validate()?;
        Ok(envelope)
    }

    /// Field-level validation after structural deserialization.
    pub fn validate(&self) -> Result<(), SidecarError> {
        if self.protocol_version != crate::version::PROTOCOL_VERSION {
            return Err(SidecarError::new(
                SidecarErrorKind::ProtocolVersionMismatch,
                format!("unsupported protocol version: {:?}", self.protocol_version),
                Some(self.correlation_id.clone()),
                Some(self.tenant_id.clone()),
                Some(self.connector_id.clone()),
            ));
        }
        if self.schema_version != ENVELOPE_SCHEMA_VERSION {
            return Err(SidecarError::validation(
                format!(
                    "unsupported envelope schema version: {:?}",
                    self.schema_version
                ),
                Some(self.correlation_id.clone()),
            ));
        }
        if !validate_correlation_id(&self.correlation_id) {
            return Err(SidecarError::validation(
                "correlation_id failed validation (length/charset)",
                None,
            ));
        }
        if !validate_request_id(&self.request_id) {
            return Err(SidecarError::validation(
                "request_id failed validation (length/charset)",
                Some(self.correlation_id.clone()),
            ));
        }
        if let Some(key) = &self.idempotency_key {
            if !validate_idempotency_key(key) {
                return Err(SidecarError::validation(
                    "idempotency_key failed validation (length/charset)",
                    Some(self.correlation_id.clone()),
                ));
            }
            if self.operation != RequestOperation::Command {
                return Err(SidecarError::validation(
                    "idempotency_key only valid for COMMAND operations",
                    Some(self.correlation_id.clone()),
                ));
            }
        }
        if self.operation == RequestOperation::Command && self.idempotency_key.is_none() {
            return Err(SidecarError::validation(
                "COMMAND operation requires idempotency_key",
                Some(self.correlation_id.clone()),
            ));
        }
        if self.tenant_id.is_empty()
            || self.connector_id.is_empty()
            || self.capability_id.is_empty()
        {
            return Err(SidecarError::validation(
                "tenant_id, connector_id, and capability_id must not be empty",
                Some(self.correlation_id.clone()),
            ));
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for RequestEnvelope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(EnvelopeVisitor)
    }
}

struct EnvelopeVisitor;

impl<'de> Visitor<'de> for EnvelopeVisitor {
    type Value = RequestEnvelope;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a strict sidecar request envelope object")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut protocol_version: Option<String> = None;
        let mut correlation_id: Option<String> = None;
        let mut request_id: Option<String> = None;
        let mut tenant_id: Option<String> = None;
        let mut connector_id: Option<String> = None;
        let mut capability_id: Option<String> = None;
        let mut operation: Option<RequestOperation> = None;
        let mut transport: Option<SidecarTransport> = None;
        let mut idempotency_key: Option<String> = None;
        let mut schema_version: Option<String> = None;
        let mut input: Option<serde_json::Value> = None;
        let mut seen: Vec<String> = Vec::new();

        while let Some(key) = map.next_key::<String>()? {
            // Duplicate top-level keys are rejected deterministically
            // (directive E). This is the single documented behavior.
            if seen.iter().any(|k| k == &key) {
                return Err(A::Error::custom(format!(
                    "duplicate envelope field rejected: {key}"
                )));
            }
            seen.push(key.clone());
            match key.as_str() {
                "protocol_version" => protocol_version = Some(map.next_value()?),
                "correlation_id" => correlation_id = Some(map.next_value()?),
                "request_id" => request_id = Some(map.next_value()?),
                "tenant_id" => tenant_id = Some(map.next_value()?),
                "connector_id" => connector_id = Some(map.next_value()?),
                "capability_id" => capability_id = Some(map.next_value()?),
                "operation" => operation = Some(map.next_value()?),
                "transport" => transport = Some(map.next_value()?),
                "idempotency_key" => idempotency_key = Some(map.next_value()?),
                "schema_version" => schema_version = Some(map.next_value()?),
                "input" => input = Some(map.next_value()?),
                other => {
                    return Err(A::Error::custom(format!(
                        "unknown top-level envelope field rejected: {other}"
                    )));
                }
            }
        }

        for field in REQUIRED_FIELDS {
            let present = match field {
                "protocol_version" => protocol_version.is_some(),
                "correlation_id" => correlation_id.is_some(),
                "request_id" => request_id.is_some(),
                "tenant_id" => tenant_id.is_some(),
                "connector_id" => connector_id.is_some(),
                "capability_id" => capability_id.is_some(),
                "operation" => operation.is_some(),
                "transport" => transport.is_some(),
                "schema_version" => schema_version.is_some(),
                _ => false,
            };
            if !present {
                return Err(A::Error::custom(format!(
                    "missing required envelope field: {field}"
                )));
            }
        }

        Ok(RequestEnvelope {
            protocol_version: protocol_version.unwrap(),
            correlation_id: correlation_id.unwrap(),
            request_id: request_id.unwrap(),
            tenant_id: tenant_id.unwrap(),
            connector_id: connector_id.unwrap(),
            capability_id: capability_id.unwrap(),
            operation: operation.unwrap(),
            transport: transport.unwrap(),
            idempotency_key,
            schema_version: schema_version.unwrap(),
            input: input.unwrap_or(serde_json::Value::Null),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_body() -> serde_json::Value {
        serde_json::json!({
            "protocol_version": "1",
            "correlation_id": "018f0f6f-9c1e-7b6e-8000-000000000002",
            "request_id": "018f0f6f-9c1e-7b6e-8000-000000000001",
            "tenant_id": "018f0f6f-9c1e-7b6e-8000-000000000003",
            "connector_id": "fixture-connector",
            "capability_id": "fixture.contacts.query",
            "operation": "QUERY",
            "transport": "REST",
            "schema_version": "1.0",
            "input": { "contact_id": "c1" }
        })
    }

    #[test]
    fn ep011_unit_sidecar_envelope_parses_valid() {
        let env = RequestEnvelope::parse(valid_body().to_string().as_bytes()).unwrap();
        assert_eq!(env.operation, RequestOperation::Query);
        assert_eq!(env.transport, SidecarTransport::Rest);
    }

    #[test]
    fn ep011_unit_sidecar_envelope_rejects_duplicate_security_keys() {
        let mut body = valid_body();
        let obj = body.as_object_mut().unwrap();
        obj.insert(
            "tenant_id".to_string(),
            serde_json::json!("018f0f6f-9c1e-7b6e-8000-000000000099"),
        );
        // Insert a duplicate key at the raw JSON level (serde_json
        // Value maps collapse duplicates, so build raw text).
        let raw = r#"{
            "protocol_version": "1",
            "correlation_id": "018f0f6f-9c1e-7b6e-8000-000000000002",
            "request_id": "018f0f6f-9c1e-7b6e-8000-000000000001",
            "tenant_id": "018f0f6f-9c1e-7b6e-8000-000000000003",
            "tenant_id": "018f0f6f-9c1e-7b6e-8000-000000000099",
            "connector_id": "fixture-connector",
            "capability_id": "fixture.contacts.query",
            "operation": "QUERY",
            "transport": "REST",
            "schema_version": "1.0",
            "input": {}
        }"#;
        let err = RequestEnvelope::parse(raw.as_bytes()).unwrap_err();
        assert!(err.message.contains("duplicate envelope field"));
        let _ = body;
    }

    #[test]
    fn ep011_unit_sidecar_envelope_rejects_unknown_top_level_field() {
        let mut body = valid_body();
        body.as_object_mut()
            .unwrap()
            .insert("admin".to_string(), serde_json::json!(true));
        let err = RequestEnvelope::parse(body.to_string().as_bytes()).unwrap_err();
        assert!(err.message.contains("unknown top-level envelope field"));
    }

    #[test]
    fn ep011_unit_sidecar_envelope_rejects_missing_required() {
        let mut body = valid_body();
        body.as_object_mut().unwrap().remove("operation");
        let err = RequestEnvelope::parse(body.to_string().as_bytes()).unwrap_err();
        assert!(err.message.contains("missing required envelope field"));
    }

    #[test]
    fn ep011_unit_sidecar_envelope_rejects_invalid_correlation_injection() {
        let mut body = valid_body();
        body.as_object_mut().unwrap().insert(
            "correlation_id".to_string(),
            serde_json::json!("ok\nX-Injected: true"),
        );
        let err = RequestEnvelope::parse(body.to_string().as_bytes()).unwrap_err();
        assert!(err.message.contains("correlation_id failed validation"));
    }

    #[test]
    fn ep011_unit_sidecar_envelope_command_requires_idempotency() {
        let mut body = valid_body();
        body.as_object_mut()
            .unwrap()
            .insert("operation".to_string(), serde_json::json!("COMMAND"));
        body.as_object_mut().unwrap().insert(
            "capability_id".to_string(),
            serde_json::json!("fixture.contacts.command"),
        );
        let err = RequestEnvelope::parse(body.to_string().as_bytes()).unwrap_err();
        assert!(err.message.contains("requires idempotency_key"));
    }

    #[test]
    fn ep011_unit_sidecar_envelope_rejects_unsupported_schema_version() {
        let mut body = valid_body();
        body.as_object_mut()
            .unwrap()
            .insert("schema_version".to_string(), serde_json::json!("9.9"));
        let err = RequestEnvelope::parse(body.to_string().as_bytes()).unwrap_err();
        assert!(err.message.contains("unsupported envelope schema version"));
    }
}
