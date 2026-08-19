//! EP-029 Postiz observability (M2).
//!
//! Bounded, poison-safe observability for the Postiz connector:
//!   - a bounded redacted audit ring (latest operations; credentials
//!     and private payloads never stored - redaction on insert);
//!   - typed counters (operations by outcome/code);
//!   - incident correlation ids (canonical `postiz-<nanos>-<seq>`)
//!     preserved across error paths.
//!
//! Telemetry NEVER contains API keys, OAuth tokens, post content,
//! prompts, or private content (SECURITY.md; SPEC-015 privacy). Safe
//! telemetry: business/binding fingerprints, operation, state, error
//! class, correlation id.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

/// One audited Postiz operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocialAuditEntry {
    /// Canonical correlation id for the operation.
    pub correlation: String,
    /// Canonical operation name (e.g. "PUBLISH_VARIANT", "REPLY",
    /// "EXECUTE_GOVERNED", "LIST_METRICS").
    pub operation: String,
    /// Canonical outcome: ok | <ERROR_CODE>.
    pub outcome: String,
    /// Redacted detail (secrets replaced with *** before storing).
    pub detail: String,
    /// Structured SAFE fields: business fingerprint, action kind,
    /// error class. Never raw content or credentials.
    pub fields: BTreeMap<String, String>,
}

/// Bounded redacted audit ring + counters for the Postiz adapter.
///
/// `max_entries` bounds memory (default 256). `detail` is redacted at
/// insert: any occurrence of a configured secret is replaced with
/// `***` BEFORE storage, so the ring is safe to dump even after a
/// poisoned error.
#[derive(Debug)]
pub struct SocialObservability {
    ring: VecDeque<SocialAuditEntry>,
    counters: BTreeMap<String, u64>,
    max_entries: usize,
    secrets: Vec<String>,
    seq: AtomicU64,
}

impl Default for SocialObservability {
    fn default() -> Self {
        Self::new(256, Vec::new())
    }
}

impl SocialObservability {
    pub fn new(max_entries: usize, secrets: Vec<String>) -> Self {
        Self {
            ring: VecDeque::new(),
            counters: BTreeMap::new(),
            max_entries,
            secrets,
            seq: AtomicU64::new(0),
        }
    }

    /// Mint the next canonical correlation id (`postiz-<nanos>-<seq>`).
    pub fn next_correlation(&self) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        format!("postiz-{nanos}-{seq}")
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
    pub fn record(&mut self, entry: SocialAuditEntry) {
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
    pub fn audit(&self) -> Vec<SocialAuditEntry> {
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
    fn ep029_unit_observability_redacts_secrets_at_insert() {
        let mut obs = SocialObservability::new(256, vec!["sekret-token".to_string()]);
        obs.record(SocialAuditEntry {
            correlation: "postiz-1-0".into(),
            operation: "PUBLISH_VARIANT".into(),
            outcome: "ok".into(),
            detail: "posted with sekret-token".into(),
            fields: BTreeMap::from([("auth".into(), "sekret-token".into())]),
        });
        let audit = obs.audit();
        assert_eq!(audit.len(), 1);
        assert!(!audit[0].detail.contains("sekret-token"));
        assert!(audit[0].detail.contains("***"));
        assert!(!audit[0].fields["auth"].contains("sekret-token"));
        assert_eq!(obs.counter("op:PUBLISH_VARIANT"), 1);
        assert_eq!(obs.counter("outcome:ok"), 1);
    }

    #[test]
    fn ep029_unit_observability_ring_is_bounded() {
        let mut obs = SocialObservability::new(4, Vec::new());
        for i in 0..10 {
            obs.record(SocialAuditEntry {
                correlation: format!("postiz-{i}-0"),
                operation: "LIST_METRICS".into(),
                outcome: "ok".into(),
                detail: String::new(),
                fields: BTreeMap::new(),
            });
        }
        let audit = obs.audit();
        assert_eq!(audit.len(), 4);
        // Oldest entries were dropped; the newest is present.
        assert!(audit.iter().any(|e| e.correlation == "postiz-9-0"));
        assert!(!audit.iter().any(|e| e.correlation == "postiz-0-0"));
    }

    #[test]
    fn ep029_unit_observability_correlation_is_unique_and_canonical() {
        let obs = SocialObservability::new(256, Vec::new());
        let a = obs.next_correlation();
        let b = obs.next_correlation();
        assert!(a.starts_with("postiz-"));
        assert!(b.starts_with("postiz-"));
        assert_ne!(a, b);
    }
}
