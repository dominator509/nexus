//! Nexus memory behavior layer (EP-004 M2).
//!
//! Owns the deterministic behavior around the memory/data contracts in
//! `nexus-data`:
//!
//! - `policy`: proposal evaluation - a memory write is a proposal that
//!   becomes an `ACTIVE` canonical fact only after policy approval
//!   (SPEC-002 behavior 5).
//! - `retention`: retention enforcement and legal hold (SPEC-002 behavior
//!   4, SPEC-020).
//! - `lifecycle`: supersession and deletion state transitions (SPEC-002
//!   behaviors 4 and 8).
//! - `retrieval`: hybrid retrieval policy combining filters, recency,
//!   confidence, and diversity (SPEC-002 behavior 6).
//!
//! This crate imports `nexus-domain` (typed IDs, vocabulary) and
//! `nexus-data` (contracts, ports) only. No infrastructure crate may be
//! imported here; the dependency-direction tests enforce it. PostgreSQL and
//! pgvector adapters implement the ports in later milestones.

#![forbid(unsafe_code)]

pub mod lifecycle;
pub mod policy;
pub mod retention;
pub mod retrieval;

pub use lifecycle::{LifecycleEngine, LifecycleError};
pub use policy::{ProposalEvaluator, ProposalOutcome};
pub use retention::{RetentionEngine, RetentionError};
pub use retrieval::{RetrievalPolicy, RetrievalPolicyError};
