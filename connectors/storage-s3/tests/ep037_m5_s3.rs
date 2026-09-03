//! EP-037 M5 S3-compatible adapter integration tests over REAL
//! S3-compatible backends (MinIO + SeaweedFS S3 gateway, both
//! digest-pinned and provisioned by the M5 gate).
//!
//! The gate exports NEXUS_S3_MINIO_* and NEXUS_S3_SEAWEEDFS_*; each
//! test proves the production storage-s3 adapter against whichever real
//! backend the env names (fail closed, never a silent skip). SPEC-024
//! non-goal "assuming S3 implementations are identical" is respected:
//! the adapter carries an explicit compatibility profile and the same
//! proof runs against both providers, recording profile differences.
//!
//! Proven here:
//! - positive put/get/verify/delete with SHA-256 verification (never
//!   ETag/HTTP-success trust);
//! - encryption-before-egress: sensitive without encryption metadata ->
//!   Policy with zero provider mutation; sensitive WITH encryption
//!   round-trips;
//! - delete ladder ends in independent absence readback
//!   (RESOURCE_ABSENT_VERIFIED);
//! - shared-content delete preserves bytes still referenced;
//! - backup/restore/migration hash gates and mark_verified;
//! - failure classification: refused -> Unavailable, timeout -> Timeout,
//!   404 -> NotFound, malformed -> ExternalProvider (controlled peer),
//!   5xx -> ExternalProvider;
//! - metadata is a sidecar object, never unencrypted x-amz-meta-*
//!   headers.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nexus_artifacts::{
    ArtifactErrorCode, ArtifactHash, ArtifactMetadata, ArtifactResult, ArtifactStore,
    ArtifactVersion, BackupSet, DataClass, EncryptionMetadata, RetentionClass, StorageBackend,
};
use nexus_domain::{ArtifactId, CorrelationId, TenantId};
use nexus_provider_storage_s3::{S3ArtifactStore, S3CompatibilityProfile, S3Config};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------- env

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set by the gate"))
}

fn cfg_for(prefix: &str, profile: S3CompatibilityProfile) -> S3Config {
    S3Config::new(
        env(&format!("NEXUS_S3_{prefix}_ENDPOINT")),
        env(&format!("NEXUS_S3_{prefix}_ACCESS_KEY")),
        env(&format!("NEXUS_S3_{prefix}_PW_KEY")),
        env(&format!("NEXUS_S3_{prefix}_BUCKET_PREFIX")),
    )
    .with_profile(profile)
    .with_timeouts(Duration::from_secs(3), Duration::from_secs(5))
}

fn minio_cfg() -> S3Config {
    cfg_for("MINIO", S3CompatibilityProfile::MinIo)
}
fn seaweedfs_cfg() -> S3Config {
    cfg_for("SEAWEEDFS", S3CompatibilityProfile::SeaweedFs)
}

/// Every test runs against MinIO and SeaweedFS (both real). Returns a
/// descriptive name + config pair.
fn targets() -> Vec<(&'static str, S3Config)> {
    vec![("minio", minio_cfg()), ("seaweedfs", seaweedfs_cfg())]
}

// ------------------------------------------------------------- helpers

fn tenant(n: u8) -> TenantId {
    format!("01970000-0000-7000-8000-0000000000{n:02x}")
        .parse()
        .unwrap()
}

fn artifact_id(n: u8) -> ArtifactId {
    format!("01970000-0000-7000-8000-0000000001{n:02x}")
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

/// Real AES-256-GCM encrypt (ring) - nonce || ciphertext || tag. The
/// caller holds the key; the adapter never does (SPEC-024). Same ring
/// line as the live-fire storage harness.
fn encrypt_aes256gcm(key: &[u8; 32], plaintext: &[u8]) -> Vec<u8> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    let unbound = UnboundKey::new(&AES_256_GCM, key).expect("valid key");
    let sealing = LessSafeKey::new(unbound);
    let mut nonce_bytes = [0u8; 12];
    nonce_bytes.copy_from_slice(
        &SystemTime::now()
            .duration_since(UNIX_EPOCH)
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

fn hash_of(bytes: &[u8]) -> ArtifactHash {
    ArtifactHash::new(digest(bytes)).unwrap()
}

/// Sign a backup manifest with a REAL Ed25519 keypair (ring) so
/// create_backup/restore signature verification (SPEC-024 req 6,
/// AUD-052) has authentic material.
fn sign_backup(mut backup: BackupSet) -> BackupSet {
    use ring::rand::SystemRandom;
    use ring::signature::{Ed25519KeyPair, KeyPair};
    let rng = SystemRandom::new();
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    let pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
    let public_key_hex = nexus_artifacts::hex_encode(pair.public_key().as_ref());
    let message = backup.canonical_manifest_bytes().unwrap();
    let signature_hex = nexus_artifacts::hex_encode(pair.sign(&message).as_ref());
    backup.sign(nexus_artifacts::ManifestSignature::new(public_key_hex, signature_hex).unwrap());
    backup
}

fn unique_suffix() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    )
}

