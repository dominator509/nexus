//! EP-037 NAS ArtifactStore adapter (SPEC-024).
//!
//! A NAS share is a real filesystem mounted over the network. This
//! adapter is a REAL filesystem-backed store over the NAS mount root
//! (composed from the local adapter's proven core), with the SPEC-024
//! encryption-before-egress policy enforced at the adapter boundary:
//! NAS leaves the node, so a sensitive-class artifact WITHOUT encryption
//! metadata is rejected before any byte is written.
//!
//! The same truthfulness ladders hold as for local storage: a written
//! artifact is not a verified artifact, a backup created is not a restore
//! proven, and delete is a ladder ending in RESOURCE_ABSENT_VERIFIED.

use std::path::PathBuf;

use nexus_artifacts::{
    ArtifactHash, ArtifactMetadata, ArtifactResult, ArtifactStore, BackupSet, RestorePlan,
    RetentionClass, StorageMigration,
};
use nexus_domain::{ArtifactId, CorrelationId, TenantId};
use nexus_provider_storage_local::LocalArtifactStore;
use sha2::{Digest, Sha256};

/// NAS ArtifactStore over a mounted NAS root.
///
/// The root is the NAS mount point (e.g. /mnt/nas/nexus). The adapter
/// composes the real filesystem store and adds the non-local-backend
/// encryption policy.
#[derive(Debug, Clone)]
pub struct NasArtifactStore {
    inner: LocalArtifactStore,
}

impl NasArtifactStore {
    /// Open (creating if needed) a real NAS mount root.
    pub fn open(root: impl Into<PathBuf>) -> ArtifactResult<Self> {
        let inner = LocalArtifactStore::open(root)?;
        Ok(Self { inner })
    }

    pub fn root(&self) -> &std::path::Path {
        self.inner.root()
    }

    /// Real SHA-256 of bytes (canonical lowercase hex) for the
    /// encryption-before-egress plaintext-hash verification.
    fn sha256_hex(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let out = hasher.finalize();
        out.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl ArtifactStore for NasArtifactStore {
    fn put(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        expected_hash: &ArtifactHash,
        bytes: &[u8],
        metadata: &ArtifactMetadata,
        correlation: &CorrelationId,
    ) -> ArtifactResult<ArtifactMetadata> {
        // Encryption-before-egress: NAS leaves the node. A sensitive-class
        // artifact must carry encryption metadata AND the bytes about to
        // be persisted must not be the plaintext (AUD-051) - verified
        // BEFORE any byte reaches the share. The adapter never holds the
        // key; the encrypting caller recorded the plaintext's SHA-256 in
        // the metadata, and we verify the stored bytes hash differs from
        // it.
        metadata.verify_encryption_before_egress(&Self::sha256_hex(bytes))?;
        self.inner.put(
            tenant,
            artifact_id,
            expected_hash,
            bytes,
            metadata,
            correlation,
        )
    }

    fn get(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        correlation: &CorrelationId,
    ) -> ArtifactResult<(ArtifactMetadata, Vec<u8>)> {
        self.inner.get(tenant, artifact_id, correlation)
    }

    fn verify(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        correlation: &CorrelationId,
    ) -> ArtifactResult<ArtifactHash> {
        self.inner.verify(tenant, artifact_id, correlation)
    }

    fn delete(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        correlation: &CorrelationId,
    ) -> ArtifactResult<()> {
        self.inner.delete(tenant, artifact_id, correlation)
    }

    fn create_backup(
        &mut self,
        tenant: &TenantId,
        backup: &BackupSet,
        correlation: &CorrelationId,
    ) -> ArtifactResult<BackupSet> {
        self.inner.create_backup(tenant, backup, correlation)
    }

    fn restore(
        &mut self,
        tenant: &TenantId,
        plan: &RestorePlan,
        correlation: &CorrelationId,
    ) -> ArtifactResult<RestorePlan> {
        self.inner.restore(tenant, plan, correlation)
    }

    fn migrate(
        &mut self,
        tenant: &TenantId,
        migration: &StorageMigration,
        correlation: &CorrelationId,
    ) -> ArtifactResult<StorageMigration> {
        self.inner.migrate(tenant, migration, correlation)
    }

    fn list(
        &mut self,
        tenant: &TenantId,
        cursor: Option<&str>,
        limit: usize,
    ) -> ArtifactResult<(Vec<ArtifactMetadata>, Option<String>)> {
        self.inner.list(tenant, cursor, limit)
    }

    fn set_retention(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        retention: RetentionClass,
        correlation: &CorrelationId,
    ) -> ArtifactResult<()> {
        self.inner
            .set_retention(tenant, artifact_id, retention, correlation)
    }
}

/// Convenience re-export so callers can name the error code type without
/// importing the contract crate twice.
pub use nexus_artifacts::ArtifactErrorCode;
