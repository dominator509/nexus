//! EP-032 push connector (M2): mobile push channel provider behind the
//! nexus-notifications `ChannelProvider` port (SPEC-014 behavior 7;
//! channel class MOBILE_PUSH).
//!
//! The push connector is provider-neutral. Its transport writes the
//! canonical `NotificationEnvelope` (schema `notification-envelope`)
//! as a JSON line to an arbitrary duplex byte source (socket, pipe,
//! file) and reads one JSON ack line back. The ack wire shape is
//! owned and documented by this connector (no external push provider
//! API is claimed):
//!
//! ```json
//! {"provider_ref":"...","delivered":true,"delivered_at_ms":123,"error":null}
//! ```
//!
//! Permanent invariants (SPEC-014; EP-032):
//! - A delivery is only ever reported through a `DeliveryReceipt`;
//!   the connector never fabricates success (Reality rule).
//! - Unbound providers advertise nothing and fail closed.
//! - A malformed ack fails closed (External), never guessed.
//! - A duplicate notification id is rejected with Conflict
//!   (idempotency; a bounded recent-delivery ring).
//! - Sensitive payload content is never logged or embedded in
//!   errors/telemetry.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod transport;

pub use adapter::{PushChannelProvider, PushChannelProviderError};
pub use transport::{JsonPushTransport, PushAck, PushTransport};
