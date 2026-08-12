//! Memory proposal evaluation (SPEC-002 behavior 5).
//!
//! Models cannot directly create canonical semantic facts. A write enters
//! as a `MemoryProposal`; the evaluator approves or rejects it based on
//! deterministic policy. Approved proposals may be promoted to `ACTIVE`
//! canonical records.

use nexus_data::{DataError, DataErrorCode, MemoryRecord, MemoryStatus};

/// Outcome of evaluating a memory proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalOutcome {
    /// Proposal accepted; the record may be promoted to `ACTIVE`.
    Approved,
    /// Proposal rejected; the record stays `REJECTED` and is never a fact.
    Rejected,
}

/// Deterministic proposal evaluator (SPEC-002 behavior 5, INV-014).
///
/// The evaluator checks the invariants a canonical memory record must
/// satisfy before it can become a fact. It never emits content; it returns
/// a decision only.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProposalEvaluator {
    /// Maximum allowed sensitivity for a proposal to auto-approve.
    /// Records above this ceiling require explicit human or policy review.
    pub max_auto_approve_sensitivity: u8,
}

impl ProposalEvaluator {
    /// Default evaluator: only PUBLIC and HOUSEHOLD auto-approve.
    pub fn new() -> Self {
        Self {
            max_auto_approve_sensitivity: 1, // Household = 1 in the ladder.
        }
    }

    /// Rank of a sensitivity class on the canonical ladder (higher = more
    /// sensitive). Internal ranking, never serialized.
    fn sensitivity_rank(record: &MemoryRecord) -> u8 {
        use nexus_data::Sensitivity;
        match record.sensitivity {
            Sensitivity::Public => 0,
            Sensitivity::Household => 1,
            Sensitivity::Personal => 2,
            Sensitivity::Sensitive => 3,
            Sensitivity::BusinessConfidential => 4,
            Sensitivity::Security => 5,
            Sensitivity::Secret => 6,
        }
    }

    /// Evaluate a proposal deterministically.
    pub fn evaluate(&self, record: &MemoryRecord) -> Result<ProposalOutcome, DataError> {
        record.validate()?;
        if record.status != MemoryStatus::Proposed {
            return Err(DataError::new(
                DataErrorCode::Invariant,
                "proposal must carry status PROPOSED",
            ));
        }
        // Secret memory never auto-approves; it requires explicit policy
        // review (SPEC-020). Above the configured ceiling: review required.
        if Self::sensitivity_rank(record) > self.max_auto_approve_sensitivity {
            return Ok(ProposalOutcome::Rejected);
        }
        Ok(ProposalOutcome::Approved)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_data::{RetentionPolicy, RetentionUnit, Sensitivity};
    use nexus_domain::MemoryType;
    use nexus_domain::{NexusId, TenantId};

    fn record(status: MemoryStatus, sensitivity: Sensitivity) -> MemoryRecord {
        MemoryRecord {
            memory_id: NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6b01").unwrap(),
            tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6b02").unwrap(),
            namespace: "household".to_string(),
            memory_type: MemoryType::Episodic,
            content: serde_json::json!({ "note": "test" }),
            content_hash: "b".repeat(64),
            source: "test".to_string(),
            actor: "principal".to_string(),
            created_at: "2026-08-12T00:00:00Z".to_string(),
            observed_at: "2026-08-12T00:00:00Z".to_string(),
            confidence: 0.9,
            sensitivity,
            purpose: "remember".to_string(),
            retention: RetentionPolicy::for_duration(RetentionUnit::Days, 30),
            status,
            derived_from: vec![],
            supersedes: None,
            embedding_ref: None,
        }
    }

    #[test]
    fn ep004_unit_proposal_auto_approves_low_sensitivity() {
        let evaluator = ProposalEvaluator::new();
        let r = record(MemoryStatus::Proposed, Sensitivity::Household);
        assert_eq!(evaluator.evaluate(&r).unwrap(), ProposalOutcome::Approved);
    }

    #[test]
    fn ep004_unit_proposal_rejects_secret_without_review() {
        let evaluator = ProposalEvaluator::new();
        let r = record(MemoryStatus::Proposed, Sensitivity::Secret);
        assert_eq!(evaluator.evaluate(&r).unwrap(), ProposalOutcome::Rejected);
    }

    #[test]
    fn ep004_unit_proposal_must_be_proposed() {
        let evaluator = ProposalEvaluator::new();
        let r = record(MemoryStatus::Active, Sensitivity::Household);
        let err = evaluator.evaluate(&r).unwrap_err();
        assert_eq!(err.code(), DataErrorCode::Invariant);
    }

    #[test]
    fn ep004_unit_proposal_rejects_invalid_record() {
        let evaluator = ProposalEvaluator::new();
        let mut r = record(MemoryStatus::Proposed, Sensitivity::Household);
        r.confidence = 2.0;
        let err = evaluator.evaluate(&r).unwrap_err();
        assert_eq!(err.code(), DataErrorCode::Validation);
    }
}
