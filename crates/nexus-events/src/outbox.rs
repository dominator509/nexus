//! Transactional outbox (SPEC-023 behavior 1).
//!
//! State changes and outbox records commit in one PostgreSQL transaction.
//! The outbox port takes the `nexus-data` `UnitOfWork` transaction
//! boundary so a domain write and its outbox append are atomic.

use nexus_data::UnitOfWork;
use serde::{Deserialize, Serialize};

use crate::envelope::EventEnvelope;
use crate::error::EventError;

/// Lifecycle of an outbox row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OutboxStatus {
    /// Appended inside the same transaction as the state change.
    Pending,
    /// Publish attempt in flight.
    Publishing,
    /// JetStream acknowledged the publish.
    Published,
    /// Publish failed; bounded retry applies.
    Failed,
}

impl OutboxStatus {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Publishing => "PUBLISHING",
            Self::Published => "PUBLISHED",
            Self::Failed => "FAILED",
        }
    }
}

/// A row in the transactional outbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutboxRecord {
    /// Outbox row identifier.
    pub outbox_id: String,
    /// The envelope awaiting publication.
    pub envelope: EventEnvelope,
    /// Publication state.
    pub status: OutboxStatus,
    /// Number of publish attempts (bounded retry).
    pub attempts: u32,
    /// Last failure detail, redacted.
    pub last_error: Option<String>,
}

impl OutboxRecord {
    /// Mark a record as failed with a redacted reason.
    pub fn fail(&mut self, reason: impl Into<String>) {
        self.status = OutboxStatus::Failed;
        self.attempts = self.attempts.saturating_add(1);
        self.last_error = Some(reason.into());
    }

    /// Whether the record is still awaiting publication.
    pub fn is_pending(&self) -> bool {
        self.status == OutboxStatus::Pending || self.status == OutboxStatus::Publishing
    }
}

/// Port: durable outbox behind the UnitOfWork transaction boundary.
pub trait OutboxRepository {
    /// Append an envelope atomically with the caller's state change.
    ///
    /// The `UnitOfWork` guarantees the state mutation and the outbox
    /// insert commit or roll back together (SPEC-023 behavior 1).
    fn append(
        &self,
        tx: &mut dyn UnitOfWork,
        envelope: &EventEnvelope,
    ) -> Result<OutboxRecord, EventError>;

    /// Fetch pending records for the publisher (bounded batch).
    fn fetch_pending(&self, limit: u32) -> Result<Vec<OutboxRecord>, EventError>;

    /// Mark a record as in-flight.
    fn mark_publishing(&self, outbox_id: &str) -> Result<(), EventError>;

    /// Mark a record published only after JetStream ack (SPEC-023
    /// behavior 2).
    fn mark_published(&self, outbox_id: &str) -> Result<(), EventError>;

    /// Mark a record failed with a redacted reason.
    fn mark_failed(&self, outbox_id: &str, reason: &str) -> Result<(), EventError>;
}
