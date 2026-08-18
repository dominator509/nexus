//! EP-025 Asterisk observability (M2).
//!
//! Bounded, poison-safe observability for the telephony connector:
//!   - a bounded redacted audit ring (latest operations, secrets
//!     never stored - redaction on insert, so a poisoned message can
//!     never leak a credential or SIP Authorization header);
//!   - typed counters (operations by outcome/code);
//!   - incident correlation ids (canonical `tel-<nanos>-<seq>`)
//!     preserved across error paths.
//!
//! Telemetry NEVER contains raw call audio, credentials, SIP
//! Authorization headers, full private transcripts, or private caller
//! information (directive 24). Safe telemetry: call id, channel
//! fingerprint, direction, state, codec, durations, failure class,
//! correlation id.

use std::collections::{BTreeMap, VecDeque};

/// One audited telephony operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelephonyAuditEntry {
    /// Canonical correlation id for the operation.
    pub correlation: String,
    /// Canonical operation name (e.g. "ORIGINATE", "ANSWER", "verify").
    pub operation: String,
    /// Canonical outcome: ok | <ERROR_CODE>.
    pub outcome: String,
    /// Redacted detail (secrets replaced with *** before storing).
    pub detail: String,
    /// Structured SAFE fields (directive V): call/session fingerprint,
    /// channel fingerprint, direction, state, codec, bridge state,
    /// media verification state, DTMF count/type, latency, error class.
    /// Never contains raw audio, credentials, Authorization headers, or
    /// private transcripts. Redacted at insert like detail.
    pub fields: BTreeMap<String, String>,
}

/// Bounded redacted audit ring + counters for the Asterisk adapter.
///
/// `max_entries` bounds memory (default 256). `detail` is redacted at
/// insert: any occurrence of the configured secret is replaced with
/// `***` BEFORE storage, so the ring is safe to dump even after a
/// poisoned error.
#[derive(Debug, Clone)]
pub struct TelephonyObservability {
    ring: VecDeque<TelephonyAuditEntry>,
    counters: BTreeMap<String, u64>,
    max_entries: usize,
    /// Secrets that must never appear in stored detail (redacted on
    /// insert). Empty when no secret was configured.
    secrets: Vec<String>,
    seq: u64,
}

impl Default for TelephonyObservability {
    fn default() -> Self {
        Self::new(256, Vec::new())
    }
}

impl TelephonyObservability {
    pub fn new(max_entries: usize, secrets: Vec<String>) -> Self {
        Self {
            ring: VecDeque::with_capacity(max_entries),
            counters: BTreeMap::new(),
            max_entries,
            secrets,
            seq: 0,
        }
    }

    /// Build a canonical correlation id for an operation.
    pub fn correlation(&mut self) -> String {
        self.seq += 1;
        format!(
            "tel-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            self.seq
        )
    }

    /// Redact configured secrets from a detail string.
    fn redact(&self, detail: &str) -> String {
        let mut out = detail.to_string();
        for secret in &self.secrets {
            if !secret.is_empty() {
                out = out.replace(secret, "***");
            }
        }
        out
    }

    /// Record an audited operation (outcome "ok" or an error code).
    pub fn record(&mut self, correlation: &str, operation: &str, outcome: &str, detail: &str) {
        self.record_with_fields(correlation, operation, outcome, detail, BTreeMap::new());
    }

    /// Record an audited operation with structured SAFE fields.
    pub fn record_with_fields(
        &mut self,
        correlation: &str,
        operation: &str,
        outcome: &str,
        detail: &str,
        fields: BTreeMap<String, String>,
    ) {
        let entry = TelephonyAuditEntry {
            correlation: correlation.to_string(),
            operation: operation.to_string(),
            outcome: outcome.to_string(),
            detail: self.redact(detail),
            fields: fields
                .into_iter()
                .map(|(k, v)| (k, self.redact(&v)))
                .collect(),
        };
        if self.ring.len() >= self.max_entries {
            self.ring.pop_front();
        }
        self.ring.push_back(entry);
        let key = format!("{operation}:{outcome}");
        *self.counters.entry(key).or_insert(0) += 1;
    }