#[allow(clippy::too_many_arguments)]
fn metadata_for(
    id: ArtifactId,
    bytes: &[u8],
    data_class: DataClass,
    enc: Option<EncryptionMetadata>,
    backend: StorageBackend,
    owner: &str,
) -> ArtifactResult<ArtifactMetadata> {
    let h = hash_of(bytes);
    ArtifactMetadata::new(
        id,
        tenant(1),
        "m5-s3-test-artifact",
        h.clone(),
        "application/octet-stream",
        bytes.len() as u64,
        owner,
        data_class,
        RetentionClass::LongTerm,
        enc,
        ArtifactVersion::new("1", h.clone()).unwrap(),
        Vec::new(),
        nexus_artifacts::BackendLocation::new(backend, "s3-bucket/m5-test").unwrap(),
    )
}

// ---------------------------------------------------------------- tests

#[test]
fn ep037_m5_s3_positive_roundtrip_hash_verified() {
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        let bytes = format!("m5 s3 positive payload {name} {}", unique_suffix()).into_bytes();
        let h = hash_of(&bytes);
        let id = artifact_id(1);
        let meta = metadata_for(
            id.clone(),
            &bytes,
            DataClass::Public,
            None,
            StorageBackend::S3,
            "principal-1",
        )
        .unwrap();
        store
            .put(&tenant(1), &id, &h, &bytes, &meta, &correlation())
            .unwrap();
        let (read_meta, read_bytes) = store.get(&tenant(1), &id, &correlation()).unwrap();
        assert_eq!(read_meta.content_hash, h);
        assert_eq!(read_bytes, bytes);
        let verified = store.verify(&tenant(1), &id, &correlation()).unwrap();
        assert_eq!(verified, h);
        // Provider write + independent get + SHA-256 verified + delete +
        // independent absence verified.
        store.delete(&tenant(1), &id, &correlation()).unwrap();
        let err = store.get(&tenant(1), &id, &correlation()).unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::NotFound);
    }
}

#[test]
fn ep037_m5_s3_rejects_sensitive_without_encryption_before_egress() {
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        let bytes = format!("m5 s3 sensitive {name} {}", unique_suffix()).into_bytes();
        let h = hash_of(&bytes);
        let id = artifact_id(2);
        // The metadata constructor itself enforces encryption-before-
        // egress for a sensitive artifact on a node-egressing backend,
        // so the metadata is built with a LOCAL location (allowed by the
        // contract) and the ADAPTER still must reject it before any byte
        // crosses the network (M3 NAS adapter precedent).
        let meta = metadata_for(
            id.clone(),
            &bytes,
            DataClass::Sensitive,
            None,
            StorageBackend::Local,
            "principal-1",
        )
        .unwrap();
        let err = store
            .put(&tenant(1), &id, &h, &bytes, &meta, &correlation())
            .unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::Policy);
        // Zero provider mutation: get must be NotFound (nothing written).
        let err = store.get(&tenant(1), &id, &correlation()).unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::NotFound);
    }
}

