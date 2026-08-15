//! GraphExpansionPolicy port (EP-016; SPEC-002 behavior 7; ADR-023).
//!
//! Graph-aware context construction expands from a seed node across the
//! world graph within a bounded hop mode and node budget. The policy
//! never expands past a tenant, namespace, or security boundary; the
//! result is a bounded subgraph projection for the context capsule.

use crate::error::ContextError;
use crate::vocabulary::GraphExpansionMode;
use serde::{Deserialize, Serialize};

/// A node reference in the world graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNodeRef {
    pub node_id: String,
    /// Canonical node type (entity, household, business, device,
    /// resource, capability target, ...).
    pub node_type: String,
}

/// A directed edge reference in the world graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdgeRef {
    pub source: String,
    pub relation: String,
    pub target: String,
}

/// A graph expansion request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphExpansionRequest {
    pub request_id: String,
    pub correlation_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    /// Seed node for the expansion.
    pub seed: GraphNodeRef,
    /// Bounded expansion mode.
    pub mode: GraphExpansionMode,
    /// Maximum nodes to include (bounded context construction).
    pub max_nodes: usize,
}

impl GraphExpansionRequest {
    /// Validate canonical invariants. Fails closed on empty ids or an
    /// unbounded node budget.
    pub fn validate(&self) -> Result<(), ContextError> {
        if self.request_id.is_empty() || self.correlation_id.is_empty() {
            return Err(ContextError::validation(
                "request_id and correlation_id must not be empty",
                Some("graph-expansion-request".into()),
            ));
        }
        if self.tenant_id.is_empty() || self.principal_id.is_empty() {
            return Err(ContextError::validation(
                "tenant_id and principal_id must not be empty",
                Some("graph-expansion-request".into()),
            ));
        }
        if self.seed.node_id.is_empty() || self.seed.node_type.is_empty() {
            return Err(ContextError::validation(
                "seed node_id and node_type must not be empty",
                Some("graph-expansion-request".into()),
            ));
        }
        if self.max_nodes == 0 || self.max_nodes > 64 {
            return Err(ContextError::validation(
                "max_nodes must be in 1..=64",
                Some("graph-expansion-request".into()),
            ));
        }
        Ok(())
    }
}

/// A bounded graph expansion result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphExpansion {
    pub nodes: Vec<GraphNodeRef>,
    pub edges: Vec<GraphEdgeRef>,
    /// True when the expansion was cut off by the node budget.
    pub bounded: bool,
}

/// Provider-neutral graph expansion policy port.
pub trait GraphExpansionPolicy {
    /// Expand context from the seed node within the bounded mode and
    /// node budget. The policy never crosses a tenant, namespace, or
    /// security boundary.
    fn expand(&mut self, request: &GraphExpansionRequest) -> Result<GraphExpansion, ContextError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid() -> GraphExpansionRequest {
        GraphExpansionRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
            seed: GraphNodeRef {
                node_id: "entity-1".into(),
                node_type: "ENTITY".into(),
            },
            mode: GraphExpansionMode::OneHop,
            max_nodes: 16,
        }
    }

    #[test]
    fn ep016_unit_graph_expansion_request_validates() {
        assert!(valid().validate().is_ok());
    }

    #[test]
    fn ep016_unit_graph_expansion_request_rejects_empty_seed() {
        let mut req = valid();
        req.seed.node_id.clear();
        assert!(req.validate().is_err());
    }

    #[test]
    fn ep016_unit_graph_expansion_request_rejects_unbounded_nodes() {
        let mut req = valid();
        req.max_nodes = 0;
        assert!(req.validate().is_err());
        let mut req = valid();
        req.max_nodes = 65;
        assert!(req.validate().is_err());
    }

    #[test]
    fn ep016_unit_graph_expansion_request_serde_round_trip() {
        let req = valid();
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["mode"], "ONE_HOP");
        let back: GraphExpansionRequest = serde_json::from_value(v).unwrap();
        assert_eq!(back, req);
    }
}
