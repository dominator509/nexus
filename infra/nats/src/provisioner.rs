//! NATS JetStream stream provisioning (SPEC-023 fallback doctrine).
//!
//! Runtime lifecycle: this adapter never owns a Tokio runtime. It must be
//! driven from the Nexus application composition root's async runtime.

use async_nats::jetstream;
use nexus_events::{EventError, EventErrorCode, StreamConfig, StreamProvisioner, StreamStatus};

/// `StreamProvisioner` implemented on NATS JetStream.
///
/// Uses the pinned NATS server (2.14.3) through async-nats 0.47.0. The
/// canonical stream is idempotently created or updated; subjects are
/// derived from the canonical namespace.
pub struct NatsStreamProvisioner {
    context: jetstream::Context,
}

impl NatsStreamProvisioner {
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

impl StreamProvisioner for NatsStreamProvisioner {
    async fn ensure_stream(&self, config: &StreamConfig) -> Result<StreamStatus, EventError> {
        let subjects: Vec<String> = config.subjects.clone();
        let spec = jetstream::stream::Config {
            name: config.stream.clone(),
            subjects: subjects.clone(),
            max_messages: config.max_messages,
            max_age: std::time::Duration::from_secs(config.max_age_seconds.max(0) as u64),
            ..Default::default()
        };
        let mut stream = self.context.get_or_create_stream(spec).await.map_err(|e| {
            EventError::new(
                EventErrorCode::ExternalProvider,
                format!("nats stream create: {e}"),
            )
        })?;
        let info = stream.info().await.map_err(|e| {
            EventError::new(
                EventErrorCode::ExternalProvider,
                format!("nats stream info: {e}"),
            )
        })?;
        Ok(StreamStatus {
            stream: config.stream.clone(),
            exists: true,
            message_count: Some(info.state.messages as i64),
        })
    }

    async fn status(&self, stream: &str) -> Result<StreamStatus, EventError> {
        let stream_name = stream.to_string();
        match self.context.get_stream(stream).await {
            Ok(mut stream) => {
                let info = stream.info().await.map_err(|e| {
                    EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("nats stream info: {e}"),
                    )
                })?;
                Ok(StreamStatus {
                    stream: stream_name,
                    exists: true,
                    message_count: Some(info.state.messages as i64),
                })
            }
            Err(_) => Ok(StreamStatus {
                stream: stream_name,
                exists: false,
                message_count: None,
            }),
        }
    }
}
