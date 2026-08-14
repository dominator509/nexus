//! Health capability port (SPEC-022 canonical term `HealthReport`).
//!
//! A health capability reports the operational state of a connector or
//! capability. Health state is a distinct class from query, command,
//! and workflow: it never mutates state and never returns provider
//! payloads.

use serde::{Deserialize, Serialize};

use crate::context::InvocationContext;
use crate::error::CapabilityError;
use crate::vocabulary::HealthState;

/// Health report for a capability or connector (SPEC-022).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthReport {
    /// Capability or connector key.
    pub target_id: String,
    /// Health state.
    pub state: HealthState,
    /// Optional human-readable detail (never a secret or raw provider
    /// payload).
    pub detail: Option<String>,
}

/// Provider-neutral health port (SPEC-022).
pub trait HealthCapability {
    /// Read the current health of the capability.
    fn health(&self, context: InvocationContext) -> Result<HealthReport, CapabilityError>;
}
