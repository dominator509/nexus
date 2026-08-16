//! EP-019 canary plan contracts (SPEC-018; ADR-026).
//!
//! Staged deployment: validated artifact -> canary/single instance ->
//! health/readiness -> targeted verification -> broader rollout. A
//! canary regression automatically rolls back and preserves evidence.
//! Real production canary certification is deferred to the node that
//! owns deployment; this crate records the staging contract and proves
//! the deterministic state machine.

use serde::{Deserialize, Serialize};

/// Canary lifecycle state (ADR-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CanaryState {
    /// Canary planned but not yet started.
    Planned,
    /// Validated artifact staged to a canary/single instance.
    Validating,
    /// Health/readiness and targeted verification passed.
    Healthy,
    /// Broader rollout approved after canary health.
    Promoted,
    /// Canary regression triggered automatic rollback.
    RolledBack,
    /// Canary failed and rollback also failed (blocked).
    Failed,
}

impl CanaryState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "PLANNED",
            Self::Validating => "VALIDATING",
            Self::Healthy => "HEALTHY",
            Self::Promoted => "PROMOTED",
            Self::RolledBack => "ROLLED_BACK",
            Self::Failed => "FAILED",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Healthy | Self::Promoted | Self::RolledBack | Self::Failed
        )
    }

    pub const ALL: [CanaryState; 6] = [
        Self::Planned,
        Self::Validating,
        Self::Healthy,
        Self::Promoted,
        Self::RolledBack,
        Self::Failed,
    ];
}

impl std::fmt::Display for CanaryState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for CanaryState {
    type Err = crate::error::HealingError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PLANNED" => Ok(Self::Planned),
            "VALIDATING" => Ok(Self::Validating),
            "HEALTHY" => Ok(Self::Healthy),
            "PROMOTED" => Ok(Self::Promoted),
            "ROLLED_BACK" => Ok(Self::RolledBack),
            "FAILED" => Ok(Self::Failed),
            other => Err(crate::error::HealingError::vocabulary("CanaryState", other)),
        }
    }
}

/// Health criterion observed during canary (SPEC-018 canonical term
/// `HealthCriterion`). Deployment success is not remediation success:
/// health/readiness and the original reproduction must both pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthCriterion {
    pub name: String,
    pub expected: HealthCriterionState,
    pub observed: Option<HealthCriterionState>,
}

/// Observed health state of a criterion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HealthCriterionState {
    Healthy,
    Degraded,
    Unavailable,
    Unknown,
}

impl HealthCriterionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "HEALTHY",
            Self::Degraded => "DEGRADED",
            Self::Unavailable => "UNAVAILABLE",
            Self::Unknown => "UNKNOWN",
        }
    }

    pub const fn is_healthy(self) -> bool {
        matches!(self, Self::Healthy)
    }
}

impl std::fmt::Display for HealthCriterionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for HealthCriterionState {
    type Err = crate::error::HealingError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "HEALTHY" => Ok(Self::Healthy),
            "DEGRADED" => Ok(Self::Degraded),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            "UNKNOWN" => Ok(Self::Unknown),
            other => Err(crate::error::HealingError::vocabulary(
                "HealthCriterionState",
                other,
            )),
        }
    }
}

/// Staged deployment plan (directive section 13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanaryPlan {
    /// Stages in promotion order (e.g. canary -> targeted -> broader).
    pub stages: Vec<String>,
    /// Health/readiness criteria for the canary stage.
    pub health_criteria: Vec<HealthCriterion>,
    /// Canonical patch digest the plan validates.
    pub patch_digest: String,
    /// Whether a failing criterion auto-rolls back.
    pub auto_rollback_on_regression: bool,
    /// Current canary state.
    pub state: CanaryState,
}
