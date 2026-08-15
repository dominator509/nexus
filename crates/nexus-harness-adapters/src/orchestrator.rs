//! Task orchestrator (SPEC-010 behaviors 1-3; ADR-024).
//!
//! The Nexus parent orchestrator: owns canonical task state, budgets,
//! delegations, and artifacts. Assigns agents through the registry
//! (capability-based, never by name), enforces budgets fail-closed,
//! records every delegation, and attaches artifacts immutably.
//! Direct agent-to-agent authority never bypasses this component.

use nexus_agents::{
    AdapterEvent, AgentAdapter, AgentArtifact, AgentBudget, AgentCapability, AgentCardId,
    AgentRegistry, AgentSelection, AgentTask, AgentTaskState, AgentsError, CapabilityRequest,
    CorrelationId, Delegation, DelegationState, ObjectiveId, TaskId, TenantId,
};
use std::collections::HashMap;

/// The parent orchestrator over a registry and per-card adapters.
pub struct TaskOrchestrator<R> {
    registry: R,
    /// Adapter instances keyed by agent card id (owned by Nexus).
    adapters: HashMap<String, Box<dyn AgentAdapter>>,
    tasks: HashMap<String, AgentTask>,
    delegations: Vec<Delegation>,
    next_delegation: u64,
}

impl<R: AgentRegistry> TaskOrchestrator<R> {
    pub fn new(registry: R) -> Self {
        Self {
            registry,
            adapters: HashMap::new(),
            tasks: HashMap::new(),
            delegations: Vec::new(),
            next_delegation: 0,
        }
    }

    /// Bind an adapter instance to an agent card. The adapter is owned
    /// by Nexus; agents never choose their own adapter.
    pub fn bind_adapter(&mut self, card_id: &str, adapter: Box<dyn AgentAdapter>) {
        self.adapters.insert(card_id.to_string(), adapter);
    }

    /// Create a task in REQUESTED state. Nexus owns the task.
    // The flat required identity mirrors AgentTask::new (EP-015/EP-016
    // precedent); the lint is documented and allowed.
    #[allow(clippy::too_many_arguments)]
    pub fn create_task(
        &mut self,
        task_id: TaskId,
        objective_id: ObjectiveId,
        correlation_id: CorrelationId,
        tenant_id: TenantId,
        principal_id: String,
        capability: AgentCapability,
        budget: AgentBudget,
        now_epoch_ms: u64,
    ) -> Result<AgentTask, AgentsError> {
        let task = AgentTask::new(
            task_id,
            objective_id,
            correlation_id,
            tenant_id,
            principal_id,
            capability,
            budget,
            now_epoch_ms,
        )?;
        if self.tasks.contains_key(task.task_id.as_str()) {
            return Err(AgentsError::new(
                nexus_agents::AgentsErrorCode::Conflict,
                "task already exists",
                None,
                None,
                None,
                Some("task-orchestrator".into()),
            ));
        }
        let clone = task.clone();
        self.tasks.insert(task.task_id.as_str().to_string(), task);
        Ok(clone)
    }

    /// Assign the highest-ranked eligible agent to a REQUESTED task.
    /// Records the delegation (PROPOSED -> ACCEPTED -> ACTIVE); direct
    /// agent-to-agent assignment is impossible through this API.
    pub fn assign(
        &mut self,
        task_id: &str,
        now_epoch_ms: u64,
    ) -> Result<AgentSelection, AgentsError> {
        let task = self.require_task(task_id)?;
        if task.state != AgentTaskState::Requested {
            return Err(AgentsError::validation(
                "only REQUESTED tasks can be assigned",
                Some("task-orchestrator".into()),
            ));
        }
        let request = CapabilityRequest {
            request_id: format!("assign-{task_id}"),
            correlation_id: task.correlation_id.as_str().to_string(),
            tenant_id: task.tenant_id.as_str().to_string(),
            principal_id: task.principal_id.clone(),
            objective_id: task.objective_id.as_str().to_string(),
            task_id: task.task_id.as_str().to_string(),
            capability: task.capability,
            required_permissions: vec![],
            budget: task.budget,
        };
        let mut ranked = self.registry.select_for_capability(&request)?;
        let Some(selection) = ranked.pop() else {
            return Err(AgentsError::unavailable(
                "no eligible agent for capability",
                Some("task-orchestrator".into()),
            ));
        };
        let task = self.tasks.get_mut(task_id).unwrap();
        task.assigned_agent = Some(AgentCardId(selection.card_id.clone()));
        task.transition(AgentTaskState::Assigned, now_epoch_ms)?;

        self.next_delegation += 1;
        self.delegations.push(Delegation {
            delegation_id: format!("del-{:04}", self.next_delegation),
            correlation_id: task.correlation_id.clone(),
            objective_id: task.objective_id.clone(),
            task_id: task.task_id.clone(),
            from_principal: task.principal_id.clone(),
            to_agent: AgentCardId(selection.card_id.clone()),
            state: DelegationState::Active,
            created_at_epoch_ms: now_epoch_ms,
        });
        Ok(selection)
    }

