//! EP-032 SMS connector (M3): Gammu SMSD channel provider behind the
//! nexus-notifications `ChannelProvider` port (SPEC-014 behavior 7;
//! channel class SMS).
//!
//! The connector speaks the DOCUMENTED Gammu SMSD interface
//! (docs.gammu.org "SMSD Database Structure", "SQL Service", and
//! "gammu-smsd"): messages are enqueued into the daemon's `outbox`
//! table exactly as the documented `create_outbox` query does, the
//! daemon submits them through the GSM modem, moves them to
//! `sentitems`, and - when delivery reports are enabled - records a
//! real `DeliveryOK` state with `DeliveryDateTime` only after an
//! actual SMS-STATUS-REPORT arrives from the provider path. No
//! delivery is ever inferred from outbox disappearance, process exit,
//! or modem command acceptance (Reality rule; SENT != DELIVERED).
//!
//! Permanent invariants (SPEC-014; EP-032):
//! - A delivery is only ever reported through a `DeliveryReceipt`;
//!   the connector never fabricates success.
//! - Unbound providers advertise nothing and fail closed.
//! - `DeliveryState::Delivered` requires the provider's authoritative
//!   `DeliveryOK` state WITH a recorded `DeliveryDateTime` (a real
//!   delivery report). `SendingOK`/`SendingOKNoReport`/
//!   `DeliveryPending`/`DeliveryUnknown` map to `Sending`, never to
//!   `Delivered`.
//! - A duplicate notification id is rejected with Conflict
//!   (idempotency; a bounded recent-delivery ring), and the outbox
//!   `CreatorID` binds the provider message identity to the
//!   `NotificationId` (exact correlation, never destination+body).
//! - Destinations use the provider-neutral notification value object
//!   `nexus_notifications::SmsDestination` (SPEC-014 behavior 6;
//!   canonical E.164-ish normalization, validated in `new` AND serde)
//!   and are validated BEFORE any provider mutation.
//! - Sensitive payload content (SMS body, full destination) is never
//!   logged or embedded in errors/telemetry (redaction-safe).

#![forbid(unsafe_code)]

pub mod adapter;
pub mod db;
pub mod gateway;

pub use adapter::{SmsChannelProvider, SmsChannelProviderError};
pub use db::{PostgresSmsDb, SmsDb, SmsDbStatusRow, SqliteSmsDb, CERTIFIED_SCHEMA_VERSION};
pub use gateway::{
    GammuSmsdGateway, SmsGateway, SmsProviderRef, SmsProviderState, SmsProviderStatus,
};
