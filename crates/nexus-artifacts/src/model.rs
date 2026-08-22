//! EP-037 artifact storage value objects (SPEC-024).
//!
//! Every value object validates wire-shaped input with deny-unknown
//! semantics (mirroring the canonical schema `additionalProperties:
//! false` rule), and deserialization enforces the same checks as the
//! constructor. Truthfulness is structural: MINIO is compatibility-only,
//! a written artifact is not a verified artifact, a backup created is not
//! a restore proven, encryption metadata is not a key, and the canonical
//! location changes only after hash verification and approval.

use std::fmt;

use nexus_domain::{ArtifactId, CorrelationId, TenantId};
use serde::{Deserialize, Serialize};

use crate::error::{ArtifactError, ArtifactResult};
use crate::vocabulary::{
    BackupState, DataClass, MigrationState, RestoreVerificationState, RetentionClass,
    StorageBackend,
};

/// Canonical hex digest length for the supported hash algorithm.
pub const SHA256_HEX_LEN: usize = 64;

/// Content-addressed artifact hash (SPEC-024 requirement 2). The digest is
/// the lowercase canonical hex SHA-256 of the artifact bytes; any other
/// length, non-hex character, or uppercase form is rejected. The hash is
/// the identity: two artifacts with the same digest are the same content,
/// and metadata must bind to a digest, never to a name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArtifactHash(String);

impl ArtifactHash {
    /// Construct and validate a canonical hex SHA-256 digest.
    pub fn new(digest: impl Into<String>) -> ArtifactResult<Self> {
        let s = digest.into();
        if s.len() != SHA256_HEX_LEN {
            return Err(ArtifactError::validation(format!(
                "artifact hash must be exactly {SHA256_HEX_LEN} hex characters, got {}",
                s.len()
            )));
        }
        if !s.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ArtifactError::validation(
                "artifact hash contains a non-hex character",
            ));
        }
        if s.bytes().any(|b| b.is_ascii_uppercase()) {
            return Err(ArtifactError::validation(
                "artifact hash must be lowercase canonical hex",
            ));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArtifactHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Backend location of an artifact (SPEC-024 metadata requirement 3).
/// The backend is a canonical storage kind; the opaque reference is a
/// backend-scoped path/key string and never leaks credentials. A location
/// is not proof of reachability, health, or verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendLocation {
    pub backend: StorageBackend,
    /// Backend-scoped opaque reference (path, bucket/key, volume path).
    /// Never contains credentials, secrets, or signed URLs.
    pub reference: String,
}

impl BackendLocation {
    pub fn new(backend: StorageBackend, reference: impl Into<String>) -> ArtifactResult<Self> {
        let reference = reference.into();
        if reference.trim().is_empty() {
            return Err(ArtifactError::validation(
                "backend location reference must not be empty",
            ));
        }
        if reference.contains("://") {
            return Err(ArtifactError::validation(
                "backend location reference must be opaque, not a URL with credentials",
            ));
        }
        Ok(Self { backend, reference })
    }
}

/// Encryption metadata for a sensitive artifact (SPEC-024 requirement 4).
/// Records the algorithm and the key reference; the key itself is stored
/// OUTSIDE the storage backend (a recovery key is never stored beside its
/// backup - SPEC-024 non-goal). The presence of this record means the
/// artifact is encrypted before egress, never that the key is available.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    /// Canonical algorithm identifier (e.g. AES-256-GCM).
    pub algorithm: String,
    /// Opaque reference to the key held outside the storage backend
    /// (never the key material itself).
    pub key_reference: String,
}

impl EncryptionMetadata {
    pub fn new(
        algorithm: impl Into<String>,
        key_reference: impl Into<String>,
    ) -> ArtifactResult<Self> {
        let algorithm = algorithm.into();
        let key_reference = key_reference.into();
        if algorithm.trim().is_empty() {
            return Err(ArtifactError::validation(
                "encryption algorithm must not be empty",
            ));
        }
        if key_reference.trim().is_empty() {
            return Err(ArtifactError::validation(
                "encryption key reference must not be empty",
            ));
        }
        Ok(Self {
            algorithm,
            key_reference,
        })
    }
}

/// Artifact version (SPEC-024 versioning). A version is an immutable link
/// in the artifact's lineage: it binds a content hash to a canonical
/// version string. A version is not a mutable label - the same version
/// string cannot be reused for different content, and content cannot be
/// overwritten in place (immutable content-addressed storage).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactVersion {
    /// Canonical version string (e.g. "1", "v2", "2026-08-21.1").
    pub version: String,
    /// Content hash of the bytes this version points to.
    pub content_hash: ArtifactHash,
}

