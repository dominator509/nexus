//! EP-016 memory workers (SPEC-002; ADR-023).
//!
//! Pure deterministic domain workers implementing the `nexus-context`
//! provider-neutral ports: `ContextEngine` (purpose-limited,
//! permission-filtered context capsule construction), `HybridRetriever`
//! (exact/full-text/vector/graph/recency/importance/confidence/diversity
//! signal combination), `MemoryConsolidator` (proposal-before-canonical
//! consolidation), `PrivacyFilter` (sensitivity ceilings, namespace
//! isolation, shared-room disclosure), and `GraphExpansionPolicy`
//! (bounded purpose-aware graph expansion).
//!
//! The workers are deterministic: all I/O (candidate retrieval, source
//! records, graph edges, presence, model results, time) is injected
//! through ports. The domain logic never performs SQL, HTTP,
//! filesystem, clock, random, or model calls directly (node contract
//! M2; SPEC-002 behavior 6).
//!
//! Pipeline order is fixed and is a security property:
//!
//! ```text
//! AVAILABLE MEMORY
//!     -> PERMISSION FILTER
//!     -> PURPOSE FILTER
//!     -> ACTIVE/LIFECYCLE FILTER
//!     -> HYBRID RETRIEVAL
//!     -> DIVERSITY / RELEVANCE / CONFIDENCE
//!     -> PRIVACY / DISCLOSURE FILTER
//!     -> CONTEXT CAPSULE
//! ```
//!
//! Unauthorized memory never enters the scoring pool; relevance never
//! overrides disclosure privacy; proposals are never canonical facts.
//! Every policy choice is recorded in the EP-016 Decision Log.

#![forbid(unsafe_code)]
// ContextError deliberately carries the full redacted SPEC-006 context
// set (code, message, correlation/actor/tenant/resource) by value for
// deterministic audit and serialization. The result_large_err heuristic
// would demand boxing every error path for a bounded, deterministic
// type; the lint is documented and allowed rather than silently worked
// around (EP-015/EP-016 precedent, ADR-022/ADR-023).
#![allow(clippy::result_large_err)]

pub mod budget;
pub mod consolidation;
pub mod engine;
pub mod graph;
pub mod hybrid;
pub mod lifecycle;
pub mod permission;
pub mod privacy;
pub mod purpose;
pub mod telemetry;
pub mod util;

pub use budget::{BudgetClass, ContextBudget};
pub use consolidation::{DeterministicMemoryConsolidator, SemanticConsolidator, SourceProvider};
pub use engine::DeterministicContextEngine;
pub use graph::{DeterministicGraphExpansionPolicy, GraphEdgeInfo, GraphNodeInfo, GraphProvider};
pub use hybrid::{
    CandidateProvider, CandidateSignals, DeterministicHybridRetriever, HybridScorer,
    ScoreComponents, ScoredCandidate,
};
pub use lifecycle::{ActiveMemoryLifecycleFilter, LifecycleContext};
pub use permission::{AccessProfile, PermissionFilter};
pub use privacy::{DeterministicPrivacyFilter, DisclosureContext, PrivateRoutingDecision};
pub use purpose::{PurposeLimiter, PurposePolicy};
pub use telemetry::ContextTelemetry;

// Canonical vocabulary re-exported from the contract layer so callers
// have a single import surface (never redefined here).
pub use nexus_context::{
    ConsolidationMode, ConsolidationOutcome, ConsolidationRequest, ContextEngine, ContextError,
    ContextErrorCode, ContextPurpose, ContextRequest, FilteredCandidate, GraphEdgeRef,
    GraphExpansion, GraphExpansionMode, GraphExpansionPolicy, GraphExpansionRequest, GraphNodeRef,
    HybridRetriever, MemoryConsolidator, PrivacyFilter, PrivacyFilterDecision, RetrievalSignals,
};
