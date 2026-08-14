//! Nexus capability and connector core behavior (EP-010 M2).
//!
//! Implements the deterministic invariants owned by EP-010 on top of
//! the provider-neutral ports in `nexus-capabilities`: a real
//! in-memory capability registry, a typed capability dispatcher that
//! makes a generic execute string impossible, and idempotency
//! tracking for retryable commands (SPEC-003, SPEC-006, SPEC-022).
//!
//! This crate may import `nexus-domain`, `nexus-identity`, and
//! `nexus-capabilities` plus serde only. Infrastructure, database,
//! network, and vendor crates belong in later milestones' adapters
//! and are forbidden here; the dependency-direction tests enforce
//! this boundary.

#![forbid(unsafe_code)]

pub mod dispatcher;
pub mod idempotency;
pub mod registry;

pub use dispatcher::{CapabilityDispatcher, DispatcherError};
pub use idempotency::{IdempotencyRecord, IdempotencyTracker, IdempotencyTrackerError};
pub use registry::{InMemoryCapabilityRegistry, RegistryEntry};

#[cfg(test)]
mod lib_tests;