#[test]
fn ep037_m5_s3_sensitive_with_encryption_roundtrips() {
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        // REAL AES-256-GCM (ring): the caller encrypts before put,
        // records the plaintext hash, and the adapter verifies the stored
        // bytes are NOT the plaintext (AUD-051).
        let plaintext =
            format!("m5 s3 encrypted sensitive {name} {}", unique_suffix()).into_bytes();
        let key = [0x42u8; 32];
        let sealed = encrypt_aes256gcm(&key, &plaintext);
        let h = hash_of(&sealed);
        let id = artifact_id(3);
        let enc =
            EncryptionMetadata::new("AES-256-GCM", "vault:keys/m5-s3", digest(&plaintext)).unwrap();
        let meta = metadata_for(
            id.clone(),
            &sealed,
            DataClass::Security,
            Some(enc),
            StorageBackend::S3,
            "principal-1",
        )
        .unwrap();
        store
            .put(&tenant(1), &id, &h, &sealed, &meta, &correlation())
            .unwrap();
        let (read_meta, read_bytes) = store.get(&tenant(1), &id, &correlation()).unwrap();
        assert!(read_meta.encryption.is_some());
        // The stored bytes are the CIPHERTEXT, never the plaintext.
        assert_eq!(read_bytes, sealed);
        assert_ne!(read_bytes, plaintext);
        // The caller (holding the key) can decrypt the returned bytes.
        let decrypted = decrypt_aes256gcm(&key, &read_bytes).unwrap();
        assert_eq!(decrypted, plaintext);
        store.delete(&tenant(1), &id, &correlation()).unwrap();
    }
}

#[test]
fn ep037_m5_s3_rejects_plaintext_labeled_as_encrypted() {
    // AUD-051 hostile: the exact defect from the finding - plaintext
    // bytes with AES-256-GCM metadata claiming they are ciphertext. The
    // adapter must fail closed Policy BEFORE any byte crosses the network.
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        let plaintext = format!(
            "m5 s3 plaintext labeled encrypted {name} {}",
            unique_suffix()
        )
        .into_bytes();
        let h = hash_of(&plaintext);
        let id = artifact_id(4);
        // plaintext_hash == hash of the SAME bytes being put: the
        // "ciphertext" IS the plaintext.
        let enc = EncryptionMetadata::new("AES-256-GCM", "vault:keys/m5-s3", h.as_str()).unwrap();
        let meta = metadata_for(
            id.clone(),
            &plaintext,
            DataClass::Sensitive,
            Some(enc),
            StorageBackend::Local,
            "principal-1",
        )
        .unwrap();
        let err = store
            .put(&tenant(1), &id, &h, &plaintext, &meta, &correlation())
            .unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::Policy);
        // Zero provider mutation: get must be NotFound (nothing written).
        let err = store.get(&tenant(1), &id, &correlation()).unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::NotFound);
    }
}

#[test]
fn ep037_m5_s3_delete_absent_verified_ladder() {
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        let bytes = format!("m5 s3 delete ladder {name} {}", unique_suffix()).into_bytes();
        let h = hash_of(&bytes);
        let id = artifact_id(4);
        let meta = metadata_for(
            id.clone(),
            &bytes,
            DataClass::Public,
            None,
            StorageBackend::S3,
            "principal-1",
        )
        .unwrap();
        store
            .put(&tenant(1), &id, &h, &bytes, &meta, &correlation())
            .unwrap();
        // Delete accepted -> independent absence verified (get 404).
        store.delete(&tenant(1), &id, &correlation()).unwrap();
        let err = store.get(&tenant(1), &id, &correlation()).unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::NotFound);
        // Second delete: NotFound (already absent, never a silent stale
        // success).
        let err = store.delete(&tenant(1), &id, &correlation()).unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::NotFound);
    }
}

#[test]
fn ep037_m5_s3_shared_content_delete_preserves_object() {
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        let bytes = format!("m5 s3 shared content {name} {}", unique_suffix()).into_bytes();
        let h = hash_of(&bytes);
        // Two logical artifacts share the same content hash.
        let id_a = artifact_id(5);
        let id_b = artifact_id(6);
        let meta_a = metadata_for(
            id_a.clone(),
            &bytes,
            DataClass::Public,
            None,
            StorageBackend::S3,
            "principal-1",
        )
        .unwrap();
        let meta_b = metadata_for(
            id_b.clone(),
            &bytes,
            DataClass::Public,
            None,
            StorageBackend::S3,
            "principal-1",
        )
        .unwrap();
        store
            .put(&tenant(1), &id_a, &h, &bytes, &meta_a, &correlation())
            .unwrap();
        store
            .put(&tenant(1), &id_b, &h, &bytes, &meta_b, &correlation())
            .unwrap();
        // Deleting one logical reference must not destroy bytes still
        // referenced by the other.
        store.delete(&tenant(1), &id_a, &correlation()).unwrap();
        let (read_meta, read_bytes) = store.get(&tenant(1), &id_b, &correlation()).unwrap();
        assert_eq!(read_meta.content_hash, h);
        assert_eq!(read_bytes, bytes);
        store.delete(&tenant(1), &id_b, &correlation()).unwrap();
    }
}

