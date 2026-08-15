//! PrivacyFilter port (EP-016; SPEC-020, INV-007; ADR-023).
//!
//! Context is purpose-limited and permission-filtered. The filter
//! enforces tenant/principal permission, sensitivity ceilings, purpose
//! limitation, and namespace isolation (user, household, business,
//! private, security). Private shared-room requests use private
//! response routing; a candidate is allowed, redacted (metadata only),
//! or denied outright. Nothing leaks across a namespace or privacy
//! boundary.

use crate::error::ContextError;
use crate::vocabulary::{ContextPurpose, PrivacyFilterDecision};
use nexus_data::memory::MemoryCandidate;
use serde::{Deserialize, Serialize};

/// Per-candidate filter decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilteredCandidate {
    /// The candidate record (redacted payload when `decision` is
    /// `Redact`).
    pub candidate: MemoryCandidate,
    pub decision: PrivacyFilterDecision,
    /// Redacted deterministic reason (never sensitive content).
    pub reason: Option<String>,
}

impl FilteredCandidate {
    pub fn allowed(candidate: MemoryCandidate) -> Self {
        Self {
            candidate,
            decision: PrivacyFilterDecision::Allow,
            reason: None,
        }
    }

    pub fn redacted(candidate: MemoryCandidate, reason: impl Into<String>) -> Self {
        Self {
            candidate,
            decision: PrivacyFilterDecision::Redact,
            reason: Some(reason.into()),
        }
    }

    pub fn denied(candidate: MemoryCandidate, reason: impl Into<String>) -> Self {
        Self {
            candidate,
            decision: PrivacyFilterDecision::Deny,
            reason: Some(reason.into()),
        }
    }
}

/// Provider-neutral privacy filter port.
pub trait PrivacyFilter {
    /// Filter candidates by purpose limitation, sensitivity, permission,
    /// and namespace isolation. The returned vector preserves order.
    fn filter(
        &mut self,
        tenant_id: &str,
        principal_id: &str,
        purpose: ContextPurpose,
        candidates: Vec<MemoryCandidate>,
    ) -> Result<Vec<FilteredCandidate>, ContextError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::PrivacyFilterDecision;

    #[test]
    fn ep016_unit_filtered_candidate_constructors() {
        // A minimal candidate shell is enough to prove the decision
        // wrapper shape; the filter behavior lands in M2.
        let candidate = MemoryCandidate {
            record: nexus_data::memory::MemoryRecord {
                memory_id: nexus_domain::NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01")
                    .unwrap(),
                tenant_id: nexus_domain::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02")
                    .unwrap(),
                namespace: "personal".into(),
                memory_type: nexus_domain::MemoryType::Episodic,
                content: serde_json::json!({"note": "x"}),
                content_hash: "a".repeat(64),
                source: "test".into(),
                actor: "p-1".into(),
                created_at: "2026-01-01T00:00:00Z".into(),
                observed_at: "2026-01-01T00:00:00Z".into(),
                confidence: 0.9,
                sensitivity: nexus_data::memory::Sensitivity::Personal,
                purpose: "SEARCH".into(),
                retention: nexus_data::memory::RetentionPolicy::for_duration(
                    nexus_data::memory::RetentionUnit::Days,
                    30,
                ),
                status: nexus_data::memory::MemoryStatus::Active,
                derived_from: vec![],
                supersedes: None,
                embedding_ref: None,
            },
            score: 0.8,
        };
        assert_eq!(
            FilteredCandidate::allowed(candidate.clone()).decision,
            PrivacyFilterDecision::Allow
        );
        assert_eq!(
            FilteredCandidate::redacted(candidate.clone(), "sensitivity").decision,
            PrivacyFilterDecision::Redact
        );
        assert_eq!(
            FilteredCandidate::denied(candidate, "namespace").decision,
            PrivacyFilterDecision::Deny
        );
    }

    #[test]
    fn ep016_unit_filtered_candidate_serde_round_trip() {
        let candidate = MemoryCandidate {
            record: nexus_data::memory::MemoryRecord {
                memory_id: nexus_domain::NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03")
                    .unwrap(),
                tenant_id: nexus_domain::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a04")
                    .unwrap(),
                namespace: "personal".into(),
                memory_type: nexus_domain::MemoryType::Episodic,
                content: serde_json::json!({"note": "x"}),
                content_hash: "a".repeat(64),
                source: "test".into(),
                actor: "p-1".into(),
                created_at: "2026-01-01T00:00:00Z".into(),
                observed_at: "2026-01-01T00:00:00Z".into(),
                confidence: 0.9,
                sensitivity: nexus_data::memory::Sensitivity::Personal,
                purpose: "SEARCH".into(),
                retention: nexus_data::memory::RetentionPolicy::for_duration(
                    nexus_data::memory::RetentionUnit::Days,
                    30,
                ),
                status: nexus_data::memory::MemoryStatus::Active,
                derived_from: vec![],
                supersedes: None,
                embedding_ref: None,
            },
            score: 0.8,
        };
        let filtered = FilteredCandidate::denied(candidate, "namespace");
        let v = serde_json::to_value(&filtered).unwrap();
        assert_eq!(v["decision"], "DENY");
        let back: FilteredCandidate = serde_json::from_value(v).unwrap();
        assert_eq!(back, filtered);
    }
}
