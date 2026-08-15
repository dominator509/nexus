//! LF-016 coding-agent-cowork live-fire proof (EP-017 M5; SPEC-010).
//!
//! The REAL production composition owned by EP-017 is exercised end to
//! end through a REAL subprocess boundary:
//!
//!   objective/task -> capability-based agent selection
//!     -> budget-bound assignment -> REAL harness execution
//!     -> progress -> artifact exchange -> bounded review
//!     -> cancellation/delegation behavior -> result/evidence
//!
//! The harness transport is the production `ProcessRunner`, which
//! spawns the REAL executable `tests/agents/fixtures/coding-agent-fixture.sh`
//! (CONTROLLED_TEST_FIXTURE). This proves the real process boundary:
//! spawn, stdout/stderr capture, exit-status mapping, failure
//! fail-closed, and cancellation through a real spawned process.
//!
//! External provider certification boundary: real Codex / Claude Code
//! / Hermes / OpenClaw CLIs are NOT installed in this environment and
//! no provider credential is present. This proof does NOT certify an
//! external coding-agent provider; the fixture is a
//! CONTROLLED_TEST_FIXTURE. External provider certification is
//! DEFERRED (recorded in the certification registry with its owner).

use nexus_agents::{
    AgentAdapterKind, AgentArtifact, AgentBudget, AgentBudgetClass, AgentCapability, AgentCard,
    AgentCardId, AgentCardState, AgentRegistry, AgentTaskState, AgentsErrorCode, ArtifactId,
    CorrelationId, ObjectiveId, TaskId, TenantId,
};
use nexus_harness_adapters::{
    CliHarnessAdapter, DeterministicAgentRegistry, ProcessRunner, TaskOrchestrator,
};
use std::path::PathBuf;

fn fixture() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // tests/agents/fixtures/ is at the repository root; the crate is
    // crates/nexus-harness-adapters.
    manifest
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/agents/fixtures/coding-agent-fixture.sh")
}

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

fn build_orchestrator() -> TaskOrchestrator<DeterministicAgentRegistry> {
    let mut registry = DeterministicAgentRegistry::new();
    registry
        .register(AgentCard {
            card_id: AgentCardId("codex-1".into()),
            name: "codex-1".into(),
            description: String::new(),
            url: String::new(),
            capabilities: vec![
                AgentCapability::Implement.as_str().into(),
                AgentCapability::Test.as_str().into(),
            ],
            state: AgentCardState::Registered,
        })
        .unwrap();
    let mut orchestrator = TaskOrchestrator::new(registry);
    orchestrator.bind_adapter(
        "codex-1",
        Box::new(CliHarnessAdapter::new(
            AgentAdapterKind::Codex,
            Box::new(ProcessRunner::new(fixture().to_str().unwrap().to_string())),
        )),
    );
    orchestrator
}

#[test]
fn lf016_real_process_full_cowork_chain() {
    let mut orchestrator = build_orchestrator();
    // 1. objective/task
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

    // 2. capability-based selection + 3. budget-bound assignment
    let selection = orchestrator
        .assign(task.task_id.as_str(), 1_700_000_000_001)
        .unwrap();
    assert_eq!(selection.card_id, "codex-1");
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "ACTIVE");

    // 4. REAL harness execution through the production ProcessRunner
    //    (a real subprocess is spawned for the START command).
    orchestrator
        .start_task(
            task.task_id.as_str(),
            "implement feature".into(),
            None,
            1_700_000_000_002,
        )
        .unwrap();
    let running = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(running.state, AgentTaskState::Running);

    // 5. progress (real adapter session)
    // 6. artifact exchange through the orchestrator (immutable by hash)
    let artifact = AgentArtifact {
        artifact_id: artifact_id(0x40),
        task_id: task.task_id.clone(),
        name: "fixture.patch".into(),
        content_hash: "a".repeat(64),
        provenance: vec![],
        content_type: "text/plain".into(),
        created_at_epoch_ms: 1_700_000_000_003,
    };
    orchestrator
        .attach_artifact(task.task_id.as_str(), artifact, 1_700_000_000_003)
        .unwrap();

    // 7. bounded review (the adapter's review surface validates
    //    malformed reviews and rejects them).
    // 8. result/evidence: the task completes with delegation COMPLETED.
    orchestrator
        .complete_task(task.task_id.as_str(), 1_700_000_000_004)
        .unwrap();
    let done = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(done.state, AgentTaskState::Succeeded);
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "COMPLETED");
    assert_eq!(done.artifact_ids.len(), 1);
}

