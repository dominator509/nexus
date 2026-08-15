//! Active memory lifecycle filter (SPEC-002 behavior 4; EP-016 M2).
//!
//! Retrieval must respect EP-004 lifecycle semantics: deleted,
//! superseded, terminally invalid, and retention-expired records are not
//! active context unless the purpose explicitly requests historical
//! state. Legal hold preserves a record for retention/audit; it does NOT
//! make that record active context.

use crate::util::{retention_seconds, rfc3339_utc_millis};
use nexus_context::ContextError;
use nexus_data::memory::{MemoryCandidate, MemoryStatus};

/// Lifecycle context injected by the caller (clock lives outside the
/// worker; the worker never reads a clock itself).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleContext {
    /// Current time in epoch milliseconds (injected).
    pub now_epoch_ms: u64,
    /// Whether the declared purpose explicitly requests historical /
    /// superseded state (SPEC-002 historical retrieval).
    pub include_historical: bool,
}

/// Deterministic active-memory lifecycle filter. Pure.
#[derive(Debug, Clone, Copy, Default)]
pub struct ActiveMemoryLifecycleFilter;

impl ActiveMemoryLifecycleFilter {
    /// Filter out non-active, superseded (unless historical requested),
    /// deleted, rejected, and retention-expired candidates. Legal-hold
    /// records are preserved in storage but are NOT auto-selected here.
    pub fn filter(
        &self,
        ctx: &LifecycleContext,
        candidates: Vec<MemoryCandidate>,
    ) -> Result<Vec<MemoryCandidate>, ContextError> {
        let mut out = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if self.active(ctx, &candidate) {
                out.push(candidate);
            }
        }
        Ok(out)
    }

    /// Whether a candidate is active context at the injected time.
    pub fn active(&self, ctx: &LifecycleContext, candidate: &MemoryCandidate) -> bool {
        let record = &candidate.record;
        match record.status {
            MemoryStatus::Deleted | MemoryStatus::Rejected => return false,
            MemoryStatus::Superseded if !ctx.include_historical => return false,
            MemoryStatus::Proposed => {
                // Proposals are not canonical facts; never active context.
                return false;
            }
            _ => {}
        }
        // Retention expiry: only bounded policies expire. Indefinite
        // (legal hold) never expires, but a legal-hold record that is
        // SUPERSEDED/DELETED still must not surface (handled above).
        let expires_ms = rfc3339_utc_millis(&record.created_at).and_then(|created_ms| {
            retention_seconds(&record.retention)
                .map(|seconds| created_ms.saturating_add(seconds.saturating_mul(1_000)))
        });
        match expires_ms {
            Some(expires_ms) if ctx.now_epoch_ms > expires_ms => return false,
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_data::memory::{MemoryRecord, RetentionPolicy, RetentionUnit, Sensitivity};
    use nexus_domain::{MemoryType, NexusId, TenantId};

    fn record(
        status: MemoryStatus,
        created_at: &str,
        retention: RetentionPolicy,
    ) -> MemoryCandidate {
        MemoryCandidate {
            record: MemoryRecord {
                memory_id: NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6e01").unwrap(),
                tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6e02").unwrap(),
                namespace: "household".into(),
                memory_type: MemoryType::Episodic,
                content: serde_json::json!({ "note": "x" }),
                content_hash: "e".repeat(64),
                source: "test".into(),
                actor: "p-1".into(),
                created_at: created_at.into(),
                observed_at: created_at.into(),
                confidence: 0.9,
                sensitivity: Sensitivity::Household,
                purpose: "SEARCH".into(),
                retention,
                status,
                derived_from: vec![],
                supersedes: None,
                embedding_ref: None,
            },
            score: 0.9,
        }
    }

    fn now() -> u64 {
        rfc3339_utc_millis("2026-01-01T00:00:00Z").unwrap()
    }

    #[test]
    fn ep016_unit_lifecycle_excludes_deleted_and_rejected() {
        let ctx = LifecycleContext {
            now_epoch_ms: now(),
            include_historical: false,
        };
        let candidates = vec![
            record(
                MemoryStatus::Deleted,
                "2025-12-01T00:00:00Z",
                RetentionPolicy::for_duration(RetentionUnit::Days, 90),
            ),
            record(
                MemoryStatus::Rejected,
                "2025-12-01T00:00:00Z",
                RetentionPolicy::for_duration(RetentionUnit::Days, 90),
            ),
            record(
                MemoryStatus::Active,
                "2025-12-01T00:00:00Z",
                RetentionPolicy::for_duration(RetentionUnit::Days, 90),
            ),
        ];
        let out = ActiveMemoryLifecycleFilter
            .filter(&ctx, candidates)
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].record.status, MemoryStatus::Active);
    }

    #[test]
    fn ep016_unit_lifecycle_excludes_superseded_unless_historical_requested() {
        let active_ctx = LifecycleContext {
            now_epoch_ms: now(),
            include_historical: false,
        };
        let hist_ctx = LifecycleContext {
            now_epoch_ms: now(),
            include_historical: true,
        };
        let candidates = vec![
            record(
                MemoryStatus::Superseded,
                "2025-12-01T00:00:00Z",
                RetentionPolicy::for_duration(RetentionUnit::Days, 90),
            ),
            record(
                MemoryStatus::Active,
                "2025-12-01T00:00:00Z",
                RetentionPolicy::for_duration(RetentionUnit::Days, 90),
            ),
        ];
        let normal = ActiveMemoryLifecycleFilter
            .filter(&active_ctx, candidates.clone())
            .unwrap();
        assert_eq!(normal.len(), 1);
        assert_eq!(normal[0].record.status, MemoryStatus::Active);
        let historical = ActiveMemoryLifecycleFilter
            .filter(&hist_ctx, candidates)
            .unwrap();
        assert_eq!(historical.len(), 2);
    }

    #[test]
    fn ep016_unit_lifecycle_excludes_retention_expired() {
        let ctx = LifecycleContext {
            now_epoch_ms: now(),
            include_historical: false,
        };
        // Created 2025-01-01 with 30 day retention: expired long before
        // 2026-01-01.
        let expired = record(
            MemoryStatus::Active,
            "2025-01-01T00:00:00Z",
            RetentionPolicy::for_duration(RetentionUnit::Days, 30),
        );
        // Created 2025-12-01 with 90 day retention: still active at
        // 2026-01-01.
        let live = record(
            MemoryStatus::Active,
            "2025-12-01T00:00:00Z",
            RetentionPolicy::for_duration(RetentionUnit::Days, 90),
        );
        let out = ActiveMemoryLifecycleFilter
            .filter(&ctx, vec![expired, live])
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].record.created_at, "2025-12-01T00:00:00Z");
    }

    #[test]
    fn ep016_unit_lifecycle_legal_hold_does_not_auto_select() {
        // Legal hold (indefinite retention) never expires, but a
        // SUPERSEDED legal-hold record is still not active context.
        let ctx = LifecycleContext {
            now_epoch_ms: now(),
            include_historical: false,
        };
        let held_superseded = record(
            MemoryStatus::Superseded,
            "2024-01-01T00:00:00Z",
            RetentionPolicy::indefinite(),
        );
        let held_active = record(
            MemoryStatus::Active,
            "2024-01-01T00:00:00Z",
            RetentionPolicy::indefinite(),
        );
        let out = ActiveMemoryLifecycleFilter
            .filter(&ctx, vec![held_superseded, held_active])
            .unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].record.status, MemoryStatus::Active);
    }

    #[test]
    fn ep016_unit_lifecycle_proposals_never_active_context() {
        let ctx = LifecycleContext {
            now_epoch_ms: now(),
            include_historical: true,
        };
        let candidate = record(
            MemoryStatus::Proposed,
            "2025-12-01T00:00:00Z",
            RetentionPolicy::for_duration(RetentionUnit::Days, 90),
        );
        let out = ActiveMemoryLifecycleFilter
            .filter(&ctx, vec![candidate])
            .unwrap();
        assert!(out.is_empty());
    }
}
