//! EP-012 M4 A2A failure/abuse tests (SPEC-003). Real failure
//! mechanisms on the real gateway: executor failure after state
//! change (partial side effect), cancelled work, capacity exhaustion,
//! cross-tenant denial, missing dependencies, malformed input. The
//! gateway under test is never mocked.

use nexus_a2a::error::A2AErrorCode;
use nexus_a2a::gateway::{A2AGatewayConfig, A2AGatewayImpl, TaskExecutor};
use nexus_a2a::task::{A2ATaskRecord, A2ATaskStatus, TaskMessage, TaskPriority};
use nexus_fabric::artifacts::{ArtifactExchange, ArtifactHandle, ArtifactId, ArtifactManifest};
use nexus_fabric::error::FabricError;

fn ok_executor(task: &A2ATaskRecord) -> Result<Vec<TaskMessage>, String> {
    Ok(vec![TaskMessage {
        message_id: format!("msg-{}", task.task_id),
        role: "agent".into(),
        parts: vec![serde_json::json!({"text": "done"})],
    }])
}

/// Executor that CRASHES after the task was moved to WORKING - the
/// classic partial-side-effect shape. The gateway must mark FAILED and
/// never report success.
fn crash_executor(_task: &A2ATaskRecord) -> Result<Vec<TaskMessage>, String> {
    Err("executor crashed mid-task".to_string())
}

struct MemoryArtifacts;
impl ArtifactExchange for MemoryArtifacts {
    fn publish(
        &mut self,
        _sha256: &str,
        _size_bytes: u64,
        _content_type: &str,
        _parents: &[ArtifactId],
    ) -> Result<ArtifactManifest, FabricError> {
        Err(FabricError::not_found("artifact store unavailable", None))
    }
    fn fetch(&self, _artifact_id: &ArtifactId) -> Result<ArtifactHandle, FabricError> {
        Err(FabricError::not_found("missing", None))
    }
    fn lineage(&self, _artifact_id: &ArtifactId) -> Result<Vec<ArtifactId>, FabricError> {
        Ok(vec![])
    }
    fn revoke(&mut self, _artifact_id: &ArtifactId) -> Result<(), FabricError> {
        Ok(())
    }
}

fn gateway(executor: TaskExecutor) -> A2AGatewayImpl {
    A2AGatewayImpl::new(
        A2AGatewayConfig::default(),
        executor,
        Box::new(MemoryArtifacts),
    )
}

#[test]
fn ep012_failure_a2a_partial_side_effect_never_success() {
    let mut g = gateway(crash_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    let err = g.run("t1", "tenant-1").unwrap_err();
    assert_eq!(err.code, A2AErrorCode::MalformedProviderResponse);
    // The record is FAILED, never COMPLETED, never SUCCESS.
    let record = g.get_task("t1", "tenant-1").unwrap();
    assert_eq!(record.status, A2ATaskStatus::Failed);
    // A FAILED task cannot be re-run as if new.
    assert!(g.run("t1", "tenant-1").is_err());
}

#[test]
fn ep012_failure_a2a_cancelled_task_never_runs() {
    let mut g = gateway(ok_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    g.cancel_task("t1", "tenant-1").unwrap();
    assert!(g.run("t1", "tenant-1").is_err());
    let record = g.get_task("t1", "tenant-1").unwrap();
    assert_eq!(record.status, A2ATaskStatus::Cancelled);
}

#[test]
fn ep012_failure_a2a_capacity_exhaustion_fails_closed() {
    let mut g = A2AGatewayImpl::new(
        A2AGatewayConfig { max_tasks: 1 },
        ok_executor,
        Box::new(MemoryArtifacts),
    );
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    let err = g
        .submit(
            "t2",
            "tenant-1",
            "agent:alice",
            vec![],
            TaskPriority::Normal,
        )
        .unwrap_err();
    assert_eq!(err.code, A2AErrorCode::Unavailable);
}

#[test]
fn ep012_failure_a2a_malformed_task_rejected() {
    let mut g = gateway(ok_executor);
    let err = g
        .submit("", "tenant-1", "agent:alice", vec![], TaskPriority::Normal)
        .unwrap_err();
    assert_eq!(err.code, A2AErrorCode::Validation);
}

#[test]
fn ep012_failure_a2a_duplicate_task_conflict() {
    let mut g = gateway(ok_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    let err = g
        .submit(
            "t1",
            "tenant-1",
            "agent:alice",
            vec![],
            TaskPriority::Normal,
        )
        .unwrap_err();
    assert_eq!(err.code, A2AErrorCode::Conflict);
}

#[test]
fn ep012_failure_a2a_cross_tenant_denied() {
    let mut g = gateway(ok_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    assert_eq!(
        g.get_task("t1", "tenant-2").unwrap_err().code,
        A2AErrorCode::Authorization
    );
    assert_eq!(
        g.cancel_task("t1", "tenant-2").unwrap_err().code,
        A2AErrorCode::Authorization
    );
    assert!(g.run("t1", "tenant-2").is_err());
}

#[test]
fn ep012_failure_a2a_missing_dependency_fails_closed() {
    let mut g = gateway(ok_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    // The artifact exchange reports the artifact missing; the gateway
    // must fail closed, never attach a phantom reference.
    let err = g
        .attach_artifact("t1", "tenant-1", &ArtifactId("missing-art".into()))
        .unwrap_err();
    assert_eq!(err.code, A2AErrorCode::NotFound);
}

#[test]
fn ep012_failure_a2a_invalid_lifecycle_transition_fails_closed() {
    let mut g = gateway(ok_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    // Directly attempt an invalid transition via the state machine.
    let mut record = g.get_task("t1", "tenant-1").unwrap();
    assert!(
        nexus_a2a::task::TaskStateMachine::transition(&mut record, A2ATaskStatus::Completed)
            .is_err()
    );
    assert_eq!(record.status, A2ATaskStatus::Submitted);
}

#[test]
fn ep012_failure_a2a_completed_task_cannot_be_cancelled() {
    let mut g = gateway(ok_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    g.run("t1", "tenant-1").unwrap();
    let err = g.cancel_task("t1", "tenant-1").unwrap_err();
    assert_eq!(err.code, A2AErrorCode::Conflict);
}