    /// Start the assigned adapter session for an ASSIGNED task.
    pub fn start_task(
        &mut self,
        task_id: &str,
        brief: String,
        workdir: Option<String>,
        now_epoch_ms: u64,
    ) -> Result<AdapterEvent, AgentsError> {
        let task = self.require_task(task_id)?;
        let Some(card_id) = task.assigned_agent.clone() else {
            return Err(AgentsError::validation(
                "task has no assigned agent",
                Some("task-orchestrator".into()),
            ));
        };
        let adapter = self.require_adapter(&card_id.0)?;
        let task_clone = task.clone();
        let session = adapter.start(nexus_agents::AdapterStartContext {
            task: task_clone,
            brief,
            workdir,
            extra: serde_json::json!({}),
        })?;
        let t = self.tasks.get_mut(task_id).unwrap();
        t.transition(AgentTaskState::Running, now_epoch_ms)?;
        Ok(AdapterEvent::Progress(nexus_agents::AdapterProgress {
            session_id: session.session_id,
            state: session.state,
            status: "started".to_string(),
            percent: 0,
        }))
    }

    /// Cancel a running task; the task and its delegation end
    /// cancelled/revoked.
    pub fn cancel_task(&mut self, task_id: &str, now_epoch_ms: u64) -> Result<(), AgentsError> {
        let task = self.require_task(task_id)?;
        if let Some(card_id) = task.assigned_agent.clone() {
            let adapter = self.require_adapter(&card_id.0)?;
            let session = adapter.progress(&nexus_agents::AdapterSessionId(format!(
                "{}-{task_id}-0001",
                card_id.0
            )));
            if let Ok(p) = session {
                let _ = p;
            }
        }
        let task = self.tasks.get_mut(task_id).unwrap();
        task.transition(AgentTaskState::Cancelled, now_epoch_ms)?;
        for d in self.delegations.iter_mut() {
            if d.task_id.as_str() == task_id && d.state == DelegationState::Active {
                d.state = DelegationState::Revoked;
            }
        }
        Ok(())
    }

    /// Record budget usage; fails closed when the limit is exceeded
    /// and the task is failed.
    pub fn record_usage(
        &mut self,
        task_id: &str,
        amount: u64,
        now_epoch_ms: u64,
    ) -> Result<(), AgentsError> {
        let task = self.require_task(task_id)?;
        let budget = task.budget;
        // Re-check against a copy; on policy failure the task fails.
        let mut probe = budget;
        if let Err(error) = probe.consume(amount) {
            let task = self.tasks.get_mut(task_id).unwrap();
            let _ = task.transition(AgentTaskState::Failed, now_epoch_ms);
            return Err(error);
        }
        let task = self.tasks.get_mut(task_id).unwrap();
        task.budget.consume(amount)?;
        Ok(())
    }

    /// Attach an artifact to a task. Immutable by content hash:
    /// a duplicate hash is a conflict, never a mutation.
    pub fn attach_artifact(
        &mut self,
        task_id: &str,
        artifact: AgentArtifact,
        now_epoch_ms: u64,
    ) -> Result<(), AgentsError> {
        artifact.validate()?;
        let task = self.require_task(task_id)?;
        if task
            .artifact_ids
            .iter()
            .any(|id| id == &artifact.artifact_id)
        {
            return Err(AgentsError::new(
                nexus_agents::AgentsErrorCode::Conflict,
                "artifact already attached",
                None,
                None,
                None,
                Some("task-orchestrator".into()),
            ));
        }
        let task = self.tasks.get_mut(task_id).unwrap();
        task.artifact_ids.push(artifact.artifact_id);
        task.updated_at_epoch_ms = now_epoch_ms;
        Ok(())
    }

    /// Complete a task: SUCCEEDED + delegation COMPLETED.
    pub fn complete_task(&mut self, task_id: &str, now_epoch_ms: u64) -> Result<(), AgentsError> {
        let task = self.require_task(task_id)?;
        if task.state == AgentTaskState::Requested || task.state == AgentTaskState::Assigned {
            return Err(AgentsError::validation(
                "task must start before completion",
                Some("task-orchestrator".into()),
            ));
        }
        let task = self.tasks.get_mut(task_id).unwrap();
        task.transition(AgentTaskState::Succeeded, now_epoch_ms)?;
        for d in self.delegations.iter_mut() {
            if d.task_id.as_str() == task_id && d.state == DelegationState::Active {
                d.state = DelegationState::Completed;
            }
        }
        Ok(())
    }

    pub fn get_task(&self, task_id: &str) -> Result<AgentTask, AgentsError> {
        self.require_task(task_id)
    }

    pub fn delegations(&self) -> &[Delegation] {
        &self.delegations
    }

    fn require_task(&self, task_id: &str) -> Result<AgentTask, AgentsError> {
        self.tasks.get(task_id).cloned().ok_or_else(|| {
            AgentsError::not_found("task not found", Some("task-orchestrator".into()))
        })
    }

    fn require_adapter(
        &mut self,
        card_id: &str,
    ) -> Result<&mut Box<dyn AgentAdapter>, AgentsError> {
        self.adapters.get_mut(card_id).ok_or_else(|| {
            AgentsError::unavailable(
                "no adapter bound for agent card",
                Some("task-orchestrator".into()),
            )
        })
    }
}
