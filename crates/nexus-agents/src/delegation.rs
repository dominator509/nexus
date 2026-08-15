//! Delegation (SPEC-010 canonical term `Delegation`; ADR-024).
//!
//! Direct agent-to-agent authority is forbidden: every delegation is
//! proposed, accepted, and recorded by Nexus and passes Nexus policy
//! and correlation. A delegation binds an objective/task to one agent
//! card and tracks its lifecycle.

use crate::error::AgentsError;
use crate::vocabulary::DelegationState;
use nexus_domain::{CorrelationId, ObjectiveId, TaskId};
use nexus_fabric::AgentCardId;
use serde::{Deserialize, Serialize};

/// A Nexus-recorded delegation of work to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    pub delegation_id: String,
    pub correlation_id: CorrelationId,
    pub objective_id: ObjectiveId,
    pub task_id: TaskId,
    pub from_principal: String,
    pub to_agent: AgentCardId,
    pub state: DelegationState,
    pub created_at_epoch_ms: u64,
}

impl Delegation {
    /// Canonical invariants. Fails closed on empty identities.
    pub fn validate(&self) -> Result<(), AgentsError> {
        if self.delegation_id.is_empty() || self.from_principal.is_empty() {
            return Err(AgentsError::validation(
                "delegation identity fields must not be empty",
                Some("delegation".into()),
            ));
        }
        Ok(())
    }
}
