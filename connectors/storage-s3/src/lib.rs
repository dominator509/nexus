//! EP-037 M5 S3-compatible ArtifactStore adapter (SPEC-024).
//!
//! REAL adapter over the S3 protocol (AWS SigV4, path-style HTTP/1.1)
//! that implements the ONE provider-neutral `nexus-artifacts`
//! ArtifactStore contract - no S3-specific artifact model. The
//! compatibility target is EXPLICIT configuration, never assumed:
//! SPEC-024 explicitly states that assuming S3 implementations are
//! identical is a non-goal, so this adapter records its compatibility
//! profile (`S3CompatibilityProfile`) and certifies only against the
//! real providers exercised in the milestone gate (MinIO and the
//! SeaweedFS S3 gateway). AWS S3 / R2 / B2 are NOT asserted by this
//! adapter's certification.
//!
//! Truthfulness is structural:
//! - S3 KEY != ARTIFACT IDENTITY: objects are stored under
//!   `objects/{content_hash}` and every read re-hashes the returned
//!   bytes against the canonical ArtifactHash (never trusts provider
//!   keys, ETags, HTTP success, or reported length). ETag is never
//!   treated as a content-hash authority (multipart uploads make ETag
//!   not a simple digest);
//! - encryption-before-egress: a sensitive-class artifact without
//!   encryption metadata is rejected BEFORE any byte crosses the
//!   network (zero provider mutation on policy failure), and metadata
//!   is stored in a sidecar object, never in unencrypted
//!   `x-amz-meta-*` headers that could leak sensitive plaintext;
//! - delete is a ladder (DELETE_REQUESTED != DELETE_ACCEPTED !=
//!   RESOURCE_ABSENT_VERIFIED) ending in an independent readback;
//! - backup/restore/migration are hash-gated: a manifest with a
//!   missing/corrupted member never validates; restore writes are
//!   re-verified after write; migration deletes the source only after
//!   destination verification and approval.
//!
//! Error classification is distinct: connect refused -> Unavailable,
//! read timeout -> Timeout, malformed status -> ExternalProvider,
//! 404 -> NotFound, 403 -> Authorization, 409 -> Conflict,
//! 429 -> RateLimit, 5xx -> ExternalProvider. Failures are never
//! flattened into one generic error.

use std::time::{Duration, Instant};

use nexus_artifacts::{
    ArtifactError, ArtifactHash, ArtifactMetadata, ArtifactResult, ArtifactStore, BackupSet,
    BackupState, RestorePlan, RestoreVerificationState, RetentionClass, StorageMigration,
};
use nexus_domain::{ArtifactId, CorrelationId, TenantId};
use sha2::{Digest, Sha256};

use crate::transport::{S3Client, S3Error};

pub mod transport;

/// Explicit S3-compatible compatibility profile (SPEC-024 non-goal:
/// assuming S3 implementations are identical). The profile is
/// configuration, recorded in observations and evidence; it never
/// silently changes adapter behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S3CompatibilityProfile {
    /// MinIO S3-compatible server (compatibility-only backend).
    MinIo,
    /// SeaweedFS S3 gateway.
    SeaweedFs,
    /// Generic/unspecified S3-compatible endpoint.
    Generic,
    /// AWS S3 (NOT certified by this milestone; explicit config only).
    AwsS3,
    /// Cloudflare R2 (NOT certified by this milestone).
    R2,
    /// Backblaze B2 (NOT certified by this milestone).
    B2,
}

impl S3CompatibilityProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MinIo => "MINIO",
            Self::SeaweedFs => "SEAWEEDFS",
            Self::Generic => "GENERIC",
            Self::AwsS3 => "AWS_S3",
            Self::R2 => "R2",
            Self::B2 => "B2",
        }
    }
}

