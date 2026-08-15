//! EP-017 M2 orchestrator and adapter session tests (SPEC-010;
//! ADR-024).
//!
//! Proves the parent-orchestrator lifecycle: create -> assign
//! (capability-based, delegation recorded) -> start -> cancel/complete,
//! budget enforcement fail-closed, immutable artifact attachment, and
//! the adapter session state machine over a CONTROLLED_TEST_FIXTURE
//! transport (never used to claim real provider behavior).

use nexus_agents::{
    AgentAdapter, AgentAdapterKind, AgentArtifact, AgentBudget, AgentBudgetClass, AgentCapability,
    AgentCard, AgentCardId, AgentCardState, AgentRegistry, AgentTaskState, ArtifactId,
    CorrelationId, ObjectiveId, TaskId, TenantId,
};
use nexus_harness_adapters::{
    CliHarnessAdapter, DeterministicAgentRegistry, HarnessExitStatus, HarnessOutput,
    ScriptedRunner, TaskOrchestrator, capabilities_for,
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

fn budget() -> AgentBudget {
    AgentBudget::new(AgentBudgetClass::TotalTokens, 1000)
}

fn ok_output() -> HarnessOutput {
    HarnessOutput {
        status: HarnessExitStatus::Success,
        stdout: String::new(),
        stderr: String::new(),
    }
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

fn build_orchestrator() -> TaskOrchestrator<DeterministicAgentRegistry> {
    let registry = build_registry();
    let mut orchestrator = TaskOrchestrator::new(registry);
    orchestrator.bind_adapter(
        "codex-1",
        Box::new(CliHarnessAdapter::new(
            AgentAdapterKind::Codex,
            Box::new(ScriptedRunner::new(vec![ok_output()])),
        )),
    );
    orchestrator
}

#[test]
fn ep017_unit_orchestrator_create_assign_start_complete() {
    let mut orchestrator = build_orchestrator();
    let task = orchestrator
        .create_task(
            task_id(0x01),
            objective_id(0x02),
            correlation_id(0x03),
            tenant_id(),
            "p-1".into(),
            AgentCapability::Implement,
            budget(),
            1_700_000_000_000,
        )
        .unwrap();
    assert_eq!(task.state, AgentTaskState::Requested);

    let selection = orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    assert_eq!(selection.card_id, "codex-1");
    assert_eq!(orchestrator.delegations().len(), 1);
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "ACTIVE");

    orchestrator
        .start_task(
            task.task_id.as_str(),
            "implement".into(),
            None,
            1_700_000_000_002,
        )
        .unwrap();
    let running = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(running.state, AgentTaskState::Running);

    orchestrator
        .complete_task(task.task_id.as_str(), 1_700_000_000_003)
        .unwrap();
    let done = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(done.state, AgentTaskState::Succeeded);
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "COMPLETED");
}

#[test]
fn ep017_unit_orchestrator_no_eligible_agent_fails_closed() {
    let mut orchestrator = TaskOrchestrator::new(DeterministicAgentRegistry::new());
    let task = orchestrator
        .create_task(
            task_id(0x01),
            objective_id(0x02),
            correlation_id(0x03),
            tenant_id(),
            "p-1".into(),
            AgentCapability::Implement,
            budget(),
            1_700_000_000_000,
        )
        .unwrap();
    let error = orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap_err();
    assert_eq!(error.code.as_str(), "UNAVAILABLE");
}

#[test]
fn ep017_unit_orchestrator_budget_exhaustion_fails_task() {
    let mut orchestrator = build_orchestrator();
    let task = orchestrator
        .create_task(
            task_id(0x01),
            objective_id(0x02),
            correlation_id(0x03),
            tenant_id(),
            "p-1".into(),
            AgentCapability::Implement,
            budget(),
            1_700_000_000_000,
        )
        .unwrap();
    let error = orchestrator
        .record_usage(task.task_id.as_str(), 5000, 1_700_000_000_001)
        .unwrap_err();
    assert_eq!(error.code.as_str(), "POLICY");
    let failed = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(failed.state, AgentTaskState::Failed);
}

#[test]
fn ep017_unit_orchestrator_assign_only_from_requested() {
    let mut orchestrator = build_orchestrator();
    let task = orchestrator
        .create_task(
            task_id(0x01),
            objective_id(0x02),
            correlation_id(0x03),
            tenant_id(),
            "p-1".into(),
            AgentCapability::Implement,
            budget(),
            1_700_000_000_000,
        )
        .unwrap();
    orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    let error = orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_002)
        .unwrap_err();
    assert_eq!(error.code.as_str(), "VALIDATION");
}

