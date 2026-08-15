//! Purpose limitation (SPEC-020; EP-016 M2).
//!
//! `ContextPurpose` is a hard constraint, not a ranking hint. A memory
//! that is relevant but outside the current declared purpose is
//! excluded. Different purposes produce different permissible memory
//! sets: namespaces, memory types, and sensitivity ceilings are scoped
//! per purpose.

use crate::util::{memory_type_rank, sensitivity_rank};
use nexus_context::{ContextError, ContextPurpose};
use nexus_data::memory::{MemoryCandidate, Sensitivity};
use nexus_domain::MemoryType;

/// Purpose policy: the permissible set for one declared purpose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurposePolicy {
    /// The declared purpose this policy governs.
    pub purpose: ContextPurpose,
    /// Namespaces permitted under this purpose. Empty means tenant-wide
    /// (still bounded by the permission filter and sensitivity rules).
    pub allowed_namespaces: Vec<String>,
    /// Memory types permitted under this purpose. Empty means all types.
    pub allowed_memory_types: Vec<MemoryType>,
    /// Sensitivity ceiling for this purpose. `None` means the tenant
    /// default ceiling (permission filter) applies unchanged.
    pub sensitivity_ceiling: Option<Sensitivity>,
}

/// Deterministic purpose limiter. Pure: excludes out-of-purpose
/// candidates; preserves order of the survivors.
#[derive(Debug, Clone, Copy, Default)]
pub struct PurposeLimiter;

impl PurposeLimiter {
    /// The canonical purpose policy table (SPEC-020; ADR-023).
    ///
    /// - HOME_CONTROL-style device/task purposes map to `TaskExecution`
    ///   (room/device/state/procedure context).
    /// - `BusinessTask`-style maps to `Planning` (business/project
    ///   context, excluding personal/private).
    /// - `PrivatePersonal` maps to `Search` (personal/private namespace
    ///   permitted only for that user).
    /// - `SharedRoomVoice` maps to `Notification` (shared-safe subset
    ///   only, never private/sensitive).
    /// - `SystemMaintenance` is restricted to system and security
    ///   namespaces.
    pub fn policy_for(purpose: ContextPurpose) -> PurposePolicy {
        match purpose {
            ContextPurpose::TaskExecution => PurposePolicy {
                purpose,
                allowed_namespaces: vec![
                    "household".into(),
                    "user".into(),
                    "business".into(),
                    "device".into(),
                    "room".into(),
                ],
                allowed_memory_types: vec![
                    MemoryType::Procedural,
                    MemoryType::Semantic,
                    MemoryType::Entity,
                    MemoryType::Episodic,
                    MemoryType::Decision,
                ],
                sensitivity_ceiling: Some(Sensitivity::BusinessConfidential),
            },
            ContextPurpose::Planning => PurposePolicy {
                purpose,
                allowed_namespaces: vec!["household".into(), "user".into(), "business".into()],
                allowed_memory_types: vec![
                    MemoryType::Semantic,
                    MemoryType::Entity,
                    MemoryType::Decision,
                    MemoryType::Procedural,
                ],
                sensitivity_ceiling: Some(Sensitivity::BusinessConfidential),
            },
            ContextPurpose::Search => PurposePolicy {
                purpose,
                // Search is tenant-wide: any namespace the permission
                // filter allows, including personal for that user.
                allowed_namespaces: vec![],
                allowed_memory_types: vec![],
                sensitivity_ceiling: Some(Sensitivity::Sensitive),
            },
            ContextPurpose::Notification => PurposePolicy {
                purpose,
                // Shared-safe subset: never private, never
                // business-confidential or above.
                allowed_namespaces: vec!["household".into(), "user".into()],
                allowed_memory_types: vec![
                    MemoryType::Episodic,
                    MemoryType::Decision,
                    MemoryType::Semantic,
                ],
                sensitivity_ceiling: Some(Sensitivity::Household),
            },
            ContextPurpose::SystemMaintenance => PurposePolicy {
                purpose,
                allowed_namespaces: vec!["system".into(), "security".into()],
                allowed_memory_types: vec![
                    MemoryType::System,
                    MemoryType::Working,
                    MemoryType::Procedural,
                ],
                sensitivity_ceiling: Some(Sensitivity::Security),
            },
        }
    }

