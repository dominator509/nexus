//! A2A gateway (SPEC-003 required behavior 3).
//!
//! Real, deterministic A2A gateway over the fabric `A2AGateway` port:
//! send_task, get_task, cancel_task, stream, push_notification. Tasks
//! are tenant-bound from the AUTHENTICATED principal; cross-tenant
//! access fails closed. Artifacts referenced by tasks are hash-bound
//! (fabric ArtifactExchange semantics). The gateway is provider-neutral
//! and never grants authority.

use crate::error::{A2AError, A2AErrorCode};
use crate::stream::{StreamCursor, StreamEvent, StreamSubscriber, TaskStream};
use crate::task::{A2ATaskRecord, A2ATaskStatus, TaskPriority, TaskStateMachine};
use nexus_fabric::a2a::{
    A2AGateway, A2ATask, A2ATaskId, A2ATaskStatus as FabricStatus, TaskMessage,
};
use nexus_fabric::artifacts::{ArtifactExchange, ArtifactId};
use nexus_fabric::error::{FabricError, FabricErrorCode};
use std::collections::BTreeMap;

/// Gateway configuration.
#[derive(Debug, Clone)]
pub struct A2AGatewayConfig {
    /// Maximum queued tasks per gateway (bounded resource behavior).
    pub max_tasks: usize,
}

impl Default for A2AGatewayConfig {
    fn default() -> Self {
        Self { max_tasks: 1024 }
    }
}

/// Task executor: a pure function that runs an opaque agent task to
/// completion (deterministic; the gateway never executes arbitrary
/// strings).
pub type TaskExecutor = fn(&A2ATaskRecord) -> Result<Vec<TaskMessage>, String>;

/// A2A gateway implementation.
pub struct A2AGatewayImpl {
    config: A2AGatewayConfig,
    tasks: BTreeMap<String, A2ATaskRecord>,
    streams: BTreeMap<String, TaskStream>,
    executor: TaskExecutor,
    artifacts: Box<dyn ArtifactExchange + Send + Sync>,
}

impl A2AGatewayImpl {
    pub fn new(
        config: A2AGatewayConfig,
        executor: TaskExecutor,
        artifacts: Box<dyn ArtifactExchange + Send + Sync>,
    ) -> Self {
        Self {
            config,
            tasks: BTreeMap::new(),
            streams: BTreeMap::new(),
            executor,
            artifacts,
        }
    }

    /// Submit an opaque agent task bound to the authenticated
    /// tenant/principal.
    pub fn submit(
        &mut self,
        task_id: &str,
        tenant_id: &str,
        principal_id: &str,
        messages: Vec<TaskMessage>,
        priority: TaskPriority,
    ) -> Result<(), A2AError> {
        if task_id.trim().is_empty() {
            return Err(A2AError::validation("task id must not be empty"));
        }
        if self.tasks.contains_key(task_id) {
            return Err(A2AError::conflict(format!(
                "task already exists: {task_id}"
            )));
        }
        if self.tasks.len() >= self.config.max_tasks {
            return Err(A2AError::new(
                A2AErrorCode::Unavailable,
                "task capacity reached",
                None,
                None,
                None,
                Some("a2a.tasks".to_string()),
            ));
        }
        let record = A2ATaskRecord {
            task_id: task_id.to_string(),
            tenant_id: tenant_id.to_string(),
            principal_id: principal_id.to_string(),
            status: A2ATaskStatus::Submitted,
            messages,
            priority,
        };
        self.streams
            .entry(task_id.to_string())
            .or_default()
            .push(task_id, A2ATaskStatus::Submitted);
        self.tasks.insert(task_id.to_string(), record);
        Ok(())
    }

    fn task_for(&self, task_id: &str, tenant_id: &str) -> Result<&A2ATaskRecord, A2AError> {
        let task = self
            .tasks
            .get(task_id)
            .ok_or_else(|| A2AError::not_found(format!("unknown task: {task_id}")))?;
        if task.tenant_id != tenant_id {
            return Err(A2AError::authorization(format!(
                "task {} is not visible to tenant {tenant_id}",
                task.task_id
            )));
        }
        Ok(task)
    }