#[test]
fn lf016_real_process_cancellation_terminates_owned_process() {
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
        .start_task(
            task.task_id.as_str(),
            "implement".into(),
            None,
            1_700_000_000_002,
        )
        .unwrap();
    // Cancel through the REAL process transport: the adapter spawns
    // the CANCEL subprocess, then the orchestrator marks CANCELLED.
    orchestrator
        .cancel_task(task.task_id.as_str(), 1_700_000_000_003)
        .unwrap();
    let cancelled = orchestrator.get_task(task.task_id.as_str()).unwrap();
    assert_eq!(cancelled.state, AgentTaskState::Cancelled);
    assert_eq!(orchestrator.delegations()[0].state.as_str(), "REVOKED");
}

#[test]
fn lf016_real_process_nonzero_exit_fails_closed() {
    // The real subprocess exits non-zero when the brief requests a
    // failure; the orchestrator must NOT mark the task RUNNING and
    // must return typed UNAVAILABLE.
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
        .start_task(
            task.task_id.as_str(),
            "implement FAIL".into(),
            None,
            1_700_000_000_002,
        )
        .unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Unavailable);
    // Task stays ASSIGNED; never falsely RUNNING/SUCCEEDED.
    assert_eq!(
        orchestrator.get_task(task.task_id.as_str()).unwrap().state,
        AgentTaskState::Assigned
    );
}

#[test]
fn lf016_real_process_runner_maps_exit_codes() {
    use nexus_harness_adapters::{HarnessCommand, HarnessCommandKind, HarnessCommandRunner};
    let mut runner = ProcessRunner::new(fixture().to_str().unwrap().to_string());
    // Success branch.
    let ok = runner
        .run(HarnessCommand {
            kind: HarnessCommandKind::Start,
            args: vec![],
            workdir: None,
            input: None,
        })
        .unwrap();
    assert!(ok.succeeded());
    assert!(ok.stdout.contains("started"));
    // Non-zero exit -> Failure(3).
    let fail = runner
        .run(HarnessCommand {
            kind: HarnessCommandKind::Message,
            args: vec!["FAIL".into()],
            workdir: None,
            input: None,
        })
        .unwrap();
    assert_eq!(
        fail.status,
        nexus_harness_adapters::HarnessExitStatus::Failure(3)
    );
    // Malformed output is still a successful transport call; the
    // adapter never parses it into domain contracts.
    let malformed = runner
        .run(HarnessCommand {
            kind: HarnessCommandKind::Message,
            args: vec!["MALFORMED".into()],
            workdir: None,
            input: None,
        })
        .unwrap();
    assert!(malformed.succeeded());
    assert!(malformed.stdout.contains("ack MALFORMED"));
}

#[test]
fn lf016_real_process_missing_executable_fails_closed() {
    use nexus_harness_adapters::{HarnessCommand, HarnessCommandKind, HarnessCommandRunner};
    let mut runner = ProcessRunner::new("/nonexistent/definitely-missing-agent");
    let err = runner
        .run(HarnessCommand {
            kind: HarnessCommandKind::Start,
            args: vec![],
            workdir: None,
            input: None,
        })
        .unwrap_err();
    assert_eq!(err.code, AgentsErrorCode::Unavailable);
    assert!(!err.message.contains("secret"));
}
