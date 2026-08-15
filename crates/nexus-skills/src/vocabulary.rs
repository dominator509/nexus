//! EP-018 skill vocabulary (SPEC-010; ADR-025).
//!
//! Vocabulary-locked enums for Agent Skills: trust tiers, permission
//! grants, signature algorithms, and proposal lifecycle. Unknown
//! values are rejected at parse time; no free-form strings become
//! domain contracts.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Skill trust tier (SPEC-010 canonical term `Skill Trust`).
/// Community skills begin inspect-only or sandboxed; higher tiers are
/// earned through evals and human promotion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillTrustLevel {
    InspectOnly,
    Sandboxed,
    Trusted,
    System,
}

impl SkillTrustLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InspectOnly => "INSPECT_ONLY",
            Self::Sandboxed => "SANDBOXED",
            Self::Trusted => "TRUSTED",
            Self::System => "SYSTEM",
        }
    }

    /// The minimum tier required to install a skill with declared
    /// permissions beyond inspect.
    pub const fn min_install_tier(self) -> SkillTrustLevel {
        Self::Sandboxed
    }

    /// The deterministic permission ceiling for a trust tier (ADR-025).
    /// A skill may REQUEST permissions up to this ceiling, but a
    /// request is never a grant: authorization still requires the
    /// caller's grant, tenant policy, and EP-008 authorization.
    /// Community/untrusted skills are sandboxed by construction and can
    /// never request privileged host authority.
    pub const fn permission_ceiling(self) -> SkillPermission {
        match self {
            Self::InspectOnly => SkillPermission::None,
            Self::Sandboxed => SkillPermission::Read,
            Self::Trusted => SkillPermission::Execute,
            Self::System => SkillPermission::Secrets,
        }
    }

    pub const ALL: [SkillTrustLevel; 4] = [
        Self::InspectOnly,
        Self::Sandboxed,
        Self::Trusted,
        Self::System,
    ];
}

impl fmt::Display for SkillTrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SkillTrustLevel {
    type Err = super::package::SkillPackageError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "INSPECT_ONLY" => Ok(Self::InspectOnly),
            "SANDBOXED" => Ok(Self::Sandboxed),
            "TRUSTED" => Ok(Self::Trusted),
            "SYSTEM" => Ok(Self::System),
            other => Err(super::package::SkillPackageError::vocabulary(
                "SkillTrustLevel",
                other,
            )),
        }
    }
}

/// Declared permission a skill requests. Nexus policy may narrow but
/// never widen; a skill cannot request undeclared permissions at
/// runtime (SPEC-010 behavior 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillPermission {
    #[default]
    None,
    Read,
    Write,
    Execute,
    Network,
    Secrets,
}

impl SkillPermission {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Read => "READ",
            Self::Write => "WRITE",
            Self::Execute => "EXECUTE",
            Self::Network => "NETWORK",
            Self::Secrets => "SECRETS",
        }
    }

    pub const ALL: [SkillPermission; 6] = [
        Self::None,
        Self::Read,
        Self::Write,
        Self::Execute,
        Self::Network,
        Self::Secrets,
    ];
}

impl fmt::Display for SkillPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SkillPermission {
    type Err = super::package::SkillPackageError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "NONE" => Ok(Self::None),
            "READ" => Ok(Self::Read),
            "WRITE" => Ok(Self::Write),
            "EXECUTE" => Ok(Self::Execute),
            "NETWORK" => Ok(Self::Network),
            "SECRETS" => Ok(Self::Secrets),
            other => Err(super::package::SkillPackageError::vocabulary(
                "SkillPermission",
                other,
            )),
        }
    }
}

/// Signature algorithm of a signed skill package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignatureAlgorithm {
    Ed25519,
    EcdsaP256,
}

impl SignatureAlgorithm {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ed25519 => "ED25519",
            Self::EcdsaP256 => "ECDSA_P256",
        }
    }
}

impl fmt::Display for SignatureAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SignatureAlgorithm {
    type Err = super::package::SkillPackageError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ED25519" => Ok(Self::Ed25519),
            "ECDSA_P256" => Ok(Self::EcdsaP256),
            other => Err(super::package::SkillPackageError::vocabulary(
                "SignatureAlgorithm",
                other,
            )),
        }
    }
}

/// Proposal lifecycle (SPEC-010 behavior 8: Skill Factory creates
/// candidates, tests against frozen evals, requests human promotion,
/// retains rollback versions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillProposalState {
    Proposed,
    EvalPending,
    EvalPassed,
    EvalFailed,
    AwaitingPromotion,
    Promoted,
    Rejected,
    RolledBack,
}

impl SkillProposalState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "PROPOSED",
            Self::EvalPending => "EVAL_PENDING",
            Self::EvalPassed => "EVAL_PASSED",
            Self::EvalFailed => "EVAL_FAILED",
            Self::AwaitingPromotion => "AWAITING_PROMOTION",
            Self::Promoted => "PROMOTED",
            Self::Rejected => "REJECTED",
            Self::RolledBack => "ROLLED_BACK",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Promoted | Self::Rejected | Self::EvalFailed | Self::RolledBack
        )
    }
}

impl fmt::Display for SkillProposalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SkillProposalState {
    type Err = super::package::SkillPackageError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PROPOSED" => Ok(Self::Proposed),
            "EVAL_PENDING" => Ok(Self::EvalPending),
            "EVAL_PASSED" => Ok(Self::EvalPassed),
            "EVAL_FAILED" => Ok(Self::EvalFailed),
            "AWAITING_PROMOTION" => Ok(Self::AwaitingPromotion),
            "PROMOTED" => Ok(Self::Promoted),
            "REJECTED" => Ok(Self::Rejected),
            "ROLLED_BACK" => Ok(Self::RolledBack),
            other => Err(super::package::SkillPackageError::vocabulary(
                "SkillProposalState",
                other,
            )),
        }
    }
}
