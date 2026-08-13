//! Contextual policy evaluation (SPEC-005; OPA decision shape).
//!
//! `ContextPolicyEngine` is the provider-neutral port for contextual
//! policy: time, device trust, presence, risk, and requested capability
//! combine with policy version into an allow/deny decision with a
//! machine-readable reason. This crate defines the input and decision
//! envelope; a provider implementation (OPA in M3/M4) evaluates the
//! actual policy.

use std::fmt;

use nexus_auth::AuthenticationStrength;
use nexus_domain::{CapabilityClass, Risk, TenantId};
use nexus_identity::{Principal, TrustLevel};
use serde::{Deserialize, Serialize};

/// Inputs to contextual policy evaluation (SPEC-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyInput {
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Acting principal.
    pub principal: Principal,
    /// Requested capability class.
    pub capability: CapabilityClass,
    /// Risk class of the action (R0..R4).
    pub risk: Risk,
    /// Authentication strength of the current session.
    pub strength: AuthenticationStrength,
    /// Device trust level when a device is involved.
    pub device_trust: TrustLevel,
    /// Object type being acted upon, e.g. `task`, `memory`.
    pub object_type: String,
    /// Object identifier (canonical Nexus UUIDv7 string).
    pub object_id: String,
}

impl PolicyInput {
    /// Construct a policy input; rejects empty object type/id.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tenant_id: TenantId,
        principal: Principal,
        capability: CapabilityClass,
        risk: Risk,
        strength: AuthenticationStrength,
        device_trust: TrustLevel,
        object_type: impl Into<String>,
        object_id: impl Into<String>,
    ) -> Result<Self, PolicyInputError> {
        let object_type = object_type.into();
        let object_id = object_id.into();
        if object_type.trim().is_empty() {
            return Err(PolicyInputError::EmptyObjectType);
        }
        nexus_domain::NexusId::new(&object_id).map_err(|_| PolicyInputError::InvalidObjectId)?;
        Ok(Self {
            tenant_id,
            principal,
            capability,
            risk,
            strength,
            device_trust,
            object_type,
            object_id,
        })
    }
}

/// Policy-input construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyInputError {
    /// Object type was empty/whitespace.
    EmptyObjectType,
    /// Object id is not a canonical Nexus UUIDv7.
    InvalidObjectId,
}

impl fmt::Display for PolicyInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyObjectType => "object type must not be empty",
            Self::InvalidObjectId => "object id is not a canonical Nexus UUIDv7",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for PolicyInputError {}

/// A contextual policy decision (SPEC-005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDecision {
    /// Explicit allow. Fail closed: absence of allow is a denial.
    pub allowed: bool,
    /// Machine-readable policy version that produced the decision.
    pub policy_version: String,
    /// Human-safe reason (redacted; never secrets or prompts).
    pub reason: String,
}

impl PolicyDecision {
    /// An explicit allow with the given policy version.
    pub fn allow(policy_version: impl Into<String>) -> Self {
        Self {
            allowed: true,
            policy_version: policy_version.into(),
            reason: "allowed by policy".to_string(),
        }
    }

    /// An explicit deny with the given policy version and reason.
    pub fn deny(policy_version: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            policy_version: policy_version.into(),
            reason: reason.into(),
        }
    }
}

/// Provider-neutral contextual policy port (SPEC-005).
pub trait ContextPolicyEngine {
    /// Evaluate contextual policy. Fail closed: any error is a denial,
    /// never a grant.
    fn evaluate(&self, input: &PolicyInput) -> Result<PolicyDecision, crate::error::PolicyError>;
}
