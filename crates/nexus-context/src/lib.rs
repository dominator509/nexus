//! EP-016 context engine and memory consolidation crate (SPEC-002;
//! ADR-023).
//!
//! Provider-neutral contracts for the context plane: `ContextEngine`
//! (purpose-limited, permission-filtered context capsule construction
//! for the model router), `HybridRetriever` (exact/full-text/vector/
//! graph/recency/importance/confidence/diversity signal combination),
//! `MemoryConsolidator` (model-assisted semantic consolidation that
//! always emits proposals for policy evaluation), `PrivacyFilter`
//! (purpose limitation, sensitivity ceilings, namespace isolation, and
//! private shared-room routing), and `GraphExpansionPolicy` (bounded
//! graph-aware context construction). Models can never write canonical
//! memory directly (SPEC-002 behavior 5).
//!
//! The canonical memory/context vocabulary is re-exported from lower
//! layers so callers have a single import surface: `MemoryRecord`,
//! `MemoryProposal`, `MemoryQuery`, `MemoryCandidate`, `MemoryType`,
//! `Sensitivity`, `RetentionPolicy`, `EmbeddingRef` (nexus-data /
//! nexus-domain), `ContextCapsule`, `CapsuleId`, `CapsuleState`,
//! `CapsuleReference`, `ContextCapsuleService` (nexus-fabric), and
//! `RetrievalPolicy`, `RetrievalBlend`, `ProposalEvaluator`,
//! `RetentionEngine`, `LifecycleEngine` (nexus-memory).

#![forbid(unsafe_code)]
// ContextError deliberately carries the full redacted SPEC-006 context
// set (code, message, correlation/actor/tenant/resource) by value for
// deterministic audit and serialization. The result_large_err heuristic
// would demand boxing every error path for a bounded, deterministic
// type; the lint is documented and allowed rather than silently worked
// around (EP-015 precedent, ADR-022).
#![allow(clippy::result_large_err)]

pub mod consolidation;
pub mod context_engine;
pub mod error;
pub mod graph;
pub mod hybrid;
pub mod privacy;
pub mod vocabulary;

pub use consolidation::{ConsolidationOutcome, ConsolidationRequest, MemoryConsolidator};
pub use context_engine::{ContextEngine, ContextRequest};
pub use error::{ContextError, ContextErrorCode};
pub use graph::{
    GraphEdgeRef, GraphExpansion, GraphExpansionPolicy, GraphExpansionRequest, GraphNodeRef,
};
pub use hybrid::{HybridRetriever, RetrievalSignals};
pub use privacy::{FilteredCandidate, PrivacyFilter};
pub use vocabulary::{
    ConsolidationMode, ContextPurpose, ContextVocabularyError, GraphExpansionMode,
    PrivacyFilterDecision,
};

// Re-export the canonical memory/context vocabulary from lower layers so
// EP-016 callers have a single import surface (SPEC-002 locked names are
// never redefined here).
pub use nexus_data::memory::{
    EmbeddingRef, MemoryCandidate, MemoryProposal, MemoryQuery, MemoryRecord, MemoryStatus,
    RetentionPolicy, RetentionUnit, Sensitivity,
};
pub use nexus_data::ports::{
    MemoryRepository, PostgresWorldGraphRepository, VectorRepository, WorldGraphRepository,
};
pub use nexus_domain::{MemoryType, NexusId, TenantId};
pub use nexus_fabric::{
    CapsuleId, CapsuleReference, CapsuleState, ContextCapsule, ContextCapsuleService,
};
pub use nexus_memory::retrieval::RetrievalBlend;
pub use nexus_memory::{
    LifecycleEngine, LifecycleError, ProposalEvaluator, ProposalOutcome, RetentionEngine,
    RetentionError, RetrievalPolicy, RetrievalPolicyError,
};
