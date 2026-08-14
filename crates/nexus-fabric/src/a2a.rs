//! A2A agent collaboration contract (SPEC-003 required behavior 3).
//!
//! A2A targets protocol 1.0.1 and is used for opaque agent tasks,
//! streaming status, artifacts, cancellation, and push notifications -
//! never ordinary data reads, and never as an authorization mechanism.

use crate::error::FabricError;
use crate::vocabulary::StreamState;
use serde::{Deserialize, Serialize};

/// A2A task identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A2ATaskId(pub String);

/// A2A task status (streaming state).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A2ATaskStatus {
    pub state: StreamState,
    pub message: Option<String>,
}

/// A2A task message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskMessage {
    pub message_id: String,
    pub role: String,
    pub parts: Vec<serde_json::Value>,
}

/// A2A task (opaque agent work unit).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A2ATask {
    pub task_id: A2ATaskId,
    pub status: A2ATaskStatus,
    pub messages: Vec<TaskMessage>,
}

/// A2A task lifecycle state (canonical surface for streaming).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum A2ATaskState {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Cancelled,
    Failed,
}

impl A2ATaskState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "SUBMITTED",
            Self::Working => "WORKING",
            Self::InputRequired => "INPUT_REQUIRED",
            Self::Completed => "COMPLETED",
            Self::Cancelled => "CANCELLED",
            Self::Failed => "FAILED",
        }
    }
}

impl std::str::FromStr for A2ATaskState {
    type Err = crate::vocabulary::FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "SUBMITTED" => Ok(Self::Submitted),
            "WORKING" => Ok(Self::Working),
            "INPUT_REQUIRED" => Ok(Self::InputRequired),
            "COMPLETED" => Ok(Self::Completed),
            "CANCELLED" => Ok(Self::Cancelled),
            "FAILED" => Ok(Self::Failed),
            other => Err(crate::vocabulary::FabricVocabularyError::unknown(
                "A2ATaskState",
                other,
            )),
        }
    }
}

/// Provider-neutral A2A gateway port.
pub trait A2AGateway {
    /// Send an opaque agent task; returns a task handle.
    fn send_task(&mut self, task: A2ATask) -> Result<A2ATaskId, FabricError>;
    /// Fetch current task status.
    fn get_task(&self, task_id: &A2ATaskId) -> Result<A2ATask, FabricError>;
    /// Cancel a task (idempotent).
    fn cancel_task(&mut self, task_id: &A2ATaskId) -> Result<(), FabricError>;
    /// Stream task status updates.
    fn stream(&self, task_id: &A2ATaskId) -> Result<Vec<A2ATaskStatus>, FabricError>;
    /// Register for push notifications.
    fn push_notification(&mut self, task_id: &A2ATaskId, url: &str) -> Result<(), FabricError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_a2a_task_state_round_trip() {
        for (wire, expected) in [
            ("SUBMITTED", A2ATaskState::Submitted),
            ("WORKING", A2ATaskState::Working),
            ("INPUT_REQUIRED", A2ATaskState::InputRequired),
            ("COMPLETED", A2ATaskState::Completed),
            ("CANCELLED", A2ATaskState::Cancelled),
            ("FAILED", A2ATaskState::Failed),
        ] {
            assert_eq!(wire.parse::<A2ATaskState>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("IDLE".parse::<A2ATaskState>().is_err());
    }

    #[test]
    fn ep012_unit_a2a_task_round_trip() {
        let task = A2ATask {
            task_id: A2ATaskId("task-1".into()),
            status: A2ATaskStatus {
                state: StreamState::Running,
                message: None,
            },
            messages: vec![TaskMessage {
                message_id: "m-1".into(),
                role: "agent".into(),
                parts: vec![serde_json::json!({"text": "hi"})],
            }],
        };
        let json = serde_json::to_value(&task).unwrap();
        let back: A2ATask = serde_json::from_value(json).unwrap();
        assert_eq!(back.task_id, A2ATaskId("task-1".into()));
        assert_eq!(back.status.state, StreamState::Running);
    }
}
