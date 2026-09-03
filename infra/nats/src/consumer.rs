//! NATS JetStream event consumer with durable checkpoints (SPEC-023
//! behavior 4).
//!
//! The checkpoint is the durable resume point and lives in a NATS
//! JetStream KV bucket (`nexus_checkpoints`), keyed by consumer name.
//! `save_checkpoint` writes it, `checkpoint` reads it, so consumption
//! after a restart resumes from the last checkpoint instead of the
//! beginning (AUD-008: the pre-fix adapter returned `Ok(None)` always and
//! `save_checkpoint` was a no-op - checkpoints were never persisted).
//!
//! `poll` creates an EPHEMERAL pull consumer per call, positioned by
//! `after_sequence` - the application-owned resume point read from the
//! checkpoint. Ephemeral consumers die with the connection and are never
//! persisted server-side, so polling cannot leak durable consumers (the
//! pre-fix adapter created a durable consumer per sequence,
//! `{consumer}-{after_sequence}`, accumulating unbounded server state).
//! A stable durable consumer is deliberately avoided: it would track its
//! own server-side position and ignore the checkpoint the application
//! passes in, breaking the port's resume contract.
//!
//! Explicit acknowledgement: `poll` retains the delivered JetStream
//! message handles keyed by (consumer, event_id); `ack` acknowledges the
//! matching delivery on the server. Unacked messages stay in the stream
//! (fail-closed) and are redelivered to a later poll that starts at or
//! before their sequence.
//!
//! Runtime lifecycle: this adapter never owns a Tokio runtime. It must be
//! driven from the Nexus application composition root's async runtime.

use std::collections::HashMap;
use std::sync::Mutex;

use async_nats::jetstream::{self, kv};
use nexus_events::{
    ConsumerCheckpoint, ConsumerConfig, EventConsumer, EventEnvelope, EventError, EventErrorCode,
};

use crate::encode::decode;

/// NATS KV bucket that holds the durable consumer checkpoints.
const CHECKPOINT_BUCKET: &str = "nexus_checkpoints";

/// `EventConsumer` implemented on NATS JetStream.
pub struct NatsEventConsumer {
    context: jetstream::Context,
    /// Durable checkpoint store (JetStream KV, keyed by consumer name).
    checkpoints: kv::Store,
    /// Delivered-but-unacknowledged JetStream messages keyed by
    /// `(consumer, event_id)` so `ack` can acknowledge the exact
    /// delivery (SPEC-023 behavior 4).
    pending: Mutex<HashMap<(String, String), jetstream::Message>>,
}

impl NatsEventConsumer {
    /// Connect to a NATS server and wrap its JetStream context.
    ///
    /// Ensures the checkpoint KV bucket exists (get-or-create, tolerant
    /// of a concurrent create racing this one).
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
        let checkpoints = Self::ensure_checkpoint_store(&context).await?;
        Ok(Self {
            context,
            checkpoints,
            pending: Mutex::new(HashMap::new()),
        })
    }

    async fn ensure_checkpoint_store(
        context: &jetstream::Context,
    ) -> Result<kv::Store, EventError> {
        match context.get_key_value(CHECKPOINT_BUCKET).await {
            Ok(store) => Ok(store),
            Err(_) => {
                let config = kv::Config {
                    bucket: CHECKPOINT_BUCKET.to_string(),
                    ..Default::default()
                };
                match context.create_key_value(config).await {
                    Ok(store) => Ok(store),
                    // A concurrent connect may have created the bucket
                    // between the failed get and this create; re-read.
                    Err(_) => context.get_key_value(CHECKPOINT_BUCKET).await.map_err(|e| {
                        EventError::new(
                            EventErrorCode::ExternalProvider,
                            format!("nats checkpoint bucket: {e}"),
                        )
                    }),
                }
            }
        }
    }
}

impl EventConsumer for NatsEventConsumer {
    async fn checkpoint(&self, consumer: &str) -> Result<Option<ConsumerCheckpoint>, EventError> {
        let entry = self.checkpoints.get(consumer).await.map_err(|e| {
            EventError::new(
                EventErrorCode::ExternalProvider,
                format!("nats checkpoint read {consumer}: {e}"),
            )
        })?;
        match entry {
            Some(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|e| {
                EventError::new(
                    EventErrorCode::Invariant,
                    format!("corrupt checkpoint for {consumer}: {e}"),
                )
            }),
            None => Ok(None),
        }
    }

    async fn save_checkpoint(&self, checkpoint: &ConsumerCheckpoint) -> Result<(), EventError> {
        let bytes = serde_json::to_vec(checkpoint).map_err(|e| {
            EventError::new(
                EventErrorCode::Validation,
                format!("checkpoint serialize: {e}"),
            )
        })?;
        self.checkpoints
            .put(&checkpoint.consumer, bytes.into())
            .await
            .map_err(|e| {
                EventError::new(
                    EventErrorCode::ExternalProvider,
                    format!("nats checkpoint write {}: {e}", checkpoint.consumer),
                )
            })?;
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
        // Ephemeral pull consumer, positioned by the application-owned
        // resume point. Never durable: a durable consumer per sequence
        // would accumulate server-side state (the AUD-008 defect) and a
        // single stable durable consumer would track its own position,
        // ignoring the checkpoint passed in here.
        let deliver_policy = if after_sequence == 0 {
            jetstream::consumer::DeliverPolicy::All
        } else {
            jetstream::consumer::DeliverPolicy::ByStartSequence {
                start_sequence: after_sequence,
            }
        };
        let pull = stream
            .create_consumer(jetstream::consumer::pull::Config {
                filter_subject: config.subject.clone(),
                ack_policy: jetstream::consumer::AckPolicy::Explicit,
                deliver_policy,
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
            let mut messages = pull
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
                                (
                                    config.consumer.clone(),
                                    envelope.event_id.as_str().to_string(),
                                ),
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
