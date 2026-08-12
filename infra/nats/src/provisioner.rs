//! NATS JetStream stream provisioning (SPEC-023 fallback doctrine).

use async_nats::jetstream;
use nexus_events::{EventError, EventErrorCode, StreamConfig, StreamProvisioner, StreamStatus};
use tokio::runtime::Runtime;

/// `StreamProvisioner` implemented on NATS JetStream.
///
/// Uses the pinned NATS server (2.14.3) through async-nats 0.47.0. The
/// canonical stream is idempotently created or updated; subjects are
/// derived from the canonical namespace. The port trait is synchronous,
/// so the adapter owns a tokio runtime to bridge the async client.
pub struct NatsStreamProvisioner {
    runtime: Runtime,
    context: jetstream::Context,
}

impl NatsStreamProvisioner {
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

impl StreamProvisioner for NatsStreamProvisioner {
    fn ensure_stream(&self, config: &StreamConfig) -> Result<StreamStatus, EventError> {
        let subjects: Vec<String> = config.subjects.clone();
        let spec = jetstream::stream::Config {
            name: config.stream.clone(),
            subjects: subjects.clone(),
            max_messages: config.max_messages,
            max_age: std::time::Duration::from_secs(config.max_age_seconds.max(0) as u64),
            ..Default::default()
        };
        let mut stream = self
            .runtime
            .block_on(self.context.get_or_create_stream(spec))
            .map_err(|e| {
                EventError::new(
                    EventErrorCode::ExternalProvider,
                    format!("nats stream create: {e}"),
                )
            })?;
        let info = self.runtime.block_on(stream.info()).map_err(|e| {
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

    fn status(&self, stream: &str) -> Result<StreamStatus, EventError> {
        let stream_name = stream.to_string();
        match self.runtime.block_on(self.context.get_stream(stream)) {
            Ok(mut stream) => {
                let info = self.runtime.block_on(stream.info()).map_err(|e| {
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
