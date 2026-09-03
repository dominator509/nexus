//! EP-037 M4 forced-failure, abuse-case, and observability suite over a
//! REAL SeaweedFS S3-gateway container (digest-pinned by the gate).
//!
//! Every test exercises the REAL provider and the production adapter:
//! - positive path: canonical artifact -> content hash -> encryption
//!   where required -> storage-seaweedfs adapter -> real S3 gateway ->
//!   provider write -> independent readback -> SHA-256 verification;
//! - forced failures: provider refused (Unavailable, NOT NotFound),
//!   silent peer (Timeout), malformed response (ExternalProvider),
//!   stored-data corruption (Verification), ambiguous put (no blind
//!   duplicate), partial put (never VERIFIED), delete ladder,
//!   wrong-target delete, backup member corruption, restore hash
//!   gating, migration source preservation, provider restart recovery;
//! - observability: bounded observations + zero-leakage redaction.
//!
//! The gate runs this suite with --test-threads=1. The shared provider
//! (started by the gate) is used ONLY by non-destructive tests and is
//! never stopped/restarted by a test; every destructive test owns a
//! fresh SeaweedFS runtime with its own unique name, ports, config, and
//! teardown (Drop), so lifecycle mutations can never poison later tests.

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nexus_artifacts::{
    ArtifactErrorCode, ArtifactHash, ArtifactMetadata, ArtifactResult, ArtifactStore,
    ArtifactVersion, BackupSet, DataClass, EncryptionMetadata, RetentionClass, StorageBackend,
};
use nexus_domain::{ArtifactId, CorrelationId, TenantId};
use nexus_provider_storage_local::LocalArtifactStore;
use nexus_provider_storage_seaweedfs::observability::VecSink;
use nexus_provider_storage_seaweedfs::{SeaweedFsArtifactStore, SeaweedFsConfig};
use sha2::{Digest, Sha256};

// ---------------------------------------------------------------- env

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set by the gate"))
}

fn endpoint() -> String {
    env("NEXUS_SEAWEEDFS_ENDPOINT")
}
fn access_key() -> String {
    env("NEXUS_SEAWEEDFS_ACCESS_KEY")
}
fn secret_key() -> String {
    env("NEXUS_SEAWEEDFS_PW_KEY")
}
fn bucket_prefix() -> String {
    env("NEXUS_SEAWEEDFS_BUCKET_PREFIX")
}

fn cfg() -> SeaweedFsConfig {
    SeaweedFsConfig::new(endpoint(), access_key(), secret_key(), bucket_prefix())
        .with_timeouts(Duration::from_secs(3), Duration::from_secs(5))
}

fn store() -> SeaweedFsArtifactStore {
    SeaweedFsArtifactStore::open(cfg()).unwrap()
}

fn store_with_sink() -> (SeaweedFsArtifactStore, VecSink) {
    let sink = VecSink::default();
    let s = SeaweedFsArtifactStore::open(cfg())
        .unwrap()
        .with_sink(Box::new(sink.clone()));
    (s, sink)
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

fn metadata_for(
    tenant: TenantId,
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
        tenant,
        "m4-seaweedfs-test-artifact",
        h.clone(),
        "application/octet-stream",
        bytes.len() as u64,
        owner,
        data_class,
        RetentionClass::LongTerm,
        enc,
        ArtifactVersion::new("1", h.clone()).unwrap(),
        Vec::new(),
        nexus_artifacts::BackendLocation::new(backend, "seaweedfs/m4-test").unwrap(),
    )
}

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nexus-ep037-m4-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn teardown(root: &PathBuf) {
    let _ = fs::remove_dir_all(root);
}

fn docker(args: &[&str]) -> std::process::Output {
    Command::new("docker")
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("docker {args:?} failed: {e}"))
}

// ------------------------------------------------------- owned fixtures
//
// Destructive tests (stop/unavailable, restart/recovery, backing-store
// corruption, migration destination failure) MUST own a fresh SeaweedFS
// runtime instead of bouncing the shared provider. The shared provider
// is used ONLY by non-destructive tests and is never stopped/restarted
// by a test.
//
// Each owned fixture:
//   - unique EP-037 M4-owned container name + temp config root;
//   - unique host ports (never colliding with the shared provider);
//   - runtime-generated credentials in its own s3.json;
//   - production-probe readiness before use;
//   - explicit generation tracking (every restart increments);
//   - teardown via Drop on success, panic, or assertion failure.

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn owned_image() -> String {
    env("NEXUS_SEAWEEDFS_IMAGE")
}

struct OwnedProvider {
    name: String,
    cfg_dir: PathBuf,
    endpoint: String,
    volume_endpoint: String,
    access_key: String,
    secret_key: String,
    generation: u32,
}

