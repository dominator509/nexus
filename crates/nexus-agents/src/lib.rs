//! EP-017 agent orchestrator and harness adapters (SPEC-010; ADR-024).
//!
//! Nexus owns canonical objectives, task state, context, permissions,
//! budgets, artifacts, and results. Agents request capabilities rather
//! than named peers; Nexus selects on quality, cost, trust,
//! availability, and historical success. Direct agent-to-agent
//! authority is forbidden; delegation passes through Nexus policy and
//! correlation.
//!
//! This crate owns the provider-neutral contracts:
//!
//! - `AgentRegistry`: capability-based agent selection.
//! - `AgentAdapter`: the canonical task contract (start, message,
//!   progress, input request, pause, cancel, resume, artifacts, tests,
//!   review) that every harness adapter (Codex, Claude Code, Hermes,
//!   OpenClaw) implements.
//! - `AgentTask`, `CapabilityRequest`, `AgentArtifact`, `AgentBudget`,
//!   `Delegation` value types with deterministic validation and
//!   serialization.
//!
//! Canonical vocabulary is never redefined here: nexus-domain owns the
//! typed identifiers (TaskId, ObjectiveId, CorrelationId, ArtifactId,
//! CapabilityId, TenantId, PersonId) and nexus-fabric owns the agent
//! card, A2A task, artifact manifest, and SPEC-006 error machinery.
//! EP-017-owned vocabulary (ADR-024): `AgentTaskState`,
//! `AgentAdapterKind`, `AgentCapability`, `DelegationState`,
//! `AgentBudgetClass`.

#![forbid(unsafe_code)]
#![allow(clippy::result_large_err)]

pub mod adapter;
pub mod artifact;
pub mod budget;
pub mod capability;
pub mod delegation;
pub mod error;
pub mod registry;
pub mod task;
pub mod vocabulary;

pub use adapter::{
    AdapterEvent, AdapterProgress, AdapterReview, AdapterSession, AdapterSessionId,
    AdapterSessionState, AdapterStartContext, AgentAdapter,
};
pub use artifact::AgentArtifact;
pub use budget::AgentBudget;
pub use capability::CapabilityRequest;
pub use delegation::Delegation;
pub use error::{AgentsError, AgentsErrorCode, AgentsVocabularyError};
pub use registry::{AgentRegistry, AgentSelection};
pub use task::AgentTask;
pub use vocabulary::{
    AgentAdapterKind, AgentBudgetClass, AgentCapability, AgentTaskState, DelegationState,
};

// Canonical vocabulary re-exported from the contract layers so callers
// have a single import surface (never redefined here).
pub use nexus_domain::{ArtifactId, CapabilityId, CorrelationId, ObjectiveId, TaskId, TenantId};
pub use nexus_fabric::{
    A2ATask, A2ATaskId, A2ATaskState, A2ATaskStatus, AgentCard, AgentCardId, AgentCardRegistry,
    AgentCardState, ArtifactManifest, ArtifactState, FabricError, FabricErrorCode, TaskMessage,
};
