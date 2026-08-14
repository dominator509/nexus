//! EP-012 M3 A2A gateway integration tests (SPEC-003 required behavior
//! 3). Real gateway behavior across the fabric `A2AGateway` trait
//! boundary: task lifecycle, cancellation, streaming, push
//! notifications, cross-tenant denial, and artifact fail-closed.

use nexus_a2a::error::A2AErrorCode;
use nexus_a2a::gateway::{A2AGatewayConfig, A2AGatewayImpl, TaskExecutor};
use nexus_a2a::task::{A2ATaskRecord, TaskMessage, TaskPriority};
use nexus_fabric::a2a::{A2AGateway, A2ATask, A2ATaskId};
use nexus_fabric::artifacts::{ArtifactExchange, ArtifactHandle, ArtifactId, ArtifactManifest};
use nexus_fabric::error::{FabricError, FabricErrorCode};

fn ok_executor(task: &A2ATaskRecord) -> Result<Vec<TaskMessage>, String> {
    Ok(vec![TaskMessage {
        message_id: format!("msg-{}", task.task_id),
        role: "agent".into(),
        parts: vec![serde_json::json!({"text": "done"})],
    }])
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
        Err(FabricError::not_found(
            "not implemented in integration",
            None,
        ))
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
fn ep012_integration_a2a_trait_send_get_stream() {
    let mut g = gateway(ok_executor);
    let task = A2ATask {
        task_id: A2ATaskId("task-1".into()),
        tenant_id: "tenant-1".into(),
        principal_id: "agent:alice".into(),
        status: nexus_fabric::a2a::A2ATaskStatus {
            state: nexus_fabric::vocabulary::StreamState::Pending,
            message: None,
        },
        messages: vec![TaskMessage {
            message_id: "m0".into(),
            role: "user".into(),
            parts: vec![serde_json::json!({"text": "do the thing"})],
        }],
    };
    let id = A2AGateway::send_task(&mut g, task).unwrap();
    assert_eq!(id, A2ATaskId("task-1".into()));

    // Advance through the real executor.
    g.run("task-1", "tenant-1").unwrap();

    let fetched = A2AGateway::get_task(&g, &A2ATaskId("task-1".into())).unwrap();
    assert_eq!(fetched.tenant_id, "tenant-1");
    assert_eq!(fetched.principal_id, "agent:alice");
    assert_eq!(fetched.messages.len(), 2); // user + agent

    // Streamed status events: SUBMITTED, WORKING, COMPLETED.
    let events = A2AGateway::stream(&g, &A2ATaskId("task-1".into())).unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(
        events.last().unwrap().state,
        nexus_fabric::vocabulary::StreamState::Completed
    );
}

#[test]
fn ep012_integration_a2a_cancel_idempotent_and_terminal() {
    let mut g = gateway(ok_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    A2AGateway::cancel_task(&mut g, &A2ATaskId("t1".into())).unwrap();
    A2AGateway::cancel_task(&mut g, &A2ATaskId("t1".into())).unwrap(); // idempotent
    let fetched = A2AGateway::get_task(&g, &A2ATaskId("t1".into())).unwrap();
    assert_eq!(
        fetched.status.state,
        nexus_fabric::vocabulary::StreamState::Cancelled
    );
    // A completed task cannot be cancelled.
    g.submit(
        "t2",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    g.run("t2", "tenant-1").unwrap();
    let err = A2AGateway::cancel_task(&mut g, &A2ATaskId("t2".into())).unwrap_err();
    assert_eq!(err.code, FabricErrorCode::Conflict);
}

#[test]
fn ep012_integration_a2a_cross_tenant_denied_at_boundary() {
    let mut g = gateway(ok_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    let err = g.get_task("t1", "tenant-2").unwrap_err();
    assert_eq!(err.code, A2AErrorCode::Authorization);
    assert!(g.run("t1", "tenant-2").is_err());
    assert!(g.cancel_task("t1", "tenant-2").is_err());
}

#[test]
fn ep012_integration_a2a_unknown_task_fails_closed() {
    let mut g = gateway(ok_executor);
    let err = A2AGateway::get_task(&g, &A2ATaskId("missing".into())).unwrap_err();
    assert_eq!(err.code, FabricErrorCode::NotFound);
    let err = A2AGateway::cancel_task(&mut g, &A2ATaskId("missing".into())).unwrap_err();
    assert_eq!(err.code, FabricErrorCode::NotFound);
}

#[test]
fn ep012_integration_a2a_push_notification_registered() {
    let mut g = gateway(ok_executor);
    g.submit(
        "t1",
        "tenant-1",
        "agent:alice",
        vec![],
        TaskPriority::Normal,
    )
    .unwrap();
    A2AGateway::push_notification(
        &mut g,
        &A2ATaskId("t1".into()),
        "https://push.nexus.local/t1",
    )
    .unwrap();
    let subs = g.subscribers_for("t1", "tenant-1").unwrap();
    assert_eq!(subs.len(), 1);
    assert_eq!(
        subs[0].push_url.as_deref(),
        Some("https://push.nexus.local/t1")
    );
}

#[test]
fn ep012_integration_a2a_missing_artifact_fails_closed() {
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
        .attach_artifact("t1", "tenant-1", &ArtifactId("missing-art".into()))
        .unwrap_err();
    assert_eq!(err.code, A2AErrorCode::NotFound);
}