impl OwnedProvider {
    /// Create a fresh, unique, probe-ready SeaweedFS runtime. generation
    /// starts at 0. Caller owns the fixture; Drop removes it completely.
    fn start(tag: &str) -> OwnedProvider {
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("nexus-ep037-m4-{tag}-{suffix}");
        let cfg_dir = std::env::temp_dir().join(format!("nexus-ep037-m4-{tag}-{suffix}-cfg"));
        let _ = fs::remove_dir_all(&cfg_dir);
        fs::create_dir_all(&cfg_dir).unwrap();
        let access_key = format!("nexus-m4-{tag}-{suffix}");
        let secret_key = format!("ep037-owned-{tag}-{}-x7", now_epoch());
        let s3_cfg = format!(
            "{{\"identities\":[{{\"name\":\"nexus-m4\",\"credentials\":[{{\"accessKey\":\"{access_key}\",\"secretKey\":\"{secret_key}\"}}],\"actions\":[\"Read\",\"Write\",\"List\",\"Tagging\",\"Admin\"]}}]}}"
        );
        fs::write(cfg_dir.join("s3.json"), s3_cfg).unwrap();

        let s3_port = free_port();
        let filer_port = free_port();
        let volume_port = free_port();
        let out = Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                &name,
                "-p",
                &format!("127.0.0.1:{s3_port}:8333"),
                "-p",
                &format!("127.0.0.1:{filer_port}:8888"),
                "-p",
                &format!("127.0.0.1:{volume_port}:8080"),
                "-v",
                &format!("{}:/etc/seaweedfs:ro", cfg_dir.display()),
                &owned_image(),
                "server",
                "-master.port=9333",
                "-volume.port=8080",
                "-filer.port=8888",
                "-s3.port=8333",
                "-filer",
                "-s3",
                "-s3.config=/etc/seaweedfs/s3.json",
                "-volume.max=256",
                "-dir=/data",
            ])
            .output()
            .unwrap_or_else(|e| panic!("docker run owned {tag} failed: {e}"));
        assert!(
            out.status.success(),
            "owned provider start failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let provider = OwnedProvider {
            name,
            cfg_dir,
            endpoint: format!("127.0.0.1:{s3_port}"),
            volume_endpoint: format!("127.0.0.1:{volume_port}"),
            access_key,
            secret_key,
            generation: 0,
        };
        eprintln!("[owned {tag}] generation 0 created: {}", provider.name);
        provider.wait_ready(120);
        provider
    }

    fn cfg(&self) -> SeaweedFsConfig {
        SeaweedFsConfig::new(
            &self.endpoint,
            &self.access_key,
            &self.secret_key,
            bucket_prefix(),
        )
        .with_timeouts(Duration::from_secs(3), Duration::from_secs(5))
    }

    fn store(&self) -> SeaweedFsArtifactStore {
        SeaweedFsArtifactStore::open(self.cfg()).unwrap()
    }

    fn stop(&mut self) {
        let out = docker(&["stop", &self.name]);
        assert!(out.status.success(), "owned stop failed");
    }

    fn restart(&mut self) {
        let out = docker(&["restart", &self.name]);
        assert!(out.status.success(), "owned restart failed");
        self.generation += 1;
        eprintln!(
            "[owned {}] generation -> {} at {}",
            self.name,
            self.generation,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
    }

    /// Deadline-based production-probe readiness (requirement I/J):
    /// monotonic deadline, bounded backoff, attempt counter, last
    /// observed failure. Only errors empirically proven to mean
    /// SEAWEEDFS_NOT_READY_YET (connection reset/refused, read timeout,
    /// provider 5xx / "no writable volume" / filer topology unavailable
    /// during resync) trigger another attempt. A Verification failure on
    /// the probe canary is NOT transient - it aborts immediately. This
    /// never alters production error mapping; it only decides whether to
    /// probe again.
    fn wait_ready(&self, timeout_secs: u64) {
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        let mut attempts = 0u32;
        let mut backoff = Duration::from_millis(500);
        loop {
            attempts += 1;
            match SeaweedFsArtifactStore::open(self.cfg()).and_then(|s| s.diag_probe()) {
                Ok(()) => {
                    eprintln!(
                        "[owned {}] generation {} ready after {attempts} attempts",
                        self.name, self.generation
                    );
                    return;
                }
                Err(e) => {
                    if !matches!(
                        e.code,
                        ArtifactErrorCode::Unavailable
                            | ArtifactErrorCode::Timeout
                            | ArtifactErrorCode::ExternalProvider
                    ) {
                        panic!(
                            "owned provider {} probe failed with non-transient {e}",
                            self.name
                        );
                    }
                    if Instant::now() >= deadline {
                        panic!(
                            "owned provider {} not ready after {timeout_secs}s ({attempts} attempts): last={e}",
                            self.name
                        );
                    }
                    std::thread::sleep(backoff);
                    backoff = (backoff * 2).min(Duration::from_secs(3));
                }
            }
        }
    }
}

impl Drop for OwnedProvider {
    fn drop(&mut self) {
        let _ = docker(&["rm", "-f", &self.name]);
        let _ = fs::remove_dir_all(&self.cfg_dir);
        eprintln!("[owned {}] teardown complete", self.name);
    }
}

// ------------------------------------------------------------ positive

#[test]
fn ep037_integration_seaweedfs_positive_roundtrip() {
    let (mut store, sink) = store_with_sink();
    let t = tenant(2);
    let bytes = b"m4 seaweedfs real provider payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(1);
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let (read_meta, read_bytes) = store.get(&t, &id, &correlation()).unwrap();
    assert_eq!(read_meta.content_hash, h);
    assert_eq!(read_bytes, bytes);
    let verified = store.verify(&t, &id, &correlation()).unwrap();
    assert_eq!(verified, h);
    let obs = sink.0.lock().unwrap();
    let put_obs = obs
        .iter()
        .find(|o| o.operation == "put")
        .expect("put observation");
    assert!(put_obs.integrity_verified);
    assert_eq!(put_obs.size_bytes, Some(bytes.len() as u64));
    assert_eq!(put_obs.provider, "seaweedfs:s3-gateway");
    assert!(!put_obs.artifact_hash.as_ref().unwrap().is_empty());
}

// ----------------------------------------------------- encryption gate

#[test]
fn ep037_failure_encryption_missing_zero_provider_mutation() {
    let mut store = store();
    let t = tenant(3);
    let bytes = b"m4 sensitive payload without encryption".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(2);
    // Model-level metadata is built with backend LOCAL so the adapter
    // boundary is the only enforcement point (same pattern as M3 NAS).
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Sensitive,
        None,
        StorageBackend::Local,
        "principal-m4",
    )
    .unwrap();
    let err = store
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::Policy);
    // ZERO provider mutation: the object must not exist anywhere on the
    // provider (bucket may not even have been created).
    let bucket = format!("{}{}", bucket_prefix(), t.as_str());
    let direct = nexus_provider_storage_seaweedfs::transport::S3Client::connect(
        &endpoint(),
        &access_key(),
        &secret_key(),
        Duration::from_secs(3),
        Duration::from_secs(5),
    );
    if let Ok(keys) = direct.list_keys(&bucket, "objects/") {
        assert!(keys.is_empty(), "objects leaked to provider: {keys:?}");
    }
    // bucket absent = no mutation
}

// ---------------------------------------------------------- corruption

