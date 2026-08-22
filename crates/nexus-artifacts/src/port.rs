//! EP-037 provider-neutral ArtifactStore port (SPEC-024).
//!
//! ONE contract serves local filesystem, NAS, SeaweedFS, MinIO
//! compatibility, Cloudflare R2, Backblaze B2, and Amazon S3. The port is
//! provider-neutral: implementations are adapters over real backends and
//! never leak vendor payloads into the domain contract. Every operation
//! carries correlation context, is idempotent where retryable, verifies
//! hashes before destructive steps, and fails closed.

use nexus_domain::{ArtifactId, CorrelationId, TenantId};

use crate::error::ArtifactResult;
use crate::model::{ArtifactHash, ArtifactMetadata, BackupSet, RestorePlan, StorageMigration};
use crate::vocabulary::RetentionClass;

/// Provider-neutral artifact store boundary.
///
/// Implementations must satisfy:
/// - writes are content-addressed (an artifact's identity is its hash);
/// - a write returns metadata bound to the content hash;
/// - reads verify the returned bytes against the requested hash;
/// - delete requires exact-target verification of absence (delete
///   requested != delete accepted != resource absent verified);
/// - backups encrypt before leaving the node (SPEC-024 requirement 4);
/// - restore and migration verify hashes before deletion (SPEC-024
///   requirement 8; non-goal: deleting the old provider first).
pub trait ArtifactStore {
    /// Store artifact bytes. Content-addressed: the caller supplies the
    /// expected hash and the implementation MUST verify the bytes match it
    /// before persisting (never trust a caller-supplied hash blindly).
    fn put(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        expected_hash: &ArtifactHash,
        bytes: &[u8],
        metadata: &ArtifactMetadata,
        correlation: &CorrelationId,
    ) -> ArtifactResult<ArtifactMetadata>;

    /// Read artifact bytes and verify them against the metadata content
    /// hash. A hash mismatch is a Verification error, never a silent
    /// success.
    fn get(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        correlation: &CorrelationId,
    ) -> ArtifactResult<(ArtifactMetadata, Vec<u8>)>;

    /// Verify artifact bytes on the backend without returning them.
    fn verify(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        correlation: &CorrelationId,
    ) -> ArtifactResult<ArtifactHash>;

    /// Mark an artifact for deletion. Deletion is a ladder: the artifact
    /// must first be verified absent on the backend before the delete is
    /// accepted (DELETE_REQUESTED != DELETE_ACCEPTED !=
    /// RESOURCE_ABSENT_VERIFIED).
    fn delete(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        correlation: &CorrelationId,
    ) -> ArtifactResult<()>;

    /// Create a backup set. Sensitive classes must be encrypted before
    /// egress; the recovery key reference must point OUTSIDE the backend.
    fn create_backup(
        &mut self,
        tenant: &TenantId,
        backup: &BackupSet,
        correlation: &CorrelationId,
    ) -> ArtifactResult<BackupSet>;

    /// Execute a restore against a fresh target, verifying every required
    /// hash before any destructive step completes.
    fn restore(
        &mut self,
        tenant: &TenantId,
        plan: &RestorePlan,
        correlation: &CorrelationId,
    ) -> ArtifactResult<RestorePlan>;

    /// Migrate objects between backends: copy, verify, change canonical
    /// location, observe, and delete old objects only after approval.
    fn migrate(
        &mut self,
        tenant: &TenantId,
        migration: &StorageMigration,
        correlation: &CorrelationId,
    ) -> ArtifactResult<StorageMigration>;

    /// List artifact metadata for a tenant (paged by opaque cursor).
    fn list(
        &mut self,
        tenant: &TenantId,
        cursor: Option<&str>,
        limit: usize,
    ) -> ArtifactResult<(Vec<ArtifactMetadata>, Option<String>)>;

    /// Apply a retention class to an artifact. Retention is declared
    /// policy: EXPIRING is not DELETED, and DELETED is not
    /// RESOURCE_ABSENT_VERIFIED until exact-target verification.
    fn set_retention(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        retention: RetentionClass,
        correlation: &CorrelationId,
    ) -> ArtifactResult<()>;
}
