//! EP-028 Hydra observability (M2).
//!
//! Bounded, poison-safe observability for the Hydra connector:
//!   - a bounded redacted audit ring (latest operations; credentials
//!     and private payloads never stored - redaction on insert);
//!   - typed counters (operations by outcome/code);
//!   - incident correlation ids (canonical `hydra-<nanos>-<seq>`)
//!     preserved across error paths.
//!
//! Telemetry NEVER contains bearer credentials, raw CRM records,
//! prompts, or private content (SECURITY.md; SPEC-015 privacy). Safe
//! telemetry: business/binding fingerprints, operation, state, error
//! class, correlation id.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

/// One audited Hydra operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydraAuditEntry {
    /// Canonical correlation id for the operation.
    pub correlation: String,
    /// Canonical operation name (e.g. "READ_CONTEXT", "SUBMIT_ACTION").
    pub operation: String,
    /// Canonical outcome: ok | <ERROR_CODE>.
    pub outcome: String,
    /// Redacted detail (secrets replaced with *** before storing).
    pub detail: String,
    /// Structured SAFE fields: business/binding fingerprint, state,
    /// error class. Never raw content or credentials.
    pub fields: BTreeMap<String, String>,
}

/// Bounded redacted audit ring + counters for the Hydra adapter.
///
/// `max_entries` bounds memory (default 256). `detail` is redacted at
/// insert: any occurrence of a configured secret is replaced with
/// `***` BEFORE storage, so the ring is safe to dump even after a
/// poisoned error.
#[derive(Debug)]
pub struct HydraObservability {
    ring: VecDeque<HydraAuditEntry>,
    counters: BTreeMap<String, u64>,
    max_entries: usize,
    secrets: Vec<String>,
    seq: AtomicU64,
}

impl Default for HydraObservability {
    fn default() -> Self {
        Self::new(256, Vec::new())
    }
}

impl HydraObservability {
    pub fn new(max_entries: usize, secrets: Vec<String>) -> Self {
        Self {
            ring: VecDeque::new(),
            counters: BTreeMap::new(),
            max_entries,
            secrets,
            seq: AtomicU64::new(0),
        }
    }

    /// Mint the next canonical correlation id (`hydra-<nanos>-<seq>`).
    pub fn next_correlation(&self) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        format!("hydra-{nanos}-{seq}")
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

    /// Record one operation. `detail` and every `fields` value are
    /// redacted at insert (poison-safe).
    pub fn record(&mut self, entry: HydraAuditEntry) {
        let entry = HydraAuditEntry {
            correlation: self.redact(&entry.correlation),
            detail: self.redact(&entry.detail),
            fields: entry
                .fields
                .into_iter()
                .map(|(k, v)| (k, self.redact(&v)))
                .collect(),
            ..entry
        };
        *self.counters.entry(entry.outcome.clone()).or_insert(0) += 1;
        if self.ring.len() >= self.max_entries {
            self.ring.pop_front();
        }
        self.ring.push_back(entry);
    }

    pub fn audit(&self) -> Vec<HydraAuditEntry> {
        self.ring.iter().cloned().collect()
    }

    pub fn counter(&self, key: &str) -> u64 {
        self.counters.get(key).copied().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep028_unit_observability_redacts_secret_at_insert() {
        let mut obs = HydraObservability::new(8, vec!["hydra-secret-token".into()]);
        obs.record(HydraAuditEntry {
            correlation: "c1".into(),
            operation: "SUBMIT_ACTION".into(),
            outcome: "ok".into(),
            detail: "credential hydra-secret-token embedded".into(),
            fields: BTreeMap::from([("token".into(), "hydra-secret-token".into())]),
        });
        let audit = obs.audit();
        assert_eq!(audit.len(), 1);
        assert!(!audit[0].detail.contains("hydra-secret-token"));
        assert!(audit[0].detail.contains("***"));
        assert!(!audit[0].fields["token"].contains("hydra-secret-token"));
        assert_eq!(audit[0].fields["token"], "***");
    }

    #[test]
    fn ep028_unit_observability_bounded_ring_and_counters() {
        let mut obs = HydraObservability::new(3, Vec::new());
        for i in 0..5 {
            obs.record(HydraAuditEntry {
                correlation: format!("c{i}"),
                operation: "READ_CONTEXT".into(),
                outcome: if i % 2 == 0 {
                    "ok".into()
                } else {
                    "UNAVAILABLE".into()
                },
                detail: format!("op {i}"),
                fields: BTreeMap::new(),
            });
        }
        assert_eq!(obs.audit().len(), 3);
        assert_eq!(obs.counter("ok"), 3);
        assert_eq!(obs.counter("UNAVAILABLE"), 2);
    }

    #[test]
    fn ep028_unit_observability_correlation_ids_unique() {
        let obs = HydraObservability::new(8, Vec::new());
        let a = obs.next_correlation();
        let b = obs.next_correlation();
        assert_ne!(a, b);
        assert!(a.starts_with("hydra-"));
    }
}
