//! Action gateway contract (SPEC-005/SPEC-006).
//!
//! The Action Gateway combines relationship, contextual policy, risk,
//! capability, and approval into a deterministic `ActionDecision` for a
//! single `ActionRequest`. The trait here is the provider-neutral port;
//! the deterministic gateway implementation lands in
//! `crates/nexus-action-gateway` (M2). Fail closed: any missing input
//! is a denial, never a grant.

use std::fmt;

use nexus_domain::{CorrelationId, NexusId, TenantId};
use serde::{Deserialize, Serialize};

use crate::capability::CapabilityGrant;
use crate::error::{PolicyError, PolicyErrorCode};
use crate::vocabulary::ActionLifecycleState;

/// A request to perform one consequential action (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Request identifier.
    pub request_id: NexusId,
    /// Correlation of the whole operation.
    pub correlation: CorrelationId,
    /// Tenant boundary.
    pub tenant_id: TenantId,
    /// Canonical digest binding approvals and receipts to this request.
    pub action_digest: String,
    /// Requested action, e.g. `task:complete` (action restriction).
    pub action: String,
    /// Target object identifier (resource restriction).
    pub target_id: NexusId,
    /// Requested time, unix seconds.
    pub requested_at_unix_s: i64,
}

impl ActionRequest {
    /// Construct a request; rejects empty digest/action.
    pub fn new(
        request_id: NexusId,
        correlation: CorrelationId,
        tenant_id: TenantId,
        action_digest: impl Into<String>,
        action: impl Into<String>,
        target_id: NexusId,
        requested_at_unix_s: i64,
    ) -> Result<Self, ActionRequestError> {
        let action_digest = action_digest.into();
        let action = action.into();
        if action_digest.trim().is_empty() {
            return Err(ActionRequestError::EmptyDigest);
        }
        if action.trim().is_empty() {
            return Err(ActionRequestError::EmptyAction);
        }
        Ok(Self {
            request_id,
            correlation,
            tenant_id,
            action_digest,
            action,
            target_id,
            requested_at_unix_s,
        })
    }
}

/// Action-request construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionRequestError {
    /// Action digest was empty/whitespace.
    EmptyDigest,
    /// Action name was empty/whitespace.
    EmptyAction,
}

impl fmt::Display for ActionRequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyDigest => "action digest must not be empty",
            Self::EmptyAction => "action must not be empty",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for ActionRequestError {}

/// Reason for a denied decision (SPEC-006 error classes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DenialReason {
    /// Relationship check failed (no tuple).
    Relationship,
    /// Contextual policy denied.
    Policy,
    /// Authentication strength below the risk class requirement.
    InsufficientStrength,
    /// No valid capability grant covers the request.
    NoCapability,
    /// Required approval is missing, expired, or digest-mismatched.
    MissingApproval,
    /// Verification of the observable effect failed.
    VerificationFailed,
}

impl DenialReason {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Relationship => "RELATIONSHIP",
            Self::Policy => "POLICY",
            Self::InsufficientStrength => "INSUFFICIENT_STRENGTH",
            Self::NoCapability => "NO_CAPABILITY",
            Self::MissingApproval => "MISSING_APPROVAL",
            Self::VerificationFailed => "VERIFICATION_FAILED",
        }
    }
}

impl fmt::Display for DenialReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A deterministic action decision (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionDecision {
    /// The action may proceed; the grant is bound to the request.
    Allowed { grant: CapabilityGrant },
    /// The action is denied for a machine-readable reason.
    Denied {
        reason: DenialReason,
        message: String,
    },
}

impl ActionDecision {
    /// Whether the decision is an explicit allow.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed { .. })
    }

    /// The lifecycle state corresponding to this decision.
    pub fn lifecycle_state(&self) -> ActionLifecycleState {
        match self {
            Self::Allowed { .. } => ActionLifecycleState::Approved,
            Self::Denied { .. } => ActionLifecycleState::Rejected,
        }
    }
}

/// Provider-neutral action gateway port (SPEC-005/SPEC-006).
pub trait ActionGateway {
    /// Evaluate one action request and produce a deterministic decision.
    /// Fail closed: any error is a denial, never a grant.
    fn evaluate(&self, request: &ActionRequest) -> Result<ActionDecision, PolicyError>;
}

/// Helper for gateway implementations: a fail-closed denial from a
/// canonical reason and message.
pub fn denial(reason: DenialReason, message: impl Into<String>) -> ActionDecision {
    ActionDecision::Denied {
        reason,
        message: message.into(),
    }
}

/// Map an internal policy failure into the canonical error surface.
pub fn gateway_failure(
    code: PolicyErrorCode,
    message: impl Into<String>,
    correlation: Option<CorrelationId>,
) -> PolicyError {
    PolicyError::new(code, message, correlation)
}