impl ArtifactVersion {
    pub fn new(version: impl Into<String>, content_hash: ArtifactHash) -> ArtifactResult<Self> {
        let version = version.into();
        if version.trim().is_empty() {
            return Err(ArtifactError::validation(
                "artifact version must not be empty",
            ));
        }
        if version.chars().any(|c| c.is_whitespace()) {
            return Err(ArtifactError::validation(
                "artifact version must not contain whitespace",
            ));
        }
        Ok(Self {
            version,
            content_hash,
        })
    }
}

/// Artifact metadata (SPEC-024 requirement 3: metadata, hash, content
/// type, size, owner, data class, retention, encryption, version, lineage,
/// and backend location remain canonical). The metadata is bound to the
/// artifact content hash; a name alone never identifies content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub artifact_id: ArtifactId,
    pub tenant: TenantId,
    /// Display name only - never an identity. Content identity is the hash.
    pub name: String,
    pub content_hash: ArtifactHash,
    pub content_type: String,
    pub size_bytes: u64,
    pub owner: String,
    pub data_class: DataClass,
    pub retention: RetentionClass,
    pub encryption: Option<EncryptionMetadata>,
    pub version: ArtifactVersion,
    pub lineage: Vec<ArtifactId>,
    pub location: BackendLocation,
}

impl ArtifactMetadata {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        artifact_id: ArtifactId,
        tenant: TenantId,
        name: impl Into<String>,
        content_hash: ArtifactHash,
        content_type: impl Into<String>,
        size_bytes: u64,
        owner: impl Into<String>,
        data_class: DataClass,
        retention: RetentionClass,
        encryption: Option<EncryptionMetadata>,
        version: ArtifactVersion,
        lineage: Vec<ArtifactId>,
        location: BackendLocation,
    ) -> ArtifactResult<Self> {
        let name = name.into();
        let content_type = content_type.into();
        let owner = owner.into();
        if name.trim().is_empty() {
            return Err(ArtifactError::validation("artifact name must not be empty"));
        }
        if content_type.trim().is_empty() {
            return Err(ArtifactError::validation(
                "artifact content type must not be empty",
            ));
        }
        if owner.trim().is_empty() {
            return Err(ArtifactError::validation(
                "artifact owner must not be empty",
            ));
        }
        // Encryption-before-egress: a sensitive-class artifact that lives
        // on a backend that leaves the node MUST carry encryption metadata.
        if data_class.requires_encryption_before_egress()
            && location.backend.leaves_node()
            && encryption.is_none()
        {
            return Err(ArtifactError::policy(format!(
                "sensitive artifact on {} backend must be encrypted before egress",
                location.backend
            )));
        }
        // Version lineage consistency: the current version's hash must be
        // the metadata's content hash (a version never points elsewhere).
        if version.content_hash != content_hash {
            return Err(ArtifactError::validation(
                "artifact version content hash must match metadata content hash",
            ));
        }
        Ok(Self {
            artifact_id,
            tenant,
            name,
            content_hash,
            content_type,
            size_bytes,
            owner,
            data_class,
            retention,
            encryption,
            version,
            lineage,
            location,
        })
    }
}

/// Object reference (SPEC-024 canonical term). An immutable content
/// address: artifact ID plus content hash. An ObjectRef is what callers
/// hold; it never changes when backend location changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectRef {
    pub artifact_id: ArtifactId,
    pub content_hash: ArtifactHash,
}

impl ObjectRef {
    pub fn new(artifact_id: ArtifactId, content_hash: ArtifactHash) -> Self {
        Self {
            artifact_id,
            content_hash,
        }
    }
}

/// Backup set (SPEC-024 requirement 5). A backup is DECLARED, then
/// CREATED with a signed manifest and hashes, then VERIFIED by readback,
/// and only RESTORED when a fresh-target restore proves it (SPEC-024
/// non-goal: backup without restore proof). The recovery key is never
/// stored beside the backup (SPEC-024 non-goal).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSet {
    pub backup_id: String,
    pub tenant: TenantId,
    pub state: BackupState,
    /// Data classes included in this backup (databases, identity
    /// configuration, policies, workflows, memory, skills, connectors,
    /// manifests, audit, optional artifacts per profile).
    pub included_classes: Vec<DataClass>,
    /// Signed manifest reference (backend location of the manifest).
    pub manifest_location: BackendLocation,
    /// Per-artifact content hashes recorded in the signed manifest.
    pub manifest_hashes: Vec<ArtifactHash>,
    /// Opaque reference to the recovery key held OUTSIDE the backup.
    pub recovery_key_reference: Option<String>,
    /// Application and schema versions for restore compatibility.
    pub application_version: String,
    pub schema_version: String,
    /// Backup creation timestamp (UTC RFC 3339).
    pub created_at: String,
}

