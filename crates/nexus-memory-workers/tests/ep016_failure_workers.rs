//! EP-016 M4 failure and abuse suite for the context and memory worker
//! plane (SPEC-002, SPEC-006, SPEC-020; ADR-023).
//!
//! Every test exercises a REAL failure mechanism against the production
//! workers: unavailable dependency (vector repository absent), provider
//! failure (fail closed), malformed request (structured validation
//! error), denied permission (excluded before scoring), shared-room
//! disclosure denial, budget exhaustion (required-exact floor), timeout
//! semantics via semantic adapter failure (deterministic fallback, never
//! simulated), duplicate request (idempotency), partial side effect
//! (conservative merge), cancelled/unbounded work (graph cycle bounded),
//! redacted errors, and redacted telemetry. The injected ports script
//! the failure; the worker under proof is never mocked.
//!
//! All errors are typed SPEC-006 `ContextError` codes with redacted
//! messages; no memory content or credential is ever asserted or
//! logged.

use nexus_context::{
    ConsolidationMode, ConsolidationRequest, ContextError, ContextErrorCode, ContextPurpose,
    ContextRequest, GraphExpansionMode, GraphExpansionPolicy, GraphExpansionRequest, GraphNodeRef,
    PrivacyFilterDecision,
};
use nexus_data::memory::{
    MemoryCandidate, MemoryQuery, MemoryRecord, MemoryStatus, RetentionPolicy, RetentionUnit,
    Sensitivity,
};
use nexus_domain::{MemoryType, NexusId, TenantId};
use nexus_memory_workers::budget::ContextBudget;
use nexus_memory_workers::consolidation::{
    DeterministicMemoryConsolidator, SemanticConsolidator, SourceProvider,
};
use nexus_memory_workers::engine::DeterministicContextEngine;
use nexus_memory_workers::graph::{
    DeterministicGraphExpansionPolicy, GraphEdgeInfo, GraphNodeInfo, GraphProvider,
    relations_for_purpose,
};
use nexus_memory_workers::hybrid::{CandidateProvider, CandidateSignals};
use nexus_memory_workers::lifecycle::LifecycleContext;
use nexus_memory_workers::permission::AccessProfile;
use nexus_memory_workers::privacy::{DeterministicPrivacyFilter, DisclosureContext};
use nexus_memory_workers::telemetry::ContextTelemetry;
use nexus_memory_workers::util::rfc3339_utc_millis;

const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a80";
const OTHER_TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a99";

// ---------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------

