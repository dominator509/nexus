//! EP-019 self-healing engineering loop contract crate (SPEC-018; ADR-026).
//!
//! Incident correlation, diagnosis, patching, independent review, HITL
//! approval, canary, verification, and rollback. This crate owns the
//! provider-neutral contracts and the canonical lifecycle vocabulary; a
//! model/agent may PROPOSE a diagnosis or patch but never declares its
//! own fix successful. Real evidence (reproduction before/after,
//! validation gates, approval binding, post-deploy verification) is the
//! only path to REMEDIATED/CLOSED.
//!
//! This file owns no provider behavior (M1 contract boundary); the
//! deterministic engine and real process/validation machinery are owned
//! by the EP-019 M2+ crate boundary.

pub mod canary;
pub mod contract;
pub mod error;
pub mod memory;
pub mod rollback;
pub mod vocabulary;

pub use canary::{CanaryPlan, CanaryState, HealthCriterion, HealthCriterionState};
pub use contract::{
    DiagnosisTask, Incident, IncidentEngine, IncidentSignal, IncidentSignalKind, PatchProposal,
    RemediationApproval, ReviewDecision, ReviewVerdict, SandboxVerdict, SecurityVerdict,
};
pub use error::{HealingError, HealingErrorCode};
pub use memory::{InMemoryIncidentMemory, IncidentMemory, IncidentMemoryRecord};
pub use rollback::{RollbackPlan, RollbackState};
pub use vocabulary::{DiagnosisConfidence, IncidentState};

// Re-export canonical ids and approval classes from nexus-domain so
// callers have a single import surface and locked names are never
// redefined.
pub use nexus_domain::{
    ApprovalClass, ApprovalId, CorrelationId, DeploymentId, DiagnosisId, IncidentId, PatchId, Risk,
    RollbackId, TenantId,
};
