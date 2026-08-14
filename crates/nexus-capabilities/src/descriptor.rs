//! Capability descriptor (SPEC-003 canonical term; schema
//! `capability-descriptor`).
//!
//! A descriptor is the stable advertisement of a capability:
//! schema URIs for input and output, required scopes, risk,
//! approval class, reversal semantics, idempotency contract,
//! availability, locality, data classes, and event types. Descriptors
//! are versioned (`major.minor.patch`) and provider-neutral; free-form
//! provider payloads never appear here (SPEC-003, SPEC-022).

use serde::{Deserialize, Serialize};

use nexus_domain::{
    ApprovalClass, Availability, CapabilityClass, Idempotency, Locality, Privacy, Reversal, Risk,
};

use crate::vocabulary::SchemaRef;

/// Stable version string `major.minor.patch`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityVersion(pub String);

/// Error produced when a capability descriptor is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDescriptorError(pub String);

impl std::fmt::Display for CapabilityDescriptorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid capability descriptor: {}", self.0)
    }
}

impl std::error::Error for CapabilityDescriptorError {}

/// Provider-neutral advertisement of one capability (SPEC-003,
/// SPEC-022; schema `capability-descriptor`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    /// Stable capability key (`^[a-z][a-z0-9_.-]+$`).
    pub id: String,
    /// Semver `major.minor.patch`.
    pub version: CapabilityVersion,
    /// Capability class (read / command / workflow / stream /
    /// administrative).
    pub class: CapabilityClass,
    /// Human-readable description.
    pub description: String,
    /// Canonical input schema URI.
    pub input_schema: SchemaRef,
    /// Canonical output schema URI.
    pub output_schema: SchemaRef,
    /// Scopes required to invoke the capability.
    pub required_scopes: Vec<String>,
    /// Risk class of the capability's actions.
    pub risk: Risk,
    /// Approval class required before execution.
    pub approval: ApprovalClass,
    /// Reversal semantics of the capability's actions.
    pub reversal: Reversal,
    /// Idempotency contract (SPEC-006).
    pub idempotency: Idempotency,
    /// Advertised availability.
    pub availability: Availability,
    /// Optional execution locality preference.
    pub locality: Option<Locality>,
    /// Optional data classes touched by the capability.
    pub data_classes: Vec<Privacy>,
    /// Optional event types emitted by the capability.
    pub event_types: Vec<String>,
    /// Optional provider identifier (never a credential).
    pub provider_id: Option<String>,
}

impl CapabilityDescriptor {
    /// Construct and validate a descriptor against the canonical
    /// schema's constraints.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        version: CapabilityVersion,
        class: CapabilityClass,
        description: impl Into<String>,
        input_schema: SchemaRef,
        output_schema: SchemaRef,
        required_scopes: Vec<String>,
        risk: Risk,
        approval: ApprovalClass,
        reversal: Reversal,
        idempotency: Idempotency,
        availability: Availability,
        locality: Option<Locality>,
        data_classes: Vec<Privacy>,
        event_types: Vec<String>,
        provider_id: Option<String>,
    ) -> Result<Self, CapabilityDescriptorError> {
        let id = id.into();
        let description = description.into();
        if id.is_empty() {
            return Err(CapabilityDescriptorError(
                "id must not be empty".to_string(),
            ));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'))
        {
            return Err(CapabilityDescriptorError(
                "id must match ^[a-z][a-z0-9_.-]+$".to_string(),
            ));
        }
        if description.chars().count() < 10 {
            return Err(CapabilityDescriptorError(
                "description must be at least 10 characters".to_string(),
            ));
        }
        if description.chars().count() > 1000 {
            return Err(CapabilityDescriptorError(
                "description must be at most 1000 characters".to_string(),
            ));
        }
        if required_scopes.is_empty() {
            return Err(CapabilityDescriptorError(
                "required_scopes must not be empty".to_string(),
            ));
        }
        let mut seen = std::collections::HashSet::new();
        for scope in &required_scopes {
            if scope.is_empty() || !seen.insert(scope.clone()) {
                return Err(CapabilityDescriptorError(
                    "required_scopes must be non-empty and unique".to_string(),
                ));
            }
        }
        Ok(Self {
            id,
            version,
            class,
            description,
            input_schema,
            output_schema,
            required_scopes,
            risk,
            approval,
            reversal,
            idempotency,
            availability,
            locality,
            data_classes,
            event_types,
            provider_id,
        })
    }
}
