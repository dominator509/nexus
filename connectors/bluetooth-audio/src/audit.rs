//! Structured incident records and metrics for the Bluetooth audio
//! connector (M4 forced-failure observability). Records are redacted,
//! carry incident and correlation ids, and are the audit/trace analog
//! at this layer; OpenTelemetry context wiring is owned by the control
//! plane (EP-044).

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// One structured, redacted incident record.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct IncidentRecord {
    pub incident_id: String,
    pub timestamp_ms: u64,
    pub correlation_id: Option<String>,
    pub code: String,
    pub resource: Option<String>,
    pub message: String,
    pub redacted: bool,
}

/// In-memory incident recorder with drain (audit surface).
#[derive(Debug, Default)]
pub struct IncidentRecorder {
    records: Mutex<Vec<IncidentRecord>>,
    next: AtomicU64,
}

impl IncidentRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an incident with defensive redaction. Returns the record
    /// for callers that want to observe the incident id.
    pub fn record(
        &self,
        code: &str,
        correlation_id: Option<String>,
        resource: Option<String>,
        message: &str,
    ) -> IncidentRecord {
        let safe = redact(message);
        let sequence = self.next.fetch_add(1, Ordering::Relaxed);
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let record = IncidentRecord {
            incident_id: format!("bt-{timestamp_ms:x}-{sequence}"),
            timestamp_ms,
            correlation_id,
            code: code.to_string(),
            resource,
            message: safe.clone(),
            redacted: safe != message,
        };
        self.records.lock().unwrap().push(record.clone());
        record
    }

    pub fn drain(&self) -> Vec<IncidentRecord> {
        std::mem::take(&mut *self.records.lock().unwrap())
    }

    pub fn len(&self) -> usize {
        self.records.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Defensive redaction for incident messages. Raw audio and
/// credentials never enter error payloads by construction; this is a
/// second layer that strips sensitive key/value pairs. Guaranteed to
/// make progress: the search position always advances past any
/// replaced region, so an already-redacted marker is never re-scanned.
pub fn redact(input: &str) -> String {
    let mut out = input.to_string();
    for key in [
        "secret=",
        "password=",
        "token=",
        "api_key=",
        "apikey=",
        "authorization=",
        "Bearer ",
    ] {
        let mut search_from = 0;
        while let Some(idx) = out[search_from..].find(key).map(|i| search_from + i) {
            let start = idx + key.len();
            let end = out[start..]
                .find(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '"'))
                .map(|i| start + i)
                .unwrap_or(out.len());
            if end > start {
                out.replace_range(start..end, "[REDACTED]");
                search_from = start + "[REDACTED]".len();
            } else {
                search_from = idx + key.len();
            }
        }
    }
    out
}

/// Snapshot of connector metrics (real counters, assertable).
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq, Default)]
pub struct MetricsSnapshot {
    pub connect_attempts: u64,
    pub connect_failures: u64,
    pub probe_failures: u64,
    pub policy_denials: u64,
}

/// Real atomic counters for the connector.
#[derive(Debug, Default)]
pub struct Metrics {
    connect_attempts: AtomicU64,
    connect_failures: AtomicU64,
    probe_failures: AtomicU64,
    policy_denials: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn incr_connect_attempts(&self) {
        self.connect_attempts.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_connect_failures(&self) {
        self.connect_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_probe_failures(&self) {
        self.probe_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_policy_denials(&self) {
        self.policy_denials.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            connect_attempts: self.connect_attempts.load(Ordering::Relaxed),
            connect_failures: self.connect_failures.load(Ordering::Relaxed),
            probe_failures: self.probe_failures.load(Ordering::Relaxed),
            policy_denials: self.policy_denials.load(Ordering::Relaxed),
        }
    }
}