    /// Apply the purpose policy to candidates. A candidate whose
    /// namespace, memory type, or sensitivity is outside the declared
    /// purpose is excluded. The permission filter has already run, so
    /// this is an additional hard constraint, not an authorization.
    pub fn filter(
        &self,
        policy: &PurposePolicy,
        candidates: Vec<MemoryCandidate>,
    ) -> Result<Vec<MemoryCandidate>, ContextError> {
        let mut out = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if self.allowed(policy, &candidate) {
                out.push(candidate);
            }
        }
        Ok(out)
    }

    /// Whether a candidate is inside the purpose's permissible set.
    pub fn allowed(&self, policy: &PurposePolicy, candidate: &MemoryCandidate) -> bool {
        let record = &candidate.record;
        match policy.sensitivity_ceiling {
            Some(ceiling) if sensitivity_rank(record.sensitivity) > sensitivity_rank(ceiling) => {
                return false;
            }
            _ => {}
        }
        if !policy.allowed_namespaces.is_empty()
            && !policy
                .allowed_namespaces
                .iter()
                .any(|n| n == record.namespace.as_str())
        {
            return false;
        }
        if !policy.allowed_memory_types.is_empty()
            && !policy
                .allowed_memory_types
                .iter()
                .any(|t| memory_type_rank(*t) == memory_type_rank(record.memory_type))
        {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_data::memory::{MemoryRecord, MemoryStatus, RetentionPolicy, RetentionUnit};
    use nexus_domain::{NexusId, TenantId};

    fn record(
        namespace: &str,
        memory_type: MemoryType,
        sensitivity: Sensitivity,
    ) -> MemoryCandidate {
        MemoryCandidate {
            record: MemoryRecord {
                memory_id: NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d01").unwrap(),
                tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d02").unwrap(),
                namespace: namespace.into(),
                memory_type,
                content: serde_json::json!({ "note": "x" }),
                content_hash: "d".repeat(64),
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

    #[test]
    fn ep016_unit_purpose_task_execution_allows_room_device_procedure() {
        let policy = PurposeLimiter::policy_for(ContextPurpose::TaskExecution);
        let candidates = vec![
            record("device", MemoryType::Procedural, Sensitivity::Household),
            record("room", MemoryType::Entity, Sensitivity::Household),
        ];
        let out = PurposeLimiter.filter(&policy, candidates).unwrap();
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn ep016_unit_purpose_task_execution_excludes_personal_and_secret() {
        let policy = PurposeLimiter::policy_for(ContextPurpose::TaskExecution);
        let candidates = vec![
            record("personal", MemoryType::Episodic, Sensitivity::Personal),
            record("security", MemoryType::System, Sensitivity::Secret),
        ];
        let out = PurposeLimiter.filter(&policy, candidates).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn ep016_unit_purpose_notification_is_shared_safe_subset() {
        let policy = PurposeLimiter::policy_for(ContextPurpose::Notification);
        let candidates = vec![
            record("household", MemoryType::Episodic, Sensitivity::Household),
            record("personal", MemoryType::Episodic, Sensitivity::Personal),
            record(
                "business",
                MemoryType::Semantic,
                Sensitivity::BusinessConfidential,
            ),
        ];
        let out = PurposeLimiter.filter(&policy, candidates).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].record.namespace, "household");
    }

    #[test]
    fn ep016_unit_purpose_search_allows_personal_namespace() {
        let policy = PurposeLimiter::policy_for(ContextPurpose::Search);
        let candidates = vec![
            record("personal", MemoryType::Episodic, Sensitivity::Sensitive),
            record("household", MemoryType::Episodic, Sensitivity::Household),
        ];
        let out = PurposeLimiter.filter(&policy, candidates).unwrap();
        // Search is tenant-wide; both personal and household pass the
        // purpose constraint (permission filter is the outer boundary).
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn ep016_unit_different_purposes_yield_different_sets() {
        // Same candidate set, different declared purpose -> different
        // permissible sets (hard constraint, not ranking).
        let candidates = vec![
            record("personal", MemoryType::Episodic, Sensitivity::Personal),
            record("device", MemoryType::Procedural, Sensitivity::Household),
            record(
                "business",
                MemoryType::Semantic,
                Sensitivity::BusinessConfidential,
            ),
        ];
        let task = PurposeLimiter
            .filter(
                &PurposeLimiter::policy_for(ContextPurpose::TaskExecution),
                candidates.clone(),
            )
            .unwrap();
        let search = PurposeLimiter
            .filter(
                &PurposeLimiter::policy_for(ContextPurpose::Search),
                candidates.clone(),
            )
            .unwrap();
        let notif = PurposeLimiter
            .filter(
                &PurposeLimiter::policy_for(ContextPurpose::Notification),
                candidates.clone(),
            )
            .unwrap();
        let task_ids: Vec<_> = task.iter().map(|c| c.record.namespace.as_str()).collect();
        let search_ids: Vec<_> = search.iter().map(|c| c.record.namespace.as_str()).collect();
        let notif_ids: Vec<_> = notif.iter().map(|c| c.record.namespace.as_str()).collect();
        assert_eq!(task_ids, vec!["device", "business"]);
        // Search is tenant-wide (within permission/sensitivity): both
        // personal and device survive; business-confidential exceeds the
        // search ceiling.
        assert_eq!(search_ids, vec!["personal", "device"]);
        assert!(notif_ids.is_empty());
    }
}
