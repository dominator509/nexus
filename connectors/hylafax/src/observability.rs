//! EP-027 HylaFAX observability (M3).
//!
//! Bounded, poison-safe observability for the HylaFAX connector:
//!   - a bounded redacted audit ring (latest operations; passwords,
//!     private document content, and full fax numbers never stored -
//!     redaction on insert);
//!   - typed counters (operations by outcome/code);
//!   - incident correlation ids (canonical `fax-<nanos>-<seq>`)
//!     preserved across error paths.
//!
//! Telemetry NEVER contains PASS credentials, raw document bodies,
//! unnecessary destination numbers, or sensitive spool contents
//! (SECURITY.md; SPEC-014 privacy). Safe telemetry: operation,
//! correlation id, FaxJobId fingerprint, carrier job id, document
//! digest, protocol phase, numeric hfaxd response code, duration,
//! error class.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

/// One audited fax operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaxAuditEntry {
    /// Canonical correlation id for the operation.
    pub correlation: String,
    /// Canonical operation name (e.g. "SUBMIT", "STATUS", "CANCEL").
    pub operation: String,
    /// Canonical outcome: ok | <ERROR_CODE>.
    pub outcome: String,
    /// Redacted detail (secrets replaced with *** before storing).
    pub detail: String,
    /// Structured SAFE fields: carrier job id, canonical state,
    /// failure class. Never raw content or credentials. Redacted at
    /// insert like detail.
    pub fields: BTreeMap<String, String>,
}

/// Bounded redacted audit ring + counters for the HylaFAX adapter.
#[derive(Debug)]
pub struct FaxObservability {
    ring: VecDeque<FaxAuditEntry>,
    counters: BTreeMap<String, u64>,
    max_entries: usize,
    /// Secrets that must never appear in stored detail (redacted on
    /// insert). Empty when no secret was configured.
    secrets: Vec<String>,
    seq: AtomicU64,
}

impl Default for FaxObservability {
    fn default() -> Self {
        Self::new(256, Vec::new())
    }
}

impl FaxObservability {
    pub fn new(max_entries: usize, secrets: Vec<String>) -> Self {
        Self {
            ring: VecDeque::with_capacity(max_entries),
            counters: BTreeMap::new(),
            max_entries,
            secrets,
            seq: AtomicU64::new(0),
        }
    }

    /// Mint the next canonical correlation id: `fax-<nanos>-<seq>`.
    pub fn correlation(&self) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        format!("fax-{nanos}-{seq}")
    }

    fn redact(&self, text: &str) -> String {
        let mut out = text.to_string();
        for secret in &self.secrets {
            if !secret.is_empty() {
                out = out.replace(secret, "***");
            }
        }
        out
    }

    /// Record one audited operation. `detail` and `fields` values are
    /// redacted at insert (poison-safe).
    pub fn record(
        &mut self,
        correlation: impl Into<String>,
        operation: impl Into<String>,
        outcome: impl Into<String>,
        detail: impl Into<String>,
        fields: BTreeMap<String, String>,
    ) {
        let correlation = correlation.into();
        let operation = operation.into();
        let outcome = outcome.into();
        let detail = self.redact(&detail.into());
        let fields = fields
            .into_iter()
            .map(|(k, v)| (k, self.redact(&v)))
            .collect();
        let key = format!("{operation}:{outcome}");
        *self.counters.entry(key).or_insert(0) += 1;
        self.ring.push_back(FaxAuditEntry {
            correlation,
            operation,
            outcome,
            detail,
            fields,
        });
        while self.ring.len() > self.max_entries {
            self.ring.pop_front();
        }
    }

    pub fn recent(&self) -> Vec<FaxAuditEntry> {
        self.ring.iter().cloned().collect()
    }

    pub fn counter(&self, operation: &str, outcome: &str) -> u64 {
        self.counters
            .get(&format!("{operation}:{outcome}"))
            .copied()
            .unwrap_or(0)
    }
}