fn record(id_byte: u8, namespace: &str, tenant: &str, sensitivity: Sensitivity) -> MemoryCandidate {
    MemoryCandidate {
        record: MemoryRecord {
            memory_id: NexusId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a{id_byte:02x}"))
                .unwrap(),
            tenant_id: TenantId::new(tenant).unwrap(),
            namespace: namespace.into(),
            memory_type: MemoryType::Semantic,
            content: serde_json::json!({ "fact": id_byte }),
            content_hash: format!("{:064x}", id_byte),
            source: "test".into(),
            actor: "p-1".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            observed_at: "2026-01-01T00:00:00Z".into(),
            confidence: 0.9,
            sensitivity,
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
struct StubProvider {
    items: Vec<(MemoryCandidate, CandidateSignals)>,
    error: Option<ContextError>,
}

impl StubProvider {
    fn ok(items: Vec<(MemoryCandidate, CandidateSignals)>) -> Self {
        Self { items, error: None }
    }

    fn failing(error: ContextError) -> Self {
        Self {
            items: vec![],
            error: Some(error),
        }
    }
}

impl CandidateProvider for StubProvider {
    fn fetch(
        &mut self,
        _tenant_id: &str,
        _query: &MemoryQuery,
    ) -> Result<Vec<(MemoryCandidate, CandidateSignals)>, ContextError> {
        if let Some(error) = self.error.take() {
            return Err(error);
        }
        Ok(self.items.clone())
    }
}

fn signals(exact: bool, vector: Option<f64>) -> CandidateSignals {
    CandidateSignals {
        exact,
        full_text: 0.5,
        vector,
        graph: None,
        recency: 0.5,
        importance: 0.5,
        diversity_key: String::new(),
    }
}

fn profile() -> AccessProfile {
    AccessProfile {
        tenant_id: TENANT.into(),
        principal_id: "p-1".into(),
        allowed_namespaces: vec!["household".into(), "personal".into()],
        max_sensitivity: Sensitivity::Sensitive,
        private_allowed: true,
    }
}

fn lifecycle() -> LifecycleContext {
    LifecycleContext {
        now_epoch_ms: rfc3339_utc_millis("2026-01-01T00:00:00Z").unwrap(),
        include_historical: false,
    }
}

fn request(purpose: ContextPurpose, max_items: usize, request_id: &str) -> ContextRequest {
    ContextRequest {
        request_id: request_id.into(),
        correlation_id: "c-1".into(),
        tenant_id: TENANT.into(),
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
    DeterministicContextEngine::new(provider, profile(), lifecycle(), disclosure)
}

struct StubGraph;
impl GraphProvider for StubGraph {
    fn node(
        &mut self,
        _tenant_id: &str,
        node_id: &str,
    ) -> Result<Option<GraphNodeInfo>, ContextError> {
        Ok(Some(GraphNodeInfo {
            node_id: node_id.into(),
            node_type: "ENTITY".into(),
            tenant_id: TENANT.into(),
            sensitivity: Sensitivity::Household,
            updated_at: "2026-01-01T00:00:00Z".into(),
        }))
    }

    fn edges(
        &mut self,
        _tenant_id: &str,
        _node_id: &str,
    ) -> Result<Vec<GraphEdgeInfo>, ContextError> {
        Ok(vec![])
    }
}

struct ExplodingGraph;

impl GraphProvider for ExplodingGraph {
    fn node(
        &mut self,
        _tenant_id: &str,
        node_id: &str,
    ) -> Result<Option<GraphNodeInfo>, ContextError> {
        Ok(Some(GraphNodeInfo {
            node_id: node_id.into(),
            node_type: "ENTITY".into(),
            tenant_id: TENANT.into(),
            sensitivity: Sensitivity::Household,
            updated_at: "2026-01-01T00:00:00Z".into(),
        }))
    }

    fn edges(
        &mut self,
        _tenant_id: &str,
        node_id: &str,
    ) -> Result<Vec<GraphEdgeInfo>, ContextError> {
        // Deliberate self-loop (cycle) plus a widening fanout of fresh
        // targets: an unguarded walk would never terminate, and a
        // visited-set-less walk would loop on the self-loop forever.
        let mut out = vec![GraphEdgeInfo {
            source: node_id.into(),
            relation: "part_of".into(),
            target: node_id.into(),
            tenant_id: TENANT.into(),
        }];
        for i in 0..8 {
            out.push(GraphEdgeInfo {
                source: node_id.into(),
                relation: "part_of".into(),
                target: format!("{node_id}-{i}"),
                tenant_id: TENANT.into(),
            });
        }
        Ok(out)
    }
}

#[derive(Clone)]
struct StubSources {
    records: Vec<MemoryRecord>,
}

impl SourceProvider for StubSources {
    fn fetch(
        &mut self,
        _tenant_id: &str,
        _memory_ids: &[String],
    ) -> Result<Vec<MemoryRecord>, ContextError> {
        Ok(self.records.clone())
    }
}

fn source_record(id_byte: u8, confidence: f64, sensitivity: Sensitivity) -> MemoryRecord {
    let mut candidate = record(id_byte, "personal", TENANT, sensitivity);
    candidate.record.confidence = confidence;
    candidate.record
}

struct StubSemantic {
    result: Result<Option<serde_json::Value>, ContextError>,
}

impl SemanticConsolidator for StubSemantic {
    fn consolidate(
        &mut self,
        _sources: &[MemoryRecord],
    ) -> Result<Option<serde_json::Value>, ContextError> {
        self.result.clone()
    }
}

fn consolidation_request(source_ids: Vec<String>) -> ConsolidationRequest {
    ConsolidationRequest {
        request_id: "cr-1".into(),
        correlation_id: "c-1".into(),
        tenant_id: TENANT.into(),
        principal_id: "p-1".into(),
        source_memory_ids: source_ids,
        target_type: MemoryType::Semantic,
        sensitivity: Sensitivity::Sensitive,
        purpose: ContextPurpose::Planning,
        retention: RetentionPolicy::for_duration(RetentionUnit::Days, 90),
    }
}

// ---------------------------------------------------------------------
// Unavailable dependency: vector repository absent
// ---------------------------------------------------------------------

#[test]
fn ep016_failure_vector_unavailable_renormalizes_without_synthetic_score() {
    // The vector repository is absent (None), the exact signal is
    // present: the engine must still produce a capsule and must NOT
    // invent a synthetic embedding score.
    let mut engine = engine(
        StubProvider::ok(vec![(
            record(0x01, "household", TENANT, Sensitivity::Household),
            signals(true, None),
        )]),
        DisclosureContext::PrivateChannel,
    );
    let capsule = engine
        .build(&request(ContextPurpose::Search, 5, "r-1"))
        .unwrap();
    assert_eq!(
        capsule.citations.len(),
        1,
        "vector absence must not empty the capsule"
    );
    assert!(
        capsule.payload.to_string().contains("\"vector\":null")
            || !capsule.payload.to_string().contains("\"vector\":"),
        "no synthetic vector score may appear in the capsule"
    );
}

#[test]
fn ep016_failure_provider_unavailable_fails_closed() {
    // The candidate repository is down: the engine fails closed with a
    // typed UNAVAILABLE error; no capsule is produced.
    let mut engine = engine(
        StubProvider::failing(ContextError::new(
            ContextErrorCode::Unavailable,
            "candidate repository unavailable",
            Some("c-1".into()),
            Some("p-1".into()),
            Some(TENANT.into()),
            Some("candidate-provider".into()),
        )),
        DisclosureContext::PrivateChannel,
    );
    let error = engine
        .build(&request(ContextPurpose::Search, 5, "r-2"))
        .unwrap_err();
    assert_eq!(error.code, ContextErrorCode::Unavailable);
    assert_eq!(error.code.as_str(), "UNAVAILABLE");
}

#[test]
fn ep016_failure_malformed_request_validation_fails_closed() {
    // Malformed input: an empty request_id is rejected by validation
    // before any retrieval or scoring occurs.
    let mut engine = engine(StubProvider::ok(vec![]), DisclosureContext::PrivateChannel);
    let error = engine
        .build(&request(ContextPurpose::Search, 5, ""))
        .unwrap_err();
    assert_eq!(error.code, ContextErrorCode::Validation);
    assert_eq!(error.code.as_str(), "VALIDATION");
}

#[test]
fn ep016_failure_zero_budget_rejected_fails_closed() {
    // Resource abuse: a zero-item budget is rejected, never silently
    // treated as unbounded.
    let error = ContextBudget::allocate(0).unwrap_err();
    assert_eq!(error.code, ContextErrorCode::Validation);
}

// ---------------------------------------------------------------------
// Denied permission: excluded before scoring
// ---------------------------------------------------------------------

#[test]
fn ep016_failure_cross_tenant_excluded_before_scoring() {
    // Cross-tenant memory must never enter the capsule: permission is
    // enforced before any scoring, so the foreign record is absent from
    // citations even though its provider score is high.
    let foreign = record(0x02, "household", OTHER_TENANT, Sensitivity::Household);
    let own = record(0x03, "household", TENANT, Sensitivity::Household);
    let mut engine = engine(
        StubProvider::ok(vec![
            (foreign.clone(), signals(true, Some(0.9))),
            (own.clone(), signals(true, Some(0.9))),
        ]),
        DisclosureContext::PrivateChannel,
    );
    let capsule = engine
        .build(&request(ContextPurpose::Search, 5, "r-3"))
        .unwrap();
    assert_eq!(capsule.citations.len(), 1);
    assert!(
        !capsule
            .citations
            .iter()
            .any(|c| c == foreign.record.memory_id.as_str()),
        "cross-tenant memory must be excluded before scoring"
    );
    assert!(
        capsule
            .citations
            .iter()
            .any(|c| c == own.record.memory_id.as_str())
    );
}

#[test]
fn ep016_failure_shared_room_denies_sensitive_above_ceiling() {
    // Shared-room disclosure: a SENSITIVE memory is allowed on a
    // private channel but denied in a shared room, regardless of
    // relevance. Presence is never authority.
    let sensitive = record(0x04, "personal", TENANT, Sensitivity::Sensitive);
    let filter = DeterministicPrivacyFilter::new(
        profile(),
        nexus_memory_workers::purpose::PurposeLimiter::policy_for(ContextPurpose::Search),
        DisclosureContext::SharedRoom,
    );
    let filtered = filter
        .filter_with_disclosure(
            TENANT,
            "p-1",
            ContextPurpose::Search,
            vec![sensitive.clone()],
        )
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].decision, PrivacyFilterDecision::Deny);

    let filter_private = DeterministicPrivacyFilter::new(
        profile(),
        nexus_memory_workers::purpose::PurposeLimiter::policy_for(ContextPurpose::Search),
        DisclosureContext::PrivateChannel,
    );
    let filtered_private = filter_private
        .filter_with_disclosure(TENANT, "p-1", ContextPurpose::Search, vec![sensitive])
        .unwrap();
    assert_eq!(filtered_private[0].decision, PrivacyFilterDecision::Allow);
}

#[test]
fn ep016_failure_routing_decision_recorded_delivery_not_owned() {
    // A shared-room request that needs sensitive memory records a
    // private routing decision but never asserts delivery happened.
    let filter = DeterministicPrivacyFilter::new(
        profile(),
        nexus_memory_workers::purpose::PurposeLimiter::policy_for(ContextPurpose::Search),
        DisclosureContext::SharedRoom,
    );
    let decision = filter.routing_decision(true);
    assert!(decision.private_route);
    assert!(!decision.delivery_owned, "delivery is not owned by EP-016");
}

// ---------------------------------------------------------------------
// Budget exhaustion and required-exact floor
// ---------------------------------------------------------------------

#[test]
fn ep016_failure_budget_flood_required_exact_not_crowded_out() {
    // Abuse: a flood of low-value retrieved memories must not crowd out
    // the single required exact fact when the budget is tiny.
    let mut items = Vec::new();
    for i in 0x10..0x20 {
        items.push((
            record(i, "household", TENANT, Sensitivity::Household),
            signals(false, Some(0.99)),
        ));
    }
    items.push((
        record(0x30, "household", TENANT, Sensitivity::Household),
        signals(true, Some(0.1)),
    ));
    let mut engine = engine(StubProvider::ok(items), DisclosureContext::PrivateChannel);
    let capsule = engine
        .build(&request(ContextPurpose::Search, 1, "r-4"))
        .unwrap();
    assert_eq!(capsule.citations.len(), 1, "budget must cap the capsule");
    assert!(
        capsule.citations.iter().any(|c| c.ends_with("30")),
        "the required exact fact must survive the flood"
    );
}

// ---------------------------------------------------------------------
// Duplicate request and partial side effects
// ---------------------------------------------------------------------

#[test]
fn ep016_failure_duplicate_consolidation_idempotent() {
    // Duplicate request: the identical consolidation request emits a
    // proposal exactly once; the second call is a no-op (idempotency).
    // No duplicate canonical mutation can occur from this worker.
    let sources = StubSources {
        records: vec![
            source_record(0x05, 0.9, Sensitivity::Household),
            source_record(0x06, 0.8, Sensitivity::Household),
        ],
    };
    let mut consolidator = DeterministicMemoryConsolidator::new(sources, None);
    let request = consolidation_request(vec!["m-05".into(), "m-06".into()]);

    let first = consolidator.consolidate_evaluated(&request).unwrap();
    assert_eq!(first.proposals.len(), 1);
    assert_eq!(first.proposals[0].record.status, MemoryStatus::Proposed);

    let second = consolidator.consolidate_evaluated(&request).unwrap();
    assert_eq!(
        second.proposals.len(),
        0,
        "duplicate request must be idempotent"
    );
}

#[test]
fn ep016_failure_consolidation_partial_sources_conservative_merge() {
    // Partial side effect: two sources with different confidence and
    // sensitivity merge conservatively (confidence = min, sensitivity
    // never above source max nor request ceiling, provenance preserved).
    let sources = StubSources {
        records: vec![
            source_record(0x07, 0.9, Sensitivity::Household),
            source_record(0x08, 0.4, Sensitivity::Sensitive),
        ],
    };
    let mut consolidator = DeterministicMemoryConsolidator::new(sources, None);
    let request = consolidation_request(vec!["m-07".into(), "m-08".into()]);
    let outcome = consolidator.consolidate_evaluated(&request).unwrap();
    assert_eq!(outcome.proposals.len(), 1);
    let proposal = &outcome.proposals[0].record;
    assert!(
        (proposal.confidence - 0.4).abs() < 1e-9,
        "confidence must be the minimum"
    );
    assert_eq!(
        proposal.derived_from.len(),
        2,
        "provenance chain must be preserved"
    );
    assert!(
        nexus_memory_workers::util::sensitivity_rank(proposal.sensitivity)
            <= nexus_memory_workers::util::sensitivity_rank(Sensitivity::Sensitive),
        "sensitivity must never exceed the source maximum or request ceiling"
    );
}

// ---------------------------------------------------------------------
// Semantic consolidation unavailable and failing (never simulated)
// ---------------------------------------------------------------------

#[test]
fn ep016_failure_semantic_unavailable_deterministic_fallback() {
    // Timeout-equivalent: the semantic adapter returns None (model
    // unavailable). The worker must use the deterministic fallback and
    // report DeterministicFallback; it must never claim ModelAssisted.
    let sources = StubSources {
        records: vec![source_record(0x09, 0.9, Sensitivity::Household)],
    };
    let mut consolidator = DeterministicMemoryConsolidator::new(
        sources,
        Some(Box::new(StubSemantic { result: Ok(None) })),
    );
    let request = consolidation_request(vec!["m-09".into()]);
    let outcome = consolidator.consolidate_evaluated(&request).unwrap();
    assert_eq!(outcome.mode, ConsolidationMode::DeterministicFallback);
    assert_eq!(outcome.proposals.len(), 1);
}

#[test]
fn ep016_failure_semantic_error_fails_closed() {
    // The semantic adapter itself fails: the whole consolidation fails
    // closed with a typed EXTERNAL_PROVIDER error; no canned proposal is
    // emitted to hide the failure.
    let sources = StubSources {
        records: vec![source_record(0x0a, 0.9, Sensitivity::Household)],
    };
    let mut consolidator = DeterministicMemoryConsolidator::new(
        sources,
        Some(Box::new(StubSemantic {
            result: Err(ContextError::new(
                ContextErrorCode::ExternalProvider,
                "semantic model unavailable",
                Some("c-1".into()),
                Some("p-1".into()),
                Some(TENANT.into()),
                Some("semantic-consolidator".into()),
            )),
        })),
    );
    let request = consolidation_request(vec!["m-0a".into()]);
    let error = consolidator.consolidate_evaluated(&request).unwrap_err();
    assert_eq!(error.code, ContextErrorCode::ExternalProvider);
}

#[test]
fn ep016_failure_consolidation_missing_sources_not_found() {
    // Unavailable dependency: source records are missing. The worker
    // fails with NOT_FOUND; it never fabricates a record.
    let sources = StubSources { records: vec![] };
    let mut consolidator = DeterministicMemoryConsolidator::new(sources, None);
    let request = consolidation_request(vec!["m-missing".into()]);
    let error = consolidator.consolidate_evaluated(&request).unwrap_err();
    assert_eq!(error.code, ContextErrorCode::NotFound);
}

// ---------------------------------------------------------------------
// Cancelled / unbounded work: graph cycle bounded
// ---------------------------------------------------------------------

#[test]
fn ep016_failure_graph_cycle_bounded_no_infinite_loop() {
    // The graph provider is deliberately cyclic (self-loop) and
    // fanning out with fresh targets. The policy must terminate, cap
    // nodes at the budget, and report bounded=true (cancelled
    // expansion, never an infinite walk).
    let mut provider = ExplodingGraph;
    let relations = relations_for_purpose(ContextPurpose::TaskExecution);
    let mut policy =
        DeterministicGraphExpansionPolicy::new(&mut provider, 3, relations, Sensitivity::Sensitive);
    let expansion = policy
        .expand(&GraphExpansionRequest {
            request_id: "g-1".into(),
            correlation_id: "c-1".into(),
            tenant_id: TENANT.into(),
            principal_id: "p-1".into(),
            seed: GraphNodeRef {
                node_id: "seed".into(),
                node_type: "ENTITY".into(),
            },
            mode: GraphExpansionMode::TwoHop,
            max_nodes: 6,
        })
        .unwrap();
    assert!(expansion.bounded, "cyclic expansion must report bounded");
    assert!(
        expansion.nodes.len() <= 6,
        "expansion must never exceed the node budget"
    );
}

// ---------------------------------------------------------------------
// Observability: structured redacted errors and telemetry
// ---------------------------------------------------------------------

#[test]
fn ep016_failure_error_redacted_no_memory_content() {
    // Errors are structured SPEC-006 codes with redacted messages: the
    // authorization error must not leak candidate content.
    let mut engine = engine(
        StubProvider::ok(vec![(
            record(0x0b, "personal", TENANT, Sensitivity::Sensitive),
            signals(true, Some(0.9)),
        )]),
        DisclosureContext::SharedRoom,
    );
    // Shared-room + sensitive must still be denied by the privacy stage
    // (not an error), and the resulting capsule must be empty of it.
    let capsule = engine
        .build(&request(ContextPurpose::Search, 5, "r-5"))
        .unwrap();
    assert!(
        !capsule.citations.iter().any(|c| c.ends_with("0b")),
        "sensitive memory must not leak into a shared-room capsule"
    );
    // A privacy mismatch (wrong principal) yields a typed AUTHORIZATION
    // error whose message contains no memory content.
    let filter = DeterministicPrivacyFilter::new(
        profile(),
        nexus_memory_workers::purpose::PurposeLimiter::policy_for(ContextPurpose::Search),
        DisclosureContext::PrivateChannel,
    );
    let error = filter
        .filter_with_disclosure(TENANT, "attacker", ContextPurpose::Search, vec![])
        .unwrap_err();
    assert_eq!(error.code, ContextErrorCode::Authorization);
    assert!(
        !error.message.contains("fact") && !error.message.contains("content"),
        "error message must stay redacted"
    );
}

#[test]
fn ep016_failure_telemetry_redacted_no_content() {
    // Telemetry is redacted by construction: correlation id, purpose,
    // counts, signal classes, and privacy decisions are present; raw
    // memory content, embeddings, and credentials never are.
    let mut telemetry = ContextTelemetry::new("c-1", ContextPurpose::Search);
    telemetry.with_namespaces(&["household".into(), "personal".into()]);
    telemetry.with_signal_classes(&["exact", "vector"]);
    telemetry.record_privacy(PrivacyFilterDecision::Allow);
    telemetry.record_privacy(PrivacyFilterDecision::Deny);
    telemetry.record_consolidation(ConsolidationMode::DeterministicFallback);
    let redacted = telemetry.redacted_json();
    let text = redacted.to_string();
    assert!(
        text.contains("correlation_id") && text.contains("purpose"),
        "telemetry must retain correlation and purpose"
    );
    assert!(
        !text.contains("0190e1c4") && !text.contains("\"fact\""),
        "telemetry must never carry memory content or ids"
    );
}
