//! Structured redacted observability (directive W).
//!
//! The sidecar emits JSON-lines telemetry to its telemetry sink. Every
//! event captures connector fingerprint, capability id, class,
//! transport, result/error class, latency, correlation id, tenant
//! fingerprint, and lifecycle. Full request bodies, credentials, and
//! raw tenant ids never appear.

use serde_json::json;
use std::io::Write;
use std::sync::{Arc, Mutex};

/// Canonical sidecar lifecycle/telemetry event classes (directive W).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelemetryEvent {
    SidecarStarted,
    SidecarReady,
    RequestAccepted,
    RequestRejected,
    ProviderTimeout,
    ProviderExited,
    ProviderMalformedResponse,
    CredentialBrokerDenied,
    WebhookRejected,
    SidecarStopped,
    DispatchCompleted,
    PollerRejected,
}

impl TelemetryEvent {
    /// Canonical wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SidecarStarted => "SIDECAR_STARTED",
            Self::SidecarReady => "SIDECAR_READY",
            Self::RequestAccepted => "REQUEST_ACCEPTED",
            Self::RequestRejected => "REQUEST_REJECTED",
            Self::ProviderTimeout => "PROVIDER_TIMEOUT",
            Self::ProviderExited => "PROVIDER_EXITED",
            Self::ProviderMalformedResponse => "PROVIDER_MALFORMED_RESPONSE",
            Self::CredentialBrokerDenied => "CREDENTIAL_BROKER_DENIED",
            Self::WebhookRejected => "WEBHOOK_REJECTED",
            Self::SidecarStopped => "SIDECAR_STOPPED",
            Self::DispatchCompleted => "DISPATCH_COMPLETED",
            Self::PollerRejected => "POLLER_REJECTED",
        }
    }
}

/// A redacted telemetry entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryEntry {
    /// Event class.
    pub event: TelemetryEvent,
    /// Connector fingerprint (sha256 prefix), not the raw id.
    pub connector_fingerprint: Option<String>,
    /// Capability id (metadata, never a secret).
    pub capability_id: Option<String>,
    /// Capability class (canonical SCREAMING_SNAKE).
    pub class: Option<String>,
    /// Transport family (canonical).
    pub transport: Option<String>,
    /// Result/error class (canonical SDK code).
    pub result_class: Option<String>,
    /// Latency in milliseconds.
    pub latency_ms: Option<u64>,
    /// Correlation id (validated; never a log-injection vector).
    pub correlation_id: Option<String>,
    /// Tenant fingerprint (sha256 prefix), never the raw tenant id.
    pub tenant_fingerprint: Option<String>,
    /// Lifecycle detail.
    pub detail: Option<String>,
}

/// Compute a short redacted fingerprint (sha256 hex prefix).
pub fn fingerprint(value: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex.chars().take(16).collect()
}

/// JSON-lines telemetry sink (directive W: structured + redacted).
///
/// Cloneable so spawned request handlers share one sink.
pub struct TelemetrySink {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl std::fmt::Debug for TelemetrySink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TelemetrySink")
    }
}

impl Clone for TelemetrySink {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
        }
    }
}

impl TelemetrySink {
    /// Construct a sink writing to the given writer (typically stderr).
    pub fn new(writer: Box<dyn Write + Send>) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    /// A sink writing to stderr (default for the server).
    pub fn stderr() -> Self {
        Self::new(Box::new(std::io::stderr()))
    }

    /// Emit one redacted telemetry entry.
    pub fn emit(&self, entry: &TelemetryEntry) {
        let mut obj = serde_json::Map::new();
        obj.insert("event".to_string(), json!(entry.event.as_str()));
        if let Some(v) = &entry.connector_fingerprint {
            obj.insert("connector_fingerprint".to_string(), json!(v));
        }
        if let Some(v) = &entry.capability_id {
            obj.insert("capability_id".to_string(), json!(v));
        }
        if let Some(v) = &entry.class {
            obj.insert("class".to_string(), json!(v));
        }
        if let Some(v) = &entry.transport {
            obj.insert("transport".to_string(), json!(v));
        }
        if let Some(v) = &entry.result_class {
            obj.insert("result_class".to_string(), json!(v));
        }
        if let Some(v) = &entry.latency_ms {
            obj.insert("latency_ms".to_string(), json!(v));
        }
        if let Some(v) = &entry.correlation_id {
            obj.insert("correlation_id".to_string(), json!(v));
        }
        if let Some(v) = &entry.tenant_fingerprint {
            obj.insert("tenant_fingerprint".to_string(), json!(v));
        }
        if let Some(v) = &entry.detail {
            obj.insert("detail".to_string(), json!(v));
        }
        let line = serde_json::Value::Object(obj).to_string();
        let Ok(mut writer) = self.writer.lock() else {
            return;
        };
        let _ = writeln!(writer, "{line}");
        let _ = writer.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep011_unit_sidecar_telemetry_event_values_are_canonical() {
        assert_eq!(TelemetryEvent::SidecarStarted.as_str(), "SIDECAR_STARTED");
        assert_eq!(TelemetryEvent::RequestRejected.as_str(), "REQUEST_REJECTED");
        assert_eq!(TelemetryEvent::WebhookRejected.as_str(), "WEBHOOK_REJECTED");
        assert_eq!(TelemetryEvent::SidecarStopped.as_str(), "SIDECAR_STOPPED");
    }

    #[test]
    fn ep011_unit_sidecar_fingerprint_is_short_and_stable() {
        let a = fingerprint("018f0f6f-9c1e-7b6e-8000-000000000003");
        let b = fingerprint("018f0f6f-9c1e-7b6e-8000-000000000003");
        let c = fingerprint("018f0f6f-9c1e-7b6e-8000-000000000099");
        assert_eq!(a.len(), 16);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn ep011_unit_sidecar_telemetry_emits_redacted_json_line() {
        let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let inner = buf.clone();
        struct VecWriter(Arc<Mutex<Vec<u8>>>);
        impl Write for VecWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let sink = TelemetrySink::new(Box::new(VecWriter(inner)));
        sink.emit(&TelemetryEntry {
            event: TelemetryEvent::SidecarStarted,
            connector_fingerprint: Some("abc123".to_string()),
            capability_id: None,
            class: None,
            transport: Some("REST".to_string()),
            result_class: None,
            latency_ms: None,
            correlation_id: Some("corr-1".to_string()),
            tenant_fingerprint: Some("def456".to_string()),
            detail: None,
        });
        let bytes = buf.lock().unwrap().clone();
        let line = String::from_utf8(bytes).unwrap();
        assert!(line.contains("\"event\":\"SIDECAR_STARTED\""));
        assert!(line.contains("\"connector_fingerprint\":\"abc123\""));
        assert!(line.contains("\"tenant_fingerprint\":\"def456\""));
        assert!(!line.contains("018f0f6f"));
    }
}
