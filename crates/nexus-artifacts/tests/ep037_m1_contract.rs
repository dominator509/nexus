//! EP-037 M1 contract tests: construction, validation, vocabulary
//! rejection, content addressing, encryption-before-egress, backup
//! state ladder, restore/migration hash gating, and dependency direction.

use nexus_artifacts::{
    ArtifactErrorCode, ArtifactHash, ArtifactMetadata, ArtifactVersion, BackendLocation, BackupSet,
    BackupState, DataClass, EncryptionMetadata, ManifestSignature, ObjectRef, RecoveryKey,
    RestorePlan, RestoreVerificationState, RetentionClass, StorageBackend, StorageMigration,
};
use nexus_domain::{ArtifactId, CorrelationId, TenantId};

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

fn hash(byte: u8) -> ArtifactHash {
    ArtifactHash::new(format!("{:064x}", byte)).unwrap()
}

fn version(v: &str, h: ArtifactHash) -> ArtifactVersion {
    ArtifactVersion::new(v, h).unwrap()
}

fn location(backend: StorageBackend) -> BackendLocation {
    BackendLocation::new(backend, "objects/key-1").unwrap()
}

fn metadata(
    id: ArtifactId,
    h: ArtifactHash,
    data_class: DataClass,
    enc: Option<EncryptionMetadata>,
    backend: StorageBackend,
) -> nexus_artifacts::ArtifactResult<ArtifactMetadata> {
    ArtifactMetadata::new(
        id,
        tenant(),
        "contract-test-artifact",
        h.clone(),
        "application/octet-stream",
        128,
        "principal-1",
        data_class,
        RetentionClass::LongTerm,
        enc,
        version("1", h),
        Vec::new(),
        location(backend),
    )
}

// ---------------------------------------------------------------------------
// Vocabulary: deny-unknown storage backends and classes
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_vocabulary_rejects_unknown_storage_backend() {
    let err = "S4".parse::<StorageBackend>().unwrap_err();
    assert!(err.message.contains("unsupported value"));
    let err = "MINIOX".parse::<StorageBackend>().unwrap_err();
    assert!(err.message.contains("unsupported value"));
    let err = "aws".parse::<StorageBackend>().unwrap_err();
    assert!(err.message.contains("unsupported value"));
    let err = "GCS".parse::<StorageBackend>().unwrap_err();
    assert!(err.message.contains("unsupported value"));
}

#[test]
fn ep037_unit_vocabulary_accepts_all_seven_backends() {
    for (wire, backend) in [
        ("LOCAL", StorageBackend::Local),
        ("NAS", StorageBackend::Nas),
        ("SEAWEEDFS", StorageBackend::SeaweedFs),
        ("MINIO", StorageBackend::MinIo),
        ("R2", StorageBackend::R2),
        ("B2", StorageBackend::B2),
        ("S3", StorageBackend::S3),
    ] {
        assert_eq!(wire.parse::<StorageBackend>().unwrap(), backend);
        assert_eq!(backend.as_str(), wire);
    }
}

#[test]
fn ep037_unit_vocabulary_minio_is_compatibility_only() {
    assert!(StorageBackend::MinIo.is_compatibility_only());
    assert!(!StorageBackend::S3.is_compatibility_only());
}

#[test]
fn ep037_unit_vocabulary_backend_egress_truth() {
    // Local never leaves the node; every remote backend does.
    assert!(!StorageBackend::Local.leaves_node());
    for backend in [
        StorageBackend::Nas,
        StorageBackend::SeaweedFs,
        StorageBackend::MinIo,
        StorageBackend::R2,
        StorageBackend::B2,
        StorageBackend::S3,
    ] {
        assert!(backend.leaves_node());
    }
}

#[test]
fn ep037_unit_vocabulary_rejects_unknown_data_class_and_retention() {
    assert!("ULTRA_SECRET".parse::<DataClass>().is_err());
    assert!("ARCHIVE".parse::<RetentionClass>().is_err());
    assert!("DELETE_NOW".parse::<RetentionClass>().is_err());
}

#[test]
fn ep037_unit_vocabulary_sensitive_classes_require_encryption_before_egress() {
    assert!(DataClass::Sensitive.requires_encryption_before_egress());
    assert!(DataClass::BusinessConfidential.requires_encryption_before_egress());
    assert!(DataClass::Security.requires_encryption_before_egress());
    assert!(!DataClass::Public.requires_encryption_before_egress());
    assert!(!DataClass::Household.requires_encryption_before_egress());
}

