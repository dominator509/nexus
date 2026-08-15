//! Deterministic context engine (SPEC-002; EP-016 M2).
//!
//! Composes the full context construction pipeline behind the
//! provider-neutral `ContextEngine` port:
//!
//! ```text
//! AVAILABLE MEMORY
//!   -> PERMISSION FILTER (before scoring)
//!   -> PURPOSE FILTER
//!   -> ACTIVE/LIFECYCLE FILTER
//!   -> HYBRID RETRIEVAL
//!   -> DIVERSITY / RELEVANCE / CONFIDENCE
//!   -> PRIVACY / DISCLOSURE FILTER
//!   -> BUDGET ALLOCATION
//!   -> DETERMINISTIC ORDERING
//!   -> CONTEXT CAPSULE
//! ```
//!
//! Same inputs -> same selected memories -> same order -> same capsule.
//! No HashMap iteration nondeterminism: every ordering uses the
//! documented stable tie-breakers (score, timestamp, canonical memory
//! id).

use crate::budget::{BudgetClass, ContextBudget};
use crate::graph::GraphProvider;
use crate::hybrid::{CandidateProvider, HybridScorer, ScoredCandidate};
use crate::lifecycle::{ActiveMemoryLifecycleFilter, LifecycleContext};
use crate::permission::{AccessProfile, PermissionFilter};
use crate::privacy::{DeterministicPrivacyFilter, DisclosureContext};
use crate::purpose::PurposeLimiter;
use crate::telemetry::ContextTelemetry;
use crate::util::fnv1a64;
use nexus_context::{
    CapsuleId, CapsuleState, ContextCapsule, ContextEngine, ContextError, ContextPurpose,
    ContextRequest, GraphExpansionMode, GraphExpansionPolicy, GraphExpansionRequest, GraphNodeRef,
    PrivacyFilterDecision, RetrievalSignals,
};
use std::collections::HashMap;

/// Optional graph expansion binding for the engine: when configured, the
/// engine expands from the seed and includes the bounded subgraph in the
/// capsule payload (graph context class).
#[derive(Debug, Clone)]
pub struct GraphExpansionBinding<P> {
    pub provider: P,
    pub seed: GraphNodeRef,
    pub mode: GraphExpansionMode,
    pub max_nodes: usize,
    pub max_neighbors: usize,
}

/// Deterministic context engine.
#[derive(Debug, Clone)]
pub struct DeterministicContextEngine<P, G> {
    pub provider: P,
    pub access: AccessProfile,
    pub lifecycle: LifecycleContext,
    pub disclosure: DisclosureContext,
    pub signals: RetrievalSignals,
    /// Maximum candidates retained per diversity cluster.
    pub max_per_cluster: usize,
    /// Capsule TTL in seconds (injected policy; clock lives outside).
    pub ttl_secs: u64,
    /// Optional bounded graph expansion.
    pub graph: Option<GraphExpansionBinding<G>>,
}

impl<P: CandidateProvider, G: GraphProvider> DeterministicContextEngine<P, G> {
    pub fn new(
        provider: P,
        access: AccessProfile,
        lifecycle: LifecycleContext,
        disclosure: DisclosureContext,
    ) -> Self {
        Self {
            provider,
            access,
            lifecycle,
            disclosure,
            signals: RetrievalSignals::all(),
            max_per_cluster: 2,
            ttl_secs: 300,
            graph: None,
        }
    }

    /// Deterministic capsule id from the request identity.
    fn capsule_id(&self, request: &ContextRequest) -> CapsuleId {
        let source = format!(
            "{}:{}:{}:{}",
            request.tenant_id, request.principal_id, request.task_id, request.request_id
        );
        CapsuleId(format!("cap-{:016x}", fnv1a64(&source)))
    }

