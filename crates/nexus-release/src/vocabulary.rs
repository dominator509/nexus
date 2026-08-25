//! EP-042 release/update/rollback vocabularies (SPEC-016, SPEC-024
//! canonical terms).
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

/// Canonical deployment profile modes (SPEC-016; mirrored from
/// `schemas/deployment-profile.schema.json` `mode`).
///
/// One signed distribution supports every profile; the profile selects
/// placement, never a separate distribution.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeploymentProfileMode {
    /// Nexus-operated managed control plane.
    Managed,
    /// Bring your own cloud credentials.
    Byoc,
    /// Provision onto an existing SSH-accessible host.
    ExistingSsh,
    /// Mixed managed and self-hosted placement.
    Hybrid,
    /// Fully local, no cloud dependency.
    FullyLocal,
}

impl DeploymentProfileMode {
    pub const VOCAB: &'static str = "deployment profile mode";

    pub const ALL: [DeploymentProfileMode; 5] = [
        Self::Managed,
        Self::Byoc,
        Self::ExistingSsh,
        Self::Hybrid,
        Self::FullyLocal,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Managed => "MANAGED",
            Self::Byoc => "BYOC",
            Self::ExistingSsh => "EXISTING_SSH",
            Self::Hybrid => "HYBRID",
            Self::FullyLocal => "FULLY_LOCAL",
        }
    }
}

impl FromStr for DeploymentProfileMode {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "MANAGED" => Ok(Self::Managed),
            "BYOC" => Ok(Self::Byoc),
            "EXISTING_SSH" => Ok(Self::ExistingSsh),
            "HYBRID" => Ok(Self::Hybrid),
            "FULLY_LOCAL" => Ok(Self::FullyLocal),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for DeploymentProfileMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical release channels (SPEC-016 behavior 7; mirrored from
/// `schemas/deployment-profile.schema.json` `release_channel`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReleaseChannel {
    Stable,
    Beta,
    Developer,
    Pinned,
}

impl ReleaseChannel {
    pub const VOCAB: &'static str = "release channel";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "STABLE",
            Self::Beta => "BETA",
            Self::Developer => "DEVELOPER",
            Self::Pinned => "PINNED",
        }
    }
}

impl FromStr for ReleaseChannel {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "STABLE" => Ok(Self::Stable),
            "BETA" => Ok(Self::Beta),
            "DEVELOPER" => Ok(Self::Developer),
            "PINNED" => Ok(Self::Pinned),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for ReleaseChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Signature algorithms supported by the release contract. The contract
/// surface is honest: only one algorithm is declared; anything else is
/// rejected (deny-unknown), never silently treated as valid.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignatureAlgorithm {
    Ed25519,
}

impl SignatureAlgorithm {
    pub const VOCAB: &'static str = "signature algorithm";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ed25519 => "ED25519",
        }
    }
}

impl FromStr for SignatureAlgorithm {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ED25519" => Ok(Self::Ed25519),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for SignatureAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Signature state ladder (SPEC-016 behavior 6).
///
/// `SIGNATURE PRESENT != SIGNATURE VALID`: a signature field existing on a
/// component says nothing about whether it verifies. The state is produced
/// by verification, never by presence.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignatureState {
    /// No signature was supplied.
    Unverified,
    /// A signature value is present but has not been verified.
    Present,
    /// The signature verified against the declared key.
    Valid,
    /// The signature failed verification.
    Invalid,
}

impl SignatureState {
    pub const VOCAB: &'static str = "signature state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "UNVERIFIED",
            Self::Present => "PRESENT",
            Self::Valid => "VALID",
            Self::Invalid => "INVALID",
        }
    }
}

impl FromStr for SignatureState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNVERIFIED" => Ok(Self::Unverified),
            "PRESENT" => Ok(Self::Present),
            "VALID" => Ok(Self::Valid),
            "INVALID" => Ok(Self::Invalid),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for SignatureState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Update step kinds (SPEC-016 behavior 6).
///
/// There is intentionally NO `PROMOTE` step: production promotion is an
/// exact manual action outside the transactional update plan. A plan that
/// contains promotion is invalid by construction.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpdateStepKind {
    Backup,
    Migrate,
    Canary,
    Observe,
    Rollback,
}

