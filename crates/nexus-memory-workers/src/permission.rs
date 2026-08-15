//! Permission filter (SPEC-020, INV-007; EP-016 M2).
//!
//! Permission filtering MUST precede relevance scoring. A memory the
//! principal is not allowed to access never enters the candidate
//! scoring/ranking pool. This reduces both leakage risk and side-channel
//! risk: unauthorized records are not ranked, so their existence is not
//! observable through score ordering.

use crate::util::{clamp01, sensitivity_rank};
use nexus_context::ContextError;
use nexus_data::memory::{MemoryCandidate, Sensitivity};

/// Access profile for a principal: what they may read.
///
/// This is the caller-injected permission input (port boundary). The
/// profile is intentionally a plain value the worker consumes; the
/// authoritative grants live behind the authorization plane (EP-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessProfile {
    /// Tenant the principal is authenticated in (INV-005).
    pub tenant_id: String,
    /// Principal identifier.
    pub principal_id: String,
    /// Namespaces the principal may read (INV-007): user, household,
    /// business, private, security.
    pub allowed_namespaces: Vec<String>,
    /// Sensitivity ceiling: records strictly more sensitive are excluded.
    pub max_sensitivity: Sensitivity,
    /// True when the principal may read memories marked private.
    pub private_allowed: bool,
}

impl AccessProfile {
    /// Validate canonical invariants. Fails closed on empty ids.
    pub fn validate(&self) -> Result<(), ContextError> {
        if self.tenant_id.is_empty() || self.principal_id.is_empty() {
            return Err(ContextError::validation(
                "access profile tenant_id and principal_id must not be empty",
                Some("access-profile".into()),
            ));
        }
        if self.allowed_namespaces.is_empty() {
            return Err(ContextError::validation(
                "access profile must allow at least one namespace",
                Some("access-profile".into()),
            ));
        }
        Ok(())
    }
}

/// Deterministic permission filter. Pure: takes candidates and an
/// access profile, returns only authorized candidates. Order preserved.
#[derive(Debug, Clone, Copy, Default)]
pub struct PermissionFilter;

impl PermissionFilter {
    /// Filter candidates by tenant, namespace, and sensitivity ceiling.
    /// Unauthorized candidates are excluded BEFORE any scoring.
    pub fn filter(
        &self,
        profile: &AccessProfile,
        candidates: Vec<MemoryCandidate>,
    ) -> Result<Vec<MemoryCandidate>, ContextError> {
        profile.validate()?;
        let ceiling = sensitivity_rank(profile.max_sensitivity);
        let mut allowed = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if self.allowed(profile, ceiling, &candidate) {
                allowed.push(candidate);
            }
        }
        Ok(allowed)
    }

    /// Whether a single candidate is authorized. Used by other workers
    /// (privacy, engine) to enforce the same boundary on later stages.
    pub fn allowed(
        &self,
        profile: &AccessProfile,
        ceiling: u8,
        candidate: &MemoryCandidate,
    ) -> bool {
        let record = &candidate.record;
        if record.tenant_id.as_str() != profile.tenant_id {
            return false;
        }
        if sensitivity_rank(record.sensitivity) > ceiling {
            return false;
        }
        let ns = record.namespace.as_str();
        if ns == "private" {
            profile.private_allowed && profile.allowed_namespaces.iter().any(|n| n == "private")
        } else {
            profile.allowed_namespaces.iter().any(|n| n == ns)
        }
    }

    /// Ceiling helper for callers that carry a precomputed rank.
    pub fn ceiling_rank(max_sensitivity: Sensitivity) -> u8 {
        sensitivity_rank(max_sensitivity)
    }
}

