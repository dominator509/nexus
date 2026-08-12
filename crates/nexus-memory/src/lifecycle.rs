//! Memory lifecycle: supersession and deletion (SPEC-002 behaviors 4, 8).
//!
//! A record moves PROPOSED -> ACTIVE -> SUPERSEDED (or DELETED). Deletion
//! is a terminal state; a superseded record is no longer a retrieval
//! candidate for the live view but its provenance remains. The engine is
//! deterministic and fails closed on invalid transitions.

use nexus_data::{DataError, DataErrorCode, MemoryRecord, MemoryStatus};

/// Lifecycle transition error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleError {
    /// The requested transition is invalid for the current state.
    InvalidTransition,
    /// Supersession requires an ACTIVE target and a valid successor.
    InvalidSupersession,
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTransition => f.write_str("invalid lifecycle transition"),
            Self::InvalidSupersession => f.write_str("invalid supersession"),
        }
    }
}

impl std::error::Error for LifecycleError {}

impl From<LifecycleError> for DataError {
    fn from(err: LifecycleError) -> Self {
        DataError::new(DataErrorCode::Invariant, err.to_string())
    }
}

/// Deterministic lifecycle engine (SPEC-002 behavior 4).
#[derive(Debug, Clone, Copy, Default)]
pub struct LifecycleEngine;

impl LifecycleEngine {
    /// Transition `record` to `ACTIVE` (proposal promotion).
    ///
    /// Only `PROPOSED` records can be activated; this enforces behavior 5
    /// (writes are proposals evaluated by policy).
    pub fn activate(record: &mut MemoryRecord) -> Result<(), LifecycleError> {
        if record.status != MemoryStatus::Proposed {
            return Err(LifecycleError::InvalidTransition);
        }
        record.status = MemoryStatus::Active;
        Ok(())
    }

    /// Supersede `target` with `successor`.
    ///
    /// The target must be `ACTIVE`; the successor must be a valid record
    /// carrying a reference to the target. On success, the target becomes
    /// `SUPERSEDED` and the successor is returned (still `PROPOSED` so the
    /// caller can run policy evaluation before promotion).
    pub fn supersede(
        target: &mut MemoryRecord,
        successor: MemoryRecord,
    ) -> Result<MemoryRecord, LifecycleError> {
        if target.status != MemoryStatus::Active {
            return Err(LifecycleError::InvalidSupersession);
        }
        if successor.supersedes != Some(target.memory_id.clone()) {
            return Err(LifecycleError::InvalidSupersession);
        }
        target.status = MemoryStatus::Superseded;
        Ok(successor)
    }

    /// Mark `record` deleted (terminal).
    pub fn delete(record: &mut MemoryRecord) -> Result<(), LifecycleError> {
        if record.status == MemoryStatus::Deleted {
            return Err(LifecycleError::InvalidTransition);
        }
        record.status = MemoryStatus::Deleted;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_data::{RetentionPolicy, RetentionUnit, Sensitivity};
    use nexus_domain::MemoryType;
    use nexus_domain::{NexusId, TenantId};

    fn record(id: &str, status: MemoryStatus) -> MemoryRecord {
        MemoryRecord {
            memory_id: NexusId::new(id).unwrap(),
            tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d02").unwrap(),
            namespace: "household".to_string(),
            memory_type: MemoryType::Semantic,
            content: serde_json::json!({ "fact": true }),
            content_hash: "d".repeat(64),
            source: "test".to_string(),
            actor: "principal".to_string(),
            created_at: "2026-08-12T00:00:00Z".to_string(),
            observed_at: "2026-08-12T00:00:00Z".to_string(),
            confidence: 0.9,
            sensitivity: Sensitivity::Household,
            purpose: "remember".to_string(),
            retention: RetentionPolicy::for_duration(RetentionUnit::Days, 30),
            status,
            derived_from: vec![],
            supersedes: None,
            embedding_ref: None,
        }
    }

    #[test]
    fn ep004_unit_lifecycle_proposed_to_active() {
        let mut r = record(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d03",
            MemoryStatus::Proposed,
        );
        LifecycleEngine::activate(&mut r).unwrap();
        assert_eq!(r.status, MemoryStatus::Active);
    }

    #[test]
    fn ep004_unit_lifecycle_activate_rejects_non_proposed() {
        let mut r = record("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d04", MemoryStatus::Active);
        assert_eq!(
            LifecycleEngine::activate(&mut r),
            Err(LifecycleError::InvalidTransition)
        );
    }

    #[test]
    fn ep004_unit_lifecycle_supersede_marks_target_superseded() {
        let mut target = record("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d05", MemoryStatus::Active);
        let mut successor = record(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d06",
            MemoryStatus::Proposed,
        );
        successor.supersedes = Some(target.memory_id.clone());
        let promoted = LifecycleEngine::supersede(&mut target, successor).unwrap();
        assert_eq!(target.status, MemoryStatus::Superseded);
        assert_eq!(promoted.status, MemoryStatus::Proposed);
    }

    #[test]
    fn ep004_unit_lifecycle_supersede_rejects_wrong_target() {
        let mut target = record("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d07", MemoryStatus::Active);
        let mut successor = record(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d08",
            MemoryStatus::Proposed,
        );
        successor.supersedes = Some(NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d99").unwrap());
        assert_eq!(
            LifecycleEngine::supersede(&mut target, successor),
            Err(LifecycleError::InvalidSupersession)
        );
    }

    #[test]
    fn ep004_unit_lifecycle_delete_is_terminal() {
        let mut r = record("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6d09", MemoryStatus::Active);
        LifecycleEngine::delete(&mut r).unwrap();
        assert_eq!(r.status, MemoryStatus::Deleted);
        assert_eq!(
            LifecycleEngine::delete(&mut r),
            Err(LifecycleError::InvalidTransition)
        );
    }
}
