//! Workflow capability port (SPEC-003 canonical term `Workflow`).
//!
//! A workflow is a durable, long-running invocation: it returns a
//! handle, progresses through states, and produces a final result.
//! Workflows are distinct from queries and commands; there is no
//! generic execute string anywhere in the contract.

use serde::{Deserialize, Serialize};

use crate::context::InvocationContext;
use crate::error::CapabilityError;

/// Typed workflow start request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRequest {
    /// Capability key to invoke.
    pub capability_id: String,
    /// Invocation context.
    pub context: InvocationContext,
    /// Canonical input payload.
    pub input: serde_json::Value,
    /// Idempotency key for retryable workflow starts.
    pub idempotency_key: Option<String>,
}

/// Workflow handle returned by a start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowHandle {
    /// Capability key.
    pub capability_id: String,
    /// Workflow instance identifier.
    pub workflow_id: String,
}

/// Workflow lifecycle state (SPEC-023; owned by EP-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkflowStatus {
    /// Workflow is pending or running.
    Running,
    /// Workflow completed successfully.
    Completed,
    /// Workflow failed and is not retrying.
    Failed,
    /// Workflow was cancelled.
    Cancelled,
}

impl WorkflowStatus {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "RUNNING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Cancelled => "CANCELLED",
        }
    }
}

/// Typed workflow result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// Workflow handle.
    pub handle: WorkflowHandle,
    /// Current lifecycle state.
    pub status: WorkflowStatus,
    /// Canonical output payload when the workflow completed.
    pub output: Option<serde_json::Value>,
}

/// Provider-neutral durable workflow port (SPEC-003, SPEC-023).
pub trait WorkflowCapability {
    /// Start a durable workflow.
    fn start(&self, request: WorkflowRequest) -> Result<WorkflowHandle, CapabilityError>;

    /// Read the current state of a workflow.
    fn status(&self, handle: WorkflowHandle) -> Result<WorkflowResult, CapabilityError>;
}
