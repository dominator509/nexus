//! nexus-release: provider-neutral deployment, release, update, and
//! rollback contracts (SPEC-016, SPEC-024; EP-042 M1).
//!
//! This crate owns the canonical release lifecycle model: the signed
//! release manifest, signed components, the compatibility matrix, the
//! transactional update plan, the canary ring, the rollback receipt, the
//! offline bundle, and the manual promotion decision.
//!
//! M1 is the contract layer only. No installer, update engine, signature
//! key store, artifact transport, or rollback executor exists in M1;
//! real signature verification, update execution, canary rollout,
//! backup/restore, and rollback drills are NOT asserted until later
//! milestones.
//!
//! Permanent invariants encoded here and proven by tests:
//! - RELEASE MANIFEST EXISTS != RELEASE VERIFIED
//! - SIGNATURE PRESENT != SIGNATURE VALID
//! - UPDATE PLAN EXISTS != UPDATE EXECUTED (and a plan never promotes)
//! - CANARY OBSERVING != PROMOTED
//! - PROMOTION DECISION != DEPLOYMENT (ManualPromotion never deploys)
//! - ROLLBACK RECEIPT REQUIRES BACKUP REF (backup-before-update)
//! - OFFLINE BUNDLE EXISTS != OFFLINE BUNDLE VERIFIED
//! - ONE SIGNED DISTRIBUTION SUPPORTS MANAGED/BYOC/EXISTING_SSH/HYBRID/FULLY_LOCAL
//! - Every public vocabulary is deny-unknown; every public record is
//!   versioned and serializes deterministically.

pub mod error;
pub mod model;
pub mod vocabulary;

pub use error::{ReleaseError, ReleaseErrorCode, ReleaseResult};
pub use model::{
    BundleItem, CanaryRing, CompatibilityEntry, CompatibilityMatrix, CompatibleVerdict, Digest,
    ManualPromotion, ObjectRef, OfflineBundle, PromotionRecord, ReleaseManifest, RollbackReceipt,
    Signature, SignedComponent, UpdatePlan, UpdateStep,
};
pub use vocabulary::{
    BundleKind, CanaryVerdict, DeploymentProfileMode, PromotionState, ReleaseChannel,
    RollbackState, SignatureAlgorithm, SignatureState, UpdateState, UpdateStepKind,
    VerificationState, VocabularyError,
};
