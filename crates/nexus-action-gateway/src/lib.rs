//! Nexus deterministic action gateway engine (EP-008 M2).
//!
//! Implements the provider-neutral `ActionGateway` port from
//! `nexus-policy` as a pure, deterministic engine. The gateway combines
//! relationship checks, contextual policy, risk classification,
//! capability grants, and approval assertions into a single
//! fail-closed `ActionDecision` (SPEC-005 behaviors 2-5, 9; SPEC-006
//! behaviors 4-6).
//!
//! Determinism rules (EP-008 architecture invariant):
//! - The same request, providers, and grants always produce the same
//!   decision. No wall clock, no randomness, no network in the engine
//!   body; time and provider results are passed in as inputs.
//! - Denials are explicit; any provider error is a denial, never a
//!   grant (fail closed).
//! - Models and agents cannot grant authority: approval assertions are
//!   bound to a human (or strong-human) approval class and to the exact
//!   action digest (SPEC-005 behavior 4; SPEC-006 behavior 6).

#![forbid(unsafe_code)]

pub mod engine;

pub use engine::{
    ApprovalRequirement, DecisionInput, DeterministicGateway, GatewayConfig, GatewayError,
    GatewayOutcome,
};

#[cfg(test)]
mod lib_tests;
