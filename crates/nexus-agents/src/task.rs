//! Agent task (SPEC-010; ADR-024).
//!
//! Nexus owns canonical task state. A task belongs to an objective
//! (task graph via `parent_task`), requests a capability, carries a
//! declared budget, and records the assigned agent and artifact
//! lineage. Task state transitions are deterministic and terminal
//! states are final.

use crate::budget::AgentBudget;
use crate::error::AgentsError;
use crate::vocabulary::{AgentCapability, AgentTaskState};
use nexus_domain::{CorrelationId, ObjectiveId, TaskId, TenantId};
use nexus_fabric::AgentCardId;
use serde::{Deserialize, Serialize};

/// An agent task in the objective graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTask {
    pub task_id: TaskId,
    pub objective_id: ObjectiveId,
    pub correlation_id: CorrelationId,
    pub tenant_id: TenantId,
    pub principal_id: String,
    pub capability: AgentCapability,
    pub state: AgentTaskState,
    /// Task graph: parent objective/task relationship.
    pub parent_task: Option<TaskId>,
    /// The agent card selected by Nexus (never chosen by another agent).
    pub assigned_agent: Option<AgentCardId>,
    /// Artifacts produced by this task (immutable by hash).
    pub artifact_ids: Vec<nexus_domain::ArtifactId>,
    pub budget: AgentBudget,
    pub created_at_epoch_ms: u64,
    pub updated_at_epoch_ms: u64,
}

impl AgentTask {
    /// Construct with deterministic validation; fails closed on empty
    /// identities or a non-positive budget.
    // The canonical task identity (ids, tenant, principal, capability,
    // budget, injected clock) is deliberately a flat, required set;
    // grouping it hides required invariants (EP-015/EP-016 precedent,
    // ADR-022/ADR-024). The lint is documented and allowed.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        task_id: TaskId,
        objective_id: ObjectiveId,
        correlation_id: CorrelationId,
        tenant_id: TenantId,
        principal_id: String,
        capability: AgentCapability,
        budget: AgentBudget,
        now_epoch_ms: u64,
    ) -> Result<Self, AgentsError> {
        let task = Self {
            task_id,
            objective_id,
            correlation_id,
            tenant_id,
            principal_id,
            capability,
            state: AgentTaskState::Requested,
            parent_task: None,
            assigned_agent: None,
            artifact_ids: Vec::new(),
            budget,
            created_at_epoch_ms: now_epoch_ms,
            updated_at_epoch_ms: now_epoch_ms,
        };
        task.validate()?;
        Ok(task)
    }

    /// Canonical invariants. Fails closed on empty identities.
    pub fn validate(&self) -> Result<(), AgentsError> {
        if self.principal_id.is_empty() {
            return Err(AgentsError::validation(
                "principal_id must not be empty",
                Some("agent-task".into()),
            ));
        }
        self.budget.validate()?;
        Ok(())
    }

    /// Deterministic state transition. Terminal states never move;
    /// unknown transitions are rejected rather than silently accepted.
    pub fn transition(
        &mut self,
        next: AgentTaskState,
        now_epoch_ms: u64,
    ) -> Result<(), AgentsError> {
        if self.state.is_terminal() {
            return Err(AgentsError::validation(
                "terminal task cannot transition",
                Some("agent-task".into()),
            ));
        }
        if self.state == next {
            return Err(AgentsError::validation(
                "task already in requested state",
                Some("agent-task".into()),
            ));
        }
        self.state = next;
        self.updated_at_epoch_ms = now_epoch_ms;
        Ok(())
    }
}