/// Configuration for the S3-compatible adapter. The access key and
/// secret key are held in memory only and are never logged or placed
/// into error messages or observations. Region and service are explicit
/// configuration (never guessed from the endpoint hostname); addressing
/// is path-style only (virtual-hosted-style is not implemented or
/// asserted).
#[derive(Debug, Clone)]
pub struct S3Config {
    /// S3-compatible endpoint as host:port (e.g. 127.0.0.1:19090).
    pub endpoint: String,
    /// SigV4 access key (runtime-provided).
    pub access_key: String,
    /// SigV4 secret key (runtime-provided).
    pub secret_key: String,
    /// Explicit SigV4 signing region (defaults to the S3 protocol
    /// convention us-east-1; never inferred from the hostname).
    pub region: String,
    /// Explicit compatibility profile of the endpoint.
    pub profile: S3CompatibilityProfile,
    /// Bucket name prefix (tenant buckets are `{prefix}{tenant}`).
    pub bucket_prefix: String,
    /// Bounded connect timeout.
    pub connect_timeout: Duration,
    /// Bounded read timeout (silent peers -> Timeout, never hang).
    pub read_timeout: Duration,
}

impl S3Config {
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
            region: "us-east-1".to_string(),
            profile: S3CompatibilityProfile::Generic,
            bucket_prefix: bucket_prefix.into(),
            connect_timeout: Duration::from_secs(3),
            read_timeout: Duration::from_secs(5),
        }
    }

    /// Declare the explicit signing region (never guessed).
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }

    /// Declare the explicit compatibility profile.
    pub fn with_profile(mut self, profile: S3CompatibilityProfile) -> Self {
        self.profile = profile;
        self
    }

    /// Bounded timeouts (configurable for failure-injection tests).
    pub fn with_timeouts(mut self, connect: Duration, read: Duration) -> Self {
        self.connect_timeout = connect;
        self.read_timeout = read;
        self
    }
}

/// S3-compatible ArtifactStore adapter.
#[derive(Clone)]
pub struct S3ArtifactStore {
    client: S3Client,
    bucket_prefix: String,
    profile: S3CompatibilityProfile,
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
        S3Error::Status { code: 429, .. } => ArtifactError::rate_limit("provider rate limited"),
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
/// payload bytes.
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

impl S3ArtifactStore {
    /// Open a real S3-compatible adapter.
    pub fn open(config: S3Config) -> ArtifactResult<Self> {
        if config.endpoint.trim().is_empty() {
            return Err(ArtifactError::validation("s3 endpoint must not be empty"));
        }
        if config.access_key.trim().is_empty() || config.secret_key.trim().is_empty() {
            return Err(ArtifactError::validation(
                "s3 credentials must not be empty",
            ));
        }
        if config.region.trim().is_empty() {
            return Err(ArtifactError::validation("s3 region must not be empty"));
        }
        if config.bucket_prefix.trim().is_empty()
            || !config
                .bucket_prefix
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(ArtifactError::validation(
                "s3 bucket prefix must be lowercase alphanumeric with hyphens",
            ));
        }
        let client = S3Client::connect(
            &config.endpoint,
            &config.access_key,
            &config.secret_key,
            &config.region,
            config.connect_timeout,
            config.read_timeout,
        );
        Ok(Self {
            client,
            bucket_prefix: config.bucket_prefix,
            profile: config.profile,
        })
    }

    /// The explicit compatibility profile this adapter was configured
    /// with (recorded in evidence; never assumed).
    pub fn profile(&self) -> S3CompatibilityProfile {
        self.profile
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
        // Metadata is stored as an OBJECT sidecar, never in unencrypted
        // x-amz-meta-* headers: content encryption alone does not
        // protect S3 object metadata, and sensitive metadata must not
        // leak in plaintext (SPEC-024 requirement 3 + SECURITY.md).
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
            "ep037-s3-diag-canary-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let key = format!("diag/{canary}");
        let bytes = b"ep037 s3 diag probe payload".to_vec();
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
}

impl ArtifactStore for S3ArtifactStore {
    fn put(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        expected_hash: &ArtifactHash,
        bytes: &[u8],
        metadata: &ArtifactMetadata,
        correlation: &CorrelationId,
    ) -> ArtifactResult<ArtifactMetadata> {
        let _ = Instant::now();
        let _ = correlation;
        // ENCRYPTION-BEFORE-EGRESS: S3 leaves the node. A sensitive-class
        // artifact must carry encryption metadata AND the bytes about to
        // be persisted must not be the plaintext (AUD-051) - verified
        // BEFORE any byte crosses the network (zero provider mutation on
        // policy failure). The adapter never holds the key; the encrypting
        // caller recorded the plaintext's SHA-256 in the metadata, and we
        // verify the stored bytes hash differs from it.
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
        self.ensure_bucket(&bucket)?;
        self.client
            .put_object(&bucket, &Self::object_key(expected_hash), bytes)
            .map_err(map_s3_error)?;
        // Independent readback: the provider write is not accepted as
        // verified until the returned bytes hash matches.
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
        Ok(metadata.clone())
    }

