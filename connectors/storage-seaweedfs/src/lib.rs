//! EP-037 M4 SeaweedFS ArtifactStore adapter (SPEC-024).
//!
//! REAL provider adapter over the SeaweedFS S3 gateway (documented
//! surface: `weed s3` on :8333, AWS SigV4, path-style HTTP/1.1; upstream
//! `weed/command/s3.go`, `weed/s3api/s3api_server.go`). The adapter
//! implements the ONE provider-neutral `nexus-artifacts` ArtifactStore
//! contract - no SeaweedFS-specific artifact model.
//!
//! Truthfulness is structural:
//! - PROVIDER PATH != ARTIFACT IDENTITY: artifacts are stored under
//!   `objects/{content_hash}` and every read re-hashes the returned
//!   bytes against the canonical ArtifactHash (never trusts provider
//!   keys, ETags, HTTP success, or reported length);
//! - encryption-before-egress: a sensitive-class artifact without
//!   encryption metadata is rejected BEFORE any byte crosses the
//!   network (zero provider mutation on policy failure);
//! - delete is a ladder (DELETE_REQUESTED != DELETE_ACCEPTED !=
//!   RESOURCE_ABSENT_VERIFIED) ending in an independent readback;
//! - backup/restore/migration are hash-gated: a manifest with a
//!   missing/corrupted member never validates; restore writes are
//!   re-verified after write; migration deletes the source only after
//!   destination verification and approval.
//!
//! Error classification is distinct: connect refused -> Unavailable,
//! read timeout -> Timeout, malformed status -> ExternalProvider,
//! 404 -> NotFound, hash mismatch -> Verification. Failures are never
//! flattened into one generic error, and a provider is never assumed
//! healthy from configuration alone.

use std::time::{Duration, Instant};

use nexus_artifacts::{
    ArtifactError, ArtifactHash, ArtifactMetadata, ArtifactResult, ArtifactStore, BackupSet,
    BackupState, RestorePlan, RestoreVerificationState, RetentionClass, StorageMigration,
};
use nexus_domain::{ArtifactId, CorrelationId, TenantId};
use sha2::{Digest, Sha256};

use crate::observability::{record, started, ObservationSink};
use crate::transport::{S3Client, S3Error};

pub mod observability;
pub mod transport;

/// Configuration for the SeaweedFS S3-gateway adapter. The access key
/// and secret key are held in memory only and are never logged or
/// placed into error messages or observations.
#[derive(Debug, Clone)]
pub struct SeaweedFsConfig {
    /// S3 gateway endpoint as host:port (e.g. 127.0.0.1:18333).
    pub endpoint: String,
    /// SigV4 access key (runtime-provided).
    pub access_key: String,
    /// SigV4 secret key (runtime-provided).
    pub secret_key: String,
    /// Bucket name prefix (tenant buckets are `{prefix}{tenant}`).
    pub bucket_prefix: String,
    /// Bounded connect timeout.
    pub connect_timeout: Duration,
    /// Bounded read timeout (silent peers -> Timeout, never hang).
    pub read_timeout: Duration,
}

impl SeaweedFsConfig {
    pub fn new(
        endpoint: impl Into<String>,
        access_key: impl Into<String>,
        secret_key: impl Into<String>,
        bucket_prefix: impl Into<String>,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            access_key: access_key.into(),
            secret_key: secret_key.into(),
            bucket_prefix: bucket_prefix.into(),
            connect_timeout: Duration::from_secs(3),
            read_timeout: Duration::from_secs(5),
        }
    }

    /// Bounded timeouts (configurable for failure-injection tests).
    pub fn with_timeouts(mut self, connect: Duration, read: Duration) -> Self {
        self.connect_timeout = connect;
        self.read_timeout = read;
        self
    }
}

