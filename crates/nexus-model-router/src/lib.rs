//! EP-015 model router crate (SPEC-009, SPEC-025; ADR-022).
//!
//! Provider-neutral contracts for the model routing plane: the
//! `NexusModelRouter` port, `RoutingFeatures` (the SPEC-009 router
//! inputs), `RoutingDecision`, deterministic `RoutePolicy` (weighted
//! policy routing that can override learned routing for security),
//! `LearnedRouterAdapter` (RouteLLM/LLMRouter replaceable strategies),
//! `EscalationPolicy`, and `MicrobrainProvider` (same ReflexProvider
//! contract, shadow-before-promotion, can remain disabled).
//!
//! SPEC-009 required behavior:
//! - Router inputs include domain, complexity, privacy, risk, capability,
//!   cost, latency, locality, availability, historical success,
//!   certification, and budget.
//! - RouteLLM and LLMRouter are replaceable strategies; the policy engine
//!   can override learned routing for security.
//! - Microbrain uses the same ReflexProvider contract, begins in shadow,
//!   and can remain disabled (SPEC-009 behavior 9; SPEC-025).
//!
//! Authority boundary: routing is a deterministic control-plane decision.
//! Learned scorers and the Microbrain contribute advisory signals only;
//! they never mint routes, grant scopes, or override security policy.

#![forbid(unsafe_code)]

pub mod config;
pub mod decision;
pub mod error;
pub mod escalation;
pub mod features;
pub mod learned;
pub mod microbrain;
pub mod policy;
pub mod router;
pub mod vocabulary;

pub use config::RouterPolicyConfig;
pub use decision::RoutingDecision;
pub use error::{RouterError, RouterErrorCode};
pub use escalation::{EscalationOutcome, EscalationPolicy};
pub use features::RoutingFeatures;
pub use learned::{LearnedRouterAdapter, LearnedScores};
pub use microbrain::{MicrobrainProvider, ShadowDecision};
pub use policy::RoutePolicy;
pub use router::{AuditSink, DeterministicModelRouter, NexusModelRouter, RouteAuditRecord};
pub use vocabulary::{
    EscalationReason, MicrobrainState, RouterStrategyClass, RouterVocabularyError,
    RoutingDecisionClass, ShadowDecisionClass,
};

// Re-export the canonical routing vocabulary from lower layers so router
// callers have a single import surface: Route, Risk, Privacy (nexus-domain);
// EffortTier, ProviderHealth, ProviderHealthState, CacheHitRatio
// (nexus-model-gateway); ReflexProvider, ReflexRequest, ReflexDecision
// (nexus-reflex).
pub use nexus_domain::vocabulary::{Privacy, Risk, Route};
pub use nexus_model_gateway::health::ProviderHealth;
pub use nexus_model_gateway::vocabulary::{CacheHitRatio, EffortTier, ProviderHealthState};
pub use nexus_reflex::{ReflexProvider, ReflexRequest};