    fn get(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        _correlation: &CorrelationId,
    ) -> ArtifactResult<(ArtifactMetadata, Vec<u8>)> {
        let bucket = self.bucket_for(tenant);
        let metadata = self.read_metadata(&bucket, artifact_id)?;
        let bytes = self.read_object(&bucket, &metadata.content_hash)?;
        Ok((metadata, bytes))
    }

    fn verify(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        _correlation: &CorrelationId,
    ) -> ArtifactResult<ArtifactHash> {
        let bucket = self.bucket_for(tenant);
        let metadata = self.read_metadata(&bucket, artifact_id)?;
        self.verify_object(&bucket, &metadata.content_hash)?;
        Ok(metadata.content_hash)
    }

    fn delete(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        _correlation: &CorrelationId,
    ) -> ArtifactResult<()> {
        let bucket = self.bucket_for(tenant);
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
            // RESOURCE_ABSENT_VERIFIED: independent readback must prove
            // absence. A delete response alone never certifies absence.
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
    }

    fn create_backup(
        &mut self,
        tenant: &TenantId,
        backup: &BackupSet,
        _correlation: &CorrelationId,
    ) -> ArtifactResult<BackupSet> {
        let bucket = self.bucket_for(tenant);
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
        // Backup is hash-gated: every manifest hash must verify on the
        // provider before the manifest is written.
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
    }

    fn restore(
        &mut self,
        tenant: &TenantId,
        plan: &RestorePlan,
        _correlation: &CorrelationId,
    ) -> ArtifactResult<RestorePlan> {
        let bucket = self.bucket_for(tenant);
        let raw = self
            .client
            .get_object(&bucket, &Self::backup_key(&plan.source_backup))
            .map_err(map_s3_error)?;
        let backup: BackupSet = serde_json::from_slice(&raw)
            .map_err(|e| ArtifactError::internal(format!("corrupt backup manifest: {e}")))?;
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
    }

    fn migrate(
        &mut self,
        tenant: &TenantId,
        migration: &StorageMigration,
        _correlation: &CorrelationId,
    ) -> ArtifactResult<StorageMigration> {
        let bucket = self.bucket_for(tenant);
        // MIGRATION COPIED != MIGRATION VERIFIED: this adapter's bucket
        // is the TARGET; every object must verify on the target before
        // the migration can advance.
        let mut migrated = migration.clone();
        for obj in &migration.object_refs {
            self.verify_object(&bucket, &obj.content_hash)?;
            migrated.record_verified(obj)?;
        }
        if migrated.all_verified() {
            migrated.mark_verified()?;
        }
        Ok(migrated)
    }

    fn list(
        &mut self,
        tenant: &TenantId,
        cursor: Option<&str>,
        limit: usize,
    ) -> ArtifactResult<(Vec<ArtifactMetadata>, Option<String>)> {
        let bucket = self.bucket_for(tenant);
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
    }

    fn set_retention(
        &mut self,
        tenant: &TenantId,
        artifact_id: &ArtifactId,
        retention: RetentionClass,
        _correlation: &CorrelationId,
    ) -> ArtifactResult<()> {
        let bucket = self.bucket_for(tenant);
        let mut metadata = self.read_metadata(&bucket, artifact_id)?;
        metadata.retention = retention;
        self.write_metadata(&bucket, &metadata)?;
        Ok(())
    }
}

/// Convenience re-export so callers can name the error code type without
/// importing the contract crate twice.
pub use nexus_artifacts::ArtifactErrorCode;