/// SeaweedFS ArtifactStore adapter over the real S3 gateway.
#[derive(Clone)]
pub struct SeaweedFsArtifactStore {
    client: S3Client,
    bucket_prefix: String,
    sink: std::sync::Arc<std::sync::Mutex<Box<dyn ObservationSink + Send>>>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let out = hasher.finalize();
    out.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_of(bytes: &[u8]) -> ArtifactResult<ArtifactHash> {
    ArtifactHash::new(sha256_hex(bytes))
}

/// Verify a backup manifest's signature (SPEC-024 requirement 6 /
/// AUD-052). Structural checks live in the contract crate; the
/// CRYPTOGRAPHIC verification (real ring Ed25519 over the canonical
/// manifest bytes, excluding the signature field) is owned here in the
/// adapter. Missing, malformed, wrong-signer, or tampered signatures
/// fail closed before any hash in the manifest is trusted.
fn verify_backup_signature(backup: &BackupSet) -> ArtifactResult<()> {
    use ring::signature::{UnparsedPublicKey, ED25519};
    backup.verify_manifest_signature_structure()?;
    let sig = backup
        .manifest_signature
        .as_ref()
        .expect("structure check passed");
    let public_key = nexus_artifacts::hex_decode(&sig.public_key_hex).ok_or_else(|| {
        ArtifactError::verification("manifest signature public key is not valid hex")
    })?;
    let signature = nexus_artifacts::hex_decode(&sig.signature_hex)
        .ok_or_else(|| ArtifactError::verification("manifest signature value is not valid hex"))?;
    let message = backup.canonical_manifest_bytes()?;
    let key = UnparsedPublicKey::new(&ED25519, &public_key);
    key.verify(&message, &signature)
        .map_err(|_| ArtifactError::verification("backup manifest signature verification failed"))
}

fn map_s3_error(e: S3Error) -> ArtifactError {
    match e {
        S3Error::Connect(m) => ArtifactError::unavailable(format!("provider unreachable: {m}")),
        S3Error::Timeout => ArtifactError::timeout(
            "provider accepted connection but did not respond (never assumed successful)",
        ),
        S3Error::Malformed(m) => ArtifactError::external(format!(
            "provider returned malformed response: {m} (never guessed into success)"
        )),
        S3Error::Status { code: 404, .. } => ArtifactError::not_found("provider object not found"),
        S3Error::Status { code: 403, .. } => ArtifactError::authorization("provider denied access"),
        S3Error::Status { code: 400, .. } => ArtifactError::validation("provider rejected request"),
        S3Error::Status { code: 409, .. } => ArtifactError::conflict("provider state conflict"),
        S3Error::Status { code: 503, .. } => {
            ArtifactError::unavailable("provider unavailable (503)")
        }
        S3Error::Status { code, body } => ArtifactError::external(format!(
            "provider returned status {code}: {}",
            sanitize_body(&body)
        )),
    }
}

/// Truncate and sanitize a provider response body for error messages:
/// never leak credential-shaped or secret-shaped content, never echo
/// payload bytes (only the first 200 chars of a status body, and only
/// after stripping anything that resembles a secret literal).
fn sanitize_body(body: &str) -> String {
    let truncated: String = body.chars().take(200).collect();
    let mut out = String::new();
    for line in truncated.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("accesskey")
            || lower.contains("secretkey")
            || lower.contains("signature")
            || lower.contains("authorization")
        {
            out.push_str("[redacted provider detail]");
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out.trim().to_string()
}

impl SeaweedFsArtifactStore {
    /// Open a real SeaweedFS S3-gateway adapter.
    pub fn open(config: SeaweedFsConfig) -> ArtifactResult<Self> {
        if config.endpoint.trim().is_empty() {
            return Err(ArtifactError::validation(
                "seaweedfs endpoint must not be empty",
            ));
        }
        if config.access_key.trim().is_empty() || config.secret_key.trim().is_empty() {
            return Err(ArtifactError::validation(
                "seaweedfs credentials must not be empty",
            ));
        }
        if config.bucket_prefix.trim().is_empty()
            || !config
                .bucket_prefix
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(ArtifactError::validation(
                "seaweedfs bucket prefix must be lowercase alphanumeric with hyphens",
            ));
        }
        let client = S3Client::connect(
            &config.endpoint,
            &config.access_key,
            &config.secret_key,
            config.connect_timeout,
            config.read_timeout,
        );
        Ok(Self {
            client,
            bucket_prefix: config.bucket_prefix,
            sink: std::sync::Arc::new(std::sync::Mutex::new(Box::new(
                crate::observability::NullSink,
            ))),
        })
    }

    /// Attach an observation sink (tests use VecSink to assert bounded
    /// observability and redaction).
    pub fn with_sink(mut self, sink: Box<dyn ObservationSink + Send>) -> Self {
        self.sink = std::sync::Arc::new(std::sync::Mutex::new(sink));
        self
    }

    fn bucket_for(&self, tenant: &TenantId) -> String {
        format!("{}{}", self.bucket_prefix, tenant.as_str())
    }

    fn object_key(hash: &ArtifactHash) -> String {
        format!("objects/{}", hash.as_str())
    }

    fn meta_key(id: &ArtifactId) -> String {
        format!("meta/{id}.json")
    }

    fn backup_key(backup_id: &str) -> String {
        format!("backups/{backup_id}.json")
    }

    fn ensure_bucket(&self, bucket: &str) -> ArtifactResult<()> {
        self.client.create_bucket(bucket).map_err(map_s3_error)
    }

    fn read_metadata(&self, bucket: &str, id: &ArtifactId) -> ArtifactResult<ArtifactMetadata> {
        let raw = self
            .client
            .get_object(bucket, &Self::meta_key(id))
            .map_err(map_s3_error)?;
        serde_json::from_slice(&raw)
            .map_err(|e| ArtifactError::internal(format!("corrupt metadata sidecar: {e}")))
    }

    fn write_metadata(&self, bucket: &str, metadata: &ArtifactMetadata) -> ArtifactResult<()> {
        let raw = serde_json::to_vec(metadata)
            .map_err(|e| ArtifactError::internal(format!("cannot serialize metadata: {e}")))?;
        self.client
            .put_object(bucket, &Self::meta_key(&metadata.artifact_id), &raw)
            .map_err(map_s3_error)?;
        Ok(())
    }

    /// Read object bytes and verify them against the content address.
    /// Provider path/key/ETag/HTTP success are never integrity proof;
    /// only the re-computed SHA-256 is.
    fn read_object(&self, bucket: &str, hash: &ArtifactHash) -> ArtifactResult<Vec<u8>> {
        let bytes = self
            .client
            .get_object(bucket, &Self::object_key(hash))
            .map_err(map_s3_error)?;
        let actual = hash_of(&bytes)?;
        if &actual != hash {
            return Err(ArtifactError::verification(format!(
                "object {} failed hash verification on read",
                hash.as_str()
            )));
        }
        Ok(bytes)
    }

    /// Verify an object's bytes on the provider without returning them.
    fn verify_object(&self, bucket: &str, hash: &ArtifactHash) -> ArtifactResult<()> {
        self.read_object(bucket, hash).map(|_| ())
    }

    /// True when another artifact's metadata references the same hash.
    /// Delete must never remove an object still referenced by another
    /// artifact (exact-target mapping is authoritative).
    fn other_refs_exist(
        &self,
        bucket: &str,
        id: &ArtifactId,
        hash: &ArtifactHash,
    ) -> ArtifactResult<bool> {
        let keys = self
            .client
            .list_keys(bucket, "meta/")
            .map_err(map_s3_error)?;
        for key in keys {
            let Some(rest) = key.strip_prefix("meta/") else {
                continue;
            };
            let Some(meta_id) = rest.strip_suffix(".json") else {
                continue;
            };
            if meta_id == id.as_str() {
                continue;
            }
            let Ok(other_id) = meta_id.parse::<ArtifactId>() else {
                continue;
            };
            if let Ok(meta) = self.read_metadata(bucket, &other_id) {
                if &meta.content_hash == hash {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Operations diagnostic probe: creates a canary object, reads it
    /// back, verifies the digest, and deletes it. Returns Ok only when
    /// the full read/write probe verified. Never prints credentials.
    pub fn diag_probe(&self) -> ArtifactResult<()> {
        let tenant: TenantId = "01970000-0000-7000-8000-0000000000ff".parse().unwrap();
        let bucket = self.bucket_for(&tenant);
        self.ensure_bucket(&bucket)?;
        let canary = format!(
            "ep037-diag-canary-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let key = format!("diag/{canary}");
        let bytes = b"ep037 seaweedfs diag probe payload".to_vec();
        let digest = sha256_hex(&bytes);
        self.client
            .put_object(&bucket, &key, &bytes)
            .map_err(map_s3_error)?;
        let read_back = self
            .client
            .get_object(&bucket, &key)
            .map_err(map_s3_error)?;
        if sha256_hex(&read_back) != digest {
            return Err(ArtifactError::verification(
                "diag probe: readback digest mismatch",
            ));
        }
        self.client
            .delete_object(&bucket, &key)
            .map_err(map_s3_error)?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)] // explicit observation fields; same pattern as nexus-hydra/nexus-compute
    fn observe(
        &self,
        started: Instant,
        operation: &str,
        artifact_hash: Option<String>,
        size_bytes: Option<u64>,
        correlation: Option<String>,
        encryption_applied: bool,
        fingerprint: Option<String>,
        result: Result<(), &str>,
        integrity_verified: bool,
    ) {
        let mut sink = self.sink.lock().unwrap();
        record(
            sink.as_mut(),
            operation,
            artifact_hash,
            size_bytes,
            correlation,
            encryption_applied,
            fingerprint,
            started,
            result,
            integrity_verified,
        );
    }
}

impl ArtifactStore for SeaweedFsArtifactStore {
    fn put(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        expected_hash: &ArtifactHash,
        bytes: &[u8],
        metadata: &ArtifactMetadata,
        correlation: &CorrelationId,
    ) -> ArtifactResult<ArtifactMetadata> {
        let started = started();
        // ENCRYPTION-BEFORE-EGRESS: SeaweedFS leaves the node. A
        // sensitive-class artifact must carry encryption metadata AND the
        // bytes about to be persisted must not be the plaintext (AUD-051)
        // - verified BEFORE any byte crosses the network (zero provider
        // mutation on policy failure). The adapter never holds the key;
        // the encrypting caller recorded the plaintext's SHA-256 in the
        // metadata, and we verify the stored bytes hash differs from it.
        metadata.verify_encryption_before_egress(&sha256_hex(bytes))?;
        if &metadata.content_hash != expected_hash {
            return Err(ArtifactError::validation(
                "metadata content hash does not match expected hash",
            ));
        }
        if &metadata.artifact_id != artifact_id {
            return Err(ArtifactError::validation(
                "metadata artifact id does not match request id",
            ));
        }
        // Verify the caller-supplied hash against the actual bytes
        // BEFORE any provider write (never trust a caller hash blindly).
        let actual = hash_of(bytes)?;
        if &actual != expected_hash {
            return Err(ArtifactError::verification(
                "caller-supplied hash does not match artifact bytes",
            ));
        }
        let bucket = self.bucket_for(tenant);
        let result = (|| -> ArtifactResult<()> {
            self.ensure_bucket(&bucket)?;
            self.client
                .put_object(&bucket, &Self::object_key(expected_hash), bytes)
                .map_err(map_s3_error)?;
            // Independent readback: the provider write is not accepted
            // as verified until the returned bytes hash matches.
            let read_back = self
                .client
                .get_object(&bucket, &Self::object_key(expected_hash))
                .map_err(map_s3_error)?;
            if sha256_hex(&read_back) != expected_hash.as_str() {
                return Err(ArtifactError::verification(
                    "provider readback failed hash verification after put",
                ));
            }
            self.write_metadata(&bucket, metadata)?;
            Ok(())
        })();
        let encryption_applied = metadata.encryption.is_some();
        let integrity_verified = result.is_ok();
        self.observe(
            started,
            "put",
            Some(expected_hash.as_str().to_string()),
            Some(bytes.len() as u64),
            Some(correlation.as_str().to_string()),
            encryption_applied,
            None,
            result.as_ref().map(|_| ()).map_err(|e| err_class(e)),
            integrity_verified,
        );
        result?;
        Ok(metadata.clone())
    }

    fn get(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        correlation: &CorrelationId,
    ) -> ArtifactResult<(ArtifactMetadata, Vec<u8>)> {
        let started = started();
        let bucket = self.bucket_for(tenant);
        let result = (|| -> ArtifactResult<(ArtifactMetadata, Vec<u8>)> {
            let metadata = self.read_metadata(&bucket, artifact_id)?;
            let bytes = self.read_object(&bucket, &metadata.content_hash)?;
            Ok((metadata, bytes))
        })();
        let integrity_verified = result.is_ok();
        let hash = result
            .as_ref()
            .ok()
            .map(|(m, _)| m.content_hash.as_str().to_string());
        let size = result.as_ref().ok().map(|(_, b)| b.len() as u64);
        let enc = result
            .as_ref()
            .ok()
            .map(|(m, _)| m.encryption.is_some())
            .unwrap_or(false);
        self.observe(
            started,
            "get",
            hash,
            size,
            Some(correlation.as_str().to_string()),
            enc,
            None,
            result.as_ref().map(|_| ()).map_err(|e| err_class(e)),
            integrity_verified,
        );
        result
    }

    fn verify(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        correlation: &CorrelationId,
    ) -> ArtifactResult<ArtifactHash> {
        let started = started();
        let bucket = self.bucket_for(tenant);
        let result = (|| -> ArtifactResult<ArtifactHash> {
            let metadata = self.read_metadata(&bucket, artifact_id)?;
            self.verify_object(&bucket, &metadata.content_hash)?;
            Ok(metadata.content_hash)
        })();
        let integrity_verified = result.is_ok();
        let hash = result.as_ref().ok().map(|h| h.as_str().to_string());
        self.observe(
            started,
            "verify",
            hash,
            None,
            Some(correlation.as_str().to_string()),
            false,
            None,
            result.as_ref().map(|_| ()).map_err(|e| err_class(e)),
            integrity_verified,
        );
        result
    }

    fn delete(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        correlation: &CorrelationId,
    ) -> ArtifactResult<()> {
        let started = started();
        let bucket = self.bucket_for(tenant);
        let result = (|| -> ArtifactResult<()> {
            let metadata = self.read_metadata(&bucket, artifact_id)?;
            let hash = metadata.content_hash.clone();
            // DELETE_REQUESTED: verify the object exists and is intact
            // before removal (never delete a corrupt/absent object
            // silently).
            self.verify_object(&bucket, &hash)?;
            let shared = self.other_refs_exist(&bucket, artifact_id, &hash)?;
            if !shared {
                // DELETE_ACCEPTED: provider acknowledged the delete.
                self.client
                    .delete_object(&bucket, &Self::object_key(&hash))
                    .map_err(map_s3_error)?;
                // RESOURCE_ABSENT_VERIFIED: independent readback must
                // prove absence. A delete response alone never certifies
                // absence.
                match self.client.get_object(&bucket, &Self::object_key(&hash)) {
                    Err(S3Error::Status { code: 404, .. }) => {}
                    Err(e) => return Err(map_s3_error(e)),
                    Ok(_) => {
                        return Err(ArtifactError::verification(
                            "delete failed: resource not absent after delete",
                        ));
                    }
                }
            }
            self.client
                .delete_object(&bucket, &Self::meta_key(artifact_id))
                .map_err(map_s3_error)?;
            Ok(())
        })();
        let integrity_verified = result.is_ok();
        self.observe(
            started,
            "delete",
            None,
            None,
            Some(correlation.as_str().to_string()),
            false,
            None,
            result.as_ref().map(|_| ()).map_err(|e| err_class(e)),
            integrity_verified,
        );
        result
    }

    fn create_backup(
        &mut self,
        tenant: &TenantId,
        backup: &BackupSet,
        correlation: &CorrelationId,
    ) -> ArtifactResult<BackupSet> {
        let started = started();
        let bucket = self.bucket_for(tenant);
        let result = (|| -> ArtifactResult<BackupSet> {
            let manifest_key = Self::backup_key(&backup.backup_id);
            match self.client.get_object(&bucket, &manifest_key) {
                Ok(_) => {
                    return Err(ArtifactError::conflict(format!(
                        "backup {} already exists",
                        backup.backup_id
                    )));
                }
                Err(S3Error::Status { code: 404, .. }) => {}
                Err(e) => return Err(map_s3_error(e)),
            }
            // Backup is hash-gated: every manifest hash must verify on
            // the provider before the manifest is written. SPEC-024
            // requirement 6: the manifest must be signed and the
            // signature verified before it is written (fail closed).
            verify_backup_signature(backup)?;
            for h in &backup.manifest_hashes {
                self.verify_object(&bucket, h)?;
            }
            let raw = serde_json::to_vec(backup)
                .map_err(|e| ArtifactError::internal(format!("cannot serialize backup: {e}")))?;
            self.client
                .put_object(&bucket, &manifest_key, &raw)
                .map_err(map_s3_error)?;
            let mut created = backup.clone();
            created.state = BackupState::Created;
            Ok(created)
        })();
        let integrity_verified = result.is_ok();
        self.observe(
            started,
            "create_backup",
            None,
            None,
            Some(correlation.as_str().to_string()),
            false,
            Some(backup.backup_id.clone()),
            result.as_ref().map(|_| ()).map_err(|e| err_class(e)),
            integrity_verified,
        );
        result
    }

    fn restore(
        &mut self,
        tenant: &TenantId,
        plan: &RestorePlan,
        correlation: &CorrelationId,
    ) -> ArtifactResult<RestorePlan> {
        let started = started();
        let bucket = self.bucket_for(tenant);
        let result = (|| -> ArtifactResult<RestorePlan> {
            let raw = self
                .client
                .get_object(&bucket, &Self::backup_key(&plan.source_backup))
                .map_err(map_s3_error)?;
            let backup: BackupSet = serde_json::from_slice(&raw)
                .map_err(|e| ArtifactError::internal(format!("corrupt backup manifest: {e}")))?;
            // SPEC-024 requirement 6: restore authenticates signer +
            // signature before trusting manifest content (fail closed on
            // missing/tampered signature).
            verify_backup_signature(&backup)?;
            for required in &plan.required_hashes {
                if !backup.manifest_hashes.contains(required) {
                    return Err(ArtifactError::verification(format!(
                        "restore plan requires hash {} not present in backup manifest",
                        required.as_str()
                    )));
                }
            }
            let mut executed = plan.clone();
            for required in &plan.required_hashes {
                // RESTORE WRITTEN != RESTORE HASH VERIFIED: re-verify the
                // bytes on the fresh target after restore.
                self.verify_object(&bucket, required)?;
                executed.record_verified(required)?;
            }
            if executed.all_hashes_verified() {
                executed.state = RestoreVerificationState::Validated;
            }
            Ok(executed)
        })();
        let integrity_verified = result.is_ok();
        self.observe(
            started,
            "restore",
            None,
            None,
            Some(correlation.as_str().to_string()),
            false,
            Some(plan.restore_id.clone()),
            result.as_ref().map(|_| ()).map_err(|e| err_class(e)),
            integrity_verified,
        );
        result
    }

    fn migrate(
        &mut self,
        tenant: &TenantId,
        migration: &StorageMigration,
        correlation: &CorrelationId,
    ) -> ArtifactResult<StorageMigration> {
        let started = started();
        let bucket = self.bucket_for(tenant);
        let result = (|| -> ArtifactResult<StorageMigration> {
            // MIGRATION COPIED != MIGRATION VERIFIED: this adapter's
            // bucket is the TARGET; every object must verify on the
            // target before the migration can advance. The harness
            // drives the copy phase through a source store; the source
            // is never deleted before destination verification.
            let mut migrated = migration.clone();
            for obj in &migration.object_refs {
                self.verify_object(&bucket, &obj.content_hash)?;
                migrated.record_verified(obj)?;
            }
            if migrated.all_verified() {
                migrated.mark_verified()?;
            }
            Ok(migrated)
        })();
        let integrity_verified = result.is_ok();
        self.observe(
            started,
            "migrate",
            None,
            None,
            Some(correlation.as_str().to_string()),
            false,
            Some(migration.migration_id.clone()),
            result.as_ref().map(|_| ()).map_err(|e| err_class(e)),
            integrity_verified,
        );
        result
    }

    fn list(
        &mut self,
        tenant: &TenantId,
        cursor: Option<&str>,
        limit: usize,
    ) -> ArtifactResult<(Vec<ArtifactMetadata>, Option<String>)> {
        let started = started();
        let bucket = self.bucket_for(tenant);
        let result = (|| -> ArtifactResult<(Vec<ArtifactMetadata>, Option<String>)> {
            let keys = self
                .client
                .list_keys(&bucket, "meta/")
                .map_err(map_s3_error)?;
            let mut entries: Vec<(String, ArtifactMetadata)> = Vec::new();
            for key in keys {
                let Some(rest) = key.strip_prefix("meta/") else {
                    continue;
                };
                let Some(id) = rest.strip_suffix(".json") else {
                    continue;
                };
                let Ok(id) = id.parse::<ArtifactId>() else {
                    continue;
                };
                if let Ok(meta) = self.read_metadata(&bucket, &id) {
                    entries.push((id.as_str().to_string(), meta));
                }
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            let start = match cursor {
                Some(c) => entries
                    .iter()
                    .position(|(id, _)| id == c)
                    .map(|i| i + 1)
                    .ok_or_else(|| {
                        ArtifactError::validation(format!(
                            "list continuation token {c} does not match any artifact"
                        ))
                    })?,
                None => 0,
            };
            let page: Vec<ArtifactMetadata> = entries
                .iter()
                .skip(start)
                .take(limit)
                .map(|(_, m)| m.clone())
                .collect();
            let next = if start + page.len() < entries.len() {
                entries
                    .get(start + page.len() - 1)
                    .map(|(id, _)| id.clone())
            } else {
                None
            };
            Ok((page, next))
        })();
        let integrity_verified = result.is_ok();
        self.observe(
            started,
            "list",
            None,
            None,
            None,
            false,
            None,
            result.as_ref().map(|_| ()).map_err(|e| err_class(e)),
            integrity_verified,
        );
        result
    }

    fn set_retention(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        retention: RetentionClass,
        correlation: &CorrelationId,
    ) -> ArtifactResult<()> {
        let started = started();
        let bucket = self.bucket_for(tenant);
        let result = (|| -> ArtifactResult<()> {
            let mut metadata = self.read_metadata(&bucket, artifact_id)?;
            metadata.retention = retention;
            self.write_metadata(&bucket, &metadata)?;
            Ok(())
        })();
        let integrity_verified = result.is_ok();
        self.observe(
            started,
            "set_retention",
            None,
            None,
            Some(correlation.as_str().to_string()),
            false,
            None,
            result.as_ref().map(|_| ()).map_err(|e| err_class(e)),
            integrity_verified,
        );
        result
    }
}

/// Canonical error class name for observations (safe: no messages).
fn err_class(e: &ArtifactError) -> &'static str {
    match e.code {
        ArtifactErrorCode::Validation => "Validation",
        ArtifactErrorCode::Authentication => "Authentication",
        ArtifactErrorCode::Authorization => "Authorization",
        ArtifactErrorCode::Policy => "Policy",
        ArtifactErrorCode::Unavailable => "Unavailable",
        ArtifactErrorCode::Timeout => "Timeout",
        ArtifactErrorCode::Conflict => "Conflict",
        ArtifactErrorCode::NotFound => "NotFound",
        ArtifactErrorCode::RateLimit => "RateLimit",
        ArtifactErrorCode::ExternalProvider => "ExternalProvider",
        ArtifactErrorCode::Verification => "Verification",
        ArtifactErrorCode::Compensation => "Compensation",
        ArtifactErrorCode::Vocabulary => "Vocabulary",
        ArtifactErrorCode::Internal => "Internal",
    }
}

/// Convenience re-export so callers can name the error code type without
/// importing the contract crate twice.
pub use nexus_artifacts::ArtifactErrorCode;