#[test]
fn ep037_failure_corrupted_stored_bytes_verification() {
    let (mut store, sink) = store_with_sink();
    let t = tenant(4);
    let bytes = b"original artifact bytes that will be tampered".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(3);
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    // Real provider-state tamper by a second writer: overwrite the
    // object at its content address with DIFFERENT bytes.
    let bucket = format!("{}{}", bucket_prefix(), t.as_str());
    let direct = nexus_provider_storage_seaweedfs::transport::S3Client::connect(
        &endpoint(),
        &access_key(),
        &secret_key(),
        Duration::from_secs(3),
        Duration::from_secs(5),
    );
    direct
        .put_object(
            &bucket,
            &format!("objects/{}", h.as_str()),
            b"tampered bytes",
        )
        .unwrap();
    let err = store.get(&t, &id, &correlation()).unwrap_err();
    assert_eq!(
        err.code,
        ArtifactErrorCode::Verification,
        "corrupted stored bytes must fail verification, got: {err}"
    );
    let obs = sink.0.lock().unwrap();
    let get_obs = obs
        .iter()
        .find(|o| o.operation == "get")
        .expect("get observation");
    assert!(!get_obs.integrity_verified);
    assert_eq!(get_obs.result, "Verification");
}

#[test]
fn ep037_failure_volume_dat_corruption_fails_closed() {
    // Deep provider-state tamper on an OWNED fixture (requirement K):
    // fresh SeaweedFS -> production adapter write of the exact target ->
    // verify target -> identify the collection volumes through the real
    // volume-server /status API -> capture pre-corruption .dat hashes ->
    // corrupt the needle DATA region of the owned target collection
    // volumes (NEVER the 8-byte superblock: zeroing the superblock makes
    // the volume unloadable and the provider never returns; the semantic
    // requirement is corrupted backing data, so the target cannot become
    // verified, per L) -> prove at least one file hash changed -> restart
    // the SAME owned provider (generation 1) -> wait for readiness using
    // a DIFFERENT probe namespace (diag tenant ff bucket, never the
    // corrupted target) -> read the target through the production adapter
    // -> require integrity NOT VERIFIED -> destroy the fixture.
    let mut owned = OwnedProvider::start("corrupt");
    let mut store = owned.store();
    let t = tenant(5);
    let bytes = b"only needle in the fresh volume".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(4);
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    // Target must be verified BEFORE corruption (positive proof).
    let verified = store.verify(&t, &id, &correlation()).unwrap();
    assert_eq!(verified, h);

    // Identify the volumes holding this bucket's collection via the real
    // volume-server /status JSON (object and metadata needles may land in
    // different volumes of the growth batch).
    let bucket = format!("{}{}", bucket_prefix(), t.as_str());
    let volume_ids = volume_ids_for_collection(&owned.volume_endpoint, &bucket);
    assert!(!volume_ids.is_empty(), "no volumes for collection");

    // Pre-corruption file hashes (sha256 of every owned collection .dat).
    let mut pre_hashes: Vec<String> = Vec::new();
    for volume_id in &volume_ids {
        let dat_path = format!("/data/{bucket}_{volume_id}.dat");
        let out = docker(&[
            "exec",
            &owned.name,
            "sh",
            "-c",
            &format!("sha256sum {dat_path}"),
        ]);
        assert!(out.status.success(), "sha256sum failed");
        pre_hashes.push(String::from_utf8_lossy(&out.stdout).to_string());
    }

    // Corrupt needle DATA (offset 64..320, past the 8-byte superblock)
    // in every owned collection volume. The superblock stays intact so
    // the provider can restart; the artifact bytes can no longer verify.
    let mut corrupted = 0usize;
    for volume_id in &volume_ids {
        let dat_path = format!("/data/{bucket}_{volume_id}.dat");
        let corrupt = docker(&[
            "exec",
            &owned.name,
            "sh",
            "-c",
            &format!(
                "dd conv=notrunc if=/dev/zero of={dat_path} bs=1 count=256 seek=64 2>/dev/null"
            ),
        ]);
        if corrupt.status.success() {
            corrupted += 1;
        }
    }
    assert!(corrupted > 0, "no volume .dat corrupted");

    // Prove the corruption actually changed at least one owned file hash.
    let mut post_hashes: Vec<String> = Vec::new();
    for volume_id in &volume_ids {
        let dat_path = format!("/data/{bucket}_{volume_id}.dat");
        let out = docker(&[
            "exec",
            &owned.name,
            "sh",
            "-c",
            &format!("sha256sum {dat_path}"),
        ]);
        assert!(out.status.success(), "sha256sum failed");
        post_hashes.push(String::from_utf8_lossy(&out.stdout).to_string());
    }
    assert!(
        pre_hashes
            .iter()
            .zip(post_hashes.iter())
            .any(|(a, b)| a != b),
        "corruption must change at least one collection volume file hash"
    );

    // Restart the SAME owned provider: generation 0 -> 1.
    owned.restart();
    // Readiness probe runs in a DIFFERENT namespace (diag tenant ff
    // bucket) and must never interact with the corrupted target (M).
    owned.wait_ready(120);

    // Generation-aware client: a fresh store is opened AFTER the restart
    // (requirement F); a generation-0 client must not certify generation
    // 1 state.
    let mut store = owned.store();
    // The production read of the corrupted target must NOT verify.
    // Canonical manifestations accepted: Verification (hash mismatch),
    // ExternalProvider (provider decode/read failure), or NotFound
    // (missing/corrupt object) - whichever is the truthful result of
    // real corruption. A verified artifact is forbidden (L).
    let result = store.get(&t, &id, &correlation());
    assert!(
        result.is_err(),
        "corrupted provider storage must never return a verified artifact"
    );
    eprintln!(
        "[owned corrupt] target read classified: {:?}",
        result.unwrap_err().code
    );
    // Fixture destroyed by Drop; no subsequent test ever inherits it.
}

