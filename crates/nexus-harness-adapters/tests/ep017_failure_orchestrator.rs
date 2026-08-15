//! EP-017 M4 orchestrator failure and abuse suite (SPEC-010
//! behaviors 1-4; ADR-024).
//!
//! Proves the parent orchestrator fails safely: zero budget never
//! starts, exhaustion fails the task and schedules no further work,
//! delegations cannot be resumed after revoke or reactivated after
//! completion, cancellation is idempotent and dominates later
//! completion, artifacts are immutable and task-bound (never
//! cross-tenant), and the CANCELLED state can never become COMPLETED.

use nexus_agents::{
    AgentAdapterKind, AgentArtifact, AgentBudget, AgentBudgetClass, AgentCapability, AgentCard,
    AgentCardId, AgentCardState, AgentRegistry, AgentTaskState, AgentsErrorCode, ArtifactId,
    CorrelationId, ObjectiveId, TaskId, TenantId,
};
use nexus_harness_adapters::{
    CliHarnessAdapter, DeterministicAgentRegistry, HarnessExitStatus, HarnessOutput,
    ScriptedRunner, TaskOrchestrator,
};

fn task_id(n: u8) -> TaskId {
    TaskId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a{n:02x}")).unwrap()
}

fn objective_id(n: u8) -> ObjectiveId {
    ObjectiveId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a{n:02x}")).unwrap()
}

fn correlation_id(n: u8) -> CorrelationId {
    CorrelationId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a{n:02x}")).unwrap()
}

fn tenant_id() -> TenantId {
    TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a80").unwrap()
}

fn artifact_id(n: u8) -> ArtifactId {
    ArtifactId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a{n:02x}")).unwrap()
}

fn budget(limit: u64) -> AgentBudget {
    AgentBudget::new(AgentBudgetClass::TotalTokens, limit)
}

fn ok_output() -> HarnessOutput {
    HarnessOutput {
        status: HarnessExitStatus::Success,
        stdout: String::new(),
        stderr: String::new(),
    }
}

/// A scripted transport that succeeds for the first N calls then dies.
fn runner_with(n_ok: usize) -> ScriptedRunner {
    let mut responses = Vec::new();
    for _ in 0..n_ok {
        responses.push(ok_output());
    }
    ScriptedRunner::new(responses)
}

fn build_registry() -> DeterministicAgentRegistry {
    let mut registry = DeterministicAgentRegistry::new();
    registry
        .register(AgentCard {
            card_id: AgentCardId("codex-1".into()),
            name: "codex-1".into(),
            description: String::new(),
            url: String::new(),
            capabilities: vec![AgentCapability::Implement.as_str().into()],
            state: AgentCardState::Registered,
        })
        .unwrap();
    registry
}

fn build_orchestrator(n_ok: usize) -> TaskOrchestrator<DeterministicAgentRegistry> {
    let mut orchestrator = TaskOrchestrator::new(build_registry());
    orchestrator.bind_adapter(
        "codex-1",
        Box::new(CliHarnessAdapter::new(
            AgentAdapterKind::Codex,
            Box::new(runner_with(n_ok)),
        )),
    );
    orchestrator
}

fn create(
    orchestrator: &mut TaskOrchestrator<DeterministicAgentRegistry>,
    n: u8,
    cap: AgentCapability,
    b: AgentBudget,
) -> nexus_agents::AgentTask {
    orchestrator
        .create_task(
            task_id(n),
            objective_id(0x02),
            correlation_id(0x03),
            tenant_id(),
            "p-1".into(),
            cap,
            b,
            1_700_000_000_000,
        )
        .unwrap()
}

#[test]
fn ep017_failure_zero_budget_fails_before_start() {
    let mut orchestrator = build_orchestrator(2);
    let error = orchestrator
        .create_task(
            task_id(0x01),
            objective_id(0x02),
            correlation_id(0x03),
            tenant_id(),
            "p-1".into(),
            AgentCapability::Implement,
            budget(0),
            1_700_000_000_000,
        )
        .unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Validation);
}

