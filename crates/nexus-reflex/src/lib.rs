//! EP-014 reflex provider crate (SPEC-009).
//!
//! Provider-neutral contracts for the reflex plane: the `ReflexProvider`
//! port, `DeepSeekFlashProvider` (V1 primary), `EffortPolicy` for
//! deterministic effort-tier selection, canonical `PromptSegment`
//! assembly, `CacheLedger` for rolling token cache-hit accounting
//! (target >= 0.97 on the cacheable corpus), and
//! `NexusControlObjectValidator` for deterministic control-object
//! validation (reject extra or invalid fields).
//!
//! SPEC-009 required behavior:
//! - Deterministic tasks bypass the model (EffortTier::Deterministic).
//! - Non-thinking, high, and max effort are policy selected; MAX is
//!   never the default for trivial work.
//! - Stable prefix segments are canonical and versioned; volatile IDs
//!   and timestamps stay in the tail.
//! - Rolling token cache-hit ratio is measured and targets at least
//!   0.97 on the cacheable corpus.
//! - Only validated NexusControlObject output continues.
//!
//! Authority boundary: models carry intelligence, never authority. A
//! model cannot grant scopes, approve actions, modify policies, reveal
//! secrets, or bypass output validation (SPEC-009 behavior 10).
//! Provider credentials never leave the adapter; credentials are
//! referenced by id, never by value.

#![forbid(unsafe_code)]

pub mod cache;
pub mod decision;
pub mod effort;
pub mod error;
pub mod provider;
pub mod segments;
pub mod validator;
pub mod vocabulary;

pub use cache::CacheLedger;
pub use decision::{ReflexDecision, ReflexDecisionClass};
pub use effort::{EffortInput, EffortPolicy};
pub use error::{ReflexError, ReflexErrorCode};
pub use provider::{DeepSeekFlashProvider, ReflexProvider, ReflexRequest};
pub use segments::{
    PromptSegmentCatalog, PromptSegmentVersion, StablePrefix,
};
pub use validator::NexusControlObjectValidator;
pub use vocabulary::ReflexVocabularyError;

// Re-export the canonical model-plane vocabulary so reflex callers have
// a single import surface: PromptSegment, EffortTier, NexusControlObject,
// CacheHitRatio, ProviderHealth, ModelGatewayError.
pub use nexus_model_gateway::model::{NexusControlObject, PromptSegment, PromptSegmentPart, UsageReport};
pub use nexus_model_gateway::vocabulary::{CacheHitRatio, EffortTier, ProviderHealthState};
pub use nexus_model_gateway::health::ProviderHealth;