impl UpdateStepKind {
    pub const VOCAB: &'static str = "update step kind";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Backup => "BACKUP",
            Self::Migrate => "MIGRATE",
            Self::Canary => "CANARY",
            Self::Observe => "OBSERVE",
            Self::Rollback => "ROLLBACK",
        }
    }
}

impl FromStr for UpdateStepKind {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BACKUP" => Ok(Self::Backup),
            "MIGRATE" => Ok(Self::Migrate),
            "CANARY" => Ok(Self::Canary),
            "OBSERVE" => Ok(Self::Observe),
            "ROLLBACK" => Ok(Self::Rollback),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for UpdateStepKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Update lifecycle states (SPEC-016 behavior 6).
///
/// `READY_TO_PROMOTE` is the furthest an update engine may take a canary;
/// the promotion itself remains an exact manual action.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UpdateState {
    Planned,
    Pending,
    InProgress,
    Observing,
    ReadyToPromote,
    RolledBack,
    Failed,
}

impl UpdateState {
    pub const VOCAB: &'static str = "update state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "PLANNED",
            Self::Pending => "PENDING",
            Self::InProgress => "IN_PROGRESS",
            Self::Observing => "OBSERVING",
            Self::ReadyToPromote => "READY_TO_PROMOTE",
            Self::RolledBack => "ROLLED_BACK",
            Self::Failed => "FAILED",
        }
    }
}

impl FromStr for UpdateState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PLANNED" => Ok(Self::Planned),
            "PENDING" => Ok(Self::Pending),
            "IN_PROGRESS" => Ok(Self::InProgress),
            "OBSERVING" => Ok(Self::Observing),
            "READY_TO_PROMOTE" => Ok(Self::ReadyToPromote),
            "ROLLED_BACK" => Ok(Self::RolledBack),
            "FAILED" => Ok(Self::Failed),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for UpdateState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canary verdicts (SPEC-016 behavior 6).
///
/// A canary can never promote; it can only recommend. `READY_TO_PROMOTE`
/// means the observation window is healthy and a human may now perform the
/// exact manual promotion action.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CanaryVerdict {
    Observing,
    ReadyToPromote,
    Rollback,
}

impl CanaryVerdict {
    pub const VOCAB: &'static str = "canary verdict";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Observing => "OBSERVING",
            Self::ReadyToPromote => "READY_TO_PROMOTE",
            Self::Rollback => "ROLLBACK",
        }
    }
}

impl FromStr for CanaryVerdict {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "OBSERVING" => Ok(Self::Observing),
            "READY_TO_PROMOTE" => Ok(Self::ReadyToPromote),
            "ROLLBACK" => Ok(Self::Rollback),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for CanaryVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Verification states shared by digest, signature, and receipt proofs.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationState {
    Unverified,
    Verified,
    Mismatch,
    Missing,
}

impl VerificationState {
    pub const VOCAB: &'static str = "verification state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "UNVERIFIED",
            Self::Verified => "VERIFIED",
            Self::Mismatch => "MISMATCH",
            Self::Missing => "MISSING",
        }
    }
}

impl FromStr for VerificationState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "UNVERIFIED" => Ok(Self::Unverified),
            "VERIFIED" => Ok(Self::Verified),
            "MISMATCH" => Ok(Self::Mismatch),
            "MISSING" => Ok(Self::Missing),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for VerificationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Offline bundle content kinds (SPEC-016 behavior 5).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BundleKind {
    Image,
    Model,
    License,
    Sbom,
    Migration,
    RecoveryTool,
}

impl BundleKind {
    pub const VOCAB: &'static str = "bundle kind";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Image => "IMAGE",
            Self::Model => "MODEL",
            Self::License => "LICENSE",
            Self::Sbom => "SBOM",
            Self::Migration => "MIGRATION",
            Self::RecoveryTool => "RECOVERY_TOOL",
        }
    }
}

impl FromStr for BundleKind {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "IMAGE" => Ok(Self::Image),
            "MODEL" => Ok(Self::Model),
            "LICENSE" => Ok(Self::License),
            "SBOM" => Ok(Self::Sbom),
            "MIGRATION" => Ok(Self::Migration),
            "RECOVERY_TOOL" => Ok(Self::RecoveryTool),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for BundleKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Rollback lifecycle states (SPEC-016 behavior 6).
///
/// `REQUIRES_BACKUP` is the initial state: a rollback cannot even be
/// planned without a verified backup reference.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RollbackState {
    RequiresBackup,
    BackupVerified,
    RollbackVerified,
    Failed,
}

