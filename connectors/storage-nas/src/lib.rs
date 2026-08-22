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
    ArtifactError, ArtifactHash, ArtifactMetadata, ArtifactResult, ArtifactStore, BackupSet,
    RestorePlan, RetentionClass, StorageMigration,
};
use nexus_domain::{ArtifactId, CorrelationId, TenantId};
use nexus_provider_storage_local::LocalArtifactStore;

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
        // artifact without encryption metadata must fail closed BEFORE
        // any byte reaches the share.
        if metadata.data_class.requires_encryption_before_egress() && metadata.encryption.is_none()
        {
            return Err(ArtifactError::policy(format!(
                "sensitive artifact on NAS backend must be encrypted before egress (class {})",
                metadata.data_class
            )));
        }
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
