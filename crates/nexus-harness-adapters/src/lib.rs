//! EP-017 harness adapter implementations (SPEC-010; ADR-024).
//!
//! Production behavior owned by EP-017 M2:
//!
//! - `DeterministicAgentRegistry`: capability-based agent selection.
//!   Agents request capabilities rather than named peers; selection
//!   ranks candidates deterministically on quality, cost, trust,
//!   availability, and historical success. Never returns a named agent
//!   without a capability match.
//! - `TaskOrchestrator`: the Nexus parent orchestrator. Owns task
//!   state, budgets, delegations, and artifacts; assigns agents via
//!   the registry; enforces budgets fail-closed; records every
//!   delegation (direct agent-to-agent authority is forbidden).
//! - `CliHarnessAdapter`: implements the canonical `AgentAdapter`
//!   contract over an injected `HarnessCommandRunner` port. All
//!   process I/O lives behind the transport port; the adapter owns the
//!   deterministic session state machine and normalized events.
//!
//! Domain rules are pure; infrastructure adapters may import
//! application ports (`nexus-agents`), never the reverse.

#![forbid(unsafe_code)]
#![allow(clippy::result_large_err)]

pub mod adapter;
pub mod orchestrator;
pub mod registry;

pub use adapter::{
    capabilities_for, CliHarnessAdapter, HarnessCommand, HarnessCommandKind, HarnessCommandRunner,
    HarnessExitStatus, HarnessOutput, ScriptedRunner,
};
pub use orchestrator::TaskOrchestrator;
pub use registry::{AgentSelector, CardSignals, DeterministicAgentRegistry};
