//! EP-037 M2 core behavior tests over REAL local filesystem roots.
//!
//! Every test exercises the actual std::fs adapter: content-addressed
//! writes, hash verification on read, delete with absent-verification,
//! backup creation, restore verification, and migration verification.
//! No in-memory engine, no mocks: the filesystem IS the provider.

use std::fs;
use std::path::PathBuf;

use nexus_artifacts::{
    ArtifactErrorCode, ArtifactHash, ArtifactMetadata, ArtifactResult, ArtifactStore,
    ArtifactVersion, BackupSet, DataClass, EncryptionMetadata, RetentionClass, StorageBackend,
};
use nexus_domain::{ArtifactId, CorrelationId, TenantId};
use nexus_provider_storage_local::LocalArtifactStore;
use sha2::{Digest, Sha256};

fn tenant() -> TenantId {
    "01970000-0000-7000-8000-000000000001".parse().unwrap()
}

fn artifact_id(n: u8) -> ArtifactId {
    format!("01970000-0000-7000-8000-0000000000{n:02x}")
        .parse()
        .unwrap()
}

fn correlation() -> CorrelationId {
    "01970000-0000-7000-8000-000000000011".parse().unwrap()
}

fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_of(bytes: &[u8]) -> ArtifactHash {
    ArtifactHash::new(digest(bytes)).unwrap()
}

fn metadata_for(
    id: ArtifactId,
    bytes: &[u8],
    data_class: DataClass,
) -> ArtifactResult<ArtifactMetadata> {
    let h = hash_of(bytes);
    ArtifactMetadata::new(
        id,
        tenant(),
        "m2-test-artifact",
        h.clone(),
        "application/octet-stream",
        bytes.len() as u64,
        "principal-1",
        data_class,
        RetentionClass::LongTerm,
        // M2 tests use LOCAL roots: sensitive classes may stay
        // plaintext-at-rest locally (encryption-before-egress only).
        None,
        ArtifactVersion::new("1", h.clone()).unwrap(),
        Vec::new(),
        nexus_artifacts::BackendLocation::new(StorageBackend::Local, "objects/m2-test").unwrap(),
    )
}

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nexus-ep037-m2-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn teardown(root: &PathBuf) {
    let _ = fs::remove_dir_all(root);
}

// ---------------------------------------------------------------------------
// Put: content addressing and hash verification
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_local_put_get_roundtrip_content_addressed() {
    let root = temp_root("roundtrip");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"m2 roundtrip payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(1);
    let meta = metadata_for(id.clone(), &bytes, DataClass::Public).unwrap();
    let stored = store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    assert_eq!(stored.content_hash, h);
    // The object file exists at its content address.
    let object_path = root.join("objects").join(h.as_str());
    assert!(object_path.exists());
    // Read back and verify.
    let (read_meta, read_bytes) = store.get(&tenant(), &id, &correlation()).unwrap();
    assert_eq!(read_meta.content_hash, h);
    assert_eq!(read_bytes, bytes);
    // Verify endpoint re-hashes the on-disk object.
    let verified = store.verify(&tenant(), &id, &correlation()).unwrap();
    assert_eq!(verified, h);
    teardown(&root);
}

#[test]
fn ep037_unit_local_put_rejects_hash_mismatch() {
    let root = temp_root("mismatch");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    // The caller-supplied hash claims digest X, but the actual bytes hash
    // to Y: the store must refuse to persist and never trust the claim.
    let claimed = hash_of(b"claimed bytes");
    let actual_bytes = b"actual bytes with different digest".to_vec();
    let id = artifact_id(2);
    let meta = metadata_for(id.clone(), &actual_bytes, DataClass::Public).unwrap();
    let err = store
        .put(
            &tenant(),
            &id,
            &claimed,
            &actual_bytes,
            &meta,
            &correlation(),
        )
        .unwrap_err();
    // The store must refuse to persist content whose claimed hash does
    // not match the bytes (either Validation for the metadata mismatch or
    // Verification for the byte mismatch - never a silent success).
    assert!(matches!(
        err.code,
        ArtifactErrorCode::Validation | ArtifactErrorCode::Verification
    ));
    // Nothing was written.
    assert!(!root.join("objects").join(claimed.as_str()).exists());
    assert!(!root
        .join("objects")
        .join(meta.content_hash.as_str())
        .exists());
    teardown(&root);
}

#[test]
fn ep037_unit_local_put_content_addressed_dedup() {
    let root = temp_root("dedup");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"same content, two ids".to_vec();
    let h = hash_of(&bytes);
    let id_a = artifact_id(3);
    let id_b = artifact_id(4);
    let meta_a = metadata_for(id_a.clone(), &bytes, DataClass::Public).unwrap();
    let meta_b = metadata_for(id_b.clone(), &bytes, DataClass::Public).unwrap();
    store
        .put(&tenant(), &id_a, &h, &bytes, &meta_a, &correlation())
        .unwrap();
    store
        .put(&tenant(), &id_b, &h, &bytes, &meta_b, &correlation())
        .unwrap();
    // One object file for the shared digest.
    let count = fs::read_dir(root.join("objects"))
        .unwrap()
        .filter(|e| e.as_ref().unwrap().path().extension().is_none())
        .count();
    assert_eq!(count, 1);
    teardown(&root);
}

