//! Bounded, purpose-aware graph expansion (SPEC-002 behavior 7;
//! EP-016 M2).
//!
//! GraphExpansionPolicy expands context from a seed node across the
//! world graph within a bounded hop mode, node budget, neighbor limit,
//! allowed relation set, tenant boundary, and sensitivity boundary.
//! Expansion is cycle-safe and never crosses a tenant, namespace, or
//! security boundary. Graph proximity alone is never sufficient:
//! expansion is also purpose-aware (different purposes allow different
//! relation sets).

use crate::util::sensitivity_rank;
use nexus_context::{
    ContextError, GraphEdgeRef, GraphExpansion, GraphExpansionMode, GraphExpansionPolicy,
    GraphExpansionRequest, GraphNodeRef,
};
use nexus_data::memory::Sensitivity;
use std::collections::HashSet;

/// A world-graph node as seen by the expansion provider (injected I/O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNodeInfo {
    pub node_id: String,
    /// Canonical node type (entity, household, business, device,
    /// resource, capability target, ...).
    pub node_type: String,
    /// Tenant boundary (INV-005).
    pub tenant_id: String,
    /// Sensitivity class of the node (INV-014).
    pub sensitivity: Sensitivity,
    /// RFC 3339 timestamp the node was last updated.
    pub updated_at: String,
}

/// A directed edge as seen by the expansion provider (injected I/O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphEdgeInfo {
    pub source: String,
    pub relation: String,
    pub target: String,
    /// Tenant boundary of the edge; a cross-tenant edge is rejected.
    pub tenant_id: String,
}

/// Injected world-graph provider port. The adapter owns the actual
/// graph store; the worker only walks what the provider returns.
pub trait GraphProvider {
    /// Look up a node by id.
    fn node(
        &mut self,
        tenant_id: &str,
        node_id: &str,
    ) -> Result<Option<GraphNodeInfo>, ContextError>;
    /// Outgoing edges from a node (bounded by the provider's own
    /// storage; the worker enforces relation/tenant/sensitivity caps).
    fn edges(&mut self, tenant_id: &str, node_id: &str)
    -> Result<Vec<GraphEdgeInfo>, ContextError>;
}

/// Blanket implementation so engines can borrow a provider by `&mut`
/// reference (the worker never owns the adapter).
impl<P: GraphProvider + ?Sized> GraphProvider for &mut P {
    fn node(
        &mut self,
        tenant_id: &str,
        node_id: &str,
    ) -> Result<Option<GraphNodeInfo>, ContextError> {
        (**self).node(tenant_id, node_id)
    }

    fn edges(
        &mut self,
        tenant_id: &str,
        node_id: &str,
    ) -> Result<Vec<GraphEdgeInfo>, ContextError> {
        (**self).edges(tenant_id, node_id)
    }
}

/// Deterministic bounded graph expansion policy.
///
/// Policy (EP-016 Decision Log):
/// - Depth is bounded by `GraphExpansionMode` (DIRECT = seed only,
///   ONE_HOP = seed + immediate neighbors, TWO_HOP = two hops).
/// - Fanout per node is bounded by `max_neighbors` (deterministic:
///   edges are sorted by target id, then capped).
/// - Allowed relations are enforced per purpose; disallowed relation
///   types are never expanded.
/// - Cycles are handled with a visited set; no node is visited twice.
/// - Tenant boundary: edges/nodes whose tenant differs from the request
///   tenant are rejected/ignored.
/// - Sensitivity boundary: nodes above `max_sensitivity` are not
///   expanded through.
/// - The result is bounded by `max_nodes`; `bounded = true` when the
///   node budget cut the expansion short.
#[derive(Debug, Clone)]
pub struct DeterministicGraphExpansionPolicy<P> {
    pub provider: P,
    /// Maximum neighbors expanded per node (fanout cap).
    pub max_neighbors: usize,
    /// Allowed relation types for the request purpose. Empty means all
    /// relations are allowed (still tenant/sensitivity bounded).
    pub allowed_relations: Vec<String>,
    /// Sensitivity ceiling; nodes above it are not expanded through.
    pub max_sensitivity: Sensitivity,
}

