//! NATS JetStream publishing (SPEC-023 behavior 2).
//!
//! JetStream publish acknowledgement precedes outbox completion: the
//! publisher returns Ok only after the server acknowledges durable
//! storage. The outbox row is marked PUBLISHED by the caller only after
//! this acknowledgement.

use async_nats::jetstream;
use nexus_events::{EventEnvelope, EventError, EventErrorCode, EventPublisher};
use tokio::runtime::Runtime;

use crate::encode::encode;
use crate::subject::subject_for;

/// `EventPublisher` implemented on NATS JetStream.
pub struct NatsEventPublisher {
    runtime: Runtime,
    context: jetstream::Context,
}

impl NatsEventPublisher {
    /// Connect to a NATS server and wrap its JetStream context.
    pub fn connect(url: &str) -> Result<Self, EventError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                EventError::new(EventErrorCode::Unavailable, format!("tokio runtime: {e}"))
            })?;
        let client = runtime.block_on(async_nats::connect(url)).map_err(|e| {
            EventError::new(
                EventErrorCode::Unavailable,
                format!("nats connect {url}: {e}"),
            )
        })?;
        let context = jetstream::new(client);
        Ok(Self { runtime, context })
    }
}

impl EventPublisher for NatsEventPublisher {
    fn publish(&self, envelope: &EventEnvelope) -> Result<(), EventError> {
        let bytes = encode(envelope)?;
        let subject = subject_for(envelope);
        // JetStream publish: block until the server has acknowledged
        // durable storage (SPEC-023 behavior 2). A timeout or nack
        // surfaces as Err and the outbox row stays PENDING.
        self.runtime.block_on(async {
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
        })
    }
}
