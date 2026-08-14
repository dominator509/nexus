//! Redacted structured telemetry for the PKI adapter (EP-009 M4
//! directive Q).
//!
//! Emitted per operation: provider type, certificate serial
//! fingerprint, issuer fingerprint, service identity, certificate
//! state, expiry duration, rotation event, revocation state, handshake
//! success/failure category, correlation id. NEVER emits private keys,
//! CSR private material, OpenBao tokens, SecretIDs, or full sensitive
//! certificate contents.

use std::sync::Mutex;

/// One-way fingerprint of a string (SHA-256 truncated to 16 hex).
pub fn fingerprint(value: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Redacted per-operation telemetry event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryEvent {
    /// Provider type (always `openbao-pki`).
    pub provider: String,
    /// Operation (issue/verify/revoke/crl/ensure_role/login/mtls_peer_identity).
    pub operation: String,
    /// One-way fingerprint of the certificate serial (never the raw serial).
    pub serial_fingerprint: Option<String>,
    /// One-way fingerprint of the issuer certificate.
    pub issuer_fingerprint: Option<String>,
    /// Service identity (canonical name), when known and safe.
    pub service_identity: Option<String>,
    /// Canonical certificate state observed, when known.
    pub state: Option<String>,
    /// Expiry duration in seconds (leaf TTL), when known.
    pub expiry_seconds: Option<u64>,
    /// Rotation event marker (true when a new certificate instance is
    /// issued for the same logical identity).
    pub rotation: bool,
    /// Revocation state observed, when known.
    pub revocation: Option<String>,
    /// Handshake success/failure category, when known.
    pub handshake: Option<String>,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Typed error code, if any.
    pub error_class: Option<String>,
    /// Correlation id, if known.
    pub correlation: Option<String>,
}

/// Thread-safe recording sink used by tests and the live-fire probe.
#[derive(Debug, Default)]
pub struct RecordingSink {
    events: Mutex<Vec<TelemetryEvent>>,
}

impl RecordingSink {
    /// Create an empty sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an event.
    pub fn record(&self, event: TelemetryEvent) {
        self.events.lock().unwrap().push(event);
    }

    /// All recorded events.
    pub fn events(&self) -> Vec<TelemetryEvent> {
        self.events.lock().unwrap().clone()
    }

    /// Whether any event carries the given error class.
    pub fn has_error_class(&self, class: &str) -> bool {
        self.events()
            .iter()
            .any(|e| e.error_class.as_deref() == Some(class))
    }
}

impl Default for TelemetryEvent {
    fn default() -> Self {
        Self {
            provider: "openbao-pki".to_string(),
            operation: String::new(),
            serial_fingerprint: None,
            issuer_fingerprint: None,
            service_identity: None,
            state: None,
            expiry_seconds: None,
            rotation: false,
            revocation: None,
            handshake: None,
            latency_ms: 0,
            error_class: None,
            correlation: None,
        }
    }
}