    /// Run the deterministic pipeline and build the capsule.
    pub fn build(&mut self, request: &ContextRequest) -> Result<ContextCapsule, ContextError> {
        request.validate()?;
        let budget = ContextBudget::allocate(request.max_items)?;
        let purpose = PurposeLimiter::policy_for(request.purpose);
        let mut telemetry = ContextTelemetry::new(&request.correlation_id, request.purpose);

        // Stage 1-3: permission, purpose, lifecycle. Unauthorized or
        // out-of-purpose or inactive candidates never enter scoring.
        let raw = match &request.query {
            Some(query) => self.provider.fetch(&request.tenant_id, query)?,
            None => vec![],
        };
        let raw_candidates: Vec<_> = raw.iter().map(|(c, _)| c.clone()).collect();
        let permission = PermissionFilter.filter(&self.access, raw_candidates.clone())?;
        let purpose_filtered = PurposeLimiter.filter(&purpose, permission)?;
        let lifecycle_filtered =
            ActiveMemoryLifecycleFilter.filter(&self.lifecycle, purpose_filtered)?;

        let mut telemetry_namespaces: Vec<String> = Vec::new();
        for candidate in &lifecycle_filtered {
            telemetry_namespaces.push(candidate.record.namespace.clone());
        }

        // Stage 4: hybrid scoring with auditable components. Only
        // authorized, in-purpose, active candidates are scored.
        let mut scored: Vec<ScoredCandidate> = Vec::new();
        let mut semantic_available = true;
        for (candidate, provider_signals) in raw {
            if !lifecycle_filtered
                .iter()
                .any(|c| c.record.memory_id == candidate.record.memory_id)
            {
                continue;
            }
            let item = HybridScorer.score(&self.signals, candidate, provider_signals)?;
            if !item.components.semantic_available {
                semantic_available = false;
            }
            scored.push(item);
        }

        let mut signal_classes: Vec<&str> = Vec::new();
        if self.signals.exact {
            signal_classes.push("exact");
        }
        if self.signals.full_text {
            signal_classes.push("full_text");
        }
        if self.signals.vector {
            signal_classes.push("vector");
        }
        if self.signals.graph {
            signal_classes.push("graph");
        }
        telemetry.with_signal_classes(&signal_classes);
        telemetry.with_namespaces(&telemetry_namespaces);

        // Stage 5: deterministic order + diversity clustering.
        scored.sort_by(HybridScorer::order);
        let mut deduped: Vec<ScoredCandidate> = Vec::new();
        let mut cluster_counts: HashMap<String, usize> = HashMap::new();
        for item in scored {
            let key = item
                .candidate
                .record
                .supersedes
                .as_ref()
                .map(|id| format!("chain:{}", id.as_str()))
                .unwrap_or_else(|| {
                    let cluster = item
                        .candidate
                        .record
                        .derived_from
                        .first()
                        .map(|id| id.as_str().to_string());
                    cluster.unwrap_or_else(|| item.candidate.record.memory_id.as_str().to_string())
                });
            let count = cluster_counts.entry(key).or_insert(0);
            if *count >= self.max_per_cluster {
                continue;
            }
            *count += 1;
            deduped.push(item);
        }

        // Stage 6: privacy / disclosure filter. Shared-room requests
        // exclude private/sensitive memories; private channels may
        // include them per permission.
        let privacy =
            DeterministicPrivacyFilter::new(self.access.clone(), purpose.clone(), self.disclosure);
        let mut selected: Vec<ScoredCandidate> = Vec::new();
        for item in deduped {
            let filtered = privacy.filter_with_disclosure(
                &request.tenant_id,
                &request.principal_id,
                request.purpose,
                vec![item.candidate.clone()],
            )?;
            match filtered.first() {
                Some(f) => {
                    telemetry.record_privacy(f.decision);
                    if f.decision == PrivacyFilterDecision::Allow {
                        selected.push(item);
                    }
                }
                None => {
                    telemetry.record_privacy(PrivacyFilterDecision::Deny);
                }
            }
        }

        // Stage 7: budget allocation by priority class. Required exact
        // facts crowd out low-value retrieved memories.
        let mut allocated: Vec<ScoredCandidate> = Vec::new();
        for class in BudgetClass::ALL {
            let cap = budget.cap(class);
            let mut taken = 0usize;
            let mut remainder: Vec<ScoredCandidate> = Vec::new();
            for item in selected {
                if taken >= cap {
                    remainder.push(item);
                    continue;
                }
                if self.in_class(class, &item, request.purpose) {
                    allocated.push(item);
                    taken += 1;
                } else {
                    remainder.push(item);
                }
            }
            selected = remainder;
        }

        // Stage 8: deterministic ordering (already applied by
        // HybridScorer::order; re-sort the selected set for stability).
        allocated.sort_by(HybridScorer::order);

        // Optional graph expansion (bounded, purpose-aware).
        let mut graph_nodes: Vec<serde_json::Value> = Vec::new();
        let mut graph_edges: Vec<serde_json::Value> = Vec::new();
        let mut graph_depth = 0usize;
        if let Some(binding) = &mut self.graph {
            let purpose = request.purpose;
            let graph_request = GraphExpansionRequest {
                request_id: request.request_id.clone(),
                correlation_id: request.correlation_id.clone(),
                tenant_id: request.tenant_id.clone(),
                principal_id: request.principal_id.clone(),
                seed: binding.seed.clone(),
                mode: binding.mode,
                max_nodes: binding.max_nodes,
            };
            let relations = crate::graph::relations_for_purpose(purpose);
            let mut policy = crate::graph::DeterministicGraphExpansionPolicy::new(
                &mut binding.provider,
                binding.max_neighbors,
                relations,
                self.access.max_sensitivity,
            );
            let expansion = policy.expand(&graph_request)?;
            graph_depth = match binding.mode {
                GraphExpansionMode::Direct => 0,
                GraphExpansionMode::OneHop => 1,
                GraphExpansionMode::TwoHop => 2,
            };
            for node in expansion.nodes {
                graph_nodes.push(serde_json::json!({
                    "node_id": node.node_id,
                    "node_type": node.node_type,
                }));
            }
            for edge in expansion.edges {
                graph_edges.push(serde_json::json!({
                    "source": edge.source,
                    "relation": edge.relation,
                    "target": edge.target,
                }));
            }
        }
        telemetry.graph_depth = graph_depth;
        telemetry.candidate_count = raw_candidates.len();
        telemetry.selected_count = allocated.len();

        // Build the capsule payload with full provenance for every item.
        let mut payload_items: Vec<serde_json::Value> = Vec::new();
        let mut citations: Vec<String> = Vec::new();
        for item in &allocated {
            let record = &item.candidate.record;
            citations.push(record.memory_id.as_str().to_string());
            let derived: Vec<String> = record
                .derived_from
                .iter()
                .map(|id| id.as_str().to_string())
                .collect();
            let supersedes = record.supersedes.as_ref().map(|id| id.as_str().to_string());
            let c = &item.components;
            let components = serde_json::json!({
                "exact": c.exact,
                "full_text": c.full_text,
                "vector": c.vector,
                "graph": c.graph,
                "recency": c.recency,
                "importance": c.importance,
                "confidence": c.confidence,
            });
            payload_items.push(serde_json::json!({
                "memory_id": record.memory_id.as_str(),
                "namespace": record.namespace,
                "memory_type": record.memory_type.as_str(),
                "sensitivity": record.sensitivity.as_str(),
                "confidence": record.confidence,
                "observed_at": record.observed_at,
                "source": record.source,
                "actor": record.actor,
                "derived_from": derived,
                "supersedes": supersedes,
                "retrieval_total": item.total,
                "components": components,
                "content": record.content,
            }));
        }

        let payload = serde_json::json!({
            "purpose": request.purpose.as_str(),
            "task_id": request.task_id,
            "semantic_available": semantic_available,
            "telemetry": telemetry.redacted_json(),
            "items": payload_items,
            "graph": {
                "depth": graph_depth,
                "nodes": graph_nodes,
                "edges": graph_edges,
            },
        });

        let now = self.lifecycle.now_epoch_ms;
        let capsule = ContextCapsule {
            capsule_id: self.capsule_id(request),
            task_id: request.task_id.clone(),
            tenant_id: request.tenant_id.clone(),
            principal_id: request.principal_id.clone(),
            citations,
            payload,
            created_at_epoch_ms: now,
            expires_at_epoch_ms: now.saturating_add(self.ttl_secs.saturating_mul(1_000)),
            state: CapsuleState::Active,
        };
        Ok(capsule)
    }

