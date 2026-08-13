//! Short-lived capability grants (SPEC-005 behavior 5).
//!
//! Capability tokens are short-lived, audience restricted, resource
//! restricted, action restricted, and non-transferable where platform
//! support permits. `CapabilityGrant` is the canonical grant record:
//! it binds a capability to a target object, an actor, a scope, and an
//! expiry. Grants never outlive their expiry and never widen scope.

use std::fmt;

use nexus_domain::{CapabilityClass, NexusId, TenantId};
use serde::{Deserialize, Serialize};

/// Lifecycle of a capability grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GrantState {
    /// Issued and usable until expiry.
    Active,
    /// Consumed or cancelled by the grantor.
    Revoked,
    /// Past its expiry; never usable again.
    Expired,
}

impl GrantState {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Revoked => "REVOKED",
            Self::Expired => "EXPIRED",
        }
    }
}

impl fmt::Display for GrantState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A short-lived capability grant (SPEC-005 behavior 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    /// Grant identifier.
    pub grant_id: NexusId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Requested capability class.
    pub capability: CapabilityClass,
    /// The actor to whom the grant is bound (non-transferable).
    pub actor: NexusId,
    /// Target object identifier (resource restriction).
    pub target_id: NexusId,
    /// Granted action scope, e.g. `task:complete` (action restriction).
    pub scope: String,
    /// Issued time, unix seconds.
    pub issued_at_unix_s: i64,
    /// Expiry time, unix seconds. Grants never outlive this.
    pub expires_at_unix_s: i64,
    /// Current grant state.
    pub state: GrantState,
}

impl CapabilityGrant {
    /// Construct a grant; rejects empty scope and inverted times.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        grant_id: NexusId,
        tenant_id: TenantId,
        capability: CapabilityClass,
        actor: NexusId,
        target_id: NexusId,
        scope: impl Into<String>,
        issued_at_unix_s: i64,
        expires_at_unix_s: i64,
    ) -> Result<Self, CapabilityGrantError> {
        let scope = scope.into();
        if scope.trim().is_empty() {
            return Err(CapabilityGrantError::EmptyScope);
        }
        if expires_at_unix_s <= issued_at_unix_s {
            return Err(CapabilityGrantError::InvertedTimes);
        }
        Ok(Self {
            grant_id,
            tenant_id,
            capability,
            actor,
            target_id,
            scope,
            issued_at_unix_s,
            expires_at_unix_s,
            state: GrantState::Active,
        })
    }

    /// Whether the grant is currently usable (active and unexpired).
    pub fn is_usable_at(&self, now_unix_s: i64) -> bool {
        self.state == GrantState::Active && now_unix_s < self.expires_at_unix_s
    }

    /// Mark the grant revoked (idempotent).
    pub fn revoke(&mut self) {
        self.state = GrantState::Revoked;
    }

    /// Mark the grant expired (idempotent).
    pub fn expire(&mut self) {
        self.state = GrantState::Expired;
    }
}

/// Capability-grant construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityGrantError {
    /// Scope was empty/whitespace.
    EmptyScope,
    /// Expiry is not after issuance.
    InvertedTimes,
}

impl fmt::Display for CapabilityGrantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyScope => "grant scope must not be empty",
            Self::InvertedTimes => "expiry must be after issuance",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for CapabilityGrantError {}