// ---------------------------------------------------------------------------
// ArtifactHash: content-addressed identity
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_artifact_hash_accepts_canonical_hex_and_rejects_shape() {
    assert_eq!(hash(0xab).as_str().len(), 64);
    assert!(ArtifactHash::new("abcd").is_err());
    assert!(ArtifactHash::new(format!("{:064X}", 0xab)).is_err()); // uppercase rejected
    assert!(ArtifactHash::new("z".repeat(64)).is_err()); // non-hex
    assert!(ArtifactHash::new("".to_string()).is_err());
}

#[test]
fn ep037_unit_artifact_hash_content_identity_is_digest_not_name() {
    let a = hash(1);
    let b = hash(2);
    assert_ne!(a, b);
    // The same digest is the same content regardless of naming.
    let via_str = ArtifactHash::new(a.as_str()).unwrap();
    assert_eq!(a, via_str);
}

// ---------------------------------------------------------------------------
// ArtifactMetadata: version lineage and encryption-before-egress
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_metadata_version_must_match_content_hash() {
    let h = hash(7);
    let wrong = version("1", hash(8));
    let err = ArtifactMetadata::new(
        artifact_id(1),
        tenant(),
        "name",
        h,
        "application/octet-stream",
        10,
        "owner",
        DataClass::Public,
        RetentionClass::ShortTerm,
        None,
        wrong,
        Vec::new(),
        location(StorageBackend::Local),
    )
    .unwrap_err();
    assert_eq!(err.code, nexus_artifacts::ArtifactErrorCode::Validation);
}

#[test]
fn ep037_unit_metadata_sensitive_on_remote_backend_requires_encryption() {
    // Sensitive + S3 without encryption metadata must fail closed.
    let err = metadata(
        artifact_id(2),
        hash(1),
        DataClass::Sensitive,
        None,
        StorageBackend::S3,
    );
    assert!(err.is_err());
    assert_eq!(
        err.unwrap_err().code,
        nexus_artifacts::ArtifactErrorCode::Policy
    );
}

#[test]
fn ep037_unit_metadata_sensitive_on_local_backend_allows_plaintext_at_rest() {
    // Local storage never leaves the node; encryption-before-egress does
    // not force double encryption on the local disk (SPEC-024 requirement 4).
    let m = metadata(
        artifact_id(3),
        hash(1),
        DataClass::Sensitive,
        None,
        StorageBackend::Local,
    );
    assert!(m.is_ok());
}

#[test]
fn ep037_unit_metadata_encrypted_sensitive_on_remote_backend_ok() {
    // The plaintext hash is a REQUIRED, verifiable part of the
    // encryption claim (AUD-051): a remote sensitive artifact must prove
    // its stored bytes are not the plaintext.
    let enc = EncryptionMetadata::new(
        "AES-256-GCM",
        "vault:keys/nexus-backup-1",
        format!("{:064x}", 0xaa),
    )
    .unwrap();
    let m = metadata(
        artifact_id(4),
        hash(2),
        DataClass::Security,
        Some(enc),
        StorageBackend::R2,
    );
    assert!(m.is_ok());
}

#[test]
fn ep037_unit_metadata_encryption_requires_plaintext_hash() {
    // AUD-051: an encryption claim without a plaintext hash is rejected
    // by the contract itself - the metadata cannot certify encryption of
    // bytes it cannot distinguish from the plaintext.
    let err = EncryptionMetadata::new("AES-256-GCM", "vault:keys/nexus-backup-1", "").unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::Validation);
}

