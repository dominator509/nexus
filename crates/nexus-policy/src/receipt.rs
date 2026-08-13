//! Action receipts (SPEC-005 behavior 9; SPEC-006 behavior 5).
//!
//! Every authorization decision creates a redacted receipt with policy
//! version and evidence references. Receipts never contain secrets,
//! tokens, or prompts.

use std::fmt;

use nexus_domain::{CorrelationId, NexusId};
use serde::{Deserialize, Serialize};

use crate::gateway::{ActionDecision, DenialReason};
use crate::vocabulary::ActionLifecycleState;

/// Receipt lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReceiptState {
    /// Receipt issued with a terminal decision.
    Issued,
    /// Receipt superseded by a later compensation/verification record.
    Superseded,
}

impl ReceiptState {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Issued => "ISSUED",
            Self::Superseded => "SUPERSEDED",
        }
    }
}

impl fmt::Display for ReceiptState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A redacted authorization receipt (SPEC-005 behavior 9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionReceipt {
    /// Receipt identifier.
    pub receipt_id: NexusId,
    /// Correlation of the evaluated action.
    pub correlation: CorrelationId,
    /// Action request identifier.
    pub request_id: NexusId,
    /// Lifecycle state at issuance.
    pub lifecycle: ActionLifecycleState,
    /// Machine-readable denial reason when denied.
    pub denial_reason: Option<DenialReason>,
    /// Policy version(s) that produced the decision (redacted).
    pub policy_version: String,
    /// Evidence references (opaque refs, never content).
    pub evidence_refs: Vec<String>,
    /// Receipt state.
    pub state: ReceiptState,
    /// Issued time, unix seconds.
    pub issued_at_unix_s: i64,
}

impl ActionReceipt {
    /// Construct a receipt from a decision; rejects empty policy version.
    pub fn from_decision(
        receipt_id: NexusId,
        correlation: CorrelationId,
        request_id: NexusId,
        decision: &ActionDecision,
        policy_version: impl Into<String>,
        evidence_refs: Vec<String>,
        issued_at_unix_s: i64,
    ) -> Result<Self, ReceiptError> {
        let policy_version = policy_version.into();
        if policy_version.trim().is_empty() {
            return Err(ReceiptError::EmptyPolicyVersion);
        }
        let (lifecycle, denial_reason) = match decision {
            ActionDecision::Allowed { .. } => (ActionLifecycleState::Approved, None),
            ActionDecision::Denied { reason, .. } => {
                (ActionLifecycleState::Rejected, Some(*reason))
            }
        };
        Ok(Self {
            receipt_id,
            correlation,
            request_id,
            lifecycle,
            denial_reason,
            policy_version,
            evidence_refs,
            state: ReceiptState::Issued,
            issued_at_unix_s,
        })
    }
}

/// Receipt construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptError {
    /// Policy version was empty/whitespace.
    EmptyPolicyVersion,
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("policy version must not be empty")
    }
}

impl std::error::Error for ReceiptError {}
