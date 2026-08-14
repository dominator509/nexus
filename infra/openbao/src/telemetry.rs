//! Redacted structured telemetry for the OpenBao adapter (EP-009 M2
//! directive P).
//!
//! Emitted per operation: provider type, secret-reference fingerprint,
//! operation type, version/lease metadata where safe, latency, typed
//! result/error, correlation id. NEVER emits secret values, client
//! tokens, SecretIDs, wrapping tokens, age identities, or decrypted
//! SOPS documents.

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
    /// Provider type (always `openbao`).
    pub provider: String,
    /// One-way fingerprint of the secret reference.
    pub reference_fingerprint: String,
    /// Operation (get/put/rotate/revoke/state/wrap/unwrap/login).
    pub operation: String,
    /// Canonical secret state observed, when known.
    pub state: Option<String>,
    /// Version observed, when known.
    pub version: Option<String>,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Typed error code, if any.
    pub error_class: Option<String>,
    /// Correlation id, if known.
    pub correlation: Option<String>,
    /// Whether a wrapping token was consumed (wrap operations).
    pub wrapping: bool,
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
            provider: "openbao".to_string(),
            reference_fingerprint: String::new(),
            operation: String::new(),
            state: None,
            version: None,
            latency_ms: 0,
            error_class: None,
            correlation: None,
            wrapping: false,
        }
    }
}