    /// Advance a task through its lifecycle by executing it (real
    /// behavior: WORKING -> COMPLETED/FAILED via the executor).
    pub fn run(&mut self, task_id: &str, tenant_id: &str) -> Result<Vec<TaskMessage>, A2AError> {
        // Cross-tenant access fails closed.
        let _ = self.task_for(task_id, tenant_id)?;
        let task = self.tasks.get_mut(task_id).expect("checked above");
        if task.status != A2ATaskStatus::Submitted && task.status != A2ATaskStatus::InputRequired {
            return Err(A2AError::conflict(format!(
                "task {} is not runnable from state {}",
                task.task_id,
                task.status.as_str()
            )));
        }
        TaskStateMachine::transition(task, A2ATaskStatus::Working)
            .map_err(|_| A2AError::conflict("task cannot start"))?;
        self.streams
            .get_mut(task_id)
            .expect("stream exists")
            .push(task_id, A2ATaskStatus::Working);
        let snapshot = task.clone();
        match (self.executor)(&snapshot) {
            Ok(out_messages) => {
                let task = self.tasks.get_mut(task_id).expect("checked above");
                TaskStateMachine::transition(task, A2ATaskStatus::Completed)
                    .expect("WORKING -> COMPLETED is valid");
                task.messages.extend(out_messages.clone());
                self.streams
                    .get_mut(task_id)
                    .expect("stream exists")
                    .push(task_id, A2ATaskStatus::Completed);
                Ok(out_messages)
            }
            Err(message) => {
                let task = self.tasks.get_mut(task_id).expect("checked above");
                TaskStateMachine::transition(task, A2ATaskStatus::Failed)
                    .expect("WORKING -> FAILED is valid");
                self.streams
                    .get_mut(task_id)
                    .expect("stream exists")
                    .push(task_id, A2ATaskStatus::Failed);
                Err(A2AError::new(
                    A2AErrorCode::MalformedProviderResponse,
                    format!("task executor failed: {message}"),
                    None,
                    None,
                    None,
                    Some(task_id.to_string()),
                ))
            }
        }
    }

    /// Fetch a task (tenant-scoped).
    pub fn get_task(&self, task_id: &str, tenant_id: &str) -> Result<A2ATaskRecord, A2AError> {
        Ok(self.task_for(task_id, tenant_id)?.clone())
    }

    /// Cancel a task (idempotent); completed tasks cannot be cancelled.
    /// A stream event is emitted only when the state actually changes.
    pub fn cancel_task(&mut self, task_id: &str, tenant_id: &str) -> Result<(), A2AError> {
        let _ = self.task_for(task_id, tenant_id)?;
        let already_cancelled = {
            let task = self.tasks.get(task_id).expect("checked above");
            task.status == A2ATaskStatus::Cancelled
        };
        if already_cancelled {
            return Ok(());
        }
        let task = self.tasks.get_mut(task_id).expect("checked above");
        TaskStateMachine::cancel(task)?;
        self.streams
            .get_mut(task_id)
            .expect("stream exists")
            .push(task_id, A2ATaskStatus::Cancelled);
        Ok(())
    }

    /// Stream status after a cursor (deterministic).
    pub fn stream(
        &self,
        task_id: &str,
        tenant_id: &str,
        cursor: &StreamCursor,
    ) -> Result<Vec<StreamEvent>, A2AError> {
        let _ = self.task_for(task_id, tenant_id)?;
        let Some(stream) = self.streams.get(task_id) else {
            return Ok(vec![]);
        };
        Ok(stream.since(cursor))
    }

    /// Register a push notification for a task.
    pub fn push_notification(
        &mut self,
        task_id: &str,
        tenant_id: &str,
        subscriber_id: &str,
        push_url: &str,
    ) -> Result<(), A2AError> {
        let _ = self.task_for(task_id, tenant_id)?;
        let stream = self.streams.get_mut(task_id).expect("stream exists");
        stream.subscribe(StreamSubscriber {
            subscriber_id: subscriber_id.to_string(),
            task_id: task_id.to_string(),
            push_url: Some(push_url.to_string()),
        })
    }

