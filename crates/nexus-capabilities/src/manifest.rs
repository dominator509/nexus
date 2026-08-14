//! Connector manifest and binding (SPEC-022 canonical terms;
//! schema `connector-manifest`).
//!
//! A manifest declares a connector's tier, runtime, license, health
//! endpoint, capabilities, events, secret references, network origins,
//! data classes, and certification state. Secret references are names,
//! never values. A binding resolves the connector to an authenticated
//! tenant and account; it can never be selected by untrusted request
//! metadata (SPEC-003 behavior 7).

use serde::{Deserialize, Serialize};

use nexus_domain::vocabulary::ConnectorRuntime;
use nexus_domain::{Privacy, TenantId, Tier};

use crate::descriptor::CapabilityDescriptor;
use crate::vocabulary::Certification;

/// Connector identifier (`^[a-z][a-z0-9-]+$`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectorId(pub String);

/// Error produced when a connector manifest or binding is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorManifestError(pub String);

impl std::fmt::Display for ConnectorManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid connector manifest: {}", self.0)
    }
}

impl std::error::Error for ConnectorManifestError {}

/// Provider-neutral connector declaration (SPEC-022; schema
/// `connector-manifest`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorManifest {
    /// Connector key (`^[a-z][a-z0-9-]+$`).
    pub id: ConnectorId,
    /// Manifest version.
    pub version: String,
    /// Connector tier (1/2/3).
    pub tier: Tier,
    /// License identifier.
    pub license: String,
    /// Runtime class.
    pub runtime: ConnectorRuntime,
    /// Health endpoint URI reference.
    pub health: String,
    /// Capabilities advertised by this connector.
    pub capabilities: Vec<CapabilityDescriptor>,
    /// Event types this connector emits.
    pub events: Vec<String>,
    /// Secret references used by this connector (names, never values).
    pub secrets: Vec<String>,
    /// Network origins this connector may reach.
    pub network_origins: Vec<String>,
    /// Optional data classes touched by the connector.
    pub data_classes: Vec<Privacy>,
    /// Optional certification state.
    pub certification: Option<Certification>,
}

impl ConnectorManifest {
    /// Construct and validate a manifest against the canonical schema's
    /// constraints.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ConnectorId,
        version: impl Into<String>,
        tier: Tier,
        license: impl Into<String>,
        runtime: ConnectorRuntime,
        health: impl Into<String>,
        capabilities: Vec<CapabilityDescriptor>,
        events: Vec<String>,
        secrets: Vec<String>,
        network_origins: Vec<String>,
        data_classes: Vec<Privacy>,
        certification: Option<Certification>,
    ) -> Result<Self, ConnectorManifestError> {
        let version = version.into();
        let license = license.into();
        let health = health.into();
        if version.trim().is_empty() {
            return Err(ConnectorManifestError(
                "version must not be empty".to_string(),
            ));
        }
        if license.trim().is_empty() {
            return Err(ConnectorManifestError(
                "license must not be empty".to_string(),
            ));
        }
        if health.trim().is_empty() {
            return Err(ConnectorManifestError(
                "health must not be empty".to_string(),
            ));
        }
        Ok(Self {
            id,
            version,
            tier,
            license,
            runtime,
            health,
            capabilities,
            events,
            secrets,
            network_origins,
            data_classes,
            certification,
        })
    }
}

/// Connector binding to an authenticated tenant and account
/// (SPEC-022 canonical term).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorBinding {
    /// Connector key.
    pub connector_id: ConnectorId,
    /// Tenant boundary resolved from authenticated identity.
    pub tenant_id: TenantId,
    /// External account reference (never a credential).
    pub account_ref: String,
    /// Optional binding label.
    pub label: Option<String>,
}

impl ConnectorBinding {
    /// Construct a validated connector binding.
    pub fn new(
        connector_id: ConnectorId,
        tenant_id: TenantId,
        account_ref: impl Into<String>,
        label: Option<String>,
    ) -> Result<Self, ConnectorManifestError> {
        let account_ref = account_ref.into();
        if account_ref.trim().is_empty() {
            return Err(ConnectorManifestError(
                "account_ref must not be empty".to_string(),
            ));
        }
        Ok(Self {
            connector_id,
            tenant_id,
            account_ref,
            label,
        })
    }
}