/// Query the REAL SeaweedFS volume-server /status API (documented:
/// `weed volume` status endpoint) and return the ids of every volume
/// whose collection equals the bucket name. Never guesses the volume
/// file. Bounded retries cover the transient window while the volume
/// server's HTTP handler re-registers after a container restart.
fn volume_ids_for_collection(volume_endpoint: &str, bucket: &str) -> Vec<String> {
    for _attempt in 0..10 {
        if let Ok(mut stream) = TcpStream::connect(volume_endpoint) {
            let req = format!(
                "GET /status HTTP/1.1\r\nHost: {volume_endpoint}\r\nConnection: close\r\n\r\n"
            );
            if stream.write_all(req.as_bytes()).is_ok() {
                let mut raw = Vec::new();
                if stream.read_to_end(&mut raw).is_ok() {
                    if let Some(head_end) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                        // The volume server replies with
                        // Transfer-Encoding: chunked; decode the
                        // framing before JSON parsing.
                        let body = decode_chunked(&raw[head_end + 4..]).unwrap_or_default();
                        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) {
                            if let Some(volumes) = value["Volumes"].as_array() {
                                let ids: Vec<String> = volumes
                                    .iter()
                                    .filter(|v| v["Collection"].as_str() == Some(bucket))
                                    .filter_map(|v| v["Id"].as_u64().map(|i| i.to_string()))
                                    .collect();
                                if !ids.is_empty() {
                                    return ids;
                                }
                            }
                        }
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
    Vec::new()
}

/// Minimal HTTP chunked transfer decoding (hex size lines until 0).
fn decode_chunked(raw: &[u8]) -> Option<String> {
    let mut rest = raw;
    let mut out = Vec::new();
    loop {
        let line_end = rest.iter().position(|&b| b == b'\n')?;
        let size_line = std::str::from_utf8(&rest[..line_end]).ok()?;
        let size = usize::from_str_radix(size_line.trim().trim_end_matches('\r'), 16).ok()?;
        if size == 0 {
            return Some(String::from_utf8_lossy(&out).into_owned());
        }
        if rest.len() < line_end + 1 + size + 2 {
            return None;
        }
        out.extend_from_slice(&rest[line_end + 1..line_end + 1 + size]);
        rest = &rest[line_end + 1 + size + 2..];
    }
}

// ------------------------------------------------------- partial write

#[test]
fn ep037_failure_partial_put_never_verified() {
    // Raw transport partial write: sign a PUT for the FULL payload but
    // close the connection after half the body. The partial bytes must
    // never become a VERIFIED artifact.
    let t = tenant(6);
    let bytes = b"partial upload body that gets truncated".to_vec();
    let h = hash_of(&bytes);
    let bucket = format!("{}{}", bucket_prefix(), t.as_str());
    let direct = nexus_provider_storage_seaweedfs::transport::S3Client::connect(
        &endpoint(),
        &access_key(),
        &secret_key(),
        Duration::from_secs(3),
        Duration::from_secs(5),
    );
    direct.create_bucket(&bucket).unwrap();

    // Build a signed raw request with full Content-Length, then send
    // only half the body and drop the connection.
    let key = format!("objects/{}", h.as_str());
    let payload_hash = digest(&bytes);
    let signer = private_signer();
    let (auth, amz_date) = signer.sign(
        "PUT",
        &endpoint(),
        &format!("/{bucket}/{key}"),
        "",
        &payload_hash,
        now_epoch(),
    );
    let req = format!(
        "PUT /{bucket}/{key} HTTP/1.1\r\nHost: {}\r\nAuthorization: {auth}\r\nx-amz-date: {amz_date}\r\nx-amz-content-sha256: {payload_hash}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        endpoint(),
        bytes.len()
    );
    let half = &bytes[..bytes.len() / 2];
    let mut stream = TcpStream::connect(endpoint()).unwrap();
    stream.write_all(req.as_bytes()).unwrap();
    stream.write_all(half).unwrap();
    drop(stream); // terminate mid-body

    // Adapter read path: object must not verify (absent or hash
    // mismatch), never a successful Artifact return.
    let mut store = store();
    let id = artifact_id(5);
    let result = store.verify(&t, &id, &correlation());
    assert!(result.is_err(), "partial write must never verify");
    if let Ok(b) = direct.get_object(&bucket, &key) {
        assert_ne!(
            digest(&b),
            h.as_str(),
            "partial bytes must not match the full-content hash"
        );
    }
    // absent -> fail closed
}

// ------------------------------------------------- ambiguous put

#[test]
fn ep037_failure_ambiguous_put_deduplicates() {
    let t = tenant(7);
    let bytes = b"ambiguous put payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(6);
    let bucket = format!("{}{}", bucket_prefix(), t.as_str());
    // Simulate an ambiguous prior write: the object already exists on
    // the provider (e.g. from a lost acknowledgement) with the SAME
    // content address.
    let direct = nexus_provider_storage_seaweedfs::transport::S3Client::connect(
        &endpoint(),
        &access_key(),
        &secret_key(),
        Duration::from_secs(3),
        Duration::from_secs(5),
    );
    direct.create_bucket(&bucket).unwrap();
    direct
        .put_object(&bucket, &format!("objects/{}", h.as_str()), &bytes)
        .unwrap();

    let mut store = store();
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    // Content addressing reconciles: exactly ONE object at the content
    // address, never a blind duplicate.
    let keys = direct
        .list_keys(&bucket, &format!("objects/{}", h.as_str()))
        .unwrap();
    assert_eq!(keys.len(), 1, "ambiguous put must not duplicate: {keys:?}");
    let (read_meta, read_bytes) = store.get(&t, &id, &correlation()).unwrap();
    assert_eq!(read_meta.content_hash, h);
    assert_eq!(read_bytes, bytes);
}

// -------------------------------------------------------- delete ladder

#[test]
fn ep037_failure_delete_absent_verified_ladder() {
    let (mut store, _sink) = store_with_sink();
    let t = tenant(8);
    let bytes = b"delete ladder payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(7);
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    store.delete(&t, &id, &correlation()).unwrap();
    // Second delete: already absent -> NotFound (never silent success).
    let err = store.delete(&t, &id, &correlation()).unwrap_err();
    assert_eq!(err.code, ArtifactErrorCode::NotFound);
    let bucket = format!("{}{}", bucket_prefix(), t.as_str());
    let direct = nexus_provider_storage_seaweedfs::transport::S3Client::connect(
        &endpoint(),
        &access_key(),
        &secret_key(),
        Duration::from_secs(3),
        Duration::from_secs(5),
    );
    match direct.get_object(&bucket, &format!("objects/{}", h.as_str())) {
        Err(nexus_provider_storage_seaweedfs::transport::S3Error::Status { code: 404, .. }) => {}
        other => panic!("object must be absent after delete, got {other:?}"),
    }
}

#[test]
fn ep037_failure_wrong_target_delete_preserves_other() {
    let mut store = store();
    let t = tenant(9);
    let bytes_a = b"artifact A distinct content".to_vec();
    let bytes_b = b"artifact B distinct content".to_vec();
    let h_a = hash_of(&bytes_a);
    let h_b = hash_of(&bytes_b);
    let id_a = artifact_id(8);
    let id_b = artifact_id(9);
    let meta_a = metadata_for(
        t.clone(),
        id_a.clone(),
        &bytes_a,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    let meta_b = metadata_for(
        t.clone(),
        id_b.clone(),
        &bytes_b,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id_a, &h_a, &bytes_a, &meta_a, &correlation())
        .unwrap();
    store
        .put(&t, &id_b, &h_b, &bytes_b, &meta_b, &correlation())
        .unwrap();
    store.delete(&t, &id_a, &correlation()).unwrap();
    // B must be fully intact with correct bytes.
    let (meta_b_read, bytes_b_read) = store.get(&t, &id_b, &correlation()).unwrap();
    assert_eq!(meta_b_read.content_hash, h_b);
    assert_eq!(bytes_b_read, bytes_b);
    // A is gone.
    assert_eq!(
        store.get(&t, &id_a, &correlation()).unwrap_err().code,
        ArtifactErrorCode::NotFound
    );
}

#[test]
fn ep037_failure_shared_content_delete_preserves_object() {
    let mut store = store();
    let t = tenant(10);
    let bytes = b"shared content dedup payload".to_vec();
    let h = hash_of(&bytes);
    let id_a = artifact_id(10);
    let id_b = artifact_id(11);
    let meta_a = metadata_for(
        t.clone(),
        id_a.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    let meta_b = metadata_for(
        t.clone(),
        id_b.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id_a, &h, &bytes, &meta_a, &correlation())
        .unwrap();
    store
        .put(&t, &id_b, &h, &bytes, &meta_b, &correlation())
        .unwrap();
    // Deleting A must NOT remove the object still referenced by B.
    store.delete(&t, &id_a, &correlation()).unwrap();
    let (_, bytes_b) = store.get(&t, &id_b, &correlation()).unwrap();
    assert_eq!(bytes_b, bytes);
    // After B is gone too, the object is removed.
    store.delete(&t, &id_b, &correlation()).unwrap();
    let bucket = format!("{}{}", bucket_prefix(), t.as_str());
    let direct = nexus_provider_storage_seaweedfs::transport::S3Client::connect(
        &endpoint(),
        &access_key(),
        &secret_key(),
        Duration::from_secs(3),
        Duration::from_secs(5),
    );
    let keys = direct
        .list_keys(&bucket, &format!("objects/{}", h.as_str()))
        .unwrap();
    assert!(
        keys.is_empty(),
        "shared object must be removed after last ref: {keys:?}"
    );
}

// ------------------------------------------------- backup/restore gates

#[test]
fn ep037_failure_backup_member_corruption_blocks_verify() {
    let mut store = store();
    let t = tenant(11);
    let bytes_a = b"backup member A".to_vec();
    let bytes_b = b"backup member B".to_vec();
    let h_a = hash_of(&bytes_a);
    let h_b = hash_of(&bytes_b);
    let id_a = artifact_id(12);
    let id_b = artifact_id(13);
    let meta_a = metadata_for(
        t.clone(),
        id_a.clone(),
        &bytes_a,
        DataClass::Personal,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    let meta_b = metadata_for(
        t.clone(),
        id_b.clone(),
        &bytes_b,
        DataClass::Personal,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id_a, &h_a, &bytes_a, &meta_a, &correlation())
        .unwrap();
    store
        .put(&t, &id_b, &h_b, &bytes_b, &meta_b, &correlation())
        .unwrap();
    let backup = BackupSet::new(
        "b-m4-corrupt",
        t.clone(),
        vec![DataClass::Personal],
        nexus_artifacts::BackendLocation::new(
            StorageBackend::SeaweedFs,
            "seaweedfs/b-m4-corrupt.json",
        )
        .unwrap(),
        vec![h_a.clone(), h_b.clone()],
        Some("vault:keys/m4".to_string()),
        "0.1.0",
        "1",
        "2026-08-23T00:00:00Z",
    )
    .unwrap();
    store
        .create_backup(&t, &sign_backup(backup.clone()), &correlation())
        .unwrap();
    // Corrupt member B on the provider.
    let bucket = format!("{}{}", bucket_prefix(), t.as_str());
    let direct = nexus_provider_storage_seaweedfs::transport::S3Client::connect(
        &endpoint(),
        &access_key(),
        &secret_key(),
        Duration::from_secs(3),
        Duration::from_secs(5),
    );
    direct
        .put_object(
            &bucket,
            &format!("objects/{}", h_b.as_str()),
            b"corrupted member",
        )
        .unwrap();
    // Restore must fail: one good manifest plus one corrupted object is
    // not success.
    let plan = nexus_artifacts::RestorePlan::new(
        "r-m4-corrupt",
        t.clone(),
        "b-m4-corrupt",
        "fresh-target-1",
        vec![h_a, h_b],
        Some(correlation()),
    )
    .unwrap();
    let err = store.restore(&t, &plan, &correlation()).unwrap_err();
    assert_eq!(
        err.code,
        ArtifactErrorCode::Verification,
        "corrupted backup member must block restore, got: {err}"
    );
}

#[test]
fn ep037_failure_restore_requires_hash_verification() {
    let mut store = store();
    let t = tenant(12);
    let bytes = b"restore hash gate payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(14);
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Personal,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let backup = BackupSet::new(
        "b-m4-restore",
        t.clone(),
        vec![DataClass::Personal],
        nexus_artifacts::BackendLocation::new(
            StorageBackend::SeaweedFs,
            "seaweedfs/b-m4-restore.json",
        )
        .unwrap(),
        vec![h.clone()],
        Some("vault:keys/m4".to_string()),
        "0.1.0",
        "1",
        "2026-08-23T00:00:00Z",
    )
    .unwrap();
    store
        .create_backup(&t, &sign_backup(backup.clone()), &correlation())
        .unwrap();
    // Plan requires a hash that is NOT on the fresh target.
    let missing = ArtifactHash::new(format!("{:064x}", 0x42)).unwrap();
    let plan = nexus_artifacts::RestorePlan::new(
        "r-m4-restore",
        t.clone(),
        "b-m4-restore",
        "fresh-target-1",
        vec![h.clone(), missing],
        Some(correlation()),
    )
    .unwrap();
    let err = store.restore(&t, &plan, &correlation()).unwrap_err();
    assert_eq!(
        err.code,
        ArtifactErrorCode::Verification,
        "missing hash must fail restore, got: {err}"
    );
    // Plan referencing a nonexistent backup -> NotFound.
    let plan2 = nexus_artifacts::RestorePlan::new(
        "r-m4-restore2",
        t.clone(),
        "b-does-not-exist",
        "fresh-target-1",
        vec![h],
        Some(correlation()),
    )
    .unwrap();
    let err2 = store.restore(&t, &plan2, &correlation()).unwrap_err();
    assert_eq!(err2.code, ArtifactErrorCode::NotFound);
}

// ------------------------------------------------------------ migration

#[test]
fn ep037_failure_migration_success_verifies_destination() {
    let root = temp_root("migrate-src");
    let mut source = LocalArtifactStore::open(&root).unwrap();
    let t = tenant(13);
    let bytes = b"migration payload to seaweedfs".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(15);
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::Local,
        "principal-m4",
    )
    .unwrap();
    source
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();

    // Copy phase: source read -> target write (harness drives copy).
    let mut target = store();
    target
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let migration = nexus_artifacts::StorageMigration::new(
        "mig-m4-success",
        t.clone(),
        StorageBackend::Local,
        StorageBackend::SeaweedFs,
        vec![nexus_artifacts::ObjectRef::new(id.clone(), h.clone())],
    )
    .unwrap();
    let verified = target.migrate(&t, &migration, &correlation()).unwrap();
    assert_eq!(
        verified.state,
        nexus_artifacts::MigrationState::Verified,
        "migration must be VERIFIED only after destination readback"
    );
    // SOURCE MUST REMAIN INTACT until approval: the destination was
    // verified but the source object still exists.
    let (_, src_bytes) = source.get(&t, &id, &correlation()).unwrap();
    assert_eq!(src_bytes, bytes);
    // After approval, the source copy may be removed.
    source.delete(&t, &id, &correlation()).unwrap();
    assert_eq!(
        source.get(&t, &id, &correlation()).unwrap_err().code,
        ArtifactErrorCode::NotFound
    );
    // Destination is independently readable with verified bytes.
    let (_, dest_bytes) = target.get(&t, &id, &correlation()).unwrap();
    assert_eq!(dest_bytes, bytes);
    teardown(&root);
}

#[test]
fn ep037_failure_migration_destination_failure_preserves_source() {
    let root = temp_root("migrate-fail-src");
    let mut source = LocalArtifactStore::open(&root).unwrap();
    let t = tenant(14);
    let bytes = b"migration must not lose source".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(16);
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::Local,
        "principal-m4",
    )
    .unwrap();
    source
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();

    // Destination failure on an OWNED fixture (requirement O): the owned
    // provider is stopped before the copy completes.
    let mut owned = OwnedProvider::start("migrate-fail");
    owned.stop();
    let mut target = owned.store();
    let copy_err = target.put(&t, &id, &h, &bytes, &meta, &correlation());
    assert_eq!(
        copy_err.unwrap_err().code,
        ArtifactErrorCode::Unavailable,
        "stopped provider must be Unavailable, not NotFound or success"
    );
    // SOURCE REMAINS INTACT.
    let (_, src_bytes) = source.get(&t, &id, &correlation()).unwrap();
    assert_eq!(src_bytes, bytes);

    // Bounded recovery: restart the SAME owned provider (generation 1)
    // and the migration completes; stale client state does not fabricate
    // failure. No other test inherits this restarted fixture.
    owned.restart();
    owned.wait_ready(120);
    let mut target2 = owned.store();
    target2
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let migration = nexus_artifacts::StorageMigration::new(
        "mig-m4-fail",
        t.clone(),
        StorageBackend::Local,
        StorageBackend::SeaweedFs,
        vec![nexus_artifacts::ObjectRef::new(id.clone(), h.clone())],
    )
    .unwrap();
    let verified = target2.migrate(&t, &migration, &correlation()).unwrap();
    assert_eq!(verified.state, nexus_artifacts::MigrationState::Verified);
    let (_, dest_bytes) = target2.get(&t, &id, &correlation()).unwrap();
    assert_eq!(dest_bytes, bytes);
    teardown(&root);
}

#[test]
fn ep037_failure_retry_hash_aware_no_duplicate() {
    let t = tenant(15);
    let bytes = b"retry hash-aware dedup".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(17);
    let bucket = format!("{}{}", bucket_prefix(), t.as_str());
    let direct = nexus_provider_storage_seaweedfs::transport::S3Client::connect(
        &endpoint(),
        &access_key(),
        &secret_key(),
        Duration::from_secs(3),
        Duration::from_secs(5),
    );
    // First attempt wrote the object but the acknowledgement was lost
    // (simulated by pre-seeding the content address).
    direct.create_bucket(&bucket).unwrap();
    direct
        .put_object(&bucket, &format!("objects/{}", h.as_str()), &bytes)
        .unwrap();
    let mut target = store();
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    // Retry: already-verified destination content with the exact hash
    // resumes safely - no blind rewrite or duplicate object.
    target
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    let keys = direct
        .list_keys(&bucket, &format!("objects/{}", h.as_str()))
        .unwrap();
    assert_eq!(keys.len(), 1, "retry must not duplicate: {keys:?}");
}

// ------------------------------------------- transport classification

#[test]
fn ep037_failure_timeout_is_timeout() {
    // Controlled transport peer ONLY for transport classification: an
    // accepted connection that never responds must classify as Timeout
    // (never collapsed with Unavailable, never assumed success).
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            // Intentionally never respond; hold the connection open.
            std::thread::sleep(Duration::from_secs(10));
        }
    });
    let cfg = SeaweedFsConfig::new(
        format!("127.0.0.1:{}", addr.port()),
        access_key(),
        secret_key(),
        bucket_prefix(),
    )
    .with_timeouts(Duration::from_secs(1), Duration::from_secs(2));
    let mut store = SeaweedFsArtifactStore::open(cfg).unwrap();
    let bytes = b"timeout probe".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(18);
    let meta = metadata_for(
        tenant(16),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    let err = store
        .put(&tenant(16), &id, &h, &bytes, &meta, &correlation())
        .unwrap_err();
    assert_eq!(
        err.code,
        ArtifactErrorCode::Timeout,
        "silent peer must classify as Timeout, got: {err}"
    );
}

#[test]
fn ep037_failure_malformed_response_external() {
    // Controlled transport peer ONLY for transport classification: a
    // peer that returns an unusable status line must classify as
    // ExternalProvider, never guessed into success.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(b"GARBAGE NOT-HTTP\r\n\r\n");
        }
    });
    let cfg = SeaweedFsConfig::new(
        format!("127.0.0.1:{}", addr.port()),
        access_key(),
        secret_key(),
        bucket_prefix(),
    )
    .with_timeouts(Duration::from_secs(1), Duration::from_secs(2));
    let mut store = SeaweedFsArtifactStore::open(cfg).unwrap();
    let bytes = b"malformed probe".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(19);
    let meta = metadata_for(
        tenant(17),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    let err = store
        .put(&tenant(17), &id, &h, &bytes, &meta, &correlation())
        .unwrap_err();
    assert_eq!(
        err.code,
        ArtifactErrorCode::ExternalProvider,
        "malformed response must classify as ExternalProvider, got: {err}"
    );
}