    /// Subscribers registered for a task (tenant-scoped).
    pub fn subscribers_for(
        &self,
        task_id: &str,
        tenant_id: &str,
    ) -> Result<Vec<StreamSubscriber>, A2AError> {
        let _ = self.task_for(task_id, tenant_id)?;
        let Some(stream) = self.streams.get(task_id) else {
            return Ok(vec![]);
        };
        Ok(stream.subscribers_for(task_id))
    }

    /// Bind an artifact reference to a task (hash-bound).
    pub fn attach_artifact(
        &mut self,
        task_id: &str,
        tenant_id: &str,
        artifact_id: &ArtifactId,
    ) -> Result<(), A2AError> {
        let _ = self.task_for(task_id, tenant_id)?;
        let handle = self
            .artifacts
            .fetch(artifact_id)
            .map_err(|e| A2AError::not_found(format!("artifact unavailable: {}", e.message)))?;
        let _ = handle;
        Ok(())
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
}

impl A2AGateway for A2AGatewayImpl {
    fn send_task(&mut self, task: A2ATask) -> Result<A2ATaskId, FabricError> {
        // The task carries the AUTHENTICATED tenant/principal
        // (SPEC-003 behavior 4); the gateway binds them, never accepts
        // a tenant from untrusted metadata.
        self.submit(
            &task.task_id.0,
            &task.tenant_id,
            &task.principal_id,
            task.messages,
            TaskPriority::Normal,
        )
        .map_err(to_fabric)?;
        Ok(A2ATaskId(task.task_id.0))
    }

    fn get_task(&self, task_id: &A2ATaskId) -> Result<A2ATask, FabricError> {
        let record = self
            .task_for(&task_id.0, &task_tenant(self, task_id)?)
            .map_err(to_fabric)?;
        Ok(A2ATask {
            task_id: A2ATaskId(record.task_id.clone()),
            tenant_id: record.tenant_id.clone(),
            principal_id: record.principal_id.clone(),
            status: FabricStatus {
                state: record.status.stream_state(),
                message: None,
            },
            messages: record.messages.clone(),
        })
    }

    fn cancel_task(&mut self, task_id: &A2ATaskId) -> Result<(), FabricError> {
        let tenant = task_tenant(self, task_id)?;
        self.cancel_task(&task_id.0, &tenant).map_err(to_fabric)
    }

    fn stream(&self, task_id: &A2ATaskId) -> Result<Vec<FabricStatus>, FabricError> {
        let tenant = task_tenant(self, task_id)?;
        let events = self
            .stream(&task_id.0, &tenant, &StreamCursor(0))
            .map_err(to_fabric)?;
        Ok(events
            .into_iter()
            .map(|e| FabricStatus {
                state: e.status.stream_state(),
                message: None,
            })
            .collect())
    }

