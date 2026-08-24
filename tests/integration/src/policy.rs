//! Flake policy and consecutive-verify policy (SPEC-008 acceptance
//! obligation 4: verify passes three consecutive times and flaky behavior
//! is eliminated).
//!
//! FLAKE RETRIED GREEN != ROOT CAUSE FIXED: a retry may classify a flake
//! but never erases it; fixing requires a root cause. A consecutive
//! verify sequence completes only after N consecutive green gate results.

use nexus_test_contract::error::{TestingError, TestingResult};
use nexus_test_contract::model::{FlakeRecord, GateResult};
use nexus_test_contract::vocabulary::FlakeClassification;
use nexus_test_contract::FlakyTestPolicyPort;

/// Deterministic flake policy: a flake record must carry a known
/// classification and is fixed only with a root cause.
#[derive(Debug, Default)]
pub struct FlakePolicy;

impl FlakePolicy {
    pub fn new() -> Self {
        Self
    }
}

impl FlakyTestPolicyPort for FlakePolicy {
    fn classify(&self, record: &FlakeRecord) -> TestingResult<()> {
        if record.test_id.trim().is_empty() {
            return Err(TestingError::validation("flake test_id is required"));
        }
        // The classification enum itself is deny-unknown (FromStr), so
        // any constructed record already carries a canonical class.
        let _ = record.classification.as_str();
        Ok(())
    }
}

/// Consecutive-verify policy: the acceptance obligation requires verify
/// to pass three consecutive times. Any non-green run resets the counter;
/// the sequence is complete only after N consecutive green results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsecutiveVerify {
    /// Number of consecutive green runs required.
    pub required: u32,
    /// Consecutive green runs observed so far.
    pub consecutive_green: u32,
    /// Most recent gate result, if any.
    pub last: Option<GateResult>,
    /// Flake records observed during the sequence. A retried-green run
    /// is recorded here and is NOT fixed without a root cause.
    pub flakes: Vec<FlakeRecord>,
}

impl ConsecutiveVerify {
    pub fn new(required: u32) -> Self {
        Self {
            required: required.max(1),
            consecutive_green: 0,
            last: None,
            flakes: Vec::new(),
        }
    }

    /// Record a gate result. Green increments the counter; any non-green
    /// result resets it and records a flake (never erased, never fixed).
    pub fn record(&mut self, result: GateResult) {
        if result.is_green() {
            self.consecutive_green += 1;
        } else {
            self.consecutive_green = 0;
            self.flakes.push(
                FlakeRecord::new(
                    format!("gate:{}", result.gate),
                    FlakeClassification::RuntimeOrdering,
                )
                .retried_green(),
            );
        }
        self.last = Some(result);
    }

    /// The sequence is complete only after N consecutive green results.
    pub fn is_complete(&self) -> bool {
        self.consecutive_green >= self.required
    }

    /// A flake observed during the sequence is fixed only with a root
    /// cause; a retried-green record is never considered fixed.
    pub fn fix_flake(&mut self, index: usize, root_cause: impl Into<String>) -> TestingResult<()> {
        let record = self
            .flakes
            .get_mut(index)
            .ok_or_else(|| TestingError::validation("flake index out of range"))?;
        let cause = root_cause.into();
        if cause.trim().is_empty() {
            return Err(TestingError::flake_unresolved(
                "root cause is required to fix a flake",
            ));
        }
        record.root_cause = Some(cause);
        Ok(())
    }
}