// ------------------------------------------------------- restart/recover

#[test]
fn ep037_failure_provider_restart_bounded_recovery() {
    // Recovery test owns its entire lifecycle (requirement O): fresh
    // provider -> ready -> successful operation -> stop -> observe
    // truthful failure -> restart SAME owned provider/state -> generation
    // increments -> full production readiness probe -> subsequent
    // production operation succeeds -> teardown. Other tests never
    // inherit this restarted fixture.
    let mut owned = OwnedProvider::start("restart");
    let mut store = owned.store();
    let t = tenant(18);
    let bytes = b"restart recovery payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(20);
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    store
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    // Stop -> Unavailable (NOT NotFound, NOT empty artifact, NOT
    // successful zero-byte read).
    owned.stop();
    let err = store.get(&t, &id, &correlation()).unwrap_err();
    assert_eq!(
        err.code,
        ArtifactErrorCode::Unavailable,
        "stopped provider must be Unavailable, got: {err}"
    );
    // Restart SAME owned provider -> generation 1 -> full probe ->
    // subsequent operation succeeds.
    owned.restart();
    owned.wait_ready(120);
    let mut store2 = owned.store();
    let (_, read_bytes) = store2.get(&t, &id, &correlation()).unwrap();
    assert_eq!(read_bytes, bytes);
}

