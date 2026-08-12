//! Idempotent consumer inbox (SPEC-023 behavior 4).
//!
//! Consumers deduplicate by event ID: a delivered event is recorded in
//! the inbox exactly once per consumer, so replay does not create
//! duplicate logical effects.

use serde::{Deserialize, Serialize};

use crate::error::EventError;

/// Lifecycle of an inbox row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InboxStatus {
    /// Delivered but not yet processed.
    New,
    /// Processing in flight.
    Processing,
    /// Processed successfully.
    Done,
    /// Processing failed; bounded retry applies.
    Failed,
}

impl InboxStatus {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::New => "NEW",
            Self::Processing => "PROCESSING",
            Self::Done => "DONE",
            Self::Failed => "FAILED",
        }
    }
}

/// A row in the consumer inbox (deduplication ledger).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InboxRecord {
    /// Consumer identity that owns this row.
    pub consumer: String,
    /// The deduplication key (event ID).
    pub event_id: String,
    /// Processing state.
    pub status: InboxStatus,
    /// Number of processing attempts.
    pub attempts: u32,
}

/// Port: idempotent consumer inbox.
pub trait InboxRepository {
    /// Record a delivery; returns false when the event was already seen
    /// by this consumer (deduplication).
    fn record_delivery(&self, consumer: &str, event_id: &str) -> Result<bool, EventError>;

    /// Mark a delivery as processed.
    fn mark_done(&self, consumer: &str, event_id: &str) -> Result<(), EventError>;

    /// Mark a delivery as failed with a redacted reason.
    fn mark_failed(&self, consumer: &str, event_id: &str, reason: &str) -> Result<(), EventError>;

    /// Fetch unprocessed deliveries for a consumer (bounded batch).
    fn fetch_new(&self, consumer: &str, limit: u32) -> Result<Vec<InboxRecord>, EventError>;
}