#[test]
fn ep017_failure_budget_exhaustion_fails_task_and_blocks_new_work() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    // Consume exactly to the limit.
    orchestrator
        .record_usage(task.task_id.as_str(), 1000, 1_700_000_000_001)
        .unwrap();
    // The next consume must fail closed and fail the task.
    let error = orchestrator
        .record_usage(task.task_id.as_str(), 1, 1_700_000_000_002)
        .unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Policy);
    let failed = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(failed.state, AgentTaskState::Failed);
    // No new work can be scheduled on the failed task.
    let assign_error = orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_003)
        .unwrap_err();
    assert_eq!(assign_error.code, AgentsErrorCode::Validation);
}

#[test]
fn ep017_failure_agent_cannot_self_increase_budget() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(100),
    );
    // The adapter has no API to raise a budget; the orchestrator only
    // consumes. Exhausting the declared budget must be final.
    let error = orchestrator
        .record_usage(task.task_id.as_str(), 101, 1_700_000_000_001)
        .unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Policy);
    let failed = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(failed.budget.used, 0); // never silently exceeded
    assert_eq!(failed.state, AgentTaskState::Failed);
}

#[test]
fn ep017_failure_cancel_before_start_transitions_and_revokes() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    // Cancel a REQUESTED task: no process exists, so no adapter call;
    // the task ends CANCELLED and no delegation is recorded.
    orchestrator
        .cancel_task(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    assert_eq!(
        orchestrator.get_task(task.task_id.as_str()).unwrap().state,
        AgentTaskState::Cancelled
    );
    assert!(orchestrator.delegations().is_empty());
}

#[test]
fn ep017_failure_cancel_while_running_terminates_owned_process() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    orchestrator
        .start_task(
            task.task_id.as_str(),
            "implement".into(),
            None,
            1_700_000_000_002,
        )
        .unwrap();
    orchestrator
        .cancel_task(task.task_id.as_str(), 1_700_000_000_003)
        .unwrap();
    let cancelled = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(cancelled.state, AgentTaskState::Cancelled);
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "REVOKED");
}

#[test]
fn ep017_failure_cancel_transport_failure_fails_closed_no_orphan_claim() {
    // The runner succeeds for start but dies before the cancel command
    // is delivered. The orchestrator must NOT mark the task CANCELLED:
    // a live process must never be silently orphaned behind a
    // CANCELLED state.
    let mut orchestrator = build_orchestrator(1);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    orchestrator
        .start_task(
            task.task_id.as_str(),
            "implement".into(),
            None,
            1_700_000_000_002,
        )
        .unwrap();
    let error = orchestrator
        .cancel_task(task.task_id.as_str(), 1_700_000_000_003)
        .unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Unavailable);
    let still = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(still.state, AgentTaskState::Running);
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "ACTIVE");
}

#[test]
fn ep017_failure_duplicate_cancel_idempotent() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    orchestrator
        .cancel_task(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    // Second cancel is a no-op success (SPEC-006 idempotency).
    orchestrator
        .cancel_task(task.task_id.as_str(), 1_700_000_000_002)
        .unwrap();
    assert_eq!(
        orchestrator.get_task(task.task_id.as_str()).unwrap().state,
        AgentTaskState::Cancelled
    );
}

#[test]
fn ep017_failure_cancelled_never_becomes_completed() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    orchestrator
        .cancel_task(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    // Cancellation dominates later completion: CANCELLED != COMPLETED
    // and CANCELLED can never transition to SUCCEEDED.
    let error = orchestrator
        .complete_task(task.task_id.as_str(), 1_700_000_000_002)
        .unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Validation);
    assert_eq!(
        orchestrator.get_task(task.task_id.as_str()).unwrap().state,
        AgentTaskState::Cancelled
    );
}