#[test]
fn ep037_failure_unavailable_not_found_distinct() {
    // With the provider STOPPED, a missing artifact is Unavailable
    // (connect failure), not NotFound - failures are never flattened.
    // Owned fixture (requirement N): fresh provider -> verify ready ->
    // stop -> call production adapter -> require Unavailable -> teardown.
    // The shared provider is never stopped, so later tests stay clean.
    let mut owned = OwnedProvider::start("unavailable");
    let mut store = owned.store();
    owned.stop();
    let err = store
        .get(&tenant(19), &artifact_id(21), &correlation())
        .unwrap_err();
    assert_eq!(
        err.code,
        ArtifactErrorCode::Unavailable,
        "provider down must not be confused with NotFound, got: {err}"
    );
    // No restart of this fixture; Drop tears it down.
}

// ------------------------------------------------------------- listing

#[test]
fn ep037_integration_seaweedfs_list_pagination() {
    let mut store = store();
    let t = tenant(20);
    let mut ids = Vec::new();
    for i in 0..5u8 {
        let bytes = format!("list page payload {i}").into_bytes();
        let h = hash_of(&bytes);
        let id = artifact_id(22 + i);
        let meta = metadata_for(
            t.clone(),
            id.clone(),
            &bytes,
            DataClass::Public,
            None,
            StorageBackend::SeaweedFs,
            "principal-m4",
        )
        .unwrap();
        store
            .put(&t, &id, &h, &bytes, &meta, &correlation())
            .unwrap();
        ids.push(id.as_str().to_string());
    }
    // Multi-page adapter listing with limit=2: all 5 unique, no
    // duplicates, no skips.
    let (page1, c1) = store.list(&t, None, 2).unwrap();
    assert_eq!(page1.len(), 2);
    let (page2, c2) = store.list(&t, c1.as_deref(), 2).unwrap();
    assert_eq!(page2.len(), 2);
    let (page3, c3) = store.list(&t, c2.as_deref(), 2).unwrap();
    assert_eq!(page3.len(), 1);
    assert!(c3.is_none());
    let mut all: Vec<String> = page1
        .iter()
        .chain(page2.iter())
        .chain(page3.iter())
        .map(|m| m.artifact_id.as_str().to_string())
        .collect();
    all.sort();
    let mut expected = ids.clone();
    expected.sort();
    assert_eq!(
        all, expected,
        "listing must be complete with no dupes/skips"
    );
    // Invalid continuation token fails closed.
    let err = store
        .list(&t, Some("not-a-real-artifact-id"), 2)
        .unwrap_err();
    assert_eq!(
        err.code,
        ArtifactErrorCode::Validation,
        "invalid continuation token must fail closed, got: {err}"
    );
}