    /// Deterministic budget classification (EP-016 Decision Log).
    fn in_class(
        &self,
        class: BudgetClass,
        item: &ScoredCandidate,
        purpose: ContextPurpose,
    ) -> bool {
        match class {
            BudgetClass::RequiredExact => item.components.exact > 0.0,
            BudgetClass::ObjectiveState => {
                matches!(
                    item.candidate.record.memory_type,
                    nexus_domain::MemoryType::Decision
                        | nexus_domain::MemoryType::Working
                        | nexus_domain::MemoryType::System
                )
            }
            BudgetClass::CriticalRecent => {
                item.components.recency >= 0.8 && item.components.importance >= 0.7
            }
            BudgetClass::HighValueRetrieved => item.total >= 0.5,
            BudgetClass::GraphContext => false, // graph handled separately
            BudgetClass::OptionalSemantic => {
                item.components.semantic_available
                    && !matches!(purpose, ContextPurpose::Notification)
            }
        }
    }
}

impl<P: CandidateProvider, G: GraphProvider> ContextEngine for DeterministicContextEngine<P, G> {
    fn build_context(&mut self, request: &ContextRequest) -> Result<ContextCapsule, ContextError> {
        self.build(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hybrid::CandidateSignals;
    use crate::util::rfc3339_utc_millis;
    use nexus_context::ContextErrorCode;
    use nexus_data::memory::{
        MemoryCandidate, MemoryQuery, MemoryRecord, MemoryStatus, RetentionPolicy, RetentionUnit,
        Sensitivity,
    };
    use nexus_domain::{MemoryType, NexusId, TenantId};

    fn record(id_byte: u8, namespace: &str, observed_at: &str) -> MemoryCandidate {
        MemoryCandidate {
            record: MemoryRecord {
                memory_id: NexusId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a{id_byte:02x}"))
                    .unwrap(),
                tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a80").unwrap(),
                namespace: namespace.into(),
                memory_type: MemoryType::Semantic,
                content: serde_json::json!({ "fact": id_byte }),
                content_hash: format!("{:064x}", id_byte),
                source: "test".into(),
                actor: "p-1".into(),
                created_at: observed_at.into(),
                observed_at: observed_at.into(),
                confidence: 0.9,
                sensitivity: Sensitivity::Household,
                purpose: "SEARCH".into(),
                retention: RetentionPolicy::for_duration(RetentionUnit::Days, 90),
                status: MemoryStatus::Active,
                derived_from: vec![],
                supersedes: None,
                embedding_ref: None,
            },
            score: 0.9,
        }
    }

    #[derive(Debug, Clone)]
    struct StubProvider(Vec<(MemoryCandidate, CandidateSignals)>);

    impl CandidateProvider for StubProvider {
        fn fetch(
            &mut self,
            _tenant_id: &str,
            _query: &MemoryQuery,
        ) -> Result<Vec<(MemoryCandidate, CandidateSignals)>, ContextError> {
            Ok(self.0.clone())
        }
    }

    fn profile() -> AccessProfile {
        AccessProfile {
            tenant_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a80".into(),
            principal_id: "p-1".into(),
            allowed_namespaces: vec!["household".into(), "personal".into()],
            max_sensitivity: Sensitivity::Sensitive,
            private_allowed: true,
        }
    }

    fn signals(exact: bool) -> CandidateSignals {
        CandidateSignals {
            exact,
            full_text: 0.5,
            vector: Some(0.4),
            graph: None,
            recency: 0.5,
            importance: 0.5,
            diversity_key: String::new(),
        }
    }

    fn request(purpose: ContextPurpose, max_items: usize) -> ContextRequest {
        ContextRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            tenant_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a80".into(),
            principal_id: "p-1".into(),
            task_id: "task-1".into(),
            purpose,
            query: Some(MemoryQuery::default()),
            required_capabilities: vec![],
            max_items,
        }
    }

    fn engine(
        provider: StubProvider,
        disclosure: DisclosureContext,
    ) -> DeterministicContextEngine<StubProvider, StubGraph> {
        let mut engine = DeterministicContextEngine::new(
            provider,
            profile(),
            LifecycleContext {
                now_epoch_ms: rfc3339_utc_millis("2026-01-01T00:00:00Z").unwrap(),
                include_historical: false,
            },
            disclosure,
        );
        engine.max_per_cluster = 2;
        engine
    }

    struct StubGraph;
    impl GraphProvider for StubGraph {
        fn node(
            &mut self,
            _tenant_id: &str,
            node_id: &str,
        ) -> Result<Option<crate::graph::GraphNodeInfo>, ContextError> {
            Ok(Some(crate::graph::GraphNodeInfo {
                node_id: node_id.into(),
                node_type: "ENTITY".into(),
                tenant_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a80".into(),
                sensitivity: Sensitivity::Household,
                updated_at: "2026-01-01T00:00:00Z".into(),
            }))
        }
        fn edges(
            &mut self,
            _tenant_id: &str,
            _node_id: &str,
        ) -> Result<Vec<crate::graph::GraphEdgeInfo>, ContextError> {
            Ok(vec![])
        }
    }

    #[test]
    fn ep016_unit_engine_builds_capsule_with_provenance() {
        let provider = StubProvider(vec![
            (
                record(0x01, "household", "2025-12-01T00:00:00Z"),
                signals(true),
            ),
            (
                record(0x02, "household", "2025-12-02T00:00:00Z"),
                signals(false),
            ),
        ]);
        let mut e = engine(provider, DisclosureContext::PrivateChannel);
        let capsule = e
            .build_context(&request(ContextPurpose::Search, 20))
            .unwrap();
        assert_eq!(capsule.state, CapsuleState::Active);
        assert_eq!(capsule.citations.len(), 2);
        assert_eq!(capsule.payload["items"].as_array().unwrap().len(), 2);
        let first = &capsule.payload["items"][0];
        assert!(first["memory_id"].is_string());
        assert!(first["sensitivity"].is_string());
        assert!(first["confidence"].is_number());
        assert!(first["source"].is_string());
        assert!(first["components"]["exact"].is_number());
    }

    #[test]
    fn ep016_unit_engine_permission_before_scoring() {
        // Tenant B memory must never be scored; the capsule contains
        // only tenant A items even with high provider scores.
        let mut other = record(0x03, "household", "2025-12-01T00:00:00Z");
        other.record.tenant_id = TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8b01").unwrap();
        let provider = StubProvider(vec![
            (
                record(0x01, "household", "2025-12-01T00:00:00Z"),
                signals(true),
            ),
            (other, signals(true)),
        ]);
        let mut e = engine(provider, DisclosureContext::PrivateChannel);
        let capsule = e
            .build_context(&request(ContextPurpose::Search, 20))
            .unwrap();
        assert_eq!(capsule.citations.len(), 1);
    }

    #[test]
    fn ep016_unit_engine_shared_room_excludes_sensitive() {
        let mut sensitive = record(0x04, "personal", "2025-12-01T00:00:00Z");
        sensitive.record.sensitivity = Sensitivity::Sensitive;
        let provider = StubProvider(vec![
            (
                record(0x01, "household", "2025-12-01T00:00:00Z"),
                signals(false),
            ),
            (sensitive, signals(true)),
        ]);
        let mut e = engine(provider, DisclosureContext::SharedRoom);
        let capsule = e
            .build_context(&request(ContextPurpose::Search, 20))
            .unwrap();
        assert_eq!(capsule.citations.len(), 1);
        assert_eq!(capsule.citations[0], "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a01");
        let telemetry = &capsule.payload["telemetry"];
        assert!(
            telemetry["privacy_decisions"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d == "DENY")
        );
    }

    #[test]
    fn ep016_unit_engine_budget_caps_items() {
        let provider = StubProvider(
            (0x01..=0x0a)
                .map(|b| {
                    (
                        record(b, "household", "2025-12-01T00:00:00Z"),
                        signals(false),
                    )
                })
                .collect(),
        );
        let mut e = engine(provider, DisclosureContext::PrivateChannel);
        let capsule = e
            .build_context(&request(ContextPurpose::Search, 3))
            .unwrap();
        assert!(capsule.citations.len() <= 3);
    }

    #[test]
    fn ep016_unit_engine_deterministic_same_inputs_same_capsule() {
        let provider = StubProvider(vec![
            (
                record(0x01, "household", "2025-12-01T00:00:00Z"),
                signals(true),
            ),
            (
                record(0x02, "household", "2025-12-02T00:00:00Z"),
                signals(false),
            ),
        ]);
        let mut a = engine(provider.clone(), DisclosureContext::PrivateChannel);
        let mut b = engine(provider, DisclosureContext::PrivateChannel);
        let ca = a
            .build_context(&request(ContextPurpose::Search, 20))
            .unwrap();
        let cb = b
            .build_context(&request(ContextPurpose::Search, 20))
            .unwrap();
        assert_eq!(ca.capsule_id, cb.capsule_id);
        assert_eq!(ca.citations, cb.citations);
        assert_eq!(ca.payload, cb.payload);
    }

    #[test]
    fn ep016_unit_engine_required_exact_crowds_out_low_value() {
        // Exact match must survive a tight budget while low-value
        // retrieved memories are dropped.
        let provider = StubProvider(vec![
            (
                record(0x01, "household", "2025-12-01T00:00:00Z"),
                signals(true),
            ),
            (
                record(0x02, "household", "2025-12-02T00:00:00Z"),
                CandidateSignals {
                    exact: false,
                    full_text: 0.1,
                    vector: Some(0.1),
                    graph: None,
                    recency: 0.1,
                    importance: 0.1,
                    diversity_key: String::new(),
                },
            ),
        ]);
        let mut e = engine(provider, DisclosureContext::PrivateChannel);
        let capsule = e
            .build_context(&request(ContextPurpose::Search, 1))
            .unwrap();
        assert_eq!(capsule.citations.len(), 1);
        assert_eq!(capsule.citations[0], "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a01");
    }

    #[test]
    fn ep016_unit_engine_typed_error_invalid_budget() {
        let provider = StubProvider(vec![]);
        let mut e = engine(provider, DisclosureContext::PrivateChannel);
        let err = e
            .build_context(&request(ContextPurpose::Search, 0))
            .unwrap_err();
        assert_eq!(err.code, ContextErrorCode::Validation);
    }
}