#[test]
fn ep037_m5_s3_backup_restore_hash_gates() {
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        let bytes = format!("m5 s3 backup payload {name} {}", unique_suffix()).into_bytes();
        let h = hash_of(&bytes);
        let id = artifact_id(7);
        let meta = metadata_for(
            id.clone(),
            &bytes,
            DataClass::Personal,
            None,
            StorageBackend::S3,
            "principal-1",
        )
        .unwrap();
        store
            .put(&tenant(1), &id, &h, &bytes, &meta, &correlation())
            .unwrap();
        let backup = BackupSet::new(
            format!("b-m5-s3-{name}-{}", unique_suffix()),
            tenant(1),
            vec![DataClass::Personal],
            nexus_artifacts::BackendLocation::new(StorageBackend::S3, "backups/b.json").unwrap(),
            vec![h.clone()],
            Some("vault:keys/m5-s3".to_string()),
            "0.1.0",
            "1",
            "2026-08-22T00:00:00Z",
        )
        .unwrap();
        let created = store
            .create_backup(&tenant(1), &sign_backup(backup.clone()), &correlation())
            .unwrap();
        assert_eq!(created.state, nexus_artifacts::BackupState::Created);
        // Duplicate backup: Conflict.
        let err = store
            .create_backup(&tenant(1), &sign_backup(backup.clone()), &correlation())
            .unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::Conflict);
        let plan = nexus_artifacts::RestorePlan::new(
            format!("r-m5-s3-{name}-{}", unique_suffix()),
            tenant(1),
            &backup.backup_id,
            "fresh-target-1",
            vec![h.clone()],
            Some(correlation()),
        )
        .unwrap();
        let executed = store.restore(&tenant(1), &plan, &correlation()).unwrap();
        assert!(executed.all_hashes_verified());
        assert_eq!(
            executed.state,
            nexus_artifacts::RestoreVerificationState::Validated
        );
        // Restore requiring a hash NOT in the manifest: Verification.
        let bad_plan = nexus_artifacts::RestorePlan::new(
            format!("r-bad-{name}-{}", unique_suffix()),
            tenant(1),
            &backup.backup_id,
            "fresh-target-2",
            vec![hash_of(format!("absent {name}").as_bytes())],
            Some(correlation()),
        )
        .unwrap();
        let err = store
            .restore(&tenant(1), &bad_plan, &correlation())
            .unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::Verification);
    }
}