// ---------------------------------------------------------------------------
// Get/verify: corruption is detected, never silent
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_local_get_detects_corruption() {
    let root = temp_root("corrupt");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"corruption test payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(5);
    let meta = metadata_for(id.clone(), &bytes, DataClass::Public).unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    // Corrupt the object on disk (real failure mechanism).
    let object_path = root.join("objects").join(h.as_str());
    fs::write(&object_path, b"tampered bytes").unwrap();
    let err = store.get(&tenant(), &id, &correlation()).unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::Verification);
    teardown(&root);
}

#[test]
fn ep037_unit_local_verify_missing_object_not_found() {
    let root = temp_root("missing");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let id = artifact_id(6);
    let err = store.verify(&tenant(), &id, &correlation()).unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::NotFound);
    teardown(&root);
}

// ---------------------------------------------------------------------------
// Delete: ladder with absent verification
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_local_delete_verifies_absence() {
    let root = temp_root("delete");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"delete me".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(7);
    let meta = metadata_for(id.clone(), &bytes, DataClass::Public).unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    store.delete(&tenant(), &id, &correlation()).unwrap();
    // Resource absent verified: neither index nor object remains.
    assert!(!root.join("index").join(format!("{id}.json")).exists());
    assert!(!root.join("objects").join(h.as_str()).exists());
    // A second delete is NotFound (fail-closed, no blind success).
    let err = store.delete(&tenant(), &id, &correlation()).unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::NotFound);
    teardown(&root);
}

// ---------------------------------------------------------------------------
// Backup: manifest + hash verification, conflict on duplicate
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_local_create_backup_verifies_hashes_and_writes_manifest() {
    let root = temp_root("backup");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"backup payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(8);
    let meta = metadata_for(id.clone(), &bytes, DataClass::Personal).unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let backup = BackupSet::new(
        "b-m2-1",
        tenant(),
        vec![DataClass::Personal],
        nexus_artifacts::BackendLocation::new(StorageBackend::Local, "backups/b-m2-1.json")
            .unwrap(),
        vec![h],
        Some("vault:keys/m2-test".to_string()),
        "0.1.0",
        "1",
        "2026-08-22T00:00:00Z",
    )
    .unwrap();
    let created = store
        .create_backup(&tenant(), &backup, &correlation())
        .unwrap();
    assert_eq!(created.state, nexus_artifacts::BackupState::Created);
    assert!(root.join("backups").join("b-m2-1.json").exists());
    teardown(&root);
}

#[test]
fn ep037_unit_local_create_backup_rejects_unverifiable_hash() {
    let root = temp_root("backup-bad");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let ghost = ArtifactHash::new(format!("{:064x}", 0x99)).unwrap();
    let backup = BackupSet::new(
        "b-m2-bad",
        tenant(),
        vec![DataClass::Personal],
        nexus_artifacts::BackendLocation::new(StorageBackend::Local, "backups/b-m2-bad.json")
            .unwrap(),
        vec![ghost],
        None,
        "0.1.0",
        "1",
        "2026-08-22T00:00:00Z",
    )
    .unwrap();
    let err = store
        .create_backup(&tenant(), &backup, &correlation())
        .unwrap_err();
    // The hash does not exist on disk -> verification failure path.
    assert!(matches!(
        err.code,
        ArtifactErrorCode::Verification | ArtifactErrorCode::NotFound
    ));
    teardown(&root);
}

#[test]
fn ep037_unit_local_create_backup_duplicate_conflict() {
    let root = temp_root("backup-dup");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"dup payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(9);
    let meta = metadata_for(id.clone(), &bytes, DataClass::Public).unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let backup = BackupSet::new(
        "b-m2-dup",
        tenant(),
        vec![DataClass::Public],
        nexus_artifacts::BackendLocation::new(StorageBackend::Local, "backups/b-m2-dup.json")
            .unwrap(),
        vec![h],
        None,
        "0.1.0",
        "1",
        "2026-08-22T00:00:00Z",
    )
    .unwrap();
    store
        .create_backup(&tenant(), &backup, &correlation())
        .unwrap();
    let err = store
        .create_backup(&tenant(), &backup, &correlation())
        .unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::Conflict);
    teardown(&root);
}

