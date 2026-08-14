//! A2A streaming status (SPEC-003 required behavior 3).

use crate::error::A2AError;
use crate::task::A2ATaskStatus;
use serde::{Deserialize, Serialize};

/// A stream cursor (deterministic position marker).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamCursor(pub u64);

/// A streamed A2A status event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamEvent {
    pub sequence: u64,
    pub task_id: String,
    pub status: A2ATaskStatus,
}

/// A subscriber to a task's status stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamSubscriber {
    pub subscriber_id: String,
    pub task_id: String,
    pub push_url: Option<String>,
}

/// Deterministic per-task status stream.
#[derive(Debug, Clone, Default)]
pub struct TaskStream {
    events: Vec<StreamEvent>,
    subscribers: Vec<StreamSubscriber>,
    next_sequence: u64,
}

impl TaskStream {
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a status event; returns its sequence number.
    pub fn push(&mut self, task_id: &str, status: A2ATaskStatus) -> u64 {
        self.next_sequence += 1;
        let event = StreamEvent {
            sequence: self.next_sequence,
            task_id: task_id.to_string(),
            status,
        };
        self.events.push(event);
        self.next_sequence
    }

    /// Read events after a cursor (deterministic replay).
    pub fn since(&self, cursor: &StreamCursor) -> Vec<StreamEvent> {
        self.events
            .iter()
            .filter(|e| e.sequence > cursor.0)
            .cloned()
            .collect()
    }

    pub fn subscribe(&mut self, subscriber: StreamSubscriber) -> Result<(), A2AError> {
        if self
            .subscribers
            .iter()
            .any(|s| s.subscriber_id == subscriber.subscriber_id)
        {
            return Err(A2AError::conflict(format!(
                "subscriber already registered: {}",
                subscriber.subscriber_id
            )));
        }
        self.subscribers.push(subscriber);
        Ok(())
    }

    pub fn subscribers_for(&self, task_id: &str) -> Vec<StreamSubscriber> {
        self.subscribers
            .iter()
            .filter(|s| s.task_id == task_id)
            .cloned()
            .collect()
    }

    pub fn event_count(&self) -> u64 {
        self.events.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_a2a_stream_append_and_replay() {
        let mut stream = TaskStream::new();
        let s1 = stream.push("t1", A2ATaskStatus::Submitted);
        let s2 = stream.push("t1", A2ATaskStatus::Working);
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
        assert_eq!(stream.since(&StreamCursor(0)).len(), 2);
        assert_eq!(stream.since(&StreamCursor(1)).len(), 1);
        assert_eq!(stream.since(&StreamCursor(2)).len(), 0);
    }

    #[test]
    fn ep012_unit_a2a_stream_subscriber_unique() {
        let mut stream = TaskStream::new();
        stream
            .subscribe(StreamSubscriber {
                subscriber_id: "sub-1".into(),
                task_id: "t1".into(),
                push_url: Some("https://push.nexus.local/t1".into()),
            })
            .unwrap();
        assert!(
            stream
                .subscribe(StreamSubscriber {
                    subscriber_id: "sub-1".into(),
                    task_id: "t1".into(),
                    push_url: None,
                })
                .is_err()
        );
        assert_eq!(stream.subscribers_for("t1").len(), 1);
    }
}