#[test]
fn ep017_unit_orchestrator_cancel_revokes_delegation() {
    let mut orchestrator = build_orchestrator();
    let task = orchestrator
        .create_task(
            task_id(0x01),
            objective_id(0x02),
            correlation_id(0x03),
            tenant_id(),
            "p-1".into(),
            AgentCapability::Implement,
            budget(),
            1_700_000_000_000,
        )
        .unwrap();
    orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    orchestrator
        .cancel_task(task.task_id.as_str(), 1_700_000_000_002)
        .unwrap();
    let cancelled = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(cancelled.state, AgentTaskState::Cancelled);
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "REVOKED");
}

#[test]
fn ep017_unit_orchestrator_artifact_immutable_and_duplicate_conflict() {
    let mut orchestrator = build_orchestrator();
    let task = orchestrator
        .create_task(
            task_id(0x01),
            objective_id(0x02),
            correlation_id(0x03),
            tenant_id(),
            "p-1".into(),
            AgentCapability::Implement,
            budget(),
            1_700_000_000_000,
        )
        .unwrap();
    let artifact = AgentArtifact {
        artifact_id: artifact_id(0x40),
        task_id: task.task_id.clone(),
        name: "diff.patch".into(),
        content_hash: "a".repeat(64),
        provenance: vec![],
        content_type: "text/plain".into(),
        created_at_epoch_ms: 1_700_000_000_001,
    };
    orchestrator
        .attach_artifact(task.task_id.as_str(), artifact.clone(), 1_700_000_000_002)
        .unwrap();
    let error = orchestrator
        .attach_artifact(task.task_id.as_str(), artifact, 1_700_000_000_003)
        .unwrap_err();
    assert_eq!(error.code.as_str(), "CONFLICT");
    let t = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(t.artifact_ids.len(), 1);
}

#[test]
fn ep017_unit_adapter_capabilities_per_kind() {
    assert_eq!(capabilities_for(AgentAdapterKind::Codex).len(), 4);
    assert!(capabilities_for(AgentAdapterKind::Codex).contains(&AgentCapability::Implement));
    assert!(capabilities_for(AgentAdapterKind::ClaudeCode).contains(&AgentCapability::Review));
    assert!(capabilities_for(AgentAdapterKind::Hermes).contains(&AgentCapability::Orchestrate));
    assert!(capabilities_for(AgentAdapterKind::OpenClaw).contains(&AgentCapability::Artifact));
}

#[test]
fn ep017_unit_adapter_start_fail_closed_on_transport_failure() {
    // CONTROLLED_TEST_FIXTURE: the transport is scripted to fail; the
    // adapter must fail closed, never report a started session.
    let mut adapter = CliHarnessAdapter::new(
        AgentAdapterKind::Codex,
        Box::new(ScriptedRunner::fail_closed()),
    );
    let task = nexus_agents::AgentTask::new(
        task_id(0x01),
        objective_id(0x02),
        correlation_id(0x03),
        tenant_id(),
        "p-1".into(),
        AgentCapability::Implement,
        budget(),
        1_700_000_000_000,
    )
    .unwrap();
    let error = adapter
        .start(nexus_agents::AdapterStartContext {
            task,
            brief: "x".into(),
            workdir: None,
            extra: serde_json::json!({}),
        })
        .unwrap_err();
    assert_eq!(error.code.as_str(), "UNAVAILABLE");
}

#[test]
fn ep017_unit_adapter_session_terminal_rejects_message() {
    let mut adapter = CliHarnessAdapter::new(
        AgentAdapterKind::Codex,
        Box::new(ScriptedRunner::new(vec![ok_output(), ok_output()])),
    );
    let task = nexus_agents::AgentTask::new(
        task_id(0x01),
        objective_id(0x02),
        correlation_id(0x03),
        tenant_id(),
        "p-1".into(),
        AgentCapability::Implement,
        budget(),
        1_700_000_000_000,
    )
    .unwrap();
    let session = adapter
        .start(nexus_agents::AdapterStartContext {
            task,
            brief: "x".into(),
            workdir: None,
            extra: serde_json::json!({}),
        })
        .unwrap();
    adapter.cancel(&session.session_id).unwrap();
    let error = adapter
        .message(&session.session_id, "keep going")
        .unwrap_err();
    assert_eq!(error.code.as_str(), "VALIDATION");
}
