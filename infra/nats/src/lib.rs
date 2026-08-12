//! NATS JetStream adapter (EP-005 M2).
//!
//! Implements the provider-neutral `nexus-events` ports on NATS
//! JetStream (pinned: nats 2.14.3, async-nats 0.47.0). The adapter is
//! infrastructure: it may import application ports but never the
//! reverse.
//!
//! - `subject`: canonical subject namespace derivation.
//! - `encode`: EventEnvelope <-> JetStream message bytes.
//! - `provisioner`: `StreamProvisioner` implementation.
//! - `publisher`: `EventPublisher` implementation (ack before outbox
//!   completion, SPEC-023 behavior 2).
//! - `consumer`: `EventConsumer` implementation (durable pull consumer
//!   with checkpoints, SPEC-023 behavior 4).
//!
//! INV-004: NATS is projection/transport, never canonical truth.

#![forbid(unsafe_code)]

pub mod consumer;
pub mod encode;
pub mod provisioner;
pub mod publisher;
pub mod subject;

pub use consumer::NatsEventConsumer;
pub use provisioner::NatsStreamProvisioner;
pub use publisher::NatsEventPublisher;