// ------------------------------------------------------------ redaction

#[test]
fn ep037_failure_redaction_canary_zero_leakage() {
    let canary = format!(
        "ep037canary{}x9",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    // 1. Metadata carries the canary; observations and errors must not.
    let (mut store, sink) = store_with_sink();
    let t = tenant(21);
    let bytes = b"redaction canary payload".to_vec();
    let h = hash_of(&bytes);
    let id = artifact_id(26);
    let meta = metadata_for(
        t.clone(),
        id.clone(),
        &bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        &format!("principal-{canary}"),
    )
    .unwrap();
    store
        .put(&t, &id, &h, &bytes, &meta, &correlation())
        .unwrap();
    // Error path message must not leak the canary either.
    let err = store.get(&t, &artifact_id(27), &correlation()).unwrap_err();
    assert!(
        !err.to_string().contains(&canary),
        "error leaked metadata canary"
    );
    let obs = sink.0.lock().unwrap();
    for o in obs.iter() {
        let serialized = format!("{o:?}");
        assert!(
            !serialized.contains(&canary),
            "observation leaked canary: {serialized}"
        );
    }
    drop(obs);

    // 2. Credentials carry the canary (wrong secret -> 403); the error
    // must not echo it.
    let bad_cfg = SeaweedFsConfig::new(
        endpoint(),
        access_key(),
        format!("{}{}", secret_key(), canary),
        bucket_prefix(),
    )
    .with_timeouts(Duration::from_secs(3), Duration::from_secs(5));
    let mut bad_store = SeaweedFsArtifactStore::open(bad_cfg).unwrap();
    let bad_bytes = b"redaction probe".to_vec();
    let bad_h = hash_of(&bad_bytes);
    let bad_id = artifact_id(28);
    let bad_meta = metadata_for(
        t.clone(),
        bad_id.clone(),
        &bad_bytes,
        DataClass::Public,
        None,
        StorageBackend::SeaweedFs,
        "principal-m4",
    )
    .unwrap();
    let bad_err = bad_store
        .put(&t, &bad_id, &bad_h, &bad_bytes, &bad_meta, &correlation())
        .unwrap_err();
    assert!(
        !bad_err.to_string().contains(&canary),
        "error leaked credential canary: {bad_err}"
    );

    // 3. The ops diagnostic must not leak credentials or payload.
    let diag = env!("CARGO_BIN_EXE_seaweedfs-diag");
    let out = Command::new(diag)
        .arg("status")
        .env("NEXUS_SEAWEEDFS_ENDPOINT", endpoint())
        .env("NEXUS_SEAWEEDFS_ACCESS_KEY", access_key())
        .env(
            "NEXUS_SEAWEEDFS_PW_KEY",
            format!("{}{}", secret_key(), canary),
        )
        .env("NEXUS_SEAWEEDFS_BUCKET_PREFIX", bucket_prefix())
        .output()
        .expect("run seaweedfs-diag status");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !combined.contains(&canary),
        "seaweedfs-diag leaked canary: {combined}"
    );
}