#[test]
fn ep017_failure_revoked_delegation_cannot_resume() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    orchestrator
        .cancel_task(task.task_id.as_str(), 1_700_000_000_002)
        .unwrap();
    // A cancelled task cannot be started or re-assigned.
    let start_error = orchestrator
        .start_task(
            task.task_id.as_str(),
            "implement".into(),
            None,
            1_700_000_000_003,
        )
        .unwrap_err();
    assert_eq!(start_error.code, AgentsErrorCode::Validation);
}

#[test]
fn ep017_failure_completed_delegation_cannot_reactivate() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    orchestrator
        .start_task(
            task.task_id.as_str(),
            "implement".into(),
            None,
            1_700_000_000_002,
        )
        .unwrap();
    orchestrator
        .complete_task(task.task_id.as_str(), 1_700_000_000_003)
        .unwrap();
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "COMPLETED");
    // Terminal SUCCEEDED task cannot be re-started or re-assigned.
    let assign_error = orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_004)
        .unwrap_err();
    assert_eq!(assign_error.code, AgentsErrorCode::Validation);
}

#[test]
fn ep017_failure_artifact_wrong_hash_rejected() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    let bad = AgentArtifact {
        artifact_id: artifact_id(0x40),
        task_id: task.task_id.clone(),
        name: "diff.patch".into(),
        content_hash: "not-a-sha256".into(),
        provenance: vec![],
        content_type: "text/plain".into(),
        created_at_epoch_ms: 1_700_000_000_001,
    };
    let error = orchestrator
        .attach_artifact(task.task_id.as_str(), bad, 1_700_000_000_002)
        .unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Validation);
}

#[test]
fn ep017_failure_artifact_missing_name_rejected() {
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    let bad = AgentArtifact {
        artifact_id: artifact_id(0x40),
        task_id: task.task_id.clone(),
        name: String::new(),
        content_hash: "a".repeat(64),
        provenance: vec![],
        content_type: "text/plain".into(),
        created_at_epoch_ms: 1_700_000_000_001,
    };
    let error = orchestrator
        .attach_artifact(task.task_id.as_str(), bad, 1_700_000_000_002)
        .unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Validation);
}

#[test]
fn ep017_failure_cross_task_artifact_rejected() {
    let mut orchestrator = build_orchestrator(2);
    let task_a = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    let task_b = create(
        &mut orchestrator,
        0x02,
        AgentCapability::Implement,
        budget(1000),
    );
    // Artifact bound to task B must not be attachable to task A
    // (artifact integrity does not imply authorization).
    let cross = AgentArtifact {
        artifact_id: artifact_id(0x40),
        task_id: task_b.task_id.clone(),
        name: "secret.patch".into(),
        content_hash: "b".repeat(64),
        provenance: vec![],
        content_type: "text/plain".into(),
        created_at_epoch_ms: 1_700_000_000_001,
    };
    let error = orchestrator
        .attach_artifact(task_a.task_id.as_str(), cross, 1_700_000_000_002)
        .unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Validation);
    let t = orchestrator.get_task(task_a.task_id.as_str()).unwrap();
    assert!(t.artifact_ids.is_empty());
}

#[test]
fn ep017_failure_partial_side_effect_no_fabricated_success() {
    // The harness performs the Start command successfully (fixture
    // mutation), then dies before any completion acknowledgement. The
    // orchestrator must NOT fabricate SUCCEEDED and must NOT retry the
    // consequential work. The canonical outcome is the ambiguous
    // RUNNING state until verification evidence exists.
    let mut orchestrator = build_orchestrator(2);
    let task = create(
        &mut orchestrator,
        0x01,
        AgentCapability::Implement,
        budget(1000),
    );
    orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    orchestrator
        .start_task(
            task.task_id.as_str(),
            "implement".into(),
            None,
            1_700_000_000_002,
        )
        .unwrap();
    // The harness died after the mutation. The orchestrator owns the
    // task: it never auto-completes, never auto-retries, and leaves
    // the canonical ambiguous RUNNING state. Only an explicit
    // complete_task with verification evidence could move it.
    let t = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(t.state, AgentTaskState::Running);
    assert_eq!(orchestrator.delegations().len(), 1);
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "ACTIVE");
}
