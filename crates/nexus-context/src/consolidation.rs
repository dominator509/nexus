//! MemoryConsolidator port (EP-016; SPEC-002 behaviors 4-5; ADR-023).
//!
//! Semantic consolidation turns working/episodic source records into
//! semantic/entity proposals. Models can never write canonical memory
//! directly (SPEC-002 behavior 5): the consolidator always emits
//! `MemoryProposal`s for policy evaluation. The deterministic fallback
//! satisfies the same proposal contract when model-assisted
//! consolidation is unavailable (node contract fallback).

use crate::error::ContextError;
use crate::vocabulary::{ConsolidationMode, ContextPurpose};
use nexus_data::memory::{MemoryProposal, RetentionPolicy, Sensitivity};
use nexus_domain::MemoryType;
use serde::{Deserialize, Serialize};

/// A consolidation request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsolidationRequest {
    pub request_id: String,
    pub correlation_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    /// Source memory records to consolidate (derived-from chain).
    pub source_memory_ids: Vec<String>,
    /// Target memory type for the consolidated proposal.
    pub target_type: MemoryType,
    /// Sensitivity for the consolidated proposal (never above the
    /// source maximum in an implementation).
    pub sensitivity: Sensitivity,
    /// Purpose limitation label for the proposal (SPEC-020).
    pub purpose: ContextPurpose,
    /// Retention policy for the proposal (SPEC-020).
    pub retention: RetentionPolicy,
}

impl ConsolidationRequest {
    /// Validate canonical invariants. Fails closed on empty ids, no
    /// source records, or a non-semantic/entity target type.
    pub fn validate(&self) -> Result<(), ContextError> {
        if self.request_id.is_empty() || self.correlation_id.is_empty() {
            return Err(ContextError::validation(
                "request_id and correlation_id must not be empty",
                Some("consolidation-request".into()),
            ));
        }
        if self.tenant_id.is_empty() || self.principal_id.is_empty() {
            return Err(ContextError::validation(
                "tenant_id and principal_id must not be empty",
                Some("consolidation-request".into()),
            ));
        }
        if self.source_memory_ids.is_empty() {
            return Err(ContextError::validation(
                "source_memory_ids must not be empty",
                Some("consolidation-request".into()),
            ));
        }
        if !matches!(self.target_type, MemoryType::Semantic | MemoryType::Entity) {
            return Err(ContextError::validation(
                "consolidation target must be SEMANTIC or ENTITY",
                Some("consolidation-request".into()),
            ));
        }
        Ok(())
    }
}

/// Outcome of a consolidation pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConsolidationOutcome {
    /// Proposals emitted for policy evaluation (never canonical facts).
    pub proposals: Vec<MemoryProposal>,
    /// The execution mode actually used.
    pub mode: ConsolidationMode,
}

/// Provider-neutral memory consolidator port.
pub trait MemoryConsolidator {
    /// Consolidate source memories into new proposals. The result is a
    /// set of `MemoryProposal`s for the canonical policy evaluator;
    /// nothing here writes canonical memory.
    fn consolidate(
        &mut self,
        request: &ConsolidationRequest,
    ) -> Result<ConsolidationOutcome, ContextError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_data::memory::RetentionUnit;

    fn retention() -> RetentionPolicy {
        RetentionPolicy::for_duration(RetentionUnit::Days, 30)
    }

    fn valid() -> ConsolidationRequest {
        ConsolidationRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
            source_memory_ids: vec!["m-1".into(), "m-2".into()],
            target_type: MemoryType::Semantic,
            sensitivity: Sensitivity::Personal,
            purpose: ContextPurpose::TaskExecution,
            retention: retention(),
        }
    }

    #[test]
    fn ep016_unit_consolidation_request_validates() {
        assert!(valid().validate().is_ok());
    }

    #[test]
    fn ep016_unit_consolidation_request_rejects_empty_sources() {
        let mut req = valid();
        req.source_memory_ids.clear();
        assert!(req.validate().is_err());
    }

    #[test]
    fn ep016_unit_consolidation_request_rejects_invalid_target() {
        let mut req = valid();
        req.target_type = MemoryType::System;
        assert!(req.validate().is_err());
    }

    #[test]
    fn ep016_unit_consolidation_request_serde_round_trip() {
        let req = valid();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["target_type"], "SEMANTIC");
        let back: ConsolidationRequest = serde_json::from_value(v).unwrap();
        assert_eq!(back, req);
    }
}