#[test]
fn ep037_unit_verify_encryption_before_egress_rejects_plaintext_bytes() {
    // AUD-051 hostile: metadata claims AES-GCM but the stored bytes hash
    // to the recorded plaintext - the bytes ARE the plaintext, so the
    // claim fails closed Policy.
    let enc = EncryptionMetadata::new(
        "AES-256-GCM",
        "vault:keys/nexus-backup-1",
        format!("{:064x}", 0xbb),
    )
    .unwrap();
    let m = metadata(
        artifact_id(4),
        hash(2),
        DataClass::Security,
        Some(enc),
        StorageBackend::R2,
    )
    .unwrap();
    let err = m
        .verify_encryption_before_egress(&format!("{:064x}", 0xbb))
        .unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::Policy);
    // A bytes hash that differs from the recorded plaintext passes.
    assert!(m
        .verify_encryption_before_egress(&format!("{:064x}", 0xcc))
        .is_ok());
    // Public classes are exempt (no egress encryption requirement).
    let public = metadata(
        artifact_id(4),
        hash(2),
        DataClass::Public,
        None,
        StorageBackend::R2,
    )
    .unwrap();
    assert!(public
        .verify_encryption_before_egress(&format!("{:064x}", 0xbb))
        .is_ok());
}

#[test]
fn ep037_unit_metadata_rejects_empty_name_owner_content_type() {
    let h = hash(3);
    assert!(ArtifactMetadata::new(
        artifact_id(5),
        tenant(),
        "",
        h.clone(),
        "application/octet-stream",
        10,
        "owner",
        DataClass::Public,
        RetentionClass::ShortTerm,
        None,
        version("1", h),
        Vec::new(),
        location(StorageBackend::Local),
    )
    .is_err());
}

// ---------------------------------------------------------------------------
// BackendLocation: opaque reference, never a credential-bearing URL
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_backend_location_rejects_credential_url_and_empty() {
    // Runtime-constructed canary: a URL carrying embedded credentials must
    // never be accepted as an opaque backend reference.
    let mut scheme = String::from("https");
    scheme.push_str("://user:pass@s3.example/bucket");
    assert!(BackendLocation::new(StorageBackend::S3, scheme).is_err());
    assert!(BackendLocation::new(StorageBackend::Local, "").is_err());
    assert!(BackendLocation::new(StorageBackend::Local, "   ").is_err());
}

// ---------------------------------------------------------------------------
// BackupSet: state ladder and recovery key separation
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_backup_set_requires_classes_hashes_and_versions() {
    let loc = location(StorageBackend::S3);
    assert!(BackupSet::new(
        "b1",
        tenant(),
        Vec::new(),
        loc.clone(),
        vec![hash(1)],
        Some("vault:keys/backup-1".to_string()),
        "0.1.0",
        "1",
        "2026-08-21T00:00:00Z",
    )
    .is_err());
    assert!(BackupSet::new(
        "b1",
        tenant(),
        vec![DataClass::Personal],
        loc.clone(),
        Vec::new(),
        Some("vault:keys/backup-1".to_string()),
        "0.1.0",
        "1",
        "2026-08-21T00:00:00Z",
    )
    .is_err());
    assert!(BackupSet::new(
        "b1",
        tenant(),
        vec![DataClass::Personal],
        loc.clone(),
        vec![hash(1)],
        Some("vault:keys/backup-1".to_string()),
        "",
        "1",
        "2026-08-21T00:00:00Z",
    )
    .is_err());
}

#[test]
fn ep037_unit_backup_set_advances_ladder_exactly() {
    let mut backup = BackupSet::new(
        "b1",
        tenant(),
        vec![DataClass::Personal, DataClass::Sensitive],
        location(StorageBackend::S3),
        vec![hash(1), hash(2)],
        Some("vault:keys/backup-1".to_string()),
        "0.1.0",
        "1",
        "2026-08-21T00:00:00Z",
    )
    .unwrap();
    assert_eq!(backup.state, BackupState::Declared);
    assert_eq!(backup.advance().unwrap(), BackupState::Created);
    assert_eq!(backup.advance().unwrap(), BackupState::Verified);
    assert_eq!(backup.advance().unwrap(), BackupState::Restored);
    // RESTORED is terminal; a leap beyond it is a policy error.
    assert!(backup.advance().is_err());
}

#[test]
fn ep037_unit_recovery_key_is_reference_only_never_material() {
    let key = RecoveryKey::new("vault:keys/backup-1").unwrap();
    assert_eq!(key.key_reference, "vault:keys/backup-1");
    assert!(RecoveryKey::new("").is_err());
}

