//! EP-035 SetupWizard state machine (SPEC-004 / SPEC-016).
//!
//! The wizard models STATE, not visual progress. A page being visited
//! or rendered never implies a step is complete, a provider is
//! configured, or a resource is healthy. COMPLETE_LOCAL is a local
//! checkpoint (LOCAL_PROGRESS_SAVED); VERIFIED requires an explicit
//! remote verification record (REMOTE_EFFECT_VERIFIED). Transitions are
//! typed and validated: invalid leaps (NOT_STARTED -> COMPLETED,
//! FAILED -> COMPLETED without recovery) are rejected with POLICY.

use std::collections::BTreeMap;

use nexus_domain::CorrelationId;
use serde::{Deserialize, Serialize};

use crate::error::{SetupError, SetupResult};
use crate::vocabulary::{WizardState, WizardStep, WizardStepStatus};

/// Allowed whole-wizard state transitions.
pub fn is_valid_wizard_transition(from: WizardState, to: WizardState) -> bool {
    use WizardState::*;
    matches!(
        (from, to),
        (NotStarted, InProgress)
            | (InProgress, Blocked | Failed | RecoveryRequired | Completed)
            | (Blocked, InProgress | RecoveryRequired)
            | (Failed, RecoveryRequired | InProgress)
            | (RecoveryRequired, InProgress)
    )
}

/// Allowed per-step status transitions.
pub fn is_valid_step_transition(from: WizardStepStatus, to: WizardStepStatus) -> bool {
    use WizardStepStatus::*;
    matches!(
        (from, to),
        (Pending, InProgress)
            | (InProgress, Blocked | Failed | CompleteLocal)
            | (Blocked, InProgress)
            | (Failed, InProgress)
            | (CompleteLocal, Verified)
    )
}

/// Remote verification record. Only a VERIFIED step carries one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteVerification {
    pub verified_at_unix_s: u64,
    pub verifier: String,
}

/// Per-step wizard record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WizardStepRecord {
    pub step: WizardStep,
    pub status: WizardStepStatus,
    pub last_transition_at_unix_s: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification: Option<RemoteVerification>,
}

impl WizardStepRecord {
    pub fn new(
        step: WizardStep,
        status: WizardStepStatus,
        last_transition_at_unix_s: u64,
        verification: Option<RemoteVerification>,
    ) -> SetupResult<Self> {
        if verification.is_some() && status != WizardStepStatus::Verified {
            return Err(SetupError::validation(format!(
                "step {} carries a verification record but status is {}",
                step, status
            )));
        }
        if status == WizardStepStatus::Verified && verification.is_none() {
            return Err(SetupError::verification(format!(
                "step {} is VERIFIED but has no verification record",
                step
            )));
        }
        Ok(Self {
            step,
            status,
            last_transition_at_unix_s,
            verification,
        })
    }
}

/// All steps owned by the setup wizard, in canonical order.
pub const ALL_WIZARD_STEPS: [WizardStep; 8] = [
    WizardStep::DeploymentChoice,
    WizardStep::HardwareProfile,
    WizardStep::OwnerBootstrap,
    WizardStep::RecoveryMaterial,
    WizardStep::EdgeEnrollment,
    WizardStep::Discovery,
    WizardStep::IntegrationReview,
    WizardStep::PlanReview,
];

/// SetupWizardState: the whole wizard, every step, typed transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupWizardState {
    pub state: WizardState,
    pub current_step: WizardStep,
    pub steps: Vec<WizardStepRecord>,
    pub correlation: CorrelationId,
    pub updated_at_unix_s: u64,
}

impl SetupWizardState {
    /// Canonical NOT_STARTED wizard with every step PENDING.
    pub fn not_started(correlation: CorrelationId, at_unix_s: u64) -> Self {
        let steps = ALL_WIZARD_STEPS
            .iter()
            .map(|step| WizardStepRecord {
                step: *step,
                status: WizardStepStatus::Pending,
                last_transition_at_unix_s: at_unix_s,
                verification: None,
            })
            .collect();
        Self {
            state: WizardState::NotStarted,
            current_step: WizardStep::DeploymentChoice,
            steps,
            correlation,
            updated_at_unix_s: at_unix_s,
        }
    }