/// Candidate score sanity clamp: provider scores must be in [0, 1].
pub fn normalized_score(score: f64) -> f64 {
    clamp01(score)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_data::memory::{MemoryRecord, MemoryStatus, RetentionPolicy, RetentionUnit};
    use nexus_domain::{MemoryType, NexusId, TenantId};

    fn record(tenant: &str, namespace: &str, sensitivity: Sensitivity) -> MemoryCandidate {
        MemoryCandidate {
            record: MemoryRecord {
                memory_id: NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6c01").unwrap(),
                tenant_id: TenantId::new(tenant).unwrap(),
                namespace: namespace.into(),
                memory_type: MemoryType::Episodic,
                content: serde_json::json!({ "note": "x" }),
                content_hash: "c".repeat(64),
                source: "test".into(),
                actor: "p-1".into(),
                created_at: "2026-01-01T00:00:00Z".into(),
                observed_at: "2026-01-01T00:00:00Z".into(),
                confidence: 0.9,
                sensitivity,
                purpose: "SEARCH".into(),
                retention: RetentionPolicy::for_duration(RetentionUnit::Days, 30),
                status: MemoryStatus::Active,
                derived_from: vec![],
                supersedes: None,
                embedding_ref: None,
            },
            score: 0.9,
        }
    }

    fn tenant_id(seed: &str) -> TenantId {
        // 0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6c0X, X = hex digit.
        TenantId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6c0{}", seed)).unwrap()
    }

    #[test]
    fn ep016_unit_permission_filter_allows_authorized() {
        let profile = AccessProfile {
            tenant_id: tenant_id("1").as_str().to_string(),
            principal_id: "p-1".into(),
            allowed_namespaces: vec!["household".into(), "personal".into()],
            max_sensitivity: Sensitivity::Personal,
            private_allowed: true,
        };
        let candidates = vec![
            record(tenant_id("1").as_str(), "household", Sensitivity::Household),
            record(tenant_id("1").as_str(), "personal", Sensitivity::Personal),
        ];
        let out = PermissionFilter.filter(&profile, candidates).unwrap();
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn ep016_unit_permission_filter_excludes_cross_tenant_before_ranking() {
        let profile = AccessProfile {
            tenant_id: tenant_id("1").as_str().to_string(),
            principal_id: "p-1".into(),
            allowed_namespaces: vec!["household".into()],
            max_sensitivity: Sensitivity::Secret,
            private_allowed: true,
        };
        // Tenant B memory must never enter the scoring pool.
        let candidates = vec![record(
            tenant_id("2").as_str(),
            "household",
            Sensitivity::Household,
        )];
        let out = PermissionFilter.filter(&profile, candidates).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn ep016_unit_permission_filter_excludes_private_namespace_without_grant() {
        let profile = AccessProfile {
            tenant_id: tenant_id("1").as_str().to_string(),
            principal_id: "p-1".into(),
            allowed_namespaces: vec!["household".into()],
            max_sensitivity: Sensitivity::Secret,
            private_allowed: false,
        };
        let candidates = vec![record(
            tenant_id("1").as_str(),
            "private",
            Sensitivity::Personal,
        )];
        let out = PermissionFilter.filter(&profile, candidates).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn ep016_unit_permission_filter_excludes_above_sensitivity_ceiling() {
        let profile = AccessProfile {
            tenant_id: tenant_id("1").as_str().to_string(),
            principal_id: "p-1".into(),
            allowed_namespaces: vec!["household".into()],
            max_sensitivity: Sensitivity::Household,
            private_allowed: false,
        };
        let candidates = vec![record(
            tenant_id("1").as_str(),
            "household",
            Sensitivity::Sensitive,
        )];
        let out = PermissionFilter.filter(&profile, candidates).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn ep016_unit_permission_filter_never_ranks_unauthorized() {
        // Prove the boundary is exclusion, not post-hoc drop: an
        // unauthorized high-score candidate is absent entirely.
        let profile = AccessProfile {
            tenant_id: tenant_id("1").as_str().to_string(),
            principal_id: "p-1".into(),
            allowed_namespaces: vec!["household".into()],
            max_sensitivity: Sensitivity::Personal,
            private_allowed: true,
        };
        let mut hot = record(tenant_id("1").as_str(), "household", Sensitivity::Household);
        hot.score = 1.0;
        let mut other_tenant = record(tenant_id("2").as_str(), "household", Sensitivity::Household);
        other_tenant.score = 1.0;
        let out = PermissionFilter
            .filter(&profile, vec![hot, other_tenant])
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].record.tenant_id.as_str(), tenant_id("1").as_str());
    }
}
