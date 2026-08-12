//! Durable consumers and checkpoints (SPEC-023 behavior 4).
//!
//! Consumers are idempotent and maintain durable checkpoints. After a
//! restart, consumption resumes from the last checkpoint; replay does not
//! create duplicate logical effects because the inbox deduplicates by
//! event ID.

use serde::{Deserialize, Serialize};

use crate::envelope::EventEnvelope;
use crate::error::EventError;

/// A durable consumer checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerCheckpoint {
    /// Consumer identity.
    pub consumer: String,
    /// Stream the consumer reads from.
    pub stream: String,
    /// Subject filter within the stream.
    pub subject: String,
    /// Last processed stream sequence.
    pub last_sequence: u64,
}

/// Consumer configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsumerConfig {
    /// Consumer identity (durable name).
    pub consumer: String,
    /// Stream to read from.
    pub stream: String,
    /// Subject filter.
    pub subject: String,
    /// Bounded batch size per poll.
    pub batch_size: u32,
}

/// Port: durable event consumption with resumable checkpoints.
pub trait EventConsumer {
    /// Fetch the durable checkpoint for this consumer.
    fn checkpoint(&self, consumer: &str) -> Result<Option<ConsumerCheckpoint>, EventError>;

    /// Persist a checkpoint after processing (resume point).
    fn save_checkpoint(&self, checkpoint: &ConsumerCheckpoint) -> Result<(), EventError>;

    /// Poll the next batch of events at or after `after_sequence`.
    fn poll(
        &self,
        config: &ConsumerConfig,
        after_sequence: u64,
    ) -> Result<Vec<EventEnvelope>, EventError>;

    /// Acknowledge an event as processed (inbox deduplication).
    fn ack(&self, consumer: &str, event_id: &str) -> Result<(), EventError>;
}
