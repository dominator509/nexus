//! Capability request (SPEC-010 behavior 2; ADR-024).
//!
//! Agents request capabilities rather than named peers. A capability
//! request names the capability, the owning objective and task, the
//! authenticated principal, the declared required permissions (least
//! privilege), and the budget Nexus will enforce. Nexus selects the
//! adapter on quality, cost, trust, availability, and historical
//! success.

use crate::budget::AgentBudget;
use crate::error::AgentsError;
use crate::vocabulary::AgentCapability;
use serde::{Deserialize, Serialize};

/// A capability request from an objective/task to the agent registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRequest {
    pub request_id: String,
    pub correlation_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    pub objective_id: String,
    pub task_id: String,
    pub capability: AgentCapability,
    /// Declared required permissions (least privilege; Nexus policy
    /// may narrow but never widen).
    pub required_permissions: Vec<String>,
    pub budget: AgentBudget,
}

impl CapabilityRequest {
    /// Canonical invariants. Fails closed on empty identities.
    pub fn validate(&self) -> Result<(), AgentsError> {
        if self.request_id.is_empty()
            || self.correlation_id.is_empty()
            || self.tenant_id.is_empty()
            || self.principal_id.is_empty()
            || self.objective_id.is_empty()
            || self.task_id.is_empty()
        {
            return Err(AgentsError::validation(
                "capability request identity fields must not be empty",
                Some("capability-request".into()),
            ));
        }
        self.budget.validate()?;
        Ok(())
    }
}