/// Canonical purpose-aware relation allowlists (EP-016 Decision
/// Log; SPEC-002 behavior 7). A repair/device objective may expand
/// device -> room -> household -> recent incidents -> relevant
/// procedure, but never household -> every family member ->
/// unrelated private memory.
pub fn relations_for_purpose(purpose: nexus_context::ContextPurpose) -> Vec<String> {
    match purpose {
        nexus_context::ContextPurpose::TaskExecution => vec![
            "located_in".into(),
            "part_of".into(),
            "controls".into(),
            "incident_for".into(),
            "procedure_for".into(),
            "requires".into(),
        ],
        nexus_context::ContextPurpose::Planning => vec![
            "part_of".into(),
            "depends_on".into(),
            "requires".into(),
            "assigned_to".into(),
        ],
        nexus_context::ContextPurpose::Search => vec![],
        nexus_context::ContextPurpose::Notification => vec!["notifies".into()],
        nexus_context::ContextPurpose::SystemMaintenance => {
            vec!["part_of".into(), "requires".into(), "monitors".into()]
        }
    }
}

impl<P: GraphProvider> DeterministicGraphExpansionPolicy<P> {
    pub fn new(
        provider: P,
        max_neighbors: usize,
        allowed_relations: Vec<String>,
        max_sensitivity: Sensitivity,
    ) -> Self {
        Self {
            provider,
            max_neighbors: max_neighbors.max(1),
            allowed_relations,
            max_sensitivity,
        }
    }

    /// Delegates to the canonical purpose-aware relation table.
    pub fn relations_for_purpose(purpose: nexus_context::ContextPurpose) -> Vec<String> {
        relations_for_purpose(purpose)
    }

    /// Walk the bounded expansion deterministically.
    fn walk(&mut self, request: &GraphExpansionRequest) -> Result<GraphExpansion, ContextError> {
        let max_depth = match request.mode {
            GraphExpansionMode::Direct => 0,
            GraphExpansionMode::OneHop => 1,
            GraphExpansionMode::TwoHop => 2,
        };
        let ceiling = sensitivity_rank(self.max_sensitivity);
        let mut visited: HashSet<String> = HashSet::new();
        let mut nodes: Vec<GraphNodeRef> = Vec::new();
        let mut edges: Vec<GraphEdgeRef> = Vec::new();
        let mut bounded = false;

        let seed = self
            .provider
            .node(&request.tenant_id, &request.seed.node_id)?;
        let seed_info = match seed {
            Some(info) if info.tenant_id == request.tenant_id => info,
            _ => {
                return Err(ContextError::authorization(
                    "graph seed is outside the tenant boundary",
                    Some("graph-expansion".into()),
                ));
            }
        };
        if sensitivity_rank(seed_info.sensitivity) > ceiling {
            return Err(ContextError::policy(
                "graph seed exceeds the sensitivity boundary",
                Some("graph-expansion".into()),
            ));
        }
        visited.insert(seed_info.node_id.clone());
        nodes.push(GraphNodeRef {
            node_id: seed_info.node_id.clone(),
            node_type: seed_info.node_type.clone(),
        });

        let mut frontier: Vec<(String, usize)> = vec![(seed_info.node_id.clone(), 0)];
        while let Some((current, depth)) = frontier.pop() {
            if depth >= max_depth {
                continue;
            }
            if nodes.len() >= request.max_nodes {
                bounded = true;
                break;
            }
            let mut outgoing = self.provider.edges(&request.tenant_id, &current)?;
            // Deterministic fanout: sort by target id then cap.
            outgoing.sort_by(|a, b| a.target.cmp(&b.target));
            outgoing.truncate(self.max_neighbors);
            for edge in outgoing {
                if edge.tenant_id != request.tenant_id {
                    continue;
                }
                if !self.allowed_relations.is_empty()
                    && !self.allowed_relations.iter().any(|r| r == &edge.relation)
                {
                    continue;
                }
                if visited.contains(&edge.target) {
                    continue;
                }
                let target = match self.provider.node(&request.tenant_id, &edge.target)? {
                    Some(info) if info.tenant_id == request.tenant_id => info,
                    _ => continue,
                };
                if sensitivity_rank(target.sensitivity) > ceiling {
                    continue;
                }
                if nodes.len() >= request.max_nodes {
                    bounded = true;
                    break;
                }
                visited.insert(target.node_id.clone());
                nodes.push(GraphNodeRef {
                    node_id: target.node_id.clone(),
                    node_type: target.node_type.clone(),
                });
                edges.push(GraphEdgeRef {
                    source: edge.source,
                    relation: edge.relation,
                    target: edge.target,
                });
                frontier.push((target.node_id.clone(), depth + 1));
            }
            if bounded {
                break;
            }
        }

        Ok(GraphExpansion {
            nodes,
            edges,
            bounded,
        })
    }
}

