//! Stream provisioning (SPEC-023 fallback doctrine).
//!
//! One canonical stream and subject namespace before introducing stream
//! sharding. The provisioner ensures the durable stream exists and
//! reports its status; it never invents subjects or streams beyond the
//! canonical namespace.
//!
//! The port is natively async. Runtime lifecycle belongs to the Nexus
//! application composition root, never to a port or adapter.

use serde::{Deserialize, Serialize};

use crate::error::EventError;

/// Desired stream configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamConfig {
    /// Canonical stream name.
    pub stream: String,
    /// Subject namespace this stream owns.
    pub subjects: Vec<String>,
    /// Maximum messages retained (bounded storage).
    pub max_messages: i64,
    /// Maximum age of a message in seconds (bounded storage).
    pub max_age_seconds: i64,
}

/// Observed stream status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StreamStatus {
    /// Canonical stream name.
    pub stream: String,
    /// Whether the stream exists.
    pub exists: bool,
    /// Number of stored messages when known.
    pub message_count: Option<i64>,
}

/// Port: ensure the canonical stream exists.
pub trait StreamProvisioner {
    /// Create or update the canonical stream; idempotent.
    async fn ensure_stream(&self, config: &StreamConfig) -> Result<StreamStatus, EventError>;

    /// Report current stream status without mutating it.
    async fn status(&self, stream: &str) -> Result<StreamStatus, EventError>;
}