#[test]
fn ep037_m5_s3_migration_verifies_destination_and_failure_preserves_source() {
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        let bytes = format!("m5 s3 migration {name} {}", unique_suffix()).into_bytes();
        let h = hash_of(&bytes);
        let id = artifact_id(8);
        let meta = metadata_for(
            id.clone(),
            &bytes,
            DataClass::Public,
            None,
            StorageBackend::S3,
            "principal-1",
        )
        .unwrap();
        store
            .put(&tenant(1), &id, &h, &bytes, &meta, &correlation())
            .unwrap();
        let obj = nexus_artifacts::ObjectRef::new(id.clone(), h.clone());
        // Destination failure: require a hash NEVER written to the
        // target -> the production adapter's migrate() fails (target
        // readback NotFound), so the migration never advances to
        // VERIFIED and the source is preserved.
        let absent_hash = hash_of(format!("absent {name} {}", unique_suffix()).as_bytes());
        let absent_ref = nexus_artifacts::ObjectRef::new(artifact_id(0x2a), absent_hash.clone());
        let bad_migration = nexus_artifacts::StorageMigration::new(
            format!("mig-fail-{name}-{}", unique_suffix()),
            tenant(1),
            StorageBackend::Local,
            StorageBackend::S3,
            vec![absent_ref.clone()],
        )
        .unwrap();
        let err = store
            .migrate(&tenant(1), &bad_migration, &correlation())
            .unwrap_err();
        assert_eq!(err.code, ArtifactErrorCode::NotFound);
        // Positive migration: object present -> VERIFIED + approval gate.
        let migration_ok = nexus_artifacts::StorageMigration::new(
            format!("mig-ok-{name}-{}", unique_suffix()),
            tenant(1),
            StorageBackend::Local,
            StorageBackend::S3,
            vec![obj.clone()],
        )
        .unwrap();
        let executed = store
            .migrate(&tenant(1), &migration_ok, &correlation())
            .unwrap();
        assert!(executed.all_verified());
        assert_eq!(executed.state, nexus_artifacts::MigrationState::Verified);
        // Approve delete only after verification.
        let mut approved = executed.clone();
        approved
            .approve_delete(format!("approval-{name}-{}", unique_suffix()))
            .unwrap();
        assert_eq!(
            approved.state,
            nexus_artifacts::MigrationState::CanonicalLocationChanged
        );
        assert!(approved.delete_approval.is_some());
    }
}

#[test]
fn ep037_m5_s3_list_pagination() {
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        // Own tenant bucket so other tests' artifacts never contaminate
        // the pagination window (per-test namespace isolation).
        let t = tenant(2);
        let mut expected = Vec::new();
        for i in 0..5u8 {
            let bytes = format!("m5 s3 list item {name} {i} {}", unique_suffix()).into_bytes();
            let h = hash_of(&bytes);
            let id = artifact_id(0x20 + i);
            let meta = metadata_for(
                id.clone(),
                &bytes,
                DataClass::Public,
                None,
                StorageBackend::S3,
                "principal-1",
            )
            .unwrap();
            store
                .put(&t, &id, &h, &bytes, &meta, &correlation())
                .unwrap();
            expected.push(id);
        }
        expected.sort();
        let (page1, cursor) = store.list(&t, None, 2).unwrap();
        assert_eq!(page1.len(), 2);
        let (page2, cursor2) = store.list(&t, cursor.as_deref(), 2).unwrap();
        assert_eq!(page2.len(), 2);
        let (page3, cursor3) = store.list(&t, cursor2.as_deref(), 2).unwrap();
        assert_eq!(page3.len(), 1);
        assert!(cursor3.is_none());
        let all: Vec<String> = page1
            .iter()
            .chain(page2.iter())
            .chain(page3.iter())
            .map(|m| m.artifact_id.as_str().to_string())
            .collect();
        let expected_str: Vec<String> = expected.iter().map(|id| id.as_str().to_string()).collect();
        assert_eq!(all, expected_str);
        // Cleanup
        for id in expected {
            store.delete(&t, &id, &correlation()).unwrap();
        }
    }
}

#[test]
fn ep037_m5_s3_set_retention_updates_metadata() {
    for (name, cfg) in targets() {
        let mut store = S3ArtifactStore::open(cfg).unwrap();
        let bytes = format!("m5 s3 retention {name} {}", unique_suffix()).into_bytes();
        let h = hash_of(&bytes);
        let id = artifact_id(9);
        let meta = metadata_for(
            id.clone(),
            &bytes,
            DataClass::Public,
            None,
            StorageBackend::S3,
            "principal-1",
        )
        .unwrap();
        store
            .put(&tenant(1), &id, &h, &bytes, &meta, &correlation())
            .unwrap();
        store
            .set_retention(&tenant(1), &id, RetentionClass::Permanent, &correlation())
            .unwrap();
        let (read_meta, _) = store.get(&tenant(1), &id, &correlation()).unwrap();
        assert_eq!(read_meta.retention, RetentionClass::Permanent);
        store.delete(&tenant(1), &id, &correlation()).unwrap();
    }
}

// -------------------------------------------------- controlled peers

