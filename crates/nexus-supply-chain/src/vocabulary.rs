//! EP-039 supply-chain vocabularies (SPEC-019 canonical terms).
//!
//! Every public vocabulary is deny-unknown: arbitrary strings can never
//! silently become valid contract states. Each enum has a canonical
//! `as_str` form, a `FromStr` that rejects unknown values, and serde
//! serialization that fails closed on unknown wire values.

use std::fmt;
use std::str::FromStr;

/// Rejection reason for an unknown vocabulary value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocabularyError(pub &'static str);

impl fmt::Display for VocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown {} value", self.0)
    }
}

impl std::error::Error for VocabularyError {}

/// Canonical license classes (LICENSE_POLICY.md; SPEC-019 behavior 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseClass {
    /// Permissive: MIT, Apache-2.0, BSD, ISC, PostgreSQL, PSF, equivalent.
    Green,
    /// Obligation analysis required: MPL-2.0, LGPL (file-level/dynamic link).
    Review,
    /// Copyleft: GPL, AGPL - process or appliance isolation required.
    Sidecar,
    /// Commercial API or user-owned appliance governed by provider terms.
    External,
    /// Noncommercial code/model weights, unclear provenance, incompatible.
    Prohibited,
}

impl LicenseClass {
    pub const VOCAB: &'static str = "license class";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Green => "GREEN",
            Self::Review => "REVIEW",
            Self::Sidecar => "SIDECAR",
            Self::External => "EXTERNAL",
            Self::Prohibited => "PROHIBITED",
        }
    }
}

impl fmt::Display for LicenseClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LicenseClass {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GREEN" => Ok(Self::Green),
            "REVIEW" => Ok(Self::Review),
            "SIDECAR" => Ok(Self::Sidecar),
            "EXTERNAL" => Ok(Self::External),
            "PROHIBITED" => Ok(Self::Prohibited),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Outcome of a license review for a specific component+version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LicenseReview {
    /// Approved for the exact component+version by the review process.
    Approved,
    /// Denied: unknown/missing/disallowed license fails closed.
    Denied,
    /// Obligation analysis required before any embedding decision.
    NeedsReview,
}

impl LicenseReview {
    pub const VOCAB: &'static str = "license review";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "APPROVED",
            Self::Denied => "DENIED",
            Self::NeedsReview => "NEEDS_REVIEW",
        }
    }
}

impl fmt::Display for LicenseReview {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LicenseReview {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "APPROVED" => Ok(Self::Approved),
            "DENIED" => Ok(Self::Denied),
            "NEEDS_REVIEW" => Ok(Self::NeedsReview),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// How a component integrates into the product (SPEC-019 behavior 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IntegrationMode {
    /// Linked or embedded directly in the process.
    Embedded,
    /// Separate process or appliance, communicating through documented APIs.
    ProcessSidecar,
    /// Remote provider / API governed by provider terms.
    ExternalProvider,
}

impl IntegrationMode {
    pub const VOCAB: &'static str = "integration mode";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Embedded => "EMBEDDED",
            Self::ProcessSidecar => "PROCESS_SIDECAR",
            Self::ExternalProvider => "EXTERNAL_PROVIDER",
        }
    }
}

impl fmt::Display for IntegrationMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for IntegrationMode {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "EMBEDDED" => Ok(Self::Embedded),
            "PROCESS_SIDECAR" => Ok(Self::ProcessSidecar),
            "EXTERNAL_PROVIDER" => Ok(Self::ExternalProvider),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Approval state for a dependency/component admission decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApprovalState {
    /// Explicitly approved by the review process for the exact version.
    Approved,
    /// Rejected by the review process.
    Rejected,
    /// Not yet reviewed - never treated as approved.
    Pending,
}

impl ApprovalState {
    pub const VOCAB: &'static str = "approval state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
            Self::Pending => "PENDING",
        }
    }
}

impl fmt::Display for ApprovalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ApprovalState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "APPROVED" => Ok(Self::Approved),
            "REJECTED" => Ok(Self::Rejected),
            "PENDING" => Ok(Self::Pending),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Risk classification for a supply-chain component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}

impl RiskClass {
    pub const VOCAB: &'static str = "risk class";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

impl fmt::Display for RiskClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RiskClass {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LOW" => Ok(Self::Low),
            "MEDIUM" => Ok(Self::Medium),
            "HIGH" => Ok(Self::High),
            "CRITICAL" => Ok(Self::Critical),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Result of a verification operation (never inferred from presence).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationResult {
    /// Verified against real evidence (digest/readback/signature).
    Verified,
    /// Verification was attempted and failed - fail closed.
    NotVerified,
    /// Verification has not been attempted yet.
    Unverified,
}

impl VerificationResult {
    pub const VOCAB: &'static str = "verification result";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "VERIFIED",
            Self::NotVerified => "NOT_VERIFIED",
            Self::Unverified => "UNVERIFIED",
        }
    }
}

impl fmt::Display for VerificationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VerificationResult {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VERIFIED" => Ok(Self::Verified),
            "NOT_VERIFIED" => Ok(Self::NotVerified),
            "UNVERIFIED" => Ok(Self::Unverified),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// State of a dependency waiver (SPEC-019 behavior 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WaiverState {
    Active,
    Expired,
    Revoked,
}

impl WaiverState {
    pub const VOCAB: &'static str = "waiver state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "ACTIVE",
            Self::Expired => "EXPIRED",
            Self::Revoked => "REVOKED",
        }
    }
}

impl fmt::Display for WaiverState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WaiverState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ACTIVE" => Ok(Self::Active),
            "EXPIRED" => Ok(Self::Expired),
            "REVOKED" => Ok(Self::Revoked),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

/// Severity of an advisory (SPEC-019 behavior 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdvisorySeverity {
    Info,
    Low,
    Moderate,
    High,
    Critical,
}

impl AdvisorySeverity {
    pub const VOCAB: &'static str = "advisory severity";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Low => "LOW",
            Self::Moderate => "MODERATE",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }

    /// Critical advisories block release unless mitigated by a bounded ADR.
    pub fn blocks_release(self) -> bool {
        matches!(self, Self::Critical)
    }
}

impl fmt::Display for AdvisorySeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AdvisorySeverity {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "INFO" => Ok(Self::Info),
            "LOW" => Ok(Self::Low),
            "MODERATE" => Ok(Self::Moderate),
            "HIGH" => Ok(Self::High),
            "CRITICAL" => Ok(Self::Critical),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}
