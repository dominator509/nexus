//! EP-024 irrigation observability (SPEC-011; M4 directive 4).
//!
//! Bounded, poison-safe observability for the irrigation connector:
//!   - a bounded redacted audit ring (latest operations, secrets never
//!     stored - the ring redacts on insert, so a poisoned message can
//!     never leak a credential);
//!   - typed counters (operations by outcome/code);
//!   - incident correlation ids (canonical `irrigation-<nanos>-<seq>`)
//!     preserved across error paths.
//!
//! No test-mode branches exist in production code.

use std::collections::{BTreeMap, VecDeque};

/// One audited irrigation operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrrigationAuditEntry {
    /// Canonical correlation id for the operation.
    pub correlation: String,
    /// Canonical operation name (e.g. "zone_on", "verify", "availability").
    pub operation: String,
    /// Canonical outcome: ok | <ERROR_CODE>.
    pub outcome: String,
    /// Redacted detail (secrets replaced with *** before storing).
    pub detail: String,
}

/// Bounded redacted audit ring + counters for the irrigation adapter.
///
/// `max_entries` bounds memory (default 256). `detail` is redacted at
/// insert: any occurrence of the configured secret is replaced with
/// `***` BEFORE storage, so the ring is safe to dump even after a
/// poisoned error.
#[derive(Debug, Clone)]
pub struct IrrigationObservability {
    ring: VecDeque<IrrigationAuditEntry>,
    counters: BTreeMap<String, u64>,
    max_entries: usize,
    /// Secrets that must never appear in stored detail (redacted on
    /// insert). Empty when no secret was configured.
    secrets: Vec<String>,
    seq: u64,
}

impl Default for IrrigationObservability {
    fn default() -> Self {
        Self::new(256, Vec::new())
    }
}

impl IrrigationObservability {
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
            "irrigation-{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
            self.seq
        )
    }

    /// Record an operation. `detail` is redacted BEFORE storage.
    pub fn record(&mut self, correlation: String, operation: &str, outcome: &str, detail: &str) {
        let redacted = self.redact(detail);
        self.ring.push_back(IrrigationAuditEntry {
            correlation,
            operation: operation.to_string(),
            outcome: outcome.to_string(),
            detail: redacted,
        });
        while self.ring.len() > self.max_entries {
            self.ring.pop_front();
        }
        let key = format!("{operation}:{outcome}");
        *self.counters.entry(key).or_insert(0) += 1;
    }

    /// Increment a raw counter (e.g. malformed responses at the
    /// transport boundary).
    pub fn increment(&mut self, key: &str) {
        *self.counters.entry(key.to_string()).or_insert(0) += 1;
    }

    /// The bounded audit ring (oldest first, already redacted).
    pub fn audit(&self) -> Vec<IrrigationAuditEntry> {
        self.ring.iter().cloned().collect()
    }

    /// A copy of the counters.
    pub fn counters(&self) -> BTreeMap<String, u64> {
        self.counters.clone()
    }

    /// Redact configured secrets from a string. Any other string is
    /// returned unchanged.
    pub fn redact(&self, text: &str) -> String {
        let mut out = text.to_string();
        for secret in &self.secrets {
            if !secret.is_empty() {
                out = out.replace(secret, "***");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep024_unit_observability_ring_is_bounded() {
        let mut obs = IrrigationObservability::new(4, Vec::new());
        for i in 0..10 {
            let correlation = obs.correlation();
            obs.record(correlation, "zone_on", "ok", &format!("op {i}"));
        }
        assert_eq!(obs.audit().len(), 4);
        assert_eq!(obs.audit()[0].detail, "op 6");
        assert_eq!(obs.audit()[3].detail, "op 9");
    }

    #[test]
    fn ep024_unit_observability_redacts_secrets_on_insert() {
        let mut obs = IrrigationObservability::new(8, vec!["ep024-secret-token".to_string()]);
        let correlation = obs.correlation();
        obs.record(
            correlation,
            "zone_on",
            "error",
            "failed with ep024-secret-token in the message",
        );
        let entry = &obs.audit()[0];
        assert!(!entry.detail.contains("ep024-secret-token"));
        assert!(entry.detail.contains("***"));
        // The redactor is also directly testable.
        assert_eq!(obs.redact("a ep024-secret-token b"), "a *** b");
    }

    #[test]
    fn ep024_unit_observability_counters_accumulate() {
        let mut obs = IrrigationObservability::new(8, Vec::new());
        let c1 = obs.correlation();
        obs.record(c1, "zone_on", "ok", "a");
        let c2 = obs.correlation();
        obs.record(c2, "zone_on", "ok", "b");
        let c3 = obs.correlation();
        obs.record(c3, "zone_on", "UNAVAILABLE", "c");
        obs.increment("malformed");
        obs.increment("malformed");
        let counters = obs.counters();
        assert_eq!(counters.get("zone_on:ok"), Some(&2));
        assert_eq!(counters.get("zone_on:UNAVAILABLE"), Some(&1));
        assert_eq!(counters.get("malformed"), Some(&2));
    }

    #[test]
    fn ep024_unit_observability_correlation_is_unique_and_shaped() {
        let mut obs = IrrigationObservability::new(8, Vec::new());
        let a = obs.correlation();
        let b = obs.correlation();
        assert_ne!(a, b);
        assert!(a.starts_with("irrigation-"));
        assert!(b.starts_with("irrigation-"));
    }
}
