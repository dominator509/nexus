//! ContextEngine port (EP-016; SPEC-002; ADR-023).
//!
//! The context engine constructs a purpose-limited, permission-filtered
//! `ContextCapsule` for the model router. It composes hybrid retrieval,
//! privacy filtering, and bounded graph expansion behind one
//! provider-neutral contract. A capsule contains only authorized,
//! task-relevant, cited data and expires after the task or declared
//! retention (SPEC-003 required behavior 5; INV-007).

use crate::error::ContextError;
use crate::vocabulary::ContextPurpose;
use nexus_data::memory::MemoryQuery;
use nexus_fabric::ContextCapsule;
use serde::{Deserialize, Serialize};

/// A context construction request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextRequest {
    pub request_id: String,
    pub correlation_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    pub task_id: String,
    /// Purpose limitation label: the capsule may only carry data whose
    /// declared purpose permits this use (SPEC-020).
    pub purpose: ContextPurpose,
    /// Optional retrieval query driving hybrid retrieval.
    pub query: Option<MemoryQuery>,
    /// Required capabilities the constructed context must be able to
    /// satisfy (advisory to the model router; never an authorization).
    pub required_capabilities: Vec<String>,
    /// Maximum items to include in the capsule.
    pub max_items: usize,
}

impl ContextRequest {
    /// Validate canonical invariants. Fails closed on empty ids, an
    /// unbounded item count, or an invalid query.
    pub fn validate(&self) -> Result<(), ContextError> {
        if self.request_id.is_empty() || self.correlation_id.is_empty() {
            return Err(ContextError::validation(
                "request_id and correlation_id must not be empty",
                Some("context-request".into()),
            ));
        }
        if self.tenant_id.is_empty() || self.principal_id.is_empty() || self.task_id.is_empty() {
            return Err(ContextError::validation(
                "tenant_id, principal_id, and task_id must not be empty",
                Some("context-request".into()),
            ));
        }
        if self.max_items == 0 || self.max_items > 256 {
            return Err(ContextError::validation(
                "max_items must be in 1..=256",
                Some("context-request".into()),
            ));
        }
        Ok(())
    }
}

/// Provider-neutral context engine port.
pub trait ContextEngine {
    /// Build a purpose-limited, permission-filtered context capsule for
    /// the model router. Only authorized, task-relevant, cited data is
    /// included; nothing crosses a tenant or privacy boundary.
    fn build_context(&mut self, request: &ContextRequest) -> Result<ContextCapsule, ContextError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> ContextRequest {
        ContextRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
            task_id: "task-1".into(),
            purpose: ContextPurpose::TaskExecution,
            query: None,
            required_capabilities: vec![],
            max_items: 20,
        }
    }

    #[test]
    fn ep016_unit_context_request_validates() {
        assert!(valid().validate().is_ok());
    }

    #[test]
    fn ep016_unit_context_request_rejects_empty_ids() {
        let mut req = valid();
        req.request_id.clear();
        assert!(req.validate().is_err());
        let mut req = valid();
        req.tenant_id.clear();
        assert!(req.validate().is_err());
    }

    #[test]
    fn ep016_unit_context_request_rejects_unbounded_items() {
        let mut req = valid();
        req.max_items = 0;
        assert!(req.validate().is_err());
        let mut req = valid();
        req.max_items = 257;
        assert!(req.validate().is_err());
    }

    #[test]
    fn ep016_unit_context_request_serde_round_trip() {
        let req = valid();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["purpose"], "TASK_EXECUTION");
        let back: ContextRequest = serde_json::from_value(v).unwrap();
        assert_eq!(back, req);
    }
}
