//! NATS JetStream publishing (SPEC-023 behavior 2).
//!
//! JetStream publish acknowledgement precedes outbox completion: the
//! publisher returns Ok only after the server acknowledges durable
//! storage. The outbox row is marked PUBLISHED by the caller only after
//! this acknowledgement.
//!
//! Runtime lifecycle: this adapter never owns a Tokio runtime. It must be
//! driven from the Nexus application composition root's async runtime.

use async_nats::jetstream;
use nexus_events::{EventEnvelope, EventError, EventErrorCode, EventPublisher};

use crate::encode::encode;
use crate::subject::subject_for;

/// `EventPublisher` implemented on NATS JetStream.
pub struct NatsEventPublisher {
    context: jetstream::Context,
}

impl NatsEventPublisher {
    /// Connect to a NATS server and wrap its JetStream context.
    ///
    /// Must be called from inside a Tokio runtime owned by the
    /// composition root.
    pub async fn connect(url: &str) -> Result<Self, EventError> {
        let client = async_nats::connect(url).await.map_err(|e| {
            EventError::new(
                EventErrorCode::Unavailable,
                format!("nats connect {url}: {e}"),
            )
        })?;
        let context = jetstream::new(client);
        Ok(Self { context })
    }
}

impl EventPublisher for NatsEventPublisher {
    async fn publish(&self, envelope: &EventEnvelope) -> Result<(), EventError> {
        let bytes = encode(envelope)?;
        let subject = subject_for(envelope);
        // JetStream publish: return only after the server has
        // acknowledged durable storage (SPEC-023 behavior 2). A timeout
        // or nack surfaces as Err and the outbox row stays PENDING.
        let ack = self
            .context
            .publish(subject, bytes.into())
            .await
            .map_err(|e| {
                EventError::new(
                    EventErrorCode::ExternalProvider,
                    format!("nats publish: {e}"),
                )
            })?;
        ack.await.map_err(|e| {
            EventError::new(
                EventErrorCode::ExternalProvider,
                format!("nats publish ack: {e}"),
            )
        })?;
        Ok(())
    }
}