impl RollbackState {
    pub const VOCAB: &'static str = "rollback state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RequiresBackup => "REQUIRES_BACKUP",
            Self::BackupVerified => "BACKUP_VERIFIED",
            Self::RollbackVerified => "ROLLBACK_VERIFIED",
            Self::Failed => "FAILED",
        }
    }
}

impl FromStr for RollbackState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "REQUIRES_BACKUP" => Ok(Self::RequiresBackup),
            "BACKUP_VERIFIED" => Ok(Self::BackupVerified),
            "ROLLBACK_VERIFIED" => Ok(Self::RollbackVerified),
            "FAILED" => Ok(Self::Failed),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for RollbackState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Promotion gate states (SPEC-016 behavior 7).
///
/// Promotion is locked until a human approval record exists; approval
/// authorizes an exact manual command, never an automatic deployment.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PromotionState {
    Locked,
    AwaitingHumanApproval,
    ApprovedManualOnly,
}

impl PromotionState {
    pub const VOCAB: &'static str = "promotion state";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Locked => "LOCKED",
            Self::AwaitingHumanApproval => "AWAITING_HUMAN_APPROVAL",
            Self::ApprovedManualOnly => "APPROVED_MANUAL_ONLY",
        }
    }
}

impl FromStr for PromotionState {
    type Err = VocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "LOCKED" => Ok(Self::Locked),
            "AWAITING_HUMAN_APPROVAL" => Ok(Self::AwaitingHumanApproval),
            "APPROVED_MANUAL_ONLY" => Ok(Self::ApprovedManualOnly),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for PromotionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep042_unit_vocabulary_deny_unknown_modes() {
        assert!(DeploymentProfileMode::from_str("CLOUD_ONLY").is_err());
        assert!(DeploymentProfileMode::from_str("").is_err());
        assert!(DeploymentProfileMode::from_str("local").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_deny_unknown_channels() {
        assert!(ReleaseChannel::from_str("NIGHTLY").is_err());
        assert!(ReleaseChannel::from_str("canary").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_deny_unknown_signature_algorithm() {
        assert!(SignatureAlgorithm::from_str("RSA").is_err());
        assert!(SignatureAlgorithm::from_str("ECDSA").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_deny_unknown_signature_state() {
        assert!(SignatureState::from_str("TRUSTED").is_err());
        assert!(SignatureState::from_str("").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_update_step_kind_has_no_promote() {
        // A plan step kind may never contain PROMOTE: promotion is manual.
        assert!(UpdateStepKind::from_str("PROMOTE").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_deny_unknown_update_state() {
        assert!(UpdateState::from_str("DEPLOYED").is_err());
        assert!(UpdateState::from_str("LIVE").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_canary_never_promotes() {
        // The canary verdict vocabulary itself has no PROMOTED variant.
        assert!(CanaryVerdict::from_str("PROMOTED").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_deny_unknown_verification_state() {
        assert!(VerificationState::from_str("PARTIAL").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_deny_unknown_bundle_kind() {
        assert!(BundleKind::from_str("CONFIG").is_err());
        assert!(BundleKind::from_str("SECRET").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_deny_unknown_rollback_state() {
        assert!(RollbackState::from_str("CLEAN").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_deny_unknown_promotion_state() {
        assert!(PromotionState::from_str("AUTO_APPROVED").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_serde_rejects_unknown_wire_value() {
        for json in [
            "\"CLOUD_ONLY\"",
            "\"NIGHTLY\"",
            "\"RSA\"",
            "\"TRUSTED\"",
            "\"PROMOTE\"",
            "\"DEPLOYED\"",
            "\"PROMOTED\"",
            "\"PARTIAL\"",
            "\"CONFIG\"",
            "\"CLEAN\"",
            "\"AUTO_APPROVED\"",
        ] {
            assert!(
                serde_json::from_str::<serde_json::Value>(json).is_ok(),
                "test fixture must be valid JSON"
            );
        }
        assert!(serde_json::from_str::<DeploymentProfileMode>("\"CLOUD_ONLY\"").is_err());
        assert!(serde_json::from_str::<ReleaseChannel>("\"NIGHTLY\"").is_err());
        assert!(serde_json::from_str::<SignatureAlgorithm>("\"RSA\"").is_err());
        assert!(serde_json::from_str::<SignatureState>("\"TRUSTED\"").is_err());
        assert!(serde_json::from_str::<UpdateStepKind>("\"PROMOTE\"").is_err());
        assert!(serde_json::from_str::<UpdateState>("\"DEPLOYED\"").is_err());
        assert!(serde_json::from_str::<CanaryVerdict>("\"PROMOTED\"").is_err());
        assert!(serde_json::from_str::<VerificationState>("\"PARTIAL\"").is_err());
        assert!(serde_json::from_str::<BundleKind>("\"CONFIG\"").is_err());
        assert!(serde_json::from_str::<RollbackState>("\"CLEAN\"").is_err());
        assert!(serde_json::from_str::<PromotionState>("\"AUTO_APPROVED\"").is_err());
    }

    #[test]
    fn ep042_unit_vocabulary_roundtrip_all_known() {
        for mode in DeploymentProfileMode::ALL {
            let wire = mode.as_str();
            assert_eq!(DeploymentProfileMode::from_str(wire).unwrap(), mode);
            let json = serde_json::to_string(&mode).unwrap();
            assert_eq!(
                serde_json::from_str::<DeploymentProfileMode>(&json).unwrap(),
                mode
            );
        }
        for channel in [
            ReleaseChannel::Stable,
            ReleaseChannel::Beta,
            ReleaseChannel::Developer,
            ReleaseChannel::Pinned,
        ] {
            let wire = channel.as_str();
            assert_eq!(ReleaseChannel::from_str(wire).unwrap(), channel);
            let json = serde_json::to_string(&channel).unwrap();
            assert_eq!(
                serde_json::from_str::<ReleaseChannel>(&json).unwrap(),
                channel
            );
        }
        assert_eq!(
            SignatureAlgorithm::from_str("ED25519").unwrap(),
            SignatureAlgorithm::Ed25519
        );
        for state in [
            SignatureState::Unverified,
            SignatureState::Present,
            SignatureState::Valid,
            SignatureState::Invalid,
        ] {
            let wire = state.as_str();
            assert_eq!(SignatureState::from_str(wire).unwrap(), state);
        }
        for kind in [
            UpdateStepKind::Backup,
            UpdateStepKind::Migrate,
            UpdateStepKind::Canary,
            UpdateStepKind::Observe,
            UpdateStepKind::Rollback,
        ] {
            let wire = kind.as_str();
            assert_eq!(UpdateStepKind::from_str(wire).unwrap(), kind);
        }
        for state in [
            UpdateState::Planned,
            UpdateState::Pending,
            UpdateState::InProgress,
            UpdateState::Observing,
            UpdateState::ReadyToPromote,
            UpdateState::RolledBack,
            UpdateState::Failed,
        ] {
            let wire = state.as_str();
            assert_eq!(UpdateState::from_str(wire).unwrap(), state);
        }
        for verdict in [
            CanaryVerdict::Observing,
            CanaryVerdict::ReadyToPromote,
            CanaryVerdict::Rollback,
        ] {
            let wire = verdict.as_str();
            assert_eq!(CanaryVerdict::from_str(wire).unwrap(), verdict);
        }
        for state in [
            VerificationState::Unverified,
            VerificationState::Verified,
            VerificationState::Mismatch,
            VerificationState::Missing,
        ] {
            let wire = state.as_str();
            assert_eq!(VerificationState::from_str(wire).unwrap(), state);
        }
        for kind in [
            BundleKind::Image,
            BundleKind::Model,
            BundleKind::License,
            BundleKind::Sbom,
            BundleKind::Migration,
            BundleKind::RecoveryTool,
        ] {
            let wire = kind.as_str();
            assert_eq!(BundleKind::from_str(wire).unwrap(), kind);
        }
        for state in [
            RollbackState::RequiresBackup,
            RollbackState::BackupVerified,
            RollbackState::RollbackVerified,
            RollbackState::Failed,
        ] {
            let wire = state.as_str();
            assert_eq!(RollbackState::from_str(wire).unwrap(), state);
        }
        for state in [
            PromotionState::Locked,
            PromotionState::AwaitingHumanApproval,
            PromotionState::ApprovedManualOnly,
        ] {
            let wire = state.as_str();
            assert_eq!(PromotionState::from_str(wire).unwrap(), state);
        }
    }
}
