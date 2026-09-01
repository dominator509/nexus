//! EP-037 M3 NAS adapter behavior tests over REAL filesystem roots.
//!
//! The NAS adapter is a real filesystem-backed store over a NAS mount
//! root with the encryption-before-egress policy enforced at the adapter
//! boundary (NAS leaves the node). Tests exercise the real adapter:
//! sensitive artifacts without encryption metadata are rejected BEFORE
//! any byte reaches the share; encrypted sensitive artifacts round-trip;
//! public artifacts round-trip; delete verifies absence.

use std::fs;
use std::path::PathBuf;

use nexus_artifacts::{
    ArtifactErrorCode, ArtifactHash, ArtifactMetadata, ArtifactResult, ArtifactStore,
    ArtifactVersion, DataClass, EncryptionMetadata, RetentionClass, StorageBackend,
};
use nexus_domain::{ArtifactId, CorrelationId, TenantId};
use nexus_provider_storage_nas::NasArtifactStore;
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
    enc: Option<EncryptionMetadata>,
    backend: StorageBackend,
) -> ArtifactResult<ArtifactMetadata> {
    let h = hash_of(bytes);
    ArtifactMetadata::new(
        id,
        tenant(),
        "m3-nas-test-artifact",
        h.clone(),
        "application/octet-stream",
        bytes.len() as u64,
        "principal-1",
        data_class,
        RetentionClass::LongTerm,
        enc,
        ArtifactVersion::new("1", h.clone()).unwrap(),
        Vec::new(),
        nexus_artifacts::BackendLocation::new(backend, "nas-share/m3-test").unwrap(),
    )
}

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nexus-ep037-m3-nas-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn teardown(root: &PathBuf) {
    let _ = fs::remove_dir_all(root);
}

/// Real AES-256-GCM encrypt (ring) - nonce || ciphertext || tag. The
/// caller holds the key; the adapter never does (SPEC-024). Same ring
/// line as the live-fire storage harness.
fn encrypt_aes256gcm(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    let unbound = UnboundKey::new(&AES_256_GCM, key).expect("valid key");
    let sealing = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(
        &std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_le_bytes()[..12],
    );
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = plaintext.to_vec();
    sealing
        .seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .expect("seal");
    let mut out = Vec::with_capacity(12 + in_out.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&in_out);
    out
}

/// Real AES-256-GCM decrypt (ring). Returns Err on wrong/missing key.
fn decrypt_aes256gcm(key: &[u8; 32], sealed: &[u8]) -> Result<Vec<u8>, ring::error::Unspecified> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    if sealed.len() < 12 {
        return Err(ring::error::Unspecified);
    }
    let unbound = UnboundKey::new(&AES_256_GCM, key).expect("valid key");
    let opening = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(&sealed[..12]);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let mut in_out = sealed[12..].to_vec();
    let plain = opening.open_in_place(nonce, Aad::empty(), &mut in_out)?;
    Ok(plain.to_vec())
}

#[test]
fn ep037_integration_nas_public_artifact_roundtrip() {
    let root = temp_root("public");
    let mut store = NasArtifactStore::open(&root).unwrap();
    let bytes = b"nas public payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(1);
    let meta = metadata_for(
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::Nas,
    )
    .unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let (read_meta, read_bytes) = store.get(&tenant(), &id, &correlation()).unwrap();
    assert_eq!(read_meta.content_hash, h);
    assert_eq!(read_bytes, bytes);
    let verified = store.verify(&tenant(), &id, &correlation()).unwrap();
    assert_eq!(verified, h);
    teardown(&root);
}

#[test]
fn ep037_integration_nas_rejects_sensitive_without_encryption_before_egress() {
    let root = temp_root("sensitive-blocked");
    let mut store = NasArtifactStore::open(&root).unwrap();
    let bytes = b"nas sensitive payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(2);
    let meta = metadata_for(
        id.clone(),
        &bytes,
        DataClass::Sensitive,
        None,
        StorageBackend::Local,
    )
    .unwrap();
    let err = store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::Policy);
    // Fail closed BEFORE any byte reaches the share: no object file.
    assert!(!root.join("objects").join(h.as_str()).exists());
    teardown(&root);
}