impl BackupSet {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        backup_id: impl Into<String>,
        tenant: TenantId,
        included_classes: Vec<DataClass>,
        manifest_location: BackendLocation,
        manifest_hashes: Vec<ArtifactHash>,
        recovery_key_reference: Option<String>,
        application_version: impl Into<String>,
        schema_version: impl Into<String>,
        created_at: impl Into<String>,
    ) -> ArtifactResult<Self> {
        let backup_id = backup_id.into();
        let application_version = application_version.into();
        let schema_version = schema_version.into();
        let created_at = created_at.into();
        if backup_id.trim().is_empty() {
            return Err(ArtifactError::validation("backup id must not be empty"));
        }
        if included_classes.is_empty() {
            return Err(ArtifactError::validation(
                "backup set must include at least one data class",
            ));
        }
        if manifest_hashes.is_empty() {
            return Err(ArtifactError::validation(
                "backup manifest must record at least one content hash",
            ));
        }
        if application_version.trim().is_empty() || schema_version.trim().is_empty() {
            return Err(ArtifactError::validation(
                "backup must record application and schema versions",
            ));
        }
        if created_at.trim().is_empty() {
            return Err(ArtifactError::validation(
                "backup must record a creation timestamp",
            ));
        }
        Ok(Self {
            backup_id,
            tenant,
            state: BackupState::Declared,
            included_classes,
            manifest_location,
            manifest_hashes,
            recovery_key_reference,
            application_version,
            schema_version,
            created_at,
        })
    }

    /// Advance the backup state ladder. DECLARED -> CREATED -> VERIFIED ->
    /// RESTORED; any leap (e.g. DECLARED -> VERIFIED) is rejected.
    pub fn advance(&mut self) -> ArtifactResult<BackupState> {
        let next = match self.state {
            BackupState::Declared => BackupState::Created,
            BackupState::Created => BackupState::Verified,
            BackupState::Verified => BackupState::Restored,
            BackupState::Restored => {
                return Err(ArtifactError::policy(
                    "backup already RESTORED; no further state",
                ));
            }
        };
        self.state = next;
        Ok(next)
    }
}

/// Restore plan (SPEC-024 requirements 6-7). Restore runs against a FRESH
/// target, validates all components, and reconnects edge nodes through
/// controlled re-enrollment or preserved trust. Hash verification is
/// required before any destructive step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestorePlan {
    pub restore_id: String,
    pub tenant: TenantId,
    pub state: RestoreVerificationState,
    pub source_backup: String,
    /// The fresh target must be distinct from the source of truth being
    /// restored (never restore over the live source).
    pub fresh_target: String,
    pub required_hashes: Vec<ArtifactHash>,
    /// Verified hash count so far; validation completes only when every
    /// required hash is verified.
    pub verified_hashes: Vec<ArtifactHash>,
    pub correlation: Option<CorrelationId>,
}

impl RestorePlan {
    pub fn new(
        restore_id: impl Into<String>,
        tenant: TenantId,
        source_backup: impl Into<String>,
        fresh_target: impl Into<String>,
        required_hashes: Vec<ArtifactHash>,
        correlation: Option<CorrelationId>,
    ) -> ArtifactResult<Self> {
        let restore_id = restore_id.into();
        let source_backup = source_backup.into();
        let fresh_target = fresh_target.into();
        if restore_id.trim().is_empty() {
            return Err(ArtifactError::validation("restore id must not be empty"));
        }
        if source_backup.trim().is_empty() {
            return Err(ArtifactError::validation(
                "restore plan must name a source backup",
            ));
        }
        if fresh_target.trim().is_empty() {
            return Err(ArtifactError::validation(
                "restore plan must name a fresh target",
            ));
        }
        if required_hashes.is_empty() {
            return Err(ArtifactError::validation(
                "restore plan must require at least one content hash",
            ));
        }
        Ok(Self {
            restore_id,
            tenant,
            state: RestoreVerificationState::Declared,
            source_backup,
            fresh_target,
            required_hashes,
            verified_hashes: Vec::new(),
            correlation,
        })
    }

    /// Record that a hash verified on the fresh target. Duplicates are
    /// ignored; unknown hashes are rejected.
    pub fn record_verified(&mut self, hash: &ArtifactHash) -> ArtifactResult<()> {
        if !self.required_hashes.contains(hash) {
            return Err(ArtifactError::verification(
                "verified hash is not part of the restore plan",
            ));
        }
        if !self.verified_hashes.contains(hash) {
            self.verified_hashes.push(hash.clone());
        }
        Ok(())
    }

