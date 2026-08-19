//! EP-026 Gmail observability (M2).
//!
//! Bounded, poison-safe observability for the Gmail connector:
//!   - a bounded redacted audit ring (latest operations; secrets and
//!     raw message bodies never stored - redaction on insert);
//!   - typed counters (operations by outcome/code);
//!   - incident correlation ids (canonical `mail-<nanos>-<seq>`)
//!     preserved across error paths.
//!
//! Telemetry NEVER contains raw message bodies, OAuth tokens, bearer
//! credentials, attachment artifacts, or private content (SECURITY.md;
//! SPEC-014 privacy). Safe telemetry: message/thread fingerprints,
//! mailbox, direction, state, size, failure class, correlation id.

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

/// One audited mail operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailAuditEntry {
    /// Canonical correlation id for the operation.
    pub correlation: String,
    /// Canonical operation name (e.g. "FETCH", "SEND", "ARCHIVE").
    pub operation: String,
    /// Canonical outcome: ok | <ERROR_CODE>.
    pub outcome: String,
    /// Redacted detail (secrets replaced with *** before storing).
    pub detail: String,
    /// Structured SAFE fields: message/thread fingerprint, mailbox,
    /// direction, state, size, error class. Never raw content or
    /// credentials. Redacted at insert like detail.
    pub fields: BTreeMap<String, String>,
}

/// Bounded redacted audit ring + counters for the Gmail adapter.
///
/// `max_entries` bounds memory (default 256). `detail` is redacted at
/// insert: any occurrence of the configured secret is replaced with
/// `***` BEFORE storage, so the ring is safe to dump even after a
/// poisoned error.
#[derive(Debug)]
pub struct MailObservability {
    ring: VecDeque<MailAuditEntry>,
    counters: BTreeMap<String, u64>,
    max_entries: usize,
    /// Secrets that must never appear in stored detail (redacted on
    /// insert). Empty when no secret was configured.
    secrets: Vec<String>,
    seq: AtomicU64,
}

impl Default for MailObservability {
    fn default() -> Self {
        Self::new(256, Vec::new())
    }
}

impl MailObservability {
    pub fn new(max_entries: usize, secrets: Vec<String>) -> Self {
        Self {
            ring: VecDeque::with_capacity(max_entries),
            counters: BTreeMap::new(),
            max_entries,
            secrets,
            seq: AtomicU64::new(0),
        }
    }

    /// Mint the next canonical correlation id: `mail-<nanos>-<seq>`.
    pub fn correlation(&self) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        format!("mail-{nanos}-{seq}")
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
        self.ring.push_back(MailAuditEntry {
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

    pub fn recent(&self) -> Vec<MailAuditEntry> {
        self.ring.iter().cloned().collect()
    }

    pub fn counter(&self, operation: &str, outcome: &str) -> u64 {
        self.counters
            .get(&format!("{operation}:{outcome}"))
            .copied()
            .unwrap_or(0)
    }

    pub fn counters(&self) -> &BTreeMap<String, u64> {
        &self.counters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep026_unit_observability_redacts_secrets_at_insert() {
        let mut obs = MailObservability::new(16, vec!["secret-token".into()]);
        let mut fields = BTreeMap::new();
        fields.insert("detail".into(), "bearer secret-token here".into());
        obs.record(
            "mail-1-1",
            "FETCH",
            "ok",
            "fetched with secret-token",
            fields,
        );
        let entry = obs.recent().pop().expect("entry");
        assert!(!entry.detail.contains("secret-token"));
        assert!(entry.detail.contains("***"));
        assert!(!entry
            .fields
            .get("detail")
            .expect("field")
            .contains("secret-token"));
    }

    #[test]
    fn ep026_unit_observability_bounded_ring() {
        let mut obs = MailObservability::new(4, Vec::new());
        for i in 0..10 {
            obs.record(
                format!("mail-{i}-1"),
                "FETCH",
                "ok",
                format!("op {i}"),
                BTreeMap::new(),
            );
        }
        assert_eq!(obs.recent().len(), 4);
        assert!(obs
            .recent()
            .iter()
            .all(|e| e.correlation.starts_with("mail-6")
                || e.correlation.starts_with("mail-7")
                || e.correlation.starts_with("mail-8")
                || e.correlation.starts_with("mail-9")));
    }

    #[test]
    fn ep026_unit_observability_counters() {
        let mut obs = MailObservability::new(16, Vec::new());
        obs.record("mail-1-1", "FETCH", "ok", "a", BTreeMap::new());
        obs.record("mail-1-2", "FETCH", "ok", "b", BTreeMap::new());
        obs.record("mail-1-3", "FETCH", "NOT_FOUND", "c", BTreeMap::new());
        assert_eq!(obs.counter("FETCH", "ok"), 2);
        assert_eq!(obs.counter("FETCH", "NOT_FOUND"), 1);
        assert_eq!(obs.counter("SEND", "ok"), 0);
    }
}