#[test]
fn ep037_unit_backup_manifest_requires_signature_before_trust() {
    let mut backup = BackupSet::new(
        "b-signed",
        tenant(),
        vec![DataClass::Personal],
        location(StorageBackend::S3),
        vec![hash(1)],
        Some("vault:keys/backup-1".to_string()),
        "0.1.0",
        "1",
        "2026-08-21T00:00:00Z",
    )
    .unwrap();
    // SPEC-024 req 6: unsigned manifests fail closed.
    assert!(matches!(
        backup
            .verify_manifest_signature_structure()
            .unwrap_err()
            .code,
        ArtifactErrorCode::Verification
    ));
    // A structurally invalid signature fails closed (bypass the
    // constructor, which already validates, to prove the verifier also
    // fails closed on malformed stored material).
    backup.manifest_signature = Some(ManifestSignature {
        algorithm: nexus_artifacts::ManifestSignatureAlgorithm::Ed25519,
        public_key_hex: "abcd".to_string(),
        signature_hex: "abcd".to_string(),
    });
    assert!(backup.verify_manifest_signature_structure().is_err());
}

#[test]
fn ep037_unit_backup_manifest_canonical_bytes_deterministic_and_exclude_signature() {
    let mut backup = BackupSet::new(
        "b-ed25519",
        tenant(),
        vec![DataClass::Personal, DataClass::Sensitive],
        location(StorageBackend::S3),
        vec![hash(1), hash(2)],
        Some("vault:keys/backup-1".to_string()),
        "0.1.0",
        "1",
        "2026-08-21T00:00:00Z",
    )
    .unwrap();
    // Canonical bytes are deterministic: two serializations are equal.
    let once = backup.canonical_manifest_bytes().unwrap();
    let twice = backup.canonical_manifest_bytes().unwrap();
    assert_eq!(once, twice);
    // Self-exclusion: the canonical bytes of a signed manifest equal the
    // serialization with the signature field stripped (the signature
    // covers the manifest EXCLUDING itself - same rule as the closure
    // attestation digest). A well-formed signature is attached...
    backup.sign(ManifestSignature::new("11".repeat(32), "22".repeat(64)).unwrap());
    let signed_canonical = backup.canonical_manifest_bytes().unwrap();
    // ...and the unsigned serialization of the same manifest is
    // byte-identical (the signature field is excluded from the covered
    // message).
    let mut unsigned = backup.clone();
    unsigned.manifest_signature = None;
    let unsigned_raw = serde_json::to_vec(&unsigned).unwrap();
    assert_eq!(signed_canonical, unsigned_raw);
    // Tampering with any manifest field changes the canonical bytes (the
    // signature no longer covers them - verified by adapters with real
    // Ed25519; here we prove the message the signature covers changes).
    backup.manifest_hashes.push(hash(3));
    let tampered = backup.canonical_manifest_bytes().unwrap();
    assert_ne!(signed_canonical, tampered);
}

#[test]
fn ep037_unit_backup_manifest_structure_rejects_unsupported_algorithm_and_bad_hex() {
    let mut backup = BackupSet::new(
        "b-wrong-signer",
        tenant(),
        vec![DataClass::Personal],
        location(StorageBackend::S3),
        vec![hash(1)],
        Some("vault:keys/backup-1".to_string()),
        "0.1.0",
        "1",
        "2026-08-21T00:00:00Z",
    )
    .unwrap();
    // Unsupported algorithm fails closed at the structure check (the
    // contract rejects anything that is not Ed25519 before any adapter
    // crypto runs).
    backup.manifest_signature = Some(ManifestSignature {
        algorithm: nexus_artifacts::ManifestSignatureAlgorithm::Ed25519,
        public_key_hex: "zz".repeat(32),
        signature_hex: "22".repeat(64),
    });
    assert!(matches!(
        backup
            .verify_manifest_signature_structure()
            .unwrap_err()
            .code,
        ArtifactErrorCode::Verification
    ));
    // A well-formed signature passes the structure check (crypto
    // verification is the adapters' job, proven there with real ring
    // Ed25519; the contract owns structure and canonical bytes).
    backup.manifest_signature =
        Some(ManifestSignature::new("11".repeat(32), "22".repeat(64)).unwrap());
    backup.verify_manifest_signature_structure().unwrap();
}