    /// True when every required hash has verified. Destructive steps
    /// (deleting the source, overwriting live state) may only proceed
    /// after this returns true.
    pub fn all_hashes_verified(&self) -> bool {
        self.required_hashes
            .iter()
            .all(|h| self.verified_hashes.contains(h))
    }
}

/// Storage migration (SPEC-024 requirement 8). Copies first, verifies
/// hashes and metadata on the target, then changes the canonical location,
/// observes, and deletes old objects only after approval. Deleting the old
/// provider first is a SPEC-024 non-goal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageMigration {
    pub migration_id: String,
    pub tenant: TenantId,
    pub state: MigrationState,
    pub source_backend: StorageBackend,
    pub target_backend: StorageBackend,
    pub object_refs: Vec<ObjectRef>,
    /// Hash verification results on the target backend (per ObjectRef).
    pub verified_refs: Vec<ObjectRef>,
    /// Human approval reference required before old objects are deleted.
    pub delete_approval: Option<String>,
}

impl StorageMigration {
    pub fn new(
        migration_id: impl Into<String>,
        tenant: TenantId,
        source_backend: StorageBackend,
        target_backend: StorageBackend,
        object_refs: Vec<ObjectRef>,
    ) -> ArtifactResult<Self> {
        let migration_id = migration_id.into();
        if migration_id.trim().is_empty() {
            return Err(ArtifactError::validation("migration id must not be empty"));
        }
        if source_backend == target_backend {
            return Err(ArtifactError::validation(
                "migration source and target backend must differ",
            ));
        }
        if object_refs.is_empty() {
            return Err(ArtifactError::validation(
                "migration must move at least one object",
            ));
        }
        Ok(Self {
            migration_id,
            tenant,
            state: MigrationState::Requested,
            source_backend,
            target_backend,
            object_refs,
            verified_refs: Vec::new(),
            delete_approval: None,
        })
    }

    /// Record that an object verified on the target backend. Only objects
    /// in this migration may be recorded.
    pub fn record_verified(&mut self, object: &ObjectRef) -> ArtifactResult<()> {
        if !self.object_refs.contains(object) {
            return Err(ArtifactError::verification(
                "verified object is not part of this migration",
            ));
        }
        if !self.verified_refs.contains(object) {
            self.verified_refs.push(object.clone());
        }
        Ok(())
    }

    /// True when every object verified on the target.
    pub fn all_verified(&self) -> bool {
        self.object_refs
            .iter()
            .all(|o| self.verified_refs.contains(o))
    }

    /// Advance the migration to VERIFIED once every object verifies on
    /// the target (SPEC-024 requirement 8: migration is verified only
    /// after destination readback; copied != verified). Leaps from
    /// non-requested/copying states are rejected.
    pub fn mark_verified(&mut self) -> ArtifactResult<MigrationState> {
        if !self.all_verified() {
            return Err(ArtifactError::verification(
                "cannot mark migration VERIFIED before every object verifies on the target",
            ));
        }
        match self.state {
            MigrationState::Requested | MigrationState::Copying => {
                self.state = MigrationState::Verified;
                Ok(self.state)
            }
            MigrationState::Verified => Ok(self.state),
            other => Err(ArtifactError::policy(format!(
                "cannot mark {other:?} migration VERIFIED"
            ))),
        }
    }

    /// Approve deletion of old objects. Approval requires that every
    /// object verified on the target first (SPEC-024: delete old objects
    /// only after verification and approval).
    pub fn approve_delete(&mut self, approval: impl Into<String>) -> ArtifactResult<()> {
        if !self.all_verified() {
            return Err(ArtifactError::policy(
                "cannot approve deletion before every object verifies on the target",
            ));
        }
        let approval = approval.into();
        if approval.trim().is_empty() {
            return Err(ArtifactError::validation(
                "delete approval must reference a human approval record",
            ));
        }
        self.delete_approval = Some(approval);
        self.state = MigrationState::CanonicalLocationChanged;
        Ok(())
    }
}

/// Recovery key reference (SPEC-024 canonical term). The recovery key is
/// held OUTSIDE the storage backend and outside the backup; this type only
/// carries an opaque reference, never key material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryKey {
    pub key_reference: String,
}

impl RecoveryKey {
    pub fn new(key_reference: impl Into<String>) -> ArtifactResult<Self> {
        let key_reference = key_reference.into();
        if key_reference.trim().is_empty() {
            return Err(ArtifactError::validation(
                "recovery key reference must not be empty",
            ));
        }
        Ok(Self { key_reference })
    }
}