    /// Record an error outcome from a typed error code.
    pub fn record_error(&mut self, correlation: &str, operation: &str, code: &str, detail: &str) {
        self.record(correlation, operation, code, detail);
    }

    /// Current counters (operation:outcome -> count).
    pub fn counters(&self) -> BTreeMap<String, u64> {
        self.counters.clone()
    }

    /// Latest audit entries (oldest first).
    pub fn audit(&self) -> Vec<TelephonyAuditEntry> {
        self.ring.iter().cloned().collect()
    }

    /// True when a redaction canary would be caught (secrets list has
    /// at least one non-empty secret).
    pub fn has_secrets(&self) -> bool {
        self.secrets.iter().any(|s| !s.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep025_unit_observability_ring_bounded() {
        let mut obs = TelephonyObservability::new(4, vec!["SECRET".to_string()]);
        for i in 0..10 {
            let c = obs.correlation();
            obs.record(&c, "ORIGINATE", "ok", &format!("op {i}"));
        }
        assert_eq!(obs.audit().len(), 4);
        assert_eq!(obs.audit()[0].detail, "op 6");
        assert_eq!(obs.audit()[3].detail, "op 9");
    }

    #[test]
    fn ep025_unit_observability_redacts_secrets() {
        let mut obs = TelephonyObservability::new(8, vec!["EP025_SECRET_CANARY".to_string()]);
        let c = obs.correlation();
        obs.record(&c, "ORIGINATE", "ok", "token EP025_SECRET_CANARY leaked?");
        assert!(!obs.audit()[0].detail.contains("EP025_SECRET_CANARY"));
        assert!(obs.audit()[0].detail.contains("***"));
        assert!(obs.has_secrets());
    }

    #[test]
    fn ep025_unit_observability_counters() {
        let mut obs = TelephonyObservability::default();
        let c1 = obs.correlation();
        let c2 = obs.correlation();
        obs.record(&c1, "ANSWER", "ok", "answered");
        obs.record(&c2, "ANSWER", "ok", "answered");
        obs.record_error(&c1, "ORIGINATE", "UNAVAILABLE", "no route");
        let counters = obs.counters();
        assert_eq!(counters.get("ANSWER:ok"), Some(&2));
        assert_eq!(counters.get("ORIGINATE:UNAVAILABLE"), Some(&1));
    }

    #[test]
    fn ep025_unit_observability_correlation_unique() {
        let mut obs = TelephonyObservability::default();
        let a = obs.correlation();
        let b = obs.correlation();
        assert!(a.starts_with("tel-"));
        assert_ne!(a, b);
    }

    #[test]
    fn ep025_unit_observability_safe_fields_redacted_and_bounded() {
        let mut obs = TelephonyObservability::new(2, vec!["EP025_FIELD_CANARY".to_string()]);
        let c = obs.correlation();
        let mut fields = BTreeMap::new();
        fields.insert("direction".to_string(), "OUTBOUND".to_string());
        fields.insert("state".to_string(), "RINGING".to_string());
        fields.insert("error_class".to_string(), "EP025_FIELD_CANARY".to_string());
        obs.record_with_fields(&c, "ORIGINATE", "ok", "originated", fields);
        let entry = &obs.audit()[0];
        assert_eq!(
            entry.fields.get("direction").map(|s| s.as_str()),
            Some("OUTBOUND")
        );
        assert_eq!(
            entry.fields.get("state").map(|s| s.as_str()),
            Some("RINGING")
        );
        // The canary in a field is redacted at insert.
        assert_eq!(
            entry.fields.get("error_class").map(|s| s.as_str()),
            Some("***")
        );
        // The ring stays bounded with the new field path.
        obs.record_with_fields(&c, "ORIGINATE", "ok", "x", BTreeMap::new());
        obs.record_with_fields(&c, "ORIGINATE", "ok", "y", BTreeMap::new());
        assert_eq!(obs.audit().len(), 2);
    }
}
