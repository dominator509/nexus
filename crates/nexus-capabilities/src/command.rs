//! Command capability port (SPEC-003 canonical term `Command`).
//!
//! A command is an idempotent, effectful invocation. Commands require
//! an idempotency key when the descriptor's idempotency contract is
//! `REQUIRED`, and the approval class on the descriptor governs whether
//! human or policy approval is bound before execution (SPEC-006,
//! EP-008). Commands are distinct from queries and workflows; there is
//! no generic execute string anywhere in the contract.

use serde::{Deserialize, Serialize};

use crate::context::InvocationContext;
use crate::error::CapabilityError;

/// Typed command request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRequest {
    /// Capability key to invoke.
    pub capability_id: String,
    /// Invocation context.
    pub context: InvocationContext,
    /// Canonical input payload.
    pub input: serde_json::Value,
    /// Idempotency key for retryable commands (SPEC-006 behavior 2).
    pub idempotency_key: Option<String>,
}

/// Typed command result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    /// Capability key that produced the result.
    pub capability_id: String,
    /// Canonical output payload.
    pub output: serde_json::Value,
}

/// Provider-neutral idempotent command port (SPEC-003, SPEC-006).
pub trait CommandCapability {
    /// Execute an idempotent command.
    fn command(&self, request: CommandRequest) -> Result<CommandResult, CapabilityError>;
}
