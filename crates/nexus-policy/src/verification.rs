//! Verification plans and results (SPEC-006 behaviors 5 and 6).
//!
//! External success is not accepted until the verifier reads actual
//! state or an authoritative receipt. `VerificationPlan` describes the
//! expected observable state; `VerificationResult` records what was
//! actually observed. Mismatch fails closed.

use std::fmt;

use nexus_domain::NexusId;
use serde::{Deserialize, Serialize};

/// The expected observable state of a resource (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedState {
    /// Target object identifier.
    pub target_id: NexusId,
    /// Expected state name, e.g. `task:completed`.
    pub state: String,
}

impl ExpectedState {
    /// Construct an expected state; rejects empty state name.
    pub fn new(target_id: NexusId, state: impl Into<String>) -> Result<Self, ExpectedStateError> {
        let state = state.into();
        if state.trim().is_empty() {
            return Err(ExpectedStateError::EmptyState);
        }
        Ok(Self { target_id, state })
    }
}

/// Expected-state construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedStateError {
    /// State name was empty/whitespace.
    EmptyState,
}

impl fmt::Display for ExpectedStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected state must not be empty")
    }
}

impl std::error::Error for ExpectedStateError {}

/// A verification plan (SPEC-006 behavior 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationPlan {
    /// Expected observable state.
    pub expected: ExpectedState,
    /// Maximum verification wait, seconds.
    pub timeout_seconds: u64,
    /// Bounded retry count.
    pub retries: u32,
}

impl VerificationPlan {
    /// Construct a plan; rejects zero timeout.
    pub fn new(
        expected: ExpectedState,
        timeout_seconds: u64,
        retries: u32,
    ) -> Result<Self, VerificationPlanError> {
        if timeout_seconds == 0 {
            return Err(VerificationPlanError::ZeroTimeout);
        }
        Ok(Self {
            expected,
            timeout_seconds,
            retries,
        })
    }
}

/// Verification-plan construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationPlanError {
    /// Timeout must be positive.
    ZeroTimeout,
}

impl fmt::Display for VerificationPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("verification timeout must be positive")
    }
}

impl std::error::Error for VerificationPlanError {}

/// A verification result (SPEC-006 behavior 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the observed state matched the expectation.
    pub matched: bool,
    /// The state actually observed (redacted).
    pub observed_state: String,
    /// Verified time, unix seconds.
    pub verified_at_unix_s: i64,
}

impl VerificationResult {
    /// Construct a result; rejects empty observed state.
    pub fn new(
        matched: bool,
        observed_state: impl Into<String>,
        verified_at_unix_s: i64,
    ) -> Result<Self, VerificationResultError> {
        let observed_state = observed_state.into();
        if observed_state.trim().is_empty() {
            return Err(VerificationResultError::EmptyObservedState);
        }
        Ok(Self {
            matched,
            observed_state,
            verified_at_unix_s,
        })
    }
}

/// Verification-result construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationResultError {
    /// Observed state was empty/whitespace.
    EmptyObservedState,
}

impl fmt::Display for VerificationResultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("observed state must not be empty")
    }
}

impl std::error::Error for VerificationResultError {}

/// Provider-neutral verification port (SPEC-006 behavior 5).
pub trait Verifier {
    /// Verify that the target has reached the expected state within the
    /// plan's bound. Mismatch or provider failure fails closed.
    fn verify(
        &self,
        plan: &VerificationPlan,
    ) -> Result<VerificationResult, crate::error::PolicyError>;
}
