//! Event publishing (SPEC-023 behavior 2).
//!
//! JetStream publish acknowledgement precedes outbox completion: the
//! publisher marks an outbox row PUBLISHED only after the transport
//! acknowledges durable storage.
//!
//! The port is natively async. Runtime lifecycle belongs to the Nexus
//! application composition root, never to a port or adapter.

use crate::envelope::EventEnvelope;
use crate::error::EventError;

/// Port: publish events to the durable event bus.
pub trait EventPublisher {
    /// Publish an envelope.
    ///
    /// Contract: the transport must acknowledge durable storage before
    /// this method returns Ok. The outbox row is marked PUBLISHED only
    /// after this acknowledgement (SPEC-023 behavior 2).
    async fn publish(&self, envelope: &EventEnvelope) -> Result<(), EventError>;
}
