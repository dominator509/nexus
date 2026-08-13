//! NATS JetStream durable consumer (SPEC-023 behavior 4).
//!
//! Consumers are idempotent and maintain durable checkpoints. The
//! adapter exposes a pull consumer over the canonical stream; the
//! checkpoint is persisted by the application layer (PostgreSQL), and
//! resume after restart starts at the last checkpoint sequence.
//!
//! Explicit acknowledgement: `poll` retains the delivered JetStream
//! message handles keyed by (consumer, event_id); `ack` acknowledges the
//! matching delivery on the server. Unacked messages stay in the stream
//! (fail-closed) and are redelivered after the server ack-wait.
//!
//! Runtime lifecycle: this adapter never owns a Tokio runtime. It must be
//! driven from the Nexus application composition root's async runtime.

use std::collections::HashMap;
use std::sync::Mutex;

use async_nats::jetstream;
use nexus_events::{
    ConsumerCheckpoint, ConsumerConfig, EventConsumer, EventEnvelope, EventError, EventErrorCode,
};

use crate::encode::decode;

/// `EventConsumer` implemented on NATS JetStream.
pub struct NatsEventConsumer {
    context: jetstream::Context,
    /// Delivered-but-unacknowledged JetStream messages keyed by
    /// `(consumer, event_id)` so `ack` can acknowledge the exact
    /// delivery (SPEC-023 behavior 4).
    pending: Mutex<HashMap<(String, String), jetstream::Message>>,
}

impl NatsEventConsumer {
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
        Ok(Self {
            context,
            pending: Mutex::new(HashMap::new()),
        })
    }
}

impl EventConsumer for NatsEventConsumer {
    async fn checkpoint(&self, consumer: &str) -> Result<Option<ConsumerCheckpoint>, EventError> {
        // Checkpoints live in the application's durable store
        // (PostgreSQL via OutboxRepository/InboxRepository); the NATS
        // adapter does not own them. Returning None means "start from the
        // beginning" for a brand-new consumer.
        let _ = consumer;
        Ok(None)
    }

    async fn save_checkpoint(&self, _checkpoint: &ConsumerCheckpoint) -> Result<(), EventError> {
        // The application layer persists checkpoints; the adapter has no
        // authority to write application state.
        Ok(())
    }

    async fn poll(
        &self,
        config: &ConsumerConfig,
        after_sequence: u64,
    ) -> Result<Vec<EventEnvelope>, EventError> {
        let stream = self.context.get_stream(&config.stream).await.map_err(|e| {
            EventError::new(
                EventErrorCode::ExternalProvider,
                format!("nats stream get: {e}"),
            )
        })?;
        let consumer_name = format!("{}-{}", config.consumer, after_sequence);
        let durable = stream
            .create_consumer(jetstream::consumer::pull::Config {
                name: Some(consumer_name.clone()),
                durable_name: Some(consumer_name),
                filter_subject: config.subject.clone(),
                ack_policy: jetstream::consumer::AckPolicy::Explicit,
                deliver_policy: jetstream::consumer::DeliverPolicy::ByStartSequence {
                    start_sequence: after_sequence,
                },
                ..Default::default()
            })
            .await
            .map_err(|e| {
                EventError::new(
                    EventErrorCode::ExternalProvider,
                    format!("nats consumer create: {e}"),
                )
            })?;
        let fetched = async {
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
                        Ok(envelope) => {
                            // Retain the delivery for explicit ack.
                            self.pending.lock().unwrap().insert(
                                (config.consumer.clone(), envelope.event_id.as_str().to_string()),
                                msg,
                            );
                            out.push(envelope);
                        }
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
        }
        .await?;
        Ok(fetched)
    }

    async fn ack(&self, consumer: &str, event_id: &str) -> Result<(), EventError> {
        let key = (consumer.to_string(), event_id.to_string());
        let msg = self.pending.lock().unwrap().remove(&key);
        match msg {
            Some(msg) => {
                // Acknowledge the exact delivery on the server. On
                // failure the handle is retained for a retry and the
                // message stays unacked (fail-closed).
                if let Err(e) = msg.ack().await {
                    self.pending.lock().unwrap().insert(key, msg);
                    return Err(EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("nats ack: {e}"),
                    ));
                }
                Ok(())
            }
            None => {
                // Idempotent: already acknowledged or never delivered in
                // a retained batch.
                Ok(())
            }
        }
    }
}
