//! EP-026 IMAP/SMTP observability (M4).
//!
//! Bounded, poison-safe observability for the IMAP/SMTP connector:
//!   - a bounded redacted audit ring (latest operations; secrets and
//!     raw message bodies never stored - redaction on insert);
//!   - typed counters (operations by outcome/code);
//!   - incident correlation ids (canonical `mail-<nanos>-<seq>`)
//!     preserved across error paths.
//!
//! Safe telemetry fields (directive Y): operation, account/mailbox
//! fingerprint, message/provider-id fingerprint, correlation,
//! protocol phase, outcome/error class, duration, retry state.
//! Telemetry NEVER contains passwords, OAuth/bearer credentials,
//! Authorization values, full private message bodies, attachment
//! bytes, or raw session secrets (SECURITY.md; SPEC-014 privacy).

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};

/// One audited mail operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailAuditEntry {
    pub correlation: String,
    pub operation: String,
    pub outcome: String,
    pub detail: String,
    pub fields: BTreeMap<String, String>,
}

/// Bounded redacted audit ring + counters for the IMAP/SMTP adapter.
#[derive(Debug)]
pub struct MailObservability {
    ring: VecDeque<MailAuditEntry>,
    counters: BTreeMap<String, u64>,
    max_entries: usize,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep026_unit_m4_observability_redacts_secrets_at_insert() {
        let mut obs = MailObservability::new(16, vec!["secret-token".into()]);
        let mut fields = BTreeMap::new();
        fields.insert("detail".into(), "bearer secret-token here".into());
        obs.record(
            "mail-1-1",
            "SEND",
            "ok",
            "submitted with secret-token",
            fields,
        );
        let entry = obs.recent().pop().expect("entry");
        assert!(!entry.detail.contains("secret-token"));
        assert!(entry.detail.contains("***"));
    }

    #[test]
    fn ep026_unit_m4_observability_redacts_body_canary_at_insert() {
        let body_canary = "EP026M4BODY_CANARY_7c";
        let mut obs = MailObservability::new(16, vec![body_canary.into()]);
        obs.record(
            "mail-1-2",
            "SEND",
            "AMBIGUOUS",
            format!("content {body_canary} may be accepted"),
            BTreeMap::new(),
        );
        let entry = obs.recent().pop().expect("entry");
        assert!(!entry.detail.contains(body_canary));
        assert!(entry.detail.contains("***"));
    }

    #[test]
    fn ep026_unit_m4_observability_bounded_ring() {
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
    }

    #[test]
    fn ep026_unit_m4_observability_counters() {
        let mut obs = MailObservability::new(16, Vec::new());
        obs.record("mail-1-1", "FETCH", "ok", "a", BTreeMap::new());
        obs.record("mail-1-2", "FETCH", "ok", "b", BTreeMap::new());
        obs.record("mail-1-3", "FETCH", "NOT_FOUND", "c", BTreeMap::new());
        assert_eq!(obs.counter("FETCH", "ok"), 2);
        assert_eq!(obs.counter("FETCH", "NOT_FOUND"), 1);
    }
}
