//! A2A task state machine (SPEC-003 required behavior 3).
//!
//! Canonical A2A task lifecycle: SUBMITTED -> WORKING ->
//! COMPLETED / CANCELLED / FAILED (with INPUT_REQUIRED for
//! interactive tasks). Every transition is validated; invalid
//! transitions fail closed.

use crate::error::A2AError;
use nexus_fabric::vocabulary::StreamState;
use serde::{Deserialize, Serialize};

/// Canonical A2A task lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum A2ATaskStatus {
    Submitted,
    Working,
    InputRequired,
    Completed,
    Cancelled,
    Failed,
}

impl A2ATaskStatus {
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

    /// Map to the fabric stream state.
    pub fn stream_state(self) -> StreamState {
        match self {
            Self::Submitted | Self::InputRequired => StreamState::Pending,
            Self::Working => StreamState::Running,
            Self::Completed => StreamState::Completed,
            Self::Cancelled => StreamState::Cancelled,
            Self::Failed => StreamState::Failed,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

impl std::str::FromStr for A2ATaskStatus {
    type Err = A2AError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "SUBMITTED" => Ok(Self::Submitted),
            "WORKING" => Ok(Self::Working),
            "INPUT_REQUIRED" => Ok(Self::InputRequired),
            "COMPLETED" => Ok(Self::Completed),
            "CANCELLED" => Ok(Self::Cancelled),
            "FAILED" => Ok(Self::Failed),
            other => Err(A2AError::validation(format!(
                "unknown A2ATaskStatus: {other}"
            ))),
        }
    }
}

/// Task priority (deterministic ordering hint; never authority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskPriority {
    Low,
    Normal,
    High,
}

impl TaskPriority {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Normal => "NORMAL",
            Self::High => "HIGH",
        }
    }
}

/// A2A task message (canonical type owned by the fabric contract).
pub use nexus_fabric::a2a::TaskMessage;

/// A task record with its canonical lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct A2ATaskRecord {
    pub task_id: String,
    pub tenant_id: String,
    pub principal_id: String,
    pub status: A2ATaskStatus,
    pub messages: Vec<TaskMessage>,
    pub priority: TaskPriority,
}

/// Deterministic A2A task state machine.
#[derive(Debug, Clone, Default)]
pub struct TaskStateMachine;

impl TaskStateMachine {
    /// Transition validity table (fail closed on anything else).
    pub fn can_transition(from: A2ATaskStatus, to: A2ATaskStatus) -> bool {
        use A2ATaskStatus::*;
        matches!(
            (from, to),
            (Submitted, Working)
                | (Submitted, InputRequired)
                | (Submitted, Cancelled)
                | (Working, Completed)
                | (Working, Failed)
                | (Working, Cancelled)
                | (Working, InputRequired)
                | (InputRequired, Working)
                | (InputRequired, Cancelled)
                | (InputRequired, Completed)
                | (InputRequired, Failed)
        )
    }

    /// Transition a task; invalid transitions fail closed with CONFLICT.
    pub fn transition(task: &mut A2ATaskRecord, to: A2ATaskStatus) -> Result<(), A2AError> {
        if !Self::can_transition(task.status, to) {
            return Err(A2AError::conflict(format!(
                "invalid A2A task transition: {} -> {}",
                task.status.as_str(),
                to.as_str()
            )));
        }
        task.status = to;
        Ok(())
    }

    /// Cancel is idempotent for already-terminal tasks (SPEC-003
    /// cancellation). A completed task cannot be cancelled.
    pub fn cancel(task: &mut A2ATaskRecord) -> Result<(), A2AError> {
        match task.status {
            A2ATaskStatus::Cancelled => Ok(()),
            A2ATaskStatus::Completed => Err(A2AError::conflict(format!(
                "cannot cancel completed task: {}",
                task.task_id
            ))),
            _ => {
                task.status = A2ATaskStatus::Cancelled;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(task_id: &str) -> A2ATaskRecord {
        A2ATaskRecord {
            task_id: task_id.to_string(),
            tenant_id: "tenant-1".into(),
            principal_id: "agent:alice".into(),
            status: A2ATaskStatus::Submitted,
            messages: vec![],
            priority: TaskPriority::Normal,
        }
    }

    #[test]
    fn ep012_unit_a2a_status_round_trip() {
        for (wire, expected) in [
            ("SUBMITTED", A2ATaskStatus::Submitted),
            ("WORKING", A2ATaskStatus::Working),
            ("INPUT_REQUIRED", A2ATaskStatus::InputRequired),
            ("COMPLETED", A2ATaskStatus::Completed),
            ("CANCELLED", A2ATaskStatus::Cancelled),
            ("FAILED", A2ATaskStatus::Failed),
        ] {
            assert_eq!(wire.parse::<A2ATaskStatus>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("IDLE".parse::<A2ATaskStatus>().is_err());
    }

    #[test]
    fn ep012_unit_a2a_lifecycle_transitions() {
        let mut t = record("t1");
        TaskStateMachine::transition(&mut t, A2ATaskStatus::Working).unwrap();
        TaskStateMachine::transition(&mut t, A2ATaskStatus::Completed).unwrap();
        assert!(t.status.is_terminal());
        // Terminal -> anything fails closed.
        assert!(TaskStateMachine::transition(&mut t, A2ATaskStatus::Working).is_err());
    }

    #[test]
    fn ep012_unit_a2a_invalid_transition_fails_closed() {
        let mut t = record("t1");
        // SUBMITTED -> COMPLETED is not allowed.
        assert!(TaskStateMachine::transition(&mut t, A2ATaskStatus::Completed).is_err());
        assert_eq!(t.status, A2ATaskStatus::Submitted);
    }

    #[test]
    fn ep012_unit_a2a_cancel_idempotent_and_conflict() {
        let mut t = record("t1");
        TaskStateMachine::cancel(&mut t).unwrap();
        assert_eq!(t.status, A2ATaskStatus::Cancelled);
        // Idempotent.
        TaskStateMachine::cancel(&mut t).unwrap();
        // Completed cannot be cancelled.
        let mut t2 = record("t2");
        TaskStateMachine::transition(&mut t2, A2ATaskStatus::Working).unwrap();
        TaskStateMachine::transition(&mut t2, A2ATaskStatus::Completed).unwrap();
        assert!(TaskStateMachine::cancel(&mut t2).is_err());
    }
}