// ---------------------------------------------------------------------------
// Restore: hash verification before validation
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_local_restore_requires_all_hashes_verified() {
    let root = temp_root("restore");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"restore payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(10);
    let meta = metadata_for(id.clone(), &bytes, DataClass::Personal).unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let backup = BackupSet::new(
        "b-m2-restore",
        tenant(),
        vec![DataClass::Personal],
        nexus_artifacts::BackendLocation::new(StorageBackend::Local, "backups/b-m2-restore.json")
            .unwrap(),
        vec![h.clone()],
        Some("vault:keys/m2-test".to_string()),
        "0.1.0",
        "1",
        "2026-08-22T00:00:00Z",
    )
    .unwrap();
    store
        .create_backup(&tenant(), &backup, &correlation())
        .unwrap();
    // Plan requires TWO hashes: one present, one absent -> cannot validate.
    let ghost = ArtifactHash::new(format!("{:064x}", 0x77)).unwrap();
    let plan = nexus_artifacts::RestorePlan::new(
        "r-m2-1",
        tenant(),
        "b-m2-restore",
        "fresh-target-1",
        vec![h, ghost],
        Some(correlation()),
    )
    .unwrap();
    let err = store.restore(&tenant(), &plan, &correlation()).unwrap_err();
    // The ghost hash is not in the backup manifest -> verification failure.
    assert_eq!(err.code, ArtifactErrorCode::Verification);
    teardown(&root);
}

#[test]
fn ep037_unit_local_restore_validates_when_all_hashes_present() {
    let root = temp_root("restore-ok");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"restore ok payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(11);
    let meta = metadata_for(id.clone(), &bytes, DataClass::Personal).unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let backup = BackupSet::new(
        "b-m2-restore-ok",
        tenant(),
        vec![DataClass::Personal],
        nexus_artifacts::BackendLocation::new(
            StorageBackend::Local,
            "backups/b-m2-restore-ok.json",
        )
        .unwrap(),
        vec![h.clone()],
        Some("vault:keys/m2-test".to_string()),
        "0.1.0",
        "1",
        "2026-08-22T00:00:00Z",
    )
    .unwrap();
    store
        .create_backup(&tenant(), &backup, &correlation())
        .unwrap();
    let plan = nexus_artifacts::RestorePlan::new(
        "r-m2-2",
        tenant(),
        "b-m2-restore-ok",
        "fresh-target-1",
        vec![h],
        Some(correlation()),
    )
    .unwrap();
    let executed = store.restore(&tenant(), &plan, &correlation()).unwrap();
    assert!(executed.all_hashes_verified());
    assert_eq!(
        executed.state,
        nexus_artifacts::RestoreVerificationState::Validated
    );
    teardown(&root);
}

// ---------------------------------------------------------------------------
// Migration: verification on target before approval/delete
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_local_migrate_verifies_objects_on_target() {
    let root = temp_root("migrate");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"migrate payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(12);
    let meta = metadata_for(id.clone(), &bytes, DataClass::Public).unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let obj = nexus_artifacts::ObjectRef::new(id, h.clone());
    let migration = nexus_artifacts::StorageMigration::new(
        "m-m2-1",
        tenant(),
        StorageBackend::Local,
        StorageBackend::S3,
        vec![obj.clone()],
    )
    .unwrap();
    let result = store
        .migrate(&tenant(), &migration, &correlation())
        .unwrap();
    assert!(result.all_verified());
    assert_eq!(result.state, nexus_artifacts::MigrationState::Requested);
    teardown(&root);
}

// ---------------------------------------------------------------------------
// List and retention
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_local_list_pages_artifacts() {
    let root = temp_root("list");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    for n in 0..3u8 {
        let bytes = format!("list payload {n}").into_bytes();
        let h = hash_of(&bytes);
        let id = artifact_id(20 + n);
        let meta = metadata_for(id.clone(), &bytes, DataClass::Public).unwrap();
        store
            .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
            .unwrap();
    }
    let (page1, cursor) = store.list(&tenant(), None, 2).unwrap();
    assert_eq!(page1.len(), 2);
    let cursor = cursor.expect("cursor for second page");
    let (page2, cursor2) = store.list(&tenant(), Some(&cursor), 2).unwrap();
    assert_eq!(page2.len(), 1);
    assert!(cursor2.is_none());
    teardown(&root);
}

#[test]
fn ep037_unit_local_set_retention_updates_metadata() {
    let root = temp_root("retention");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"retention payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(13);
    let meta = metadata_for(id.clone(), &bytes, DataClass::Public).unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    store
        .set_retention(&tenant(), &id, RetentionClass::Permanent, &correlation())
        .unwrap();
    let (read_meta, _) = store.get(&tenant(), &id, &correlation()).unwrap();
    assert_eq!(read_meta.retention, RetentionClass::Permanent);
    teardown(&root);
}

// ---------------------------------------------------------------------------
// Encryption metadata: sensitive artifact on the adapter surface
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_local_sensitive_artifact_with_encryption_metadata_ok() {
    let root = temp_root("enc");
    let mut store = LocalArtifactStore::open(&root).unwrap();
    let bytes = b"encrypted sensitive payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(14);
    let enc = EncryptionMetadata::new("AES-256-GCM", "vault:keys/m2-test").unwrap();
    let meta = metadata_for(id.clone(), &bytes, DataClass::Sensitive).unwrap();
    let mut meta = meta;
    meta.encryption = Some(enc);
    // Metadata content hash still matches (unchanged bytes).
    assert_eq!(meta.content_hash, h);
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let (read_meta, _) = store.get(&tenant(), &id, &correlation()).unwrap();
    assert!(read_meta.encryption.is_some());
    teardown(&root);
}
