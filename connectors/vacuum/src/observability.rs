//! EP-024 M5 vacuum observability (SPEC-011; M5 directive).
//!
//! Bounded, poison-safe observability for the vacuum connector:
//!   - a bounded redacted audit ring (latest operations, secrets never
//!     stored - the ring redacts on insert, so a poisoned message can
//!     never leak a credential);
//!   - typed counters (operations by outcome/code);
//!   - incident correlation ids (canonical `vacuum-<nanos>-<seq>`)
//!     preserved across error paths.
//!
//! Map readback results carry ONLY safe metadata (digest, dimensions,
//! provider reference) - raw household map imagery is never dumped
//! into telemetry (M5 privacy boundary).

use std::collections::{BTreeMap, VecDeque};

/// One audited vacuum operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VacuumAuditEntry {
    /// Canonical correlation id for the operation.
    pub correlation: String,
    /// Canonical operation name (e.g. "START_CLEAN", "verify", "MAP_READBACK").
    pub operation: String,
    /// Canonical outcome: ok | <ERROR_CODE>.
    pub outcome: String,
    /// Redacted detail (secrets replaced with *** before storing).
    pub detail: String,
}

/// Bounded redacted audit ring + counters for the vacuum adapter.
///
/// `max_entries` bounds memory (default 256). `detail` is redacted at
/// insert: any occurrence of the configured secret is replaced with
/// `***` BEFORE storage, so the ring is safe to dump even after a
/// poisoned error.
#[derive(Debug, Clone)]
pub struct VacuumObservability {
    ring: VecDeque<VacuumAuditEntry>,
    counters: BTreeMap<String, u64>,
    max_entries: usize,
    /// Secrets that must never appear in stored detail (redacted on
    /// insert). Empty when no secret was configured.
    secrets: Vec<String>,
    seq: u64,
}

impl Default for VacuumObservability {
    fn default() -> Self {
        Self::new(256, Vec::new())
    }
}

impl VacuumObservability {
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
            "vacuum-{}-{}",
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
        self.ring.push_back(VacuumAuditEntry {
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
    pub fn audit(&self) -> Vec<VacuumAuditEntry> {
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
    fn ep024_unit_vacuum_observability_ring_is_bounded() {
        let mut obs = VacuumObservability::new(4, Vec::new());
        for i in 0..10 {
            let correlation = obs.correlation();
            obs.record(correlation, "START_CLEAN", "ok", &format!("op {i}"));
        }
        assert_eq!(obs.audit().len(), 4);
        assert_eq!(obs.audit()[0].detail, "op 6");
        assert_eq!(obs.audit()[3].detail, "op 9");
    }

    #[test]
    fn ep024_unit_vacuum_observability_redacts_secrets_on_insert() {
        let mut obs = VacuumObservability::new(8, vec!["ep024-vacuum-secret".to_string()]);
        let correlation = obs.correlation();
        obs.record(
            correlation,
            "START_CLEAN",
            "error",
            "failed with ep024-vacuum-secret in the message",
        );
        let entry = &obs.audit()[0];
        assert!(!entry.detail.contains("ep024-vacuum-secret"));
        assert!(entry.detail.contains("***"));
        assert_eq!(obs.redact("a ep024-vacuum-secret b"), "a *** b");
    }

    #[test]
    fn ep024_unit_vacuum_observability_counters_accumulate() {
        let mut obs = VacuumObservability::new(8, Vec::new());
        let c1 = obs.correlation();
        obs.record(c1, "START_CLEAN", "ok", "a");
        let c2 = obs.correlation();
        obs.record(c2, "START_CLEAN", "ok", "b");
        let c3 = obs.correlation();
        obs.record(c3, "PAUSE", "UNAVAILABLE", "c");
        obs.increment("malformed");
        let counters = obs.counters();
        assert_eq!(counters.get("START_CLEAN:ok"), Some(&2));
        assert_eq!(counters.get("PAUSE:UNAVAILABLE"), Some(&1));
        assert_eq!(counters.get("malformed"), Some(&1));
    }

    #[test]
    fn ep024_unit_vacuum_observability_correlation_is_unique_and_shaped() {
        let mut obs = VacuumObservability::new(8, Vec::new());
        let a = obs.correlation();
        let b = obs.correlation();
        assert_ne!(a, b);
        assert!(a.starts_with("vacuum-"));
        assert!(b.starts_with("vacuum-"));
    }
}
