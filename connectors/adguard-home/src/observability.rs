//! EP-030 AdGuard Home observability (M4).
//!
//! Bounded, poison-safe observability for the AdGuard Home connector:
//!   - a bounded redacted audit ring (latest operations; credentials
//!     and private payloads never stored - redaction on insert);
//!   - typed counters (operations by outcome/code);
//!   - incident correlation ids (canonical `sentinel-<nanos>-<seq>`)
//!     preserved across error paths.
//!
//! Telemetry NEVER contains usernames, passwords, session ids,
//! prompts, or private content (SECURITY.md; SPEC-013 security and
//! privacy). Safe telemetry: tenant/device fingerprints, operation,
//! state, error class, correlation id.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

/// One audited AdGuard Home operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SentinelAuditEntry {
    /// Canonical correlation id for the operation.
    pub correlation: String,
    /// Canonical operation name (e.g. "READ_TELEMETRY",
    /// "READ_BLOCKLIST", "READ_STATUS").
    pub operation: String,
    /// Canonical outcome: ok | <ERROR_CODE>.
    pub outcome: String,
    /// Redacted detail (secrets replaced with *** before storing).
    pub detail: String,
    /// Structured SAFE fields: device fingerprint, error class. Never
    /// raw content or credentials.
    pub fields: BTreeMap<String, String>,
}

/// Bounded redacted audit ring + counters for the AdGuard Home
/// adapter.
///
/// `max_entries` bounds memory (default 256). `detail` is redacted at
/// insert: any occurrence of a configured secret is replaced with
/// `***` BEFORE storage, so the ring is safe to dump even after a
/// poisoned error.
#[derive(Debug)]
pub struct SentinelObservability {
    ring: VecDeque<SentinelAuditEntry>,
    counters: BTreeMap<String, u64>,
    max_entries: usize,
    secrets: Vec<String>,
    seq: AtomicU64,
}

impl Default for SentinelObservability {
    fn default() -> Self {
        Self::new(256, Vec::new())
    }
}

impl SentinelObservability {
    pub fn new(max_entries: usize, secrets: Vec<String>) -> Self {
        Self {
            ring: VecDeque::new(),
            counters: BTreeMap::new(),
            max_entries,
            secrets,
            seq: AtomicU64::new(0),
        }
    }

    /// Mint the next canonical correlation id
    /// (`sentinel-<nanos>-<seq>`).
    pub fn next_correlation(&self) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        format!("sentinel-{nanos}-{seq}")
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

    /// Record one operation (bounded; oldest dropped beyond max).
    pub fn record(&mut self, entry: SentinelAuditEntry) {
        let mut entry = entry;
        entry.detail = self.redact(&entry.detail);
        for value in entry.fields.values_mut() {
            *value = self.redact(value);
        }
        *self
            .counters
            .entry(format!("op:{}", entry.operation))
            .or_insert(0) += 1;
        *self
            .counters
            .entry(format!("outcome:{}", entry.outcome))
            .or_insert(0) += 1;
        if self.ring.len() >= self.max_entries {
            self.ring.pop_front();
        }
        self.ring.push_back(entry);
    }

    /// Snapshot of the audit ring (oldest first).
    pub fn audit(&self) -> Vec<SentinelAuditEntry> {
        self.ring.iter().cloned().collect()
    }

    /// Counter value for a key (e.g. "outcome:UNAVAILABLE").
    pub fn counter(&self, key: &str) -> u64 {
        self.counters.get(key).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep030_unit_observability_redacts_secrets_at_insert() {
        let mut obs = SentinelObservability::new(256, vec!["sekret-pass".to_string()]);
        obs.record(SentinelAuditEntry {
            correlation: "sentinel-1-0".into(),
            operation: "READ_TELEMETRY".into(),
            outcome: "ok".into(),
            detail: "read with sekret-pass".into(),
            fields: BTreeMap::from([("auth".into(), "sekret-pass".into())]),
        });
        let audit = obs.audit();
        assert_eq!(audit.len(), 1);
        assert!(!audit[0].detail.contains("sekret-pass"));
        assert!(audit[0].detail.contains("***"));
        assert!(!audit[0].fields["auth"].contains("sekret-pass"));
        assert_eq!(obs.counter("op:READ_TELEMETRY"), 1);
        assert_eq!(obs.counter("outcome:ok"), 1);
    }

    #[test]
    fn ep030_unit_observability_ring_is_bounded() {
        let mut obs = SentinelObservability::new(4, Vec::new());
        for i in 0..10 {
            obs.record(SentinelAuditEntry {
                correlation: format!("sentinel-{i}-0"),
                operation: "READ_STATUS".into(),
                outcome: "ok".into(),
                detail: String::new(),
                fields: BTreeMap::new(),
            });
        }
        let audit = obs.audit();
        assert_eq!(audit.len(), 4);
        assert!(audit.iter().any(|e| e.correlation == "sentinel-9-0"));
        assert!(!audit.iter().any(|e| e.correlation == "sentinel-0-0"));
    }

    #[test]
    fn ep030_unit_observability_correlation_is_unique_and_canonical() {
        let obs = SentinelObservability::new(256, Vec::new());
        let a = obs.next_correlation();
        let b = obs.next_correlation();
        assert!(a.starts_with("sentinel-"));
        assert!(b.starts_with("sentinel-"));
        assert_ne!(a, b);
    }
}