    /// Validate the invariant: exactly the canonical step set, no
    /// duplicates, VERIFIED records consistent.
    pub fn validate(&self) -> SetupResult<()> {
        let mut seen: BTreeMap<WizardStep, usize> = BTreeMap::new();
        for record in &self.steps {
            *seen.entry(record.step).or_insert(0) += 1;
        }
        for step in ALL_WIZARD_STEPS {
            let count = seen.get(&step).copied().unwrap_or(0);
            if count != 1 {
                return Err(SetupError::validation(format!(
                    "wizard steps must contain exactly one record per step; {} has {}",
                    step, count
                )));
            }
        }
        for record in &self.steps {
            if record.verification.is_some() && record.status != WizardStepStatus::Verified {
                return Err(SetupError::validation(format!(
                    "step {} carries verification with status {}",
                    record.step, record.status
                )));
            }
        }
        Ok(())
    }

    pub fn step_record(&self, step: WizardStep) -> Option<&WizardStepRecord> {
        self.steps.iter().find(|record| record.step == step)
    }

    /// Advance the whole wizard. COMPLETED requires every step VERIFIED.
    pub fn advance(&self, to_state: WizardState, at_unix_s: u64) -> SetupResult<Self> {
        if !is_valid_wizard_transition(self.state, to_state) {
            return Err(SetupError::policy(format!(
                "invalid wizard transition {} -> {}",
                self.state, to_state
            )));
        }
        if to_state == WizardState::Completed {
            let unverified: Vec<WizardStep> = self
                .steps
                .iter()
                .filter(|record| record.status != WizardStepStatus::Verified)
                .map(|record| record.step)
                .collect();
            if !unverified.is_empty() {
                let names: Vec<String> = unverified.iter().map(|s| s.to_string()).collect();
                return Err(SetupError::policy(format!(
                    "wizard cannot complete with unverified steps: {}",
                    names.join(",")
                )));
            }
        }
        let mut updated = self.clone();
        updated.state = to_state;
        updated.updated_at_unix_s = at_unix_s;
        updated.validate()?;
        Ok(updated)
    }

    /// Advance one step's status. VERIFIED requires a verification
    /// record; a verification record with a non-VERIFIED status is
    /// rejected before the transition check.
    pub fn advance_step(
        &self,
        step: WizardStep,
        to_status: WizardStepStatus,
        at_unix_s: u64,
        verification: Option<RemoteVerification>,
    ) -> SetupResult<Self> {
        if to_status != WizardStepStatus::Verified && verification.is_some() {
            return Err(SetupError::validation(format!(
                "step {} cannot carry a verification record with status {}",
                step, to_status
            )));
        }
        let record = self.step_record(step).ok_or_else(|| {
            SetupError::validation(format!("wizard has no record for step {}", step))
        })?;
        if !is_valid_step_transition(record.status, to_status) {
            return Err(SetupError::policy(format!(
                "invalid step transition {}: {} -> {}",
                step, record.status, to_status
            )));
        }
        if to_status == WizardStepStatus::Verified && verification.is_none() {
            return Err(SetupError::verification(format!(
                "step {} cannot become VERIFIED without a verification record",
                step
            )));
        }
        let mut updated = self.clone();
        for entry in &mut updated.steps {
            if entry.step == step {
                entry.status = to_status;
                entry.last_transition_at_unix_s = at_unix_s;
                entry.verification = verification.clone();
            }
        }
        updated.current_step = step;
        updated.updated_at_unix_s = at_unix_s;
        updated.validate()?;
        Ok(updated)
    }

    /// Verify a step remotely: the only way a step becomes VERIFIED.
    pub fn verify_step(
        &self,
        step: WizardStep,
        verification: RemoteVerification,
    ) -> SetupResult<Self> {
        if verification.verified_at_unix_s == 0 {
            return Err(SetupError::validation(
                "verified_at_unix_s must be positive",
            ));
        }
        if verification.verifier.is_empty() {
            return Err(SetupError::validation("verifier must not be empty"));
        }
        self.advance_step(
            step,
            WizardStepStatus::Verified,
            verification.verified_at_unix_s,
            Some(verification),
        )
    }
}
