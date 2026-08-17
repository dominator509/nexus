//! EP-023 Frigate provider observability (SPEC-021; M4 content 4).
//!
//! A lightweight, dependency-free metrics and audit surface for the
//! provider adapter. Counters are monotonically increasing; audit
//! records are redacted (secrets never enter the record) and carry a
//! correlation id per operation so failures can be traced across the
//! canonical VisionError surface.
//!
//! This is NOT a Prometheus exporter (EP-045 owns full metrics
//! shipping); it is the adapter-owned observation contract that
//! exporters and operators can consume.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Redacted audit record for one provider operation.
///
/// `operation` is the canonical operation name (e.g. "health",
/// "config", "events", "go2rtc_streams", "latest_frame",
/// "availability", "discovery"). `ok` is the outcome. `detail` carries
/// bounded, REDACTED context (camera names, counts, status codes) -
/// never credentials, tokens, or raw media.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    pub operation: String,
    pub ok: bool,
    pub correlation_id: Option<Box<str>>,
    pub detail: String,
}

/// Adapter observability: operation counters + bounded redacted audit
/// ring.
#[derive(Debug)]
pub struct FrigateObservability {
    /// Total provider operations attempted.
    pub operations_total: AtomicU64,
    /// Operations that failed (any VisionError).
    pub failures_total: AtomicU64,
    /// Operations that timed out (bounded; M4).
    pub timeouts_total: AtomicU64,
    /// Authorization failures (M4 denied permission).
    pub auth_failures_total: AtomicU64,
    /// Malformed provider responses (M4).
    pub malformed_total: AtomicU64,
    audit: Mutex<Vec<AuditRecord>>,
    audit_cap: usize,
}

impl Default for FrigateObservability {
    fn default() -> Self {
        Self::new(256)
    }
}

impl FrigateObservability {
    pub fn new(audit_cap: usize) -> Self {
        Self {
            operations_total: AtomicU64::new(0),
            failures_total: AtomicU64::new(0),
            timeouts_total: AtomicU64::new(0),
            auth_failures_total: AtomicU64::new(0),
            malformed_total: AtomicU64::new(0),
            audit: Mutex::new(Vec::new()),
            audit_cap: audit_cap.max(1),
        }
    }

    /// Record the outcome of one operation. `detail` MUST be pre-redacted
    /// by the caller (adapter redacts before calling).
    pub fn record(
        &self,
        operation: &str,
        ok: bool,
        correlation_id: Option<Box<str>>,
        detail: String,
        error_code: Option<crate::VisionErrorCode>,
    ) {
        self.operations_total.fetch_add(1, Ordering::Relaxed);
        if !ok {
            self.failures_total.fetch_add(1, Ordering::Relaxed);
            match error_code {
                Some(crate::VisionErrorCode::Timeout) => {
                    self.timeouts_total.fetch_add(1, Ordering::Relaxed);
                }
                Some(crate::VisionErrorCode::Authorization) => {
                    self.auth_failures_total.fetch_add(1, Ordering::Relaxed);
                }
                Some(crate::VisionErrorCode::External) => {
                    // Malformed JSON and unexpected provider responses
                    // both surface as External; callers that know the
                    // specific detail may count them precisely.
                }
                _ => {}
            }
        }
        // Poison-safe: a panic while recording telemetry must never
        // turn a provider failure into a crash (directive C: provider
        // result semantics stay authoritative). Recover the guard.
        let mut audit = self
            .audit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if audit.len() >= self.audit_cap {
            audit.remove(0);
        }
        audit.push(AuditRecord {
            operation: operation.to_string(),
            ok,
            correlation_id,
            detail,
        });
    }

    /// Snapshot of recent audit records (redacted).
    pub fn audit(&self) -> Vec<AuditRecord> {
        self.audit
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Snapshot of all counters as a JSON value.
    pub fn metrics(&self) -> serde_json::Value {
        serde_json::json!({
            "operations_total": self.operations_total.load(Ordering::Relaxed),
            "failures_total": self.failures_total.load(Ordering::Relaxed),
            "timeouts_total": self.timeouts_total.load(Ordering::Relaxed),
            "auth_failures_total": self.auth_failures_total.load(Ordering::Relaxed),
            "malformed_total": self.malformed_total.load(Ordering::Relaxed),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VisionErrorCode;

    #[test]
    fn ep023_unit_frigate_observability_metrics_accumulate() {
        let obs = FrigateObservability::default();
        obs.record("health", true, None, "ok".into(), None);
        obs.record(
            "events",
            false,
            Some(Box::from("c1")),
            "timeout".into(),
            Some(VisionErrorCode::Timeout),
        );
        obs.record(
            "config",
            false,
            None,
            "denied".into(),
            Some(VisionErrorCode::Authorization),
        );
        assert_eq!(obs.operations_total.load(Ordering::Relaxed), 3);
        assert_eq!(obs.failures_total.load(Ordering::Relaxed), 2);
        assert_eq!(obs.timeouts_total.load(Ordering::Relaxed), 1);
        assert_eq!(obs.auth_failures_total.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn ep023_unit_frigate_observability_audit_ring_bounded() {
        let obs = FrigateObservability::new(3);
        for i in 0..10 {
            obs.record("health", true, None, format!("op{i}"), None);
        }
        let audit = obs.audit();
        assert_eq!(audit.len(), 3);
        // Ring keeps the most recent records.
        assert!(audit.iter().any(|r| r.detail == "op9"));
        assert!(!audit.iter().any(|r| r.detail == "op0"));
    }
}
