//! EP-019 healing vocabulary (SPEC-018; ADR-026).
//!
//! Vocabulary-locked enums for the self-healing engineering loop. The
//! canonical lifecycle is `OBSERVE -> INCIDENT -> CORRELATE -> DIAGNOSE
//! -> REPRODUCE -> PATCH_PROPOSED -> SANDBOX_VALIDATION ->
//! SECURITY_VALIDATION -> APPROVAL -> STAGED_DEPLOYMENT ->
//! POST_DEPLOY_VERIFICATION -> CLOSED` with explicit terminal/failure
//! states (`REJECTED`, `UNREPRODUCIBLE`, `VALIDATION_FAILED`,
//! `SECURITY_FAILED`, `ROLLED_BACK`, `BLOCKED`). Unknown values are
//! rejected at parse time; no free-form strings become domain contracts.
//!
//! DETECTED != DIAGNOSED != REPRODUCED != PATCHED != VERIFIED !=
//! APPROVED != DEPLOYED != REMEDIATED. No state may be collapsed and no
//! model/agent may declare its own fix successful.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Canonical incident lifecycle state (SPEC-018; ADR-026).
///
/// Serializes as SCREAMING_SNAKE_CASE. The states follow the exact
/// canonical ordering in the EP-019 owner directive; `CLOSED` is the
/// only healthy terminal, `REJECTED`/`UNREPRODUCIBLE`/`ROLLED_BACK` are
/// explicit terminal outcomes, and `VALIDATION_FAILED`/`SECURITY_FAILED`/
/// `BLOCKED` are explicit failure terminals. Terminal states never move;
/// resurrection is rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IncidentState {
    /// A real structured signal was observed (process failure, health
    /// failure, test failure, workflow failure, connector failure,
    /// security event, resource exhaustion, deployment regression).
    Observe,
    /// The signal was accepted as an incident candidate.
    Incident,
    /// The incident was correlated by canonical identifiers (not by raw
    /// error string alone).
    Correlate,
    /// Root-cause work: a diagnosis task exists with confidence.
    Diagnose,
    /// Minimal reproduction attempted (before/after proof).
    Reproduce,
    /// A patch proposal artifact exists.
    PatchProposed,
    /// The patch was validated in an isolated environment.
    SandboxValidation,
    /// Security gates passed for the patch.
    SecurityValidation,
    /// Human/policy approval is required and pending.
    Approval,
    /// The validated patch was deployed to a staged/canary target.
    StagedDeployment,
    /// Post-deploy verification ran against the original reproduction.
    PostDeployVerification,
    /// Incident is remediated: real observed verification closed it.
    Closed,
    /// Explicit terminal: approval denied or patch rejected.
    Rejected,
    /// Explicit terminal: the defect could not be reproduced.
    Unreproducible,
    /// Explicit terminal: sandbox/validation failed.
    ValidationFailed,
    /// Explicit terminal: security validation failed.
    SecurityFailed,
    /// Explicit terminal: rolled back to previous artifact/version.
    RolledBack,
    /// Explicit terminal: blocked with evidence report.
    Blocked,
}

