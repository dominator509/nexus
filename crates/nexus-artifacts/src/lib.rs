//! EP-037 ArtifactStore, encrypted backup, restore, and migration
//! contracts (SPEC-024).
//!
//! Provider-neutral artifact storage: local filesystem, NAS, SeaweedFS,
//! MinIO compatibility, Cloudflare R2, Backblaze B2, and Amazon S3 behind
//! ONE contract. Artifacts are content-addressed and versioned; backups
//! encrypt before leaving the node; restore and backend migration verify
//! hashes before deletion. Truthfulness is structural: a written artifact
//! is not a verified artifact, a backup created is not a restore proven,
//! a backend declaration is not a benchmark, and MinIO is compatibility
//! only (community repository archived).
//!
//! Dependency direction: this crate depends only on nexus-domain and
//! serde/serde_json. No storage SDK, transport, or framework crate
//! appears.

#![forbid(unsafe_code)]

pub mod error;
pub mod model;
pub mod port;
pub mod vocabulary;

pub use error::{ArtifactError, ArtifactErrorCode, ArtifactResult};
pub use model::{
    hex_decode, hex_encode, ArtifactHash, ArtifactMetadata, ArtifactVersion, BackendLocation,
    BackupSet, EncryptionMetadata, ManifestSignature, ManifestSignatureAlgorithm, ObjectRef,
    RecoveryKey, RestorePlan, StorageMigration,
};
pub use nexus_domain::{ArtifactId, CorrelationId, TenantId};
pub use port::ArtifactStore;
pub use vocabulary::{
    BackupState, DataClass, MigrationState, RestoreVerificationState, RetentionClass,
    StorageBackend,
};