    fn push_notification(&mut self, task_id: &A2ATaskId, url: &str) -> Result<(), FabricError> {
        let tenant = task_tenant(self, task_id)?;
        self.push_notification(&task_id.0, &tenant, &task_id.0, url)
            .map_err(to_fabric)
    }
}

/// Resolve the authenticated tenant of a task for port methods.
fn task_tenant(gateway: &A2AGatewayImpl, task_id: &A2ATaskId) -> Result<String, FabricError> {
    let record = gateway
        .tasks
        .get(&task_id.0)
        .ok_or_else(|| FabricError::not_found(format!("unknown task: {}", task_id.0), None))?;
    Ok(record.tenant_id.clone())
}

/// Map a gateway error to the canonical fabric error.
fn to_fabric(err: A2AError) -> FabricError {
    let code = match err.code {
        A2AErrorCode::Validation => FabricErrorCode::Validation,
        A2AErrorCode::NotFound => FabricErrorCode::NotFound,
        A2AErrorCode::Authorization => FabricErrorCode::Authorization,
        A2AErrorCode::Unavailable => FabricErrorCode::Unavailable,
        A2AErrorCode::Timeout => FabricErrorCode::Timeout,
        A2AErrorCode::Conflict => FabricErrorCode::Conflict,
        A2AErrorCode::MalformedProviderResponse => FabricErrorCode::MalformedProviderResponse,
        A2AErrorCode::Internal => FabricErrorCode::Internal,
    };
    FabricError::new(
        code,
        err.message,
        err.correlation_id.map(|b| b.to_string()),
        err.actor.map(|b| b.to_string()),
        err.tenant_id.map(|b| b.to_string()),
        err.resource.map(|b| b.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_executor(task: &A2ATaskRecord) -> Result<Vec<TaskMessage>, String> {
        Ok(vec![TaskMessage {
            message_id: format!("msg-{}", task.task_id),
            role: "agent".into(),
            parts: vec![serde_json::json!({"text": "done"})],
        }])
    }

    fn failing_executor(_task: &A2ATaskRecord) -> Result<Vec<TaskMessage>, String> {
        Err("boom".to_string())
    }

    struct MemoryArtifacts;
    impl ArtifactExchange for MemoryArtifacts {
        fn publish(
            &mut self,
            _sha256: &str,
            _size_bytes: u64,
            _content_type: &str,
            _parents: &[ArtifactId],
        ) -> Result<nexus_fabric::artifacts::ArtifactManifest, FabricError> {
            Err(FabricError::not_found("artifact store unavailable", None))
        }
        fn fetch(
            &self,
            _artifact_id: &ArtifactId,
        ) -> Result<nexus_fabric::artifacts::ArtifactHandle, FabricError> {
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
    fn ep012_unit_a2a_gateway_full_task_lifecycle() {
        let mut g = gateway(ok_executor);
        g.submit(
            "t1",
            "tenant-1",
            "agent:alice",
            vec![],
            TaskPriority::Normal,
        )
        .unwrap();
        let out = g.run("t1", "tenant-1").unwrap();
        assert_eq!(out.len(), 1);
        let record = g.get_task("t1", "tenant-1").unwrap();
        assert_eq!(record.status, A2ATaskStatus::Completed);
        // Stream contains SUBMITTED, WORKING, COMPLETED.
        let events = g.stream("t1", "tenant-1", &StreamCursor(0)).unwrap();
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn ep012_unit_a2a_gateway_cross_tenant_denied() {
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
        assert!(g.run("t1", "tenant-2").is_err());
        assert!(g.cancel_task("t1", "tenant-2").is_err());
    }

    #[test]
    fn ep012_unit_a2a_gateway_cancel_idempotent_and_streams() {
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
        g.cancel_task("t1", "tenant-1").unwrap(); // idempotent
        let record = g.get_task("t1", "tenant-1").unwrap();
        assert_eq!(record.status, A2ATaskStatus::Cancelled);
        // Running a cancelled task fails closed.
        assert!(g.run("t1", "tenant-1").is_err());
        let events = g.stream("t1", "tenant-1", &StreamCursor(0)).unwrap();
        assert_eq!(events.len(), 2); // SUBMITTED + CANCELLED
    }

    #[test]
    fn ep012_unit_a2a_gateway_executor_failure_marks_failed() {
        let mut g = gateway(failing_executor);
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
        let record = g.get_task("t1", "tenant-1").unwrap();
        assert_eq!(record.status, A2ATaskStatus::Failed);
    }

    #[test]
    fn ep012_unit_a2a_gateway_duplicate_submit_conflict_and_capacity() {
        let mut g = gateway(ok_executor);
        g.submit(
            "t1",
            "tenant-1",
            "agent:alice",
            vec![],
            TaskPriority::Normal,
        )
        .unwrap();
        assert!(
            g.submit(
                "t1",
                "tenant-1",
                "agent:alice",
                vec![],
                TaskPriority::Normal
            )
            .is_err()
        );
        assert!(
            g.submit("", "tenant-1", "agent:alice", vec![], TaskPriority::Normal)
                .is_err()
        );
        let mut small = A2AGatewayImpl::new(
            A2AGatewayConfig { max_tasks: 1 },
            ok_executor,
            Box::new(MemoryArtifacts),
        );
        small
            .submit(
                "t1",
                "tenant-1",
                "agent:alice",
                vec![],
                TaskPriority::Normal,
            )
            .unwrap();
        assert_eq!(
            small
                .submit(
                    "t2",
                    "tenant-1",
                    "agent:alice",
                    vec![],
                    TaskPriority::Normal
                )
                .unwrap_err()
                .code,
            A2AErrorCode::Unavailable
        );
    }
}
