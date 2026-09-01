//! Self-contained disaster recovery (SPEC-024 requirement 5 and the
//! acceptance criterion; AUD-015).
//!
//! A `BackupBundle` carries the signed manifest plus every referenced
//! object's bytes and metadata, so a WIPED target can be reconstructed
//! from the bundle alone — the source store is never consulted. This is
//! the difference between a manifest (a pointer into a live store) and a
//! backup (a portable, self-contained recovery unit).
//!
//! The functions here are provider-neutral: they compose the existing
//! `ArtifactStore` port (list/get/put/create_backup/restore) and work
//! against every adapter (local, NAS, SeaweedFS, S3) without trait
//! churn. Crypto and hashing remain in the adapters, exactly like
//! signature verification and encryption-before-egress: the contract
//! crate stays dependency-lean (M1 dependency-direction gate).

use nexus_domain::{CorrelationId, TenantId};

use crate::error::ArtifactResult;
use crate::model::{BackupBundle, BackupSet, BundleObject, RestorePlan};
use crate::port::ArtifactStore;

/// Export a self-contained DR bundle from a live store.
///
/// The signed manifest is materialized together with every referenced
/// object's bytes and metadata into a portable bundle. Fails closed:
/// any manifest hash without a matching artifact in the store makes the
/// bundle incomplete, and an incomplete bundle is never emitted (a
/// partial backup would be worse than no backup).
pub fn export_backup_bundle(
    store: &mut impl ArtifactStore,
    tenant: &TenantId,
    backup: &BackupSet,
    correlation: &CorrelationId,
) -> ArtifactResult<BackupBundle> {
    if &backup.tenant != tenant {
        return Err(crate::error::ArtifactError::policy(
            "cannot export a backup for a different tenant",
        ));
    }
    // The manifest must be signed (structure check here; cryptographic
    // verification happens at restore through the adapter).
    backup.verify_manifest_signature_structure()?;

    let mut objects: Vec<BundleObject> = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let (batch, next) = store.list(tenant, cursor.as_deref(), 100)?;
        for meta in batch {
            if backup.manifest_hashes.contains(&meta.content_hash) {
                let (_, bytes) = store.get(tenant, &meta.artifact_id, correlation)?;
                objects.push(BundleObject {
                    artifact_id: meta.artifact_id.clone(),
                    hash: meta.content_hash.clone(),
                    metadata: meta,
                    bytes,
                });
            }
        }
        match next {
            Some(n) => cursor = Some(n),
            None => break,
        }
    }

    // Fails closed on an incomplete bundle: every manifest hash must be
    // materialized. BackupBundle::new enforces this.
    BackupBundle::new(backup.clone(), objects)
}

/// Reconstruct a fresh (wiped) target from a self-contained DR bundle.
///
/// Every carried object is written through the adapter (hash-verified
/// put), the signed manifest is written (create_backup verifies the
/// signature cryptographically and every manifest hash on the target),
/// and the restore plan is then validated. The source store is NEVER
/// consulted: the bundle alone reconstructs the target (AUD-015).
/// Missing objects, tampered bytes, a tampered or missing signature, a
/// foreign-tenant manifest, or an already-populated conflicting target
/// all fail closed.
pub fn restore_bundle(
    store: &mut impl ArtifactStore,
    tenant: &TenantId,
    plan: &RestorePlan,
    bundle: &BackupBundle,
    correlation: &CorrelationId,
) -> ArtifactResult<RestorePlan> {
    bundle.verify_structure()?;
    if &bundle.manifest.tenant != tenant {
        return Err(crate::error::ArtifactError::policy(
            "cannot restore a bundle for a different tenant",
        ));
    }

    // 1. Write every carried object through the adapter; put verifies
    //    the bytes against the expected content hash before persisting
    //    (a tampered bundle object fails closed here).
    for object in &bundle.objects {
        store.put(
            tenant,
            &object.artifact_id,
            &object.hash,
            &object.bytes,
            &object.metadata,
            correlation,
        )?;
    }

    // 2. Write the signed manifest; create_backup verifies the
    //    signature cryptographically (adapter) and every manifest hash
    //    on the fresh target.
    store.create_backup(tenant, &bundle.manifest, correlation)?;

    // 3. Validate the restore plan against the reconstructed target.
    store.restore(tenant, plan, correlation)
}