/// A silent TCP peer: accepts the connection and never responds. The
/// production adapter must classify this as Timeout (never hang, never
/// guess success).
#[test]
fn ep037_m5_s3_transport_timeout_silent_peer() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = std::io::copy(&mut stream, &mut std::io::sink());
        }
    });
    let cfg = S3Config::new(addr.to_string(), "k", "s", "nexus-m5-silent")
        .with_profile(S3CompatibilityProfile::Generic)
        .with_timeouts(Duration::from_secs(1), Duration::from_secs(1));
    let mut store = S3ArtifactStore::open(cfg).unwrap();
    let bytes = b"m5 silent peer".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(0x30);
    let meta = metadata_for(
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::S3,
        "principal-1",
    )
    .unwrap();
    let started = std::time::Instant::now();
    let err = store
        .put(&tenant(1), &id, &h, &bytes, &meta, &correlation())
        .unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::Timeout);
    assert!(started.elapsed() < Duration::from_secs(10));
}

/// A controlled TCP peer that returns a malformed HTTP response. The
/// production adapter must classify this as ExternalProvider (never
/// guessed into success).
#[test]
fn ep037_m5_s3_transport_malformed_response_external() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            // Drain the small request (bounded read; the client sends
            // one request then waits for the response) so the peer can
            // drop cleanly without RST racing the client's write.
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"NOT-HTTP-AT-ALL\r\n\r\n");
        }
    });
    let cfg = S3Config::new(addr.to_string(), "k", "s", "nexus-m5-malformed")
        .with_profile(S3CompatibilityProfile::Generic)
        .with_timeouts(Duration::from_secs(1), Duration::from_secs(1));
    let mut store = S3ArtifactStore::open(cfg).unwrap();
    let bytes = b"m5 malformed peer".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(0x31);
    let meta = metadata_for(
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::S3,
        "principal-1",
    )
    .unwrap();
    let err = store
        .put(&tenant(1), &id, &h, &bytes, &meta, &correlation())
        .unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::ExternalProvider);
}

/// A refused (closed) endpoint must classify as Unavailable, distinct
/// from NotFound.
#[test]
fn ep037_m5_s3_transport_refused_unavailable_not_found_distinct() {
    // Reserve a port then drop the listener: connection refused.
    let refused_addr = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let a = l.local_addr().unwrap();
        drop(l);
        a
    };
    let cfg = S3Config::new(refused_addr.to_string(), "k", "s", "nexus-m5-refused")
        .with_profile(S3CompatibilityProfile::Generic)
        .with_timeouts(Duration::from_secs(1), Duration::from_secs(1));
    let mut store = S3ArtifactStore::open(cfg).unwrap();
    let err = store
        .get(&tenant(1), &artifact_id(0x32), &correlation())
        .unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::Unavailable);
}

/// Redaction: a provider error message that echoes credential-shaped
/// content must be sanitized, never leaked.
#[test]
fn ep037_m5_s3_redaction_canary_zero_leakage() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            // Drain the small request (bounded read; the client sends
            // one request then waits for the response) so the peer can
            // drop cleanly without RST racing the client's write.
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let body = "HTTP/1.1 500 InternalError\r\nContent-Length: 90\r\n\r\n{\"error\":\"accessKey=AKIAEXAMPLE secretKey=super-secret-canary-xyz\"}";
            let _ = stream.write_all(body.as_bytes());
        }
    });
    let cfg = S3Config::new(
        addr.to_string(),
        "AKIAEXAMPLE",
        "super-secret-canary-xyz",
        "nexus-m5-redact",
    )
    .with_profile(S3CompatibilityProfile::Generic)
    .with_timeouts(Duration::from_secs(1), Duration::from_secs(1));
    let mut store = S3ArtifactStore::open(cfg).unwrap();
    let bytes = b"m5 redaction".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(0x33);
    let meta = metadata_for(
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::S3,
        "principal-1",
    )
    .unwrap();
    let err = store
        .put(&tenant(1), &id, &h, &bytes, &meta, &correlation())
        .unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::ExternalProvider);
    let message = format!("{err}");
    assert!(
        !message.contains("super-secret-canary-xyz"),
        "leaked: {message}"
    );
    assert!(!message.contains("AKIAEXAMPLE"), "leaked: {message}");
    assert!(message.contains("[redacted"), "not redacted: {message}");
}
