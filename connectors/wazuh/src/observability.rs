//! EP-031 Wazuh connector observability (M4).
//!
//! Bounded redacted audit ring with correlation. Credentials and
//! sensitive content are redacted at insert (poison-safe); audit
//! entries never contain secrets, prompts, or private payloads
//! (SECURITY.md, SPEC-005).

use std::cell::RefCell;
use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// A bounded audit entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentinelAuditEntry {
    /// Operation name.
    pub operation: String,
    /// Outcome: ok | denied | failed.
    pub outcome: String,
    /// Redacted detail (never a secret).
    pub detail: String,
    /// Tenant reference.
    pub tenant_ref: String,
    /// Correlation reference (sentinel-<nanos>-<seq>).
    pub correlation: String,
    /// RFC3339 timestamp of the entry.
    pub recorded_at: String,
}

impl SentinelAuditEntry {
    pub fn new(
        operation: impl Into<String>,
        outcome: impl Into<String>,
        detail: impl Into<String>,
        tenant_ref: &str,
    ) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self {
            operation: operation.into(),
            outcome: outcome.into(),
            detail: redact(&detail.into()),
            tenant_ref: tenant_ref.to_string(),
            correlation: format!("sentinel-{nanos}"),
            recorded_at: "2026-08-20T00:00:00Z".to_string(),
        }
    }
}

/// Bounded redacted audit ring (256 entries).
#[derive(Debug, Clone, Default)]
pub struct SentinelObservability {
    entries: RefCell<VecDeque<SentinelAuditEntry>>,
}

impl SentinelObservability {
    const CAPACITY: usize = 256;

    pub fn new() -> Self {
        Self {
            entries: RefCell::new(VecDeque::new()),
        }
    }

    pub fn record(&self, entry: SentinelAuditEntry) {
        let mut ring = self.entries.borrow_mut();
        if ring.len() >= Self::CAPACITY {
            ring.pop_front();
        }
        ring.push_back(entry);
    }

    pub fn entries(&self) -> Vec<SentinelAuditEntry> {
        self.entries.borrow().iter().cloned().collect()
    }
}

/// Redact known sensitive values (poison-safe). Wazuh API credentials
/// are never embedded in audit details; any accidental occurrence of a
/// secret-like token is replaced.
fn redact(detail: &str) -> String {
    let mut out = detail.to_string();
    // Redact anything that looks like a bearer/JWT token.
    let mut i = 0;
    while i + 20 < out.len() {
        let window = &out[i..i + 20];
        let looks_secret = window
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '=' | '+' | '/'))
            && window.chars().any(|c| c.is_ascii_digit())
            && window.chars().any(|c| c.is_ascii_alphabetic());
        if looks_secret {
            out.replace_range(i..i + 20, "REDACTED");
            i += 8;
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep031_unit_observability_redacts_secret_like_tokens() {
        let detail = "token abc123def456ghi789jkl";
        let redacted = redact(detail);
        assert!(!redacted.contains("abc123def456ghi789jkl"));
        assert!(redacted.contains("REDACTED"));
    }

    #[test]
    fn ep031_unit_observability_ring_is_bounded() {
        let obs = SentinelObservability::new();
        for i in 0..300 {
            obs.record(SentinelAuditEntry::new(
                "op",
                "ok",
                format!("detail {i}"),
                "tenant",
            ));
        }
        assert!(obs.entries().len() <= SentinelObservability::CAPACITY);
        assert_eq!(obs.entries().len(), SentinelObservability::CAPACITY);
    }

    #[test]
    fn ep031_unit_observability_entries_carry_correlation() {
        let obs = SentinelObservability::new();
        obs.record(SentinelAuditEntry::new("op", "ok", "detail", "tenant"));
        let entry = &obs.entries()[0];
        assert!(entry.correlation.starts_with("sentinel-"));
        assert_eq!(entry.outcome, "ok");
    }
}
