//! EP-019 rollback plan contracts (SPEC-018; ADR-026).
//!
//! Deterministic rollback state machine: version N healthy -> deploy
//! N+1 -> verification fails -> rollback -> version N restored ->
//! health restored. Where production deployment is later-owned, this
//! crate proves the deterministic rollback state machine now and assigns
//! the real deployment proof to its owner. Rollback is NEVER improvised
//! from model-generated source.

use serde::{Deserialize, Serialize};

/// Rollback lifecycle state (ADR-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RollbackState {
    /// Rollback planned against a known previous artifact/version.
    Planned,
    /// Rollback executing against the known previous artifact.
    Executing,
    /// Previous version restored and health verified.
    Restored,
    /// Rollback failed (blocked with evidence).
    Failed,
}

impl RollbackState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "PLANNED",
            Self::Executing => "EXECUTING",
            Self::Restored => "RESTORED",
            Self::Failed => "FAILED",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Restored | Self::Failed)
    }

    pub const ALL: [RollbackState; 4] =
        [Self::Planned, Self::Executing, Self::Restored, Self::Failed];
}

impl std::fmt::Display for RollbackState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RollbackState {
    type Err = crate::error::HealingError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PLANNED" => Ok(Self::Planned),
            "EXECUTING" => Ok(Self::Executing),
            "RESTORED" => Ok(Self::Restored),
            "FAILED" => Ok(Self::Failed),
            other => Err(crate::error::HealingError::vocabulary(
                "RollbackState",
                other,
            )),
        }
    }
}

/// Rollback plan (SPEC-018 canonical term `Rollback`; directive
/// section 15). Bound to the known previous artifact/version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackPlan {
    /// Canonical rollback identifier.
    pub rollback_id: nexus_domain::RollbackId,
    /// Known previous artifact/version to restore (never model-generated).
    pub previous_artifact: String,
    /// Deployment/version being rolled back.
    pub deployed_version: String,
    /// Ordered rollback steps.
    pub steps: Vec<String>,
    /// Current rollback state.
    pub state: RollbackState,
    /// Whether health restoration has been verified.
    pub health_verified: bool,
}
