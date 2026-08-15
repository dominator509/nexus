//! EP-018 skill evaluator and proposal (SPEC-010 behavior 8;
//! ADR-025).
//!
//! The Skill Factory creates candidates from successful work, tests
//! them against frozen evals, requests human promotion, and retains
//! rollback versions. An evaluation is deterministic and fail-closed:
//! a skill that does not pass its frozen evals can never be promoted.
//!
//! Proposal lifecycle (ADR-025): canonical transitions only, fail
//! closed, no terminal resurrection. A model/agent may PROPOSE a skill;
//! it may not self-approve installation. Promotion to `PROMOTED`
//! requires a distinct human approver.

use crate::manifest::SkillPackageErrorCode;
use crate::manifest::{version_key, SkillPackage, SkillPackageError};
use crate::vocabulary::SkillProposalState;
use nexus_domain::{CorrelationId, SkillId, TenantId};
use serde::{Deserialize, Serialize};

/// A deterministic evaluation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvaluation {
    pub skill_id: SkillId,
    pub passed: bool,
    /// Frozen eval ids that were exercised (never empty on success).
    pub eval_ids: Vec<String>,
    /// Evaluator version that produced this verdict (ADR-025).
    pub evaluator_version: String,
    pub notes: String,
}

impl SkillEvaluation {
    pub fn validate(&self) -> Result<(), SkillPackageError> {
        if self.passed && self.eval_ids.is_empty() {
            return Err(SkillPackageError::validation(
                "passed evaluation must name frozen eval ids",
                Some("skill-evaluator".into()),
            ));
        }
        if self.passed && self.evaluator_version.is_empty() {
            return Err(SkillPackageError::validation(
                "passed evaluation must name its evaluator version",
                Some("skill-evaluator".into()),
            ));
        }
        Ok(())
    }
}

/// The evaluation port (SPEC-010 behavior 8). Deterministic: a given
/// package and frozen eval corpus produce the same verdict.
pub trait SkillEvaluator {
    fn evaluate(&self, package: &SkillPackage) -> Result<SkillEvaluation, SkillPackageError>;
}

/// Deterministic evaluation error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvaluatorError {
    pub code: SkillPackageErrorCode,
    pub message: String,
}

/// A factory proposal with lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillProposal {
    pub proposal_id: String,
    pub skill_id: SkillId,
    pub tenant_id: TenantId,
    pub correlation_id: CorrelationId,
    pub package: SkillPackage,
    pub state: SkillProposalState,
    /// Actor (model/agent/person) that proposed the skill. A model may
    /// propose; it may not self-approve promotion (ADR-025).
    pub proposed_by: String,
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
}

impl SkillProposal {
    pub fn validate(&self) -> Result<(), SkillPackageError> {
        if self.proposal_id.is_empty() {
            return Err(SkillPackageError::validation(
                "proposal_id must not be empty",
                Some("skill-proposal".into()),
            ));
        }
        if self.proposed_by.is_empty() {
            return Err(SkillPackageError::validation(
                "proposed_by must not be empty",
                Some("skill-proposal".into()),
            ));
        }
        self.package.validate()?;
        Ok(())
    }

    /// Deterministic lifecycle transition. Only canonical edges are
    /// allowed; terminal states never move; resurrection is rejected.
    ///
    /// Canonical edges:
    /// `PROPOSED -> EVAL_PENDING -> EVAL_PASSED | EVAL_FAILED`,
    /// `EVAL_PASSED -> AWAITING_PROMOTION`,
    /// `AWAITING_PROMOTION -> REJECTED`.
    /// `PROMOTED` is reached ONLY through `approve()` with a distinct
    /// human approver.
    pub fn transition(
        &mut self,
        next: SkillProposalState,
        now_epoch_ms: u64,
    ) -> Result<(), SkillPackageError> {
        let allowed = matches!(
            (self.state, next),
            (
                SkillProposalState::Proposed,
                SkillProposalState::EvalPending
            ) | (
                SkillProposalState::EvalPending,
                SkillProposalState::EvalPassed
            ) | (
                SkillProposalState::EvalPending,
                SkillProposalState::EvalFailed
            ) | (
                SkillProposalState::EvalPassed,
                SkillProposalState::AwaitingPromotion
            ) | (
                SkillProposalState::AwaitingPromotion,
                SkillProposalState::Rejected
            )
        );
        if !allowed {
            return Err(SkillPackageError::validation(
                format!(
                    "invalid proposal transition {} -> {}",
                    self.state.as_str(),
                    next.as_str()
                ),
                Some("skill-proposal".into()),
            ));
        }
        self.state = next;
        self.updated_at_epoch_ms = now_epoch_ms;
        Ok(())
    }

    /// Approve installation (ADR-025). Requires `AWAITING_PROMOTION`,
    /// a non-empty approver, and an approver distinct from the
    /// proposer: a model/agent may propose a skill, it may not
    /// self-approve installation.
    pub fn approve(
        &mut self,
        approved_by: &str,
        now_epoch_ms: u64,
    ) -> Result<(), SkillPackageError> {
        if self.state != SkillProposalState::AwaitingPromotion {
            return Err(SkillPackageError::validation(
                "only AWAITING_PROMOTION proposals can be approved",
                Some("skill-proposal".into()),
            ));
        }
        if approved_by.is_empty() {
            return Err(SkillPackageError::validation(
                "approver must not be empty",
                Some("skill-proposal".into()),
            ));
        }
        if approved_by == self.proposed_by {
            return Err(SkillPackageError::policy(
                "proposer cannot self-approve skill installation",
                Some("skill-proposal".into()),
            ));
        }
        self.state = SkillProposalState::Promoted;
        self.updated_at_epoch_ms = now_epoch_ms;
        Ok(())
    }
}

/// Version key for rollback retention (immutable by version).
pub fn proposal_version_key(package: &SkillPackage) -> String {
    version_key(&package.manifest.name, &package.manifest.version)
}
