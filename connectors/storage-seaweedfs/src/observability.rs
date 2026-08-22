//! EP-037 M4 bounded storage observability (SPEC-006 telemetry, SPEC-024
//! artifact storage; directive: safe fields only).
//!
//! Every operation records a bounded observation: operation, provider,
//! artifact hash, size, correlation, duration, result/error class,
//! encryption-applied flag, integrity verification result, and an
//! optional backup/migration fingerprint. NEVER recorded: plaintext
//! sensitive metadata, encryption keys, credentials, raw cloud secrets,
//! or artifact payload content.

use std::time::{Duration, Instant};

/// One bounded storage observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageObservation {
    /// Canonical operation (put, get, verify, delete, create_backup,
    /// restore, migrate, list, set_retention, diag_probe).
    pub operation: String,
    /// Provider identity ("seaweedfs:s3-gateway").
    pub provider: String,
    /// Content hash (artifact identity) when known.
    pub artifact_hash: Option<String>,
    /// Size in bytes when known (never payload content).
    pub size_bytes: Option<u64>,
    /// Correlation id when provided.
    pub correlation: Option<String>,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// "ok" or the canonical error class (code debug name).
    pub result: String,
    /// Encryption-applied boolean (true only when the artifact carries
    /// encryption metadata; the key reference is never recorded).
    pub encryption_applied: bool,
    /// Integrity verification result for read/verify/restore/migrate.
    pub integrity_verified: bool,
    /// Backup/migration identity fingerprint when applicable.
    pub fingerprint: Option<String>,
}

/// Optional sink for observations. Production callers may wire this to
/// the canonical metrics/traces pipeline; tests use it to assert that
/// redaction holds and verification results are recorded.
pub trait ObservationSink {
    fn observe(&mut self, obs: StorageObservation);
}

/// No-op sink (default; avoids allocation in hot paths).
#[derive(Debug, Clone, Default)]
pub struct NullSink;

impl ObservationSink for NullSink {
    fn observe(&mut self, _obs: StorageObservation) {}
}

/// Collects observations in memory (test/diagnostic use). Clone shares
/// the same collection so tests can hand one handle to the adapter and
/// read observations from the other.
#[derive(Debug, Clone, Default)]
pub struct VecSink(pub std::sync::Arc<std::sync::Mutex<Vec<StorageObservation>>>);

impl VecSink {
    pub fn new() -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(Vec::new())))
    }
}

impl ObservationSink for VecSink {
    fn observe(&mut self, obs: StorageObservation) {
        self.0.lock().unwrap().push(obs);
    }
}

/// Helper to time and record one operation observation.
#[allow(clippy::too_many_arguments)] // explicit observation fields; same pattern as nexus-hydra/nexus-compute
pub(crate) fn record<S: ObservationSink + ?Sized>(
    sink: &mut S,
    operation: &str,
    artifact_hash: Option<String>,
    size_bytes: Option<u64>,
    correlation: Option<String>,
    encryption_applied: bool,
    fingerprint: Option<String>,
    started: Instant,
    result: Result<(), &str>,
    integrity_verified: bool,
) {
    let obs = StorageObservation {
        operation: operation.to_string(),
        provider: "seaweedfs:s3-gateway".to_string(),
        artifact_hash,
        size_bytes,
        correlation,
        duration_ms: started.elapsed().as_millis() as u64,
        result: match result {
            Ok(()) => "ok".to_string(),
            Err(class) => class.to_string(),
        },
        encryption_applied,
        integrity_verified,
        fingerprint,
    };
    sink.observe(obs);
}

/// Convenience for measuring elapsed time.
pub(crate) fn started() -> Instant {
    Instant::now()
}

/// Duration helper (kept for API stability; used by tests).
pub fn as_ms(d: Duration) -> u64 {
    d.as_millis() as u64
}