impl<P: GraphProvider> GraphExpansionPolicy for DeterministicGraphExpansionPolicy<P> {
    fn expand(&mut self, request: &GraphExpansionRequest) -> Result<GraphExpansion, ContextError> {
        request.validate()?;
        self.walk(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_context::ContextPurpose;

    struct StubGraph {
        nodes: Vec<GraphNodeInfo>,
        edges: Vec<GraphEdgeInfo>,
    }

    impl GraphProvider for StubGraph {
        fn node(
            &mut self,
            _tenant_id: &str,
            node_id: &str,
        ) -> Result<Option<GraphNodeInfo>, ContextError> {
            Ok(self.nodes.iter().find(|n| n.node_id == node_id).cloned())
        }

        fn edges(
            &mut self,
            _tenant_id: &str,
            node_id: &str,
        ) -> Result<Vec<GraphEdgeInfo>, ContextError> {
            Ok(self
                .edges
                .iter()
                .filter(|e| e.source == node_id)
                .cloned()
                .collect())
        }
    }

    fn node(id: &str, tenant: &str, sensitivity: Sensitivity) -> GraphNodeInfo {
        GraphNodeInfo {
            node_id: id.into(),
            node_type: "ENTITY".into(),
            tenant_id: tenant.into(),
            sensitivity,
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    fn edge(source: &str, relation: &str, target: &str, tenant: &str) -> GraphEdgeInfo {
        GraphEdgeInfo {
            source: source.into(),
            relation: relation.into(),
            target: target.into(),
            tenant_id: tenant.into(),
        }
    }

    fn request(mode: GraphExpansionMode, max_nodes: usize) -> GraphExpansionRequest {
        GraphExpansionRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
            seed: GraphNodeRef {
                node_id: "device-1".into(),
                node_type: "DEVICE".into(),
            },
            mode,
            max_nodes,
        }
    }

    fn policy(graph: StubGraph) -> DeterministicGraphExpansionPolicy<StubGraph> {
        DeterministicGraphExpansionPolicy::new(
            graph,
            8,
            relations_for_purpose(ContextPurpose::TaskExecution),
            Sensitivity::Security,
        )
    }

    #[test]
    fn ep016_unit_graph_bounded_depth_one_hop() {
        let graph = StubGraph {
            nodes: vec![
                node("device-1", "t-1", Sensitivity::Household),
                node("room-1", "t-1", Sensitivity::Household),
                node("room-2", "t-1", Sensitivity::Household),
            ],
            edges: vec![
                edge("device-1", "located_in", "room-1", "t-1"),
                edge("device-1", "located_in", "room-2", "t-1"),
            ],
        };
        let mut p = policy(graph);
        let expansion = p.expand(&request(GraphExpansionMode::OneHop, 16)).unwrap();
        assert_eq!(expansion.nodes.len(), 3);
        assert!(!expansion.bounded);
    }

    #[test]
    fn ep016_unit_graph_depth_limit_enforced_two_hops() {
        let graph = StubGraph {
            nodes: vec![
                node("device-1", "t-1", Sensitivity::Household),
                node("room-1", "t-1", Sensitivity::Household),
                node("house-1", "t-1", Sensitivity::Household),
            ],
            edges: vec![
                edge("device-1", "located_in", "room-1", "t-1"),
                edge("room-1", "part_of", "house-1", "t-1"),
            ],
        };
        let mut p = policy(graph);
        let direct = p.expand(&request(GraphExpansionMode::Direct, 16)).unwrap();
        assert_eq!(direct.nodes.len(), 1);
        let one = p.expand(&request(GraphExpansionMode::OneHop, 16)).unwrap();
        assert_eq!(one.nodes.len(), 2);
        let two = p.expand(&request(GraphExpansionMode::TwoHop, 16)).unwrap();
        assert_eq!(two.nodes.len(), 3);
    }

    #[test]
    fn ep016_unit_graph_cycle_safe() {
        let graph = StubGraph {
            nodes: vec![
                node("a", "t-1", Sensitivity::Household),
                node("b", "t-1", Sensitivity::Household),
            ],
            edges: vec![
                edge("a", "part_of", "b", "t-1"),
                edge("b", "part_of", "a", "t-1"),
            ],
        };
        let mut p = policy(graph);
        let expansion = p
            .expand(&GraphExpansionRequest {
                seed: GraphNodeRef {
                    node_id: "a".into(),
                    node_type: "ENTITY".into(),
                },
                ..request(GraphExpansionMode::TwoHop, 16)
            })
            .unwrap();
        // a + b only; the cycle terminates (no infinite traversal).
        assert_eq!(expansion.nodes.len(), 2);
    }

    #[test]
    fn ep016_unit_graph_neighbor_limit_enforced() {
        let graph = StubGraph {
            nodes: vec![
                node("device-1", "t-1", Sensitivity::Household),
                node("n1", "t-1", Sensitivity::Household),
                node("n2", "t-1", Sensitivity::Household),
                node("n3", "t-1", Sensitivity::Household),
            ],
            edges: vec![
                edge("device-1", "part_of", "n1", "t-1"),
                edge("device-1", "part_of", "n2", "t-1"),
                edge("device-1", "part_of", "n3", "t-1"),
            ],
        };
        let mut p = DeterministicGraphExpansionPolicy::new(
            graph,
            2, // fanout cap
            relations_for_purpose(ContextPurpose::TaskExecution),
            Sensitivity::Security,
        );
        let expansion = p.expand(&request(GraphExpansionMode::OneHop, 16)).unwrap();
        // seed + 2 neighbors (deterministic sorted cap).
        assert_eq!(expansion.nodes.len(), 3);
    }

    #[test]
    fn ep016_unit_graph_cross_tenant_edge_rejected() {
        let graph = StubGraph {
            nodes: vec![
                node("device-1", "t-1", Sensitivity::Household),
                node("other", "t-2", Sensitivity::Household),
            ],
            edges: vec![edge("device-1", "part_of", "other", "t-2")],
        };
        let mut p = policy(graph);
        let expansion = p.expand(&request(GraphExpansionMode::OneHop, 16)).unwrap();
        assert_eq!(expansion.nodes.len(), 1);
        assert!(expansion.edges.is_empty());
    }

    #[test]
    fn ep016_unit_graph_disallowed_relation_not_expanded() {
        let graph = StubGraph {
            nodes: vec![
                node("device-1", "t-1", Sensitivity::Household),
                node("person", "t-1", Sensitivity::Personal),
            ],
            edges: vec![edge("device-1", "family_member_of", "person", "t-1")],
        };
        let mut p = policy(graph);
        let expansion = p.expand(&request(GraphExpansionMode::OneHop, 16)).unwrap();
        assert_eq!(expansion.nodes.len(), 1);
    }

    #[test]
    fn ep016_unit_graph_sensitivity_boundary_enforced() {
        let graph = StubGraph {
            nodes: vec![
                node("device-1", "t-1", Sensitivity::Household),
                node("secret", "t-1", Sensitivity::Secret),
            ],
            edges: vec![edge("device-1", "part_of", "secret", "t-1")],
        };
        let mut p = policy(graph);
        let expansion = p.expand(&request(GraphExpansionMode::OneHop, 16)).unwrap();
        assert_eq!(expansion.nodes.len(), 1);
    }

    #[test]
    fn ep016_unit_graph_node_budget_marks_bounded() {
        let graph = StubGraph {
            nodes: vec![
                node("device-1", "t-1", Sensitivity::Household),
                node("n1", "t-1", Sensitivity::Household),
                node("n2", "t-1", Sensitivity::Household),
                node("n3", "t-1", Sensitivity::Household),
            ],
            edges: vec![
                edge("device-1", "part_of", "n1", "t-1"),
                edge("device-1", "part_of", "n2", "t-1"),
                edge("device-1", "part_of", "n3", "t-1"),
            ],
        };
        let mut p = policy(graph);
        let expansion = p.expand(&request(GraphExpansionMode::OneHop, 3)).unwrap();
        assert_eq!(expansion.nodes.len(), 3);
        assert!(expansion.bounded);
    }

    #[test]
    fn ep016_unit_graph_purpose_aware_relation_sets() {
        // A device repair purpose allows device -> room -> household,
        // but not device -> family member (not in the allowlist).
        let repair = relations_for_purpose(ContextPurpose::TaskExecution);
        assert!(repair.iter().any(|r| r == "located_in"));
        assert!(repair.iter().any(|r| r == "procedure_for"));
        assert!(!repair.iter().any(|r| r == "family_member_of"));
        let notification = relations_for_purpose(ContextPurpose::Notification);
        assert_eq!(notification, vec!["notifies".to_string()]);
    }
}
