//! Short-lived capability token issuer (SPEC-005 behavior 5).
//!
//! Capability tokens are the executable authority handed to a service
//! or connector for ONE scoped operation: audience restricted, resource
//! restricted, action restricted, tenant restricted, short-lived, and
//! non-transferable. The issuer port issues and verifies tokens; tokens
//! never outlive their expiry and are never long-lived universal bearer
//! credentials (EP-009 acceptance obligation 1).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::TrustError;
use crate::vocabulary::TokenState;

/// A short-lived capability token (SPEC-005 behavior 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityToken {
    /// Token identifier.
    pub token_id: String,
    /// Audience the token is restricted to (e.g. a service name).
    pub audience: String,
    /// Tenant boundary.
    pub tenant_id: String,
    /// Resource restriction (object type:object id, or `*` for none).
    pub resource: String,
    /// Action restriction (e.g. `task:complete`).
    pub action: String,
    /// Actor/principal the token is bound to (non-transferable).
    pub actor: String,
    /// Issued time, unix seconds.
    pub issued_at_unix_s: i64,
    /// Expiry time, unix seconds. Tokens never outlive this.
    pub expires_at_unix_s: i64,
    /// Current token state.
    pub state: TokenState,
}

impl CapabilityToken {
    /// Construct a token; rejects empty fields and inverted times.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        token_id: impl Into<String>,
        audience: impl Into<String>,
        tenant_id: impl Into<String>,
        resource: impl Into<String>,
        action: impl Into<String>,
        actor: impl Into<String>,
        issued_at_unix_s: i64,
        expires_at_unix_s: i64,
    ) -> Result<Self, CapabilityTokenIssuerError> {
        let token_id = token_id.into();
        let audience = audience.into();
        let tenant_id = tenant_id.into();
        let resource = resource.into();
        let action = action.into();
        let actor = actor.into();
        if token_id.trim().is_empty()
            || audience.trim().is_empty()
            || tenant_id.trim().is_empty()
            || resource.trim().is_empty()
            || action.trim().is_empty()
            || actor.trim().is_empty()
        {
            return Err(CapabilityTokenIssuerError::EmptyField);
        }
        if expires_at_unix_s <= issued_at_unix_s {
            return Err(CapabilityTokenIssuerError::InvertedTimes);
        }
        Ok(Self {
            token_id,
            audience,
            tenant_id,
            resource,
            action,
            actor,
            issued_at_unix_s,
            expires_at_unix_s,
            state: TokenState::Active,
        })
    }

    /// Whether the token is currently usable (active and unexpired).
    pub fn is_usable_at(&self, now_unix_s: i64) -> bool {
        self.state == TokenState::Active && now_unix_s < self.expires_at_unix_s
    }

    /// Whether the token is bound to the exact audience/resource/action/
    /// tenant/actor required for an operation.
    pub fn covers(
        &self,
        audience: &str,
        resource: &str,
        action: &str,
        tenant_id: &str,
        actor: &str,
    ) -> bool {
        self.audience == audience
            && self.resource == resource
            && self.action == action
            && self.tenant_id == tenant_id
            && self.actor == actor
    }

    /// Mark the token revoked (idempotent).
    pub fn revoke(&mut self) {
        self.state = TokenState::Revoked;
    }

    /// Mark the token expired (idempotent).
    pub fn expire(&mut self) {
        self.state = TokenState::Expired;
    }
}

impl fmt::Display for CapabilityToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "token {} for {}@{} ({})",
            self.token_id, self.actor, self.tenant_id, self.action
        )
    }
}

/// Provider-neutral capability token issuer port.
pub trait CapabilityTokenIssuer {
    /// Issue a short-lived token bound to the exact scope.
    #[allow(clippy::too_many_arguments)]
    fn issue(
        &self,
        audience: &str,
        tenant_id: &str,
        resource: &str,
        action: &str,
        actor: &str,
        ttl_seconds: i64,
        now_unix_s: i64,
    ) -> Result<CapabilityToken, TrustError>;
    /// Verify a token for one operation at a time.
    fn verify(&self, token: &CapabilityToken, now_unix_s: i64) -> Result<(), TrustError>;
    /// Revoke a token before expiry.
    fn revoke(&self, token_id: &str) -> Result<(), TrustError>;
}

/// Capability token construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityTokenIssuerError {
    /// A required field was empty/whitespace.
    EmptyField,
    /// Expiry is not after issuance.
    InvertedTimes,
}

impl fmt::Display for CapabilityTokenIssuerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyField => "capability token fields must not be empty",
            Self::InvertedTimes => "token expiry must be after issuance",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for CapabilityTokenIssuerError {}
