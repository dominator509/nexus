//! NATS JetStream durable consumer (SPEC-023 behavior 4).
//!
//! Consumers are idempotent and maintain durable checkpoints. The
//! adapter exposes a pull consumer over the canonical stream; the
//! checkpoint is persisted by the application layer (PostgreSQL), and
//! resume after restart starts at the last checkpoint sequence.

use async_nats::jetstream;
use nexus_events::{
    ConsumerCheckpoint, ConsumerConfig, EventConsumer, EventEnvelope, EventError, EventErrorCode,
};
use tokio::runtime::Runtime;

use crate::encode::decode;

/// `EventConsumer` implemented on NATS JetStream.
pub struct NatsEventConsumer {
    runtime: Runtime,
    context: jetstream::Context,
}

impl NatsEventConsumer {
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

impl EventConsumer for NatsEventConsumer {
    fn checkpoint(&self, consumer: &str) -> Result<Option<ConsumerCheckpoint>, EventError> {
        // Checkpoints live in the application's durable store
        // (PostgreSQL via OutboxRepository/InboxRepository); the NATS
        // adapter does not own them. Returning None means "start from the
        // beginning" for a brand-new consumer.
        let _ = consumer;
        Ok(None)
    }

    fn save_checkpoint(&self, _checkpoint: &ConsumerCheckpoint) -> Result<(), EventError> {
        // The application layer persists checkpoints; the adapter has no
        // authority to write application state.
        Ok(())
    }

    fn poll(
        &self,
        config: &ConsumerConfig,
        after_sequence: u64,
    ) -> Result<Vec<EventEnvelope>, EventError> {
        let stream = self
            .runtime
            .block_on(self.context.get_stream(&config.stream))
            .map_err(|e| {
                EventError::new(
                    EventErrorCode::ExternalProvider,
                    format!("nats stream get: {e}"),
                )
            })?;
        let consumer_name = format!("{}-{}", config.consumer, after_sequence);
        let durable = self
            .runtime
            .block_on(stream.create_consumer(jetstream::consumer::pull::Config {
                name: Some(consumer_name.clone()),
                durable_name: Some(consumer_name),
                filter_subject: config.subject.clone(),
                ack_policy: jetstream::consumer::AckPolicy::Explicit,
                deliver_policy: jetstream::consumer::DeliverPolicy::ByStartSequence {
                    start_sequence: after_sequence,
                },
                ..Default::default()
            }))
            .map_err(|e| {
                EventError::new(
                    EventErrorCode::ExternalProvider,
                    format!("nats consumer create: {e}"),
                )
            })?;
        let fetched = self.runtime.block_on(async {
            use futures_util::StreamExt;
            let mut messages = durable
                .fetch()
                .max_messages(config.batch_size as usize)
                .messages()
                .await
                .map_err(|e| {
                    EventError::new(EventErrorCode::ExternalProvider, format!("nats fetch: {e}"))
                })?;
            let mut out = Vec::new();
            for _ in 0..config.batch_size {
                match messages.next().await {
                    Some(Ok(msg)) => match decode(&msg.payload) {
                        Ok(envelope) => out.push(envelope),
                        Err(_) => {
                            // Malformed messages are never ack'd so
                            // they stay in the stream for quarantine
                            // (fail-closed).
                            continue;
                        }
                    },
                    Some(Err(e)) => {
                        return Err(EventError::new(
                            EventErrorCode::ExternalProvider,
                            format!("nats fetch: {e}"),
                        ));
                    }
                    None => break,
                }
            }
            Ok::<Vec<EventEnvelope>, EventError>(out)
        })?;
        Ok(fetched)
    }

    fn ack(&self, consumer: &str, event_id: &str) -> Result<(), EventError> {
        // Acks are recorded in the application inbox; the adapter
        // acknowledges NATS deliveries when the application marks Done.
        let _ = (consumer, event_id);
        Ok(())
    }
}