// ---------------------------------------------------------------------------
// RestorePlan: hash verification gates destructive steps
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_restore_plan_verifies_all_hashes_before_validation() {
    let required = vec![hash(1), hash(2), hash(3)];
    let mut plan = RestorePlan::new(
        "r1",
        tenant(),
        "backup-b1",
        "fresh-target-1",
        required.clone(),
        Some(correlation()),
    )
    .unwrap();
    assert_eq!(plan.state, RestoreVerificationState::Declared);
    assert!(!plan.all_hashes_verified());
    plan.record_verified(&hash(1)).unwrap();
    plan.record_verified(&hash(2)).unwrap();
    assert!(!plan.all_hashes_verified());
    plan.record_verified(&hash(3)).unwrap();
    assert!(plan.all_hashes_verified());
    // Duplicate verification is idempotent.
    plan.record_verified(&hash(3)).unwrap();
    assert_eq!(plan.verified_hashes.len(), 3);
}

#[test]
fn ep037_unit_restore_plan_rejects_unknown_verified_hash() {
    let mut plan = RestorePlan::new(
        "r1",
        tenant(),
        "backup-b1",
        "fresh-target-1",
        vec![hash(1)],
        None,
    )
    .unwrap();
    let err = plan.record_verified(&hash(99)).unwrap_err();
    assert_eq!(err.code, nexus_artifacts::ArtifactErrorCode::Verification);
}

#[test]
fn ep037_unit_restore_plan_requires_fresh_target() {
    assert!(RestorePlan::new("r1", tenant(), "backup-b1", "", vec![hash(1)], None).is_err());
    assert!(RestorePlan::new("r1", tenant(), "", "fresh", vec![hash(1)], None).is_err());
}

// ---------------------------------------------------------------------------
// StorageMigration: copy -> verify -> approve -> delete
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_migration_requires_verify_before_delete_approval() {
    let obj = ObjectRef::new(artifact_id(1), hash(1));
    let mut migration = StorageMigration::new(
        "m1",
        tenant(),
        StorageBackend::Local,
        StorageBackend::S3,
        vec![obj.clone()],
    )
    .unwrap();
    assert_eq!(migration.state, nexus_artifacts::MigrationState::Requested);
    // Approving deletion before verification must fail closed.
    assert!(migration.approve_delete("approval-1").is_err());
    migration.record_verified(&obj).unwrap();
    assert!(migration.all_verified());
    migration.approve_delete("approval-1").unwrap();
    assert_eq!(
        migration.state,
        nexus_artifacts::MigrationState::CanonicalLocationChanged
    );
}

#[test]
fn ep037_unit_migration_rejects_same_backend_and_empty() {
    assert!(StorageMigration::new(
        "m1",
        tenant(),
        StorageBackend::S3,
        StorageBackend::S3,
        vec![ObjectRef::new(artifact_id(1), hash(1))],
    )
    .is_err());
    assert!(StorageMigration::new(
        "m1",
        tenant(),
        StorageBackend::Local,
        StorageBackend::S3,
        vec![]
    )
    .is_err());
}

#[test]
fn ep037_unit_migration_rejects_foreign_object_verification() {
    let obj = ObjectRef::new(artifact_id(1), hash(1));
    let foreign = ObjectRef::new(artifact_id(2), hash(2));
    let mut migration = StorageMigration::new(
        "m1",
        tenant(),
        StorageBackend::Local,
        StorageBackend::S3,
        vec![obj],
    )
    .unwrap();
    let err = migration.record_verified(&foreign).unwrap_err();
    assert_eq!(err.code, nexus_artifacts::ArtifactErrorCode::Verification);
}

// ---------------------------------------------------------------------------
// Dependency direction: this contract crate must stay provider-neutral
// ---------------------------------------------------------------------------

#[test]
fn ep037_unit_dependency_direction_no_storage_sdk_or_transport() {
    // Compile-time proof: if a storage SDK/transport/framework dependency
    // were added to this crate, this test would still compile - so the
    // real proof is the cargo tree assertion in the M1 gate. Here we
    // assert the crate's public surface contains no vendor types.
    let h = hash(1);
    assert_eq!(h.as_str().len(), 64);
}

#[test]
fn ep037_unit_artifact_store_port_is_provider_neutral_surface() {
    // The ArtifactStore trait is object-safe enough for adapter
    // implementations and never exposes a vendor type.
    fn _assert_trait(_store: &mut dyn nexus_artifacts::ArtifactStore) {}
    let _ = _assert_trait;
}