impl IncidentState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Observe => "OBSERVE",
            Self::Incident => "INCIDENT",
            Self::Correlate => "CORRELATE",
            Self::Diagnose => "DIAGNOSE",
            Self::Reproduce => "REPRODUCE",
            Self::PatchProposed => "PATCH_PROPOSED",
            Self::SandboxValidation => "SANDBOX_VALIDATION",
            Self::SecurityValidation => "SECURITY_VALIDATION",
            Self::Approval => "APPROVAL",
            Self::StagedDeployment => "STAGED_DEPLOYMENT",
            Self::PostDeployVerification => "POST_DEPLOY_VERIFICATION",
            Self::Closed => "CLOSED",
            Self::Rejected => "REJECTED",
            Self::Unreproducible => "UNREPRODUCIBLE",
            Self::ValidationFailed => "VALIDATION_FAILED",
            Self::SecurityFailed => "SECURITY_FAILED",
            Self::RolledBack => "ROLLED_BACK",
            Self::Blocked => "BLOCKED",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Closed
                | Self::Rejected
                | Self::Unreproducible
                | Self::ValidationFailed
                | Self::SecurityFailed
                | Self::RolledBack
                | Self::Blocked
        )
    }

    pub const fn is_healthy_terminal(self) -> bool {
        matches!(self, Self::Closed)
    }

    pub const ALL: [IncidentState; 18] = [
        Self::Observe,
        Self::Incident,
        Self::Correlate,
        Self::Diagnose,
        Self::Reproduce,
        Self::PatchProposed,
        Self::SandboxValidation,
        Self::SecurityValidation,
        Self::Approval,
        Self::StagedDeployment,
        Self::PostDeployVerification,
        Self::Closed,
        Self::Rejected,
        Self::Unreproducible,
        Self::ValidationFailed,
        Self::SecurityFailed,
        Self::RolledBack,
        Self::Blocked,
    ];
}

impl fmt::Display for IncidentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for IncidentState {
    type Err = super::error::HealingError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "OBSERVE" => Ok(Self::Observe),
            "INCIDENT" => Ok(Self::Incident),
            "CORRELATE" => Ok(Self::Correlate),
            "DIAGNOSE" => Ok(Self::Diagnose),
            "REPRODUCE" => Ok(Self::Reproduce),
            "PATCH_PROPOSED" => Ok(Self::PatchProposed),
            "SANDBOX_VALIDATION" => Ok(Self::SandboxValidation),
            "SECURITY_VALIDATION" => Ok(Self::SecurityValidation),
            "APPROVAL" => Ok(Self::Approval),
            "STAGED_DEPLOYMENT" => Ok(Self::StagedDeployment),
            "POST_DEPLOY_VERIFICATION" => Ok(Self::PostDeployVerification),
            "CLOSED" => Ok(Self::Closed),
            "REJECTED" => Ok(Self::Rejected),
            "UNREPRODUCIBLE" => Ok(Self::Unreproducible),
            "VALIDATION_FAILED" => Ok(Self::ValidationFailed),
            "SECURITY_FAILED" => Ok(Self::SecurityFailed),
            "ROLLED_BACK" => Ok(Self::RolledBack),
            "BLOCKED" => Ok(Self::Blocked),
            other => Err(super::error::HealingError::vocabulary(
                "IncidentState",
                other,
            )),
        }
    }
}

/// Diagnosis confidence (SPEC-018; ADR-026).
///
/// A model-generated explanation ALWAYS begins as `HYPOTHESIS` and only
/// becomes `VALIDATED` after reproducible evidence supports it. A
/// hypothesis is never root cause by assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosisConfidence {
    /// Model/agent explanation, no reproducible evidence yet.
    Hypothesis,
    /// Correlated evidence supports the explanation but it is not yet
    /// reproduced.
    Supported,
    /// A minimal reproduction exhibits the failure.
    Reproduced,
    /// Reproducible evidence plus verification support the explanation.
    Validated,
}

impl DiagnosisConfidence {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hypothesis => "HYPOTHESIS",
            Self::Supported => "SUPPORTED",
            Self::Reproduced => "REPRODUCED",
            Self::Validated => "VALIDATED",
        }
    }

    pub const fn is_authoritative(self) -> bool {
        matches!(self, Self::Validated)
    }

    pub const ALL: [DiagnosisConfidence; 4] = [
        Self::Hypothesis,
        Self::Supported,
        Self::Reproduced,
        Self::Validated,
    ];
}

impl fmt::Display for DiagnosisConfidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for DiagnosisConfidence {
    type Err = super::error::HealingError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "HYPOTHESIS" => Ok(Self::Hypothesis),
            "SUPPORTED" => Ok(Self::Supported),
            "REPRODUCED" => Ok(Self::Reproduced),
            "VALIDATED" => Ok(Self::Validated),
            other => Err(super::error::HealingError::vocabulary(
                "DiagnosisConfidence",
                other,
            )),
        }
    }
}