// ------------------------------------------------------------ diag ops

#[test]
fn ep037_integration_seaweedfs_diag_probe_verified() {
    let diag = env!("CARGO_BIN_EXE_seaweedfs-diag");
    let out = Command::new(diag)
        .arg("status")
        .env("NEXUS_SEAWEEDFS_ENDPOINT", endpoint())
        .env("NEXUS_SEAWEEDFS_ACCESS_KEY", access_key())
        .env("NEXUS_SEAWEEDFS_PW_KEY", secret_key())
        .env("NEXUS_SEAWEEDFS_BUCKET_PREFIX", bucket_prefix())
        .output()
        .expect("run seaweedfs-diag status");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        out.status.success(),
        "diag must exit 0 when healthy: {stdout}"
    );
    assert!(stdout.contains("configured: true"), "{stdout}");
    assert!(stdout.contains("probe_verified: true"), "{stdout}");
    assert!(stdout.contains("state: OK"), "{stdout}");
}

// -------------------------------------------------------------- helpers

/// Private-signer bridge for the raw partial-write test: rebuild the
/// SigV4 signer using the transport's public client internals.
fn private_signer() -> RawSigner {
    RawSigner {
        access_key: access_key(),
        secret_key: secret_key(),
    }
}

struct RawSigner {
    access_key: String,
    secret_key: String,
}

impl RawSigner {
    fn sign(
        &self,
        method: &str,
        host: &str,
        path: &str,
        query: &str,
        payload_hash: &str,
        now: u64,
    ) -> (String, String) {
        use sha2::{Digest, Sha256};
        fn h256(bytes: &[u8]) -> String {
            let mut h = Sha256::new();
            h.update(bytes);
            h.finalize().iter().map(|b| format!("{b:02x}")).collect()
        }
        fn hmac(key: &[u8], msg: &[u8]) -> Vec<u8> {
            const BLOCK: usize = 64;
            let mut key = key.to_vec();
            if key.len() > BLOCK {
                key = h256(&key).into_bytes();
            }
            key.resize(BLOCK, 0);
            let mut ipad = vec![0x36u8; BLOCK];
            let mut opad = vec![0x5cu8; BLOCK];
            for i in 0..BLOCK {
                ipad[i] ^= key[i];
                opad[i] ^= key[i];
            }
            let mut inner = Sha256::new();
            inner.update(&ipad);
            inner.update(msg);
            let d = inner.finalize();
            let mut outer = Sha256::new();
            outer.update(&opad);
            outer.update(d);
            outer.finalize().to_vec()
        }
        fn civil_from_days(z: i64) -> (i64, i64, i64) {
            let z = z + 719468;
            let era = if z >= 0 { z } else { z - 146096 } / 146097;
            let doe = z - era * 146097;
            let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
            let y = yoe + era * 400;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let d = doy - (153 * mp + 2) / 5 + 1;
            let m = if mp < 10 { mp + 3 } else { mp - 9 };
            (if m <= 2 { y + 1 } else { y }, m, d)
        }
        let days = now / 86400;
        let (y, m, d) = civil_from_days(days as i64);
        let date_stamp = format!("{y:04}{m:02}{d:02}");
        let secs = now % 86400;
        let amz_date = format!(
            "{date_stamp}T{:02}{:02}{:02}Z",
            secs / 3600,
            (secs % 3600) / 60,
            secs % 60
        );
        let region = "us-east-1";
        let canonical_headers =
            format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "{method}\n{path}\n{query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
        );
        let scope = format!("{date_stamp}/{region}/s3/aws4_request");
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
            h256(canonical_request.as_bytes())
        );
        let k_date = hmac(
            format!("AWS4{}", self.secret_key).as_bytes(),
            date_stamp.as_bytes(),
        );
        let k_region = hmac(&k_date, region.as_bytes());
        let k_service = hmac(&k_region, b"s3");
        let k_signing = hmac(&k_service, b"aws4_request");
        let signature = hmac(&k_signing, string_to_sign.as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let auth = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.access_key, scope, signed_headers, signature
        );
        (auth, amz_date)
    }
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