#[test]
fn ep037_integration_nas_sensitive_with_encryption_roundtrips() {
    let root = temp_root("sensitive-enc");
    let mut store = NasArtifactStore::open(&root).unwrap();
    // REAL AES-256-GCM (ring): the caller encrypts before put, records
    // the plaintext hash, and the adapter verifies the stored bytes are
    // NOT the plaintext (AUD-051).
    let plaintext = b"nas encrypted sensitive payload".to_vec();
    let key = [0x42u8; 32];
    let sealed = encrypt_aes256gcm(&key, &plaintext);
    let h = hash_of(&sealed);
    let id = artifact_id(3);
    let plaintext_hash = digest(&plaintext);
    let enc = EncryptionMetadata::new("AES-256-GCM", "vault:keys/nas-m3", &plaintext_hash).unwrap();
    let meta = metadata_for(
        id.clone(),
        &sealed,
        DataClass::Security,
        Some(enc),
        StorageBackend::Nas,
    )
    .unwrap();
    store
        .put(&tenant(), &id, &h, &sealed, &meta, &correlation())
        .unwrap();
    let (read_meta, read_bytes) = store.get(&tenant(), &id, &correlation()).unwrap();
    assert!(read_meta.encryption.is_some());
    // The stored bytes are the CIPHERTEXT, never the plaintext.
    assert_eq!(read_bytes, sealed);
    assert_ne!(read_bytes, plaintext);
    // The caller (holding the key) can decrypt the returned bytes.
    let decrypted = decrypt_aes256gcm(&key, &read_bytes).unwrap();
    assert_eq!(decrypted, plaintext);
    teardown(&root);
}

#[test]
fn ep037_integration_nas_rejects_plaintext_labeled_as_encrypted() {
    // AUD-051 hostile: the exact defect from the finding - plaintext
    // bytes with AES-256-GCM metadata claiming they are ciphertext. The
    // adapter must fail closed Policy BEFORE any byte reaches the share.
    let root = temp_root("sensitive-fake-enc");
    let mut store = NasArtifactStore::open(&root).unwrap();
    let plaintext = b"nas plaintext labeled encrypted".to_vec();
    let h = hash_of(&plaintext);
    let id = artifact_id(4);
    // plaintext_hash == hash of the SAME bytes the caller is putting:
    // the "ciphertext" IS the plaintext.
    let enc = EncryptionMetadata::new("AES-256-GCM", "vault:keys/nas-m3", h.as_str()).unwrap();
    let meta = metadata_for(
        id.clone(),
        &plaintext,
        DataClass::Sensitive,
        Some(enc),
        StorageBackend::Local,
    )
    .unwrap();
    let err = store
        .put(&tenant(), &id, &h, &plaintext, &meta, &correlation())
        .unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::Policy);
    // Fail closed: no object written.
    assert!(!root.join("objects").join(h.as_str()).exists());
    teardown(&root);
}

#[test]
fn ep037_integration_nas_delete_verifies_absence() {
    let root = temp_root("delete");
    let mut store = NasArtifactStore::open(&root).unwrap();
    let bytes = b"nas delete payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(4);
    let meta = metadata_for(
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::Nas,
    )
    .unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    store.delete(&tenant(), &id, &correlation()).unwrap();
    assert!(!root.join("index").join(format!("{id}.json")).exists());
    assert!(!root.join("objects").join(h.as_str()).exists());
    teardown(&root);
}

#[test]
fn ep037_integration_nas_backup_manifest_and_restore_validation() {
    let root = temp_root("backup");
    let mut store = NasArtifactStore::open(&root).unwrap();
    let bytes = b"nas backup payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(5);
    let meta = metadata_for(
        id.clone(),
        &bytes,
        DataClass::Personal,
        None,
        StorageBackend::Nas,
    )
    .unwrap();
    store
        .put(&tenant(), &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let backup = nexus_artifacts::BackupSet::new(
        "b-nas-m3",
        tenant(),
        vec![DataClass::Personal],
        nexus_artifacts::BackendLocation::new(StorageBackend::Nas, "nas-backups/b-nas-m3.json")
            .unwrap(),
        vec![h.clone()],
        Some("vault:keys/nas-m3".to_string()),
        "0.1.0",
        "1",
        "2026-08-22T00:00:00Z",
    )
    .unwrap();
    let created = store
        .create_backup(&tenant(), &backup, &correlation())
        .unwrap();
    assert_eq!(created.state, nexus_artifacts::BackupState::Created);
    let plan = nexus_artifacts::RestorePlan::new(
        "r-nas-m3",
        tenant(),
        "b-nas-m3",
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
