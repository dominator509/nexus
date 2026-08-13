//! Approval assertions (SPEC-005 canonical term; SPEC-006 behavior 6).
//!
//! An `ApprovalAssertion` is the signed evidence that a specific
//! principal approved a specific action digest at a specific time, with
//! the authentication strength and approval class recorded. It binds
//! the approver to the exact action digest - a digest mismatch is a
//! rejection, never a reuse.

use std::fmt;

use nexus_auth::AuthenticationStrength;
use nexus_domain::{ApprovalClass, CorrelationId, NexusId};
use serde::{Deserialize, Serialize};

/// Approval decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovalDecision {
    /// Approved.
    Approved,
    /// Rejected.
    Rejected,
}

impl ApprovalDecision {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
        }
    }
}

impl fmt::Display for ApprovalDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An approval assertion binding an approver to an exact action digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalAssertion {
    /// Assertion identifier.
    pub assertion_id: NexusId,
    /// Correlation of the approved action request.
    pub correlation: CorrelationId,
    /// Canonical digest of the action request being approved (bind).
    pub action_digest: String,
    /// Approver principal identifier.
    pub approver: NexusId,
    /// Approval class recorded (HUMAN, STRONG_HUMAN, FOUR_EYES...).
    pub approval_class: ApprovalClass,
    /// Authentication strength the approver presented.
    pub strength: AuthenticationStrength,
    /// Approval decision.
    pub decision: ApprovalDecision,
    /// Issued time, unix seconds.
    pub issued_at_unix_s: i64,
    /// Expiry time, unix seconds. Assertions never outlive this.
    pub expires_at_unix_s: i64,
}

impl ApprovalAssertion {
    /// Construct an assertion; rejects empty digest and inverted times.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        assertion_id: NexusId,
        correlation: CorrelationId,
        action_digest: impl Into<String>,
        approver: NexusId,
        approval_class: ApprovalClass,
        strength: AuthenticationStrength,
        decision: ApprovalDecision,
        issued_at_unix_s: i64,
        expires_at_unix_s: i64,
    ) -> Result<Self, ApprovalAssertionError> {
        let action_digest = action_digest.into();
        if action_digest.trim().is_empty() {
            return Err(ApprovalAssertionError::EmptyDigest);
        }
        if expires_at_unix_s <= issued_at_unix_s {
            return Err(ApprovalAssertionError::InvertedTimes);
        }
        Ok(Self {
            assertion_id,
            correlation,
            action_digest,
            approver,
            approval_class,
            strength,
            decision,
            issued_at_unix_s,
            expires_at_unix_s,
        })
    }

    /// Whether this assertion approves the exact digest and is usable
    /// at the given time. Digest mismatch or expiry is a rejection.
    pub fn approves(&self, digest: &str, now_unix_s: i64) -> bool {
        self.decision == ApprovalDecision::Approved
            && self.action_digest == digest
            && now_unix_s < self.expires_at_unix_s
    }
}

/// Approval-assertion construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalAssertionError {
    /// Action digest was empty/whitespace.
    EmptyDigest,
    /// Expiry is not after issuance.
    InvertedTimes,
}

impl fmt::Display for ApprovalAssertionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyDigest => "action digest must not be empty",
            Self::InvertedTimes => "expiry must be after issuance",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for ApprovalAssertionError {}
