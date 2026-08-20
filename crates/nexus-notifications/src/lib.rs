//! EP-032 provider-neutral notification contracts (SPEC-014 behavior 7).
//!
//! The Communication Router selects push, desktop, speaker, SMS, email,
//! phone, watch, car, or future robot based on person, presence,
//! privacy, urgency, quiet hours, cost, and availability. This crate
//! owns the provider-neutral contract layer: the canonical
//! `NotificationEnvelope` (wire contract `schemas/notification-envelope.
//! schema.json`), the `ChannelProvider` port, the `NotificationRouter`
//! port, delivery policy, privacy routing, escalation policy, and
//! delivery receipts. Connector implementations live under
//! connectors/push, connectors/sms, connectors/desktop-notify (M2+).
//!
//! Permanent invariants (SPEC-014):
//! - Person, urgency, privacy, presence, availability, quiet hours,
//!   and acknowledgement determine delivery (acceptance obligation 1).
//! - Sensitive shared-room responses route privately (acceptance
//!   obligation 2).
//! - Failures escalate across configured channels WITHOUT duplication
//!   (acceptance obligation 3).
//! - Every delivery has a receipt and correlation (acceptance
//!   obligation 4).
//! - Unbound providers advertise nothing and fail closed (Reality
//!   rule); free-form provider payloads are normalized at the
//!   infrastructure boundary and never become domain contracts.
//!
//! Dependency direction: this crate depends only on nexus-domain
//! (contract crate) and serde/serde_json. Provider implementations
//! never appear here.

#![forbid(unsafe_code)]

pub mod error;
pub mod model;
pub mod observability;
pub mod provider;
pub mod router;
pub mod vocabulary;

pub use error::{NotificationError, NotificationErrorCode};
pub use model::{
    DeliveryPolicy, DeliveryReceipt, EscalationPolicy, NotificationEnvelope, PrivacyRouting,
};
pub use observability::{NotificationObservability, NotificationObservation};
pub use provider::{
    ChannelProvider, NotificationRouter, UnboundChannelProvider, UnboundNotificationRouter,
};
pub use router::EscalatingNotificationRouter;
pub use vocabulary::{
    DeliveryReceiptId, DeliveryState, EscalationStage, NotificationId, NotificationUrgency,
    SmsDestination,
};
