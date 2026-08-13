//! Nexus event nervous system contracts (EP-005 M1).
//!
//! This crate owns the provider-neutral contracts for durable messaging:
//!
//! - `error`: typed event errors (SPEC-006 codes, correlation preserved).
//! - `envelope`: `EventEnvelope`, `EventType`, `EventDataClass` - the
//!   canonical event wire model (SPEC-023 behavior 3).
//! - `outbox`: `OutboxRecord`, `OutboxStatus`, `OutboxRepository` -
//!   transactional outbox port (SPEC-023 behavior 1).
//! - `inbox`: `InboxRecord`, `InboxStatus`, `InboxRepository` - idempotent
//!   consumer inbox port (SPEC-023 behavior 4).
//! - `consumer`: `ConsumerCheckpoint`, `EventConsumer` - durable consumer
//!   port (SPEC-023 behavior 4).
//! - `publisher`: `EventPublisher` - publish port; JetStream ack precedes
//!   outbox completion (SPEC-023 behavior 2).
//! - `provisioner`: `StreamProvisioner`, `StreamConfig` - one canonical
//!   stream and subject namespace (SPEC-023 fallback doctrine).
//!
//! This crate imports `nexus-domain` (typed IDs, vocabulary) and
//! `nexus-data` (UnitOfWork transaction boundary) only. No infrastructure
//! crate may be imported here; the dependency-direction tests enforce it.
//! NATS JetStream implements these ports in `infra/nats` (EP-005 M2+).

#![forbid(unsafe_code)]
// EP-005 architecture decision (owner directive): event ports are
// natively async, so trait methods are declared `async fn`. Auto-trait
// bounds on the returned futures are intentionally unspecified at the
// trait level; concrete adapter futures are Send (async-nats) and the
// composition root drives them from a multi-thread Tokio runtime, which
// enforces Send at compile time (the M3 integration tests run under
// `#[tokio::test(flavor = "multi_thread")]`).
#![allow(async_fn_in_trait)]

pub mod consumer;
pub mod envelope;
pub mod error;
pub mod inbox;
pub mod outbox;
pub mod provisioner;
pub mod publisher;

pub use consumer::{ConsumerCheckpoint, ConsumerConfig, EventConsumer};
pub use envelope::{EventDataClass, EventEnvelope, EventType};
pub use error::{EventError, EventErrorCode};
pub use inbox::{InboxRecord, InboxRepository, InboxStatus};
pub use outbox::{OutboxRecord, OutboxRepository, OutboxStatus};
pub use provisioner::{StreamConfig, StreamProvisioner, StreamStatus};
pub use publisher::EventPublisher;
