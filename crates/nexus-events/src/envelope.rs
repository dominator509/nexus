//! Canonical event envelope (SPEC-023 behavior 3).
//!
//! Every event carries: event ID, type, schema version, source, subject,
//! time, tenant, actor reference, correlation, causation, data class, and
//! payload. Field names and enum wire values are snake_case and mirror the
//! canonical `schemas/event-envelope.schema.json` (created in EP-005 M3).

use std::fmt;
use std::str::FromStr;

use nexus_domain::{CorrelationId, EventId, TenantId};
use serde::{Deserialize, Serialize};

use crate::error::{EventError, EventErrorCode};

/// Canonical schema version of the envelope wire model.
pub const EVENT_SCHEMA_VERSION: &str = "1.0.0";

/// Event type (SPEC-023 behavior 3).
///
/// A dotted lowercase slug identifying the domain event, e.g.
/// `memory.record.created`. Rejects uppercase, whitespace, and
/// non-ASCII characters at parse time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventType(String);

impl EventType {
    /// Validate and construct an event type slug.
    pub fn new(s: impl Into<String>) -> Result<Self, EventError> {
        let s = s.into();
        let valid = !s.is_empty()
            && s.len() <= 128
            && s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.');
        if !valid {
            return Err(EventError::new(
                EventErrorCode::Validation,
                format!("invalid event type: {s:?}"),
            ));
        }
        Ok(Self(s))
    }

    /// The canonical wire string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for EventType {
    type Err = EventError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// Data classification of an event (SPEC-023 behavior 3, SPEC-020).
///
/// Wire values match the canonical privacy ladder so event filtering and
/// redaction reuse the same policy classes as memory records (INV-014).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EventDataClass {
    /// Visible to any authenticated caller within the tenant.
    Public,
    /// Household-private.
    Household,
    /// Personal to one principal.
    Personal,
    /// Requires explicit purpose-limited access.
    Sensitive,
    /// Business-confidential.
    BusinessConfidential,
    /// Security-relevant (alerts, audit, trust).
    Security,
    /// Secret; never in prompts or logs.
    Secret,
}

impl EventDataClass {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "PUBLIC",
            Self::Household => "HOUSEHOLD",
            Self::Personal => "PERSONAL",
            Self::Sensitive => "SENSITIVE",
            Self::BusinessConfidential => "BUSINESS_CONFIDENTIAL",
            Self::Security => "SECURITY",
            Self::Secret => "SECRET",
        }
    }
}

impl fmt::Display for EventDataClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EventDataClass {
    type Err = EventError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PUBLIC" => Ok(Self::Public),
            "HOUSEHOLD" => Ok(Self::Household),
            "PERSONAL" => Ok(Self::Personal),
            "SENSITIVE" => Ok(Self::Sensitive),
            "BUSINESS_CONFIDENTIAL" => Ok(Self::BusinessConfidential),
            "SECURITY" => Ok(Self::Security),
            "SECRET" => Ok(Self::Secret),
            other => Err(EventError::new(
                EventErrorCode::Validation,
                format!("unknown event data class: {other}"),
            )),
        }
    }
}

/// Canonical event envelope (SPEC-023 behavior 3).
///
/// `additionalProperties: false` on the wire: unknown fields are rejected
/// during deserialization, matching the canonical schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventEnvelope {
    /// Unique event identifier.
    pub event_id: EventId,
    /// Event type slug (e.g. `memory.record.created`).
    pub event_type: EventType,
    /// Schema version of this envelope (constant `1.0.0` today).
    pub schema_version: String,
    /// Provenance source (channel, provider, workflow).
    pub source: String,
    /// Routing subject (canonical subject namespace).
    pub subject: String,
    /// RFC 3339 UTC event time.
    pub time: String,
    /// Tenant boundary (INV-005).
    pub tenant_id: TenantId,
    /// Actor reference (principal or system).
    pub actor: String,
    /// Correlation reference linking a logical operation.
    pub correlation_id: CorrelationId,
    /// Causation: the event that caused this one, when applicable.
    pub causation_id: Option<EventId>,
    /// Data classification (SPEC-020 ladder).
    pub data_class: EventDataClass,
    /// Structured payload; never a free-form provider blob.
    pub payload: serde_json::Value,
}

impl EventEnvelope {
    /// Validate canonical envelope invariants.
    pub fn validate(&self) -> Result<(), EventError> {
        if self.schema_version != EVENT_SCHEMA_VERSION {
            return Err(EventError::new(
                EventErrorCode::Validation,
                format!(
                    "schema_version must be {EVENT_SCHEMA_VERSION}, got {}",
                    self.schema_version
                ),
            ));
        }
        if self.source.is_empty() || self.actor.is_empty() {
            return Err(EventError::new(
                EventErrorCode::Validation,
                "source and actor must not be empty",
            ));
        }
        if self.subject.is_empty() {
            return Err(EventError::new(
                EventErrorCode::Validation,
                "subject must not be empty",
            ));
        }
        Ok(())
    }
}
