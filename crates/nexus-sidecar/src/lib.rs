//! EP-011 M4 hardened Connector Sidecar boundary (SPEC-022).
//!
//! The sidecar is the transport/process hardening layer in front of
//! the EP-011 SDK abstractions:
//!
//! ```text
//! test client
//!    |
//! real localhost TCP/HTTP
//!    |
//! nexus-sidecar          <- this crate
//!    |
//! EP-011 SDK interface
//!    |
//! fixture provider       <- real provider process
//! ```
//!
//! It owns request parsing, protocol/version validation, body/request
//! limits, class-specific dispatch routing, bounded timeouts,
//! credential-broker reference scope, webhook normalization ingress,
//! legacy-poller checkpoint integrity, structured/redacted
//! observability, and controlled shutdown. It does NOT own EP-008
//! authorization, EP-009 secret authority, EP-010 capability
//! semantics, EP-005 event durability, or EP-006 workflow durability.
//!
//! Failures are typed (`SidecarError` carrying the canonical
//! `SdkError` envelope) and fail closed. The sidecar never emits
//! secrets, never exposes a debug endpoint, and binds loopback only.

#![forbid(unsafe_code)]

pub mod credential;
pub mod dispatch;
pub mod envelope;
pub mod error;
pub mod limits;
pub mod poller;
pub mod provider;
pub mod server;
pub mod telemetry;
pub mod tenant;
pub mod version;
pub mod webhook;

pub use credential::CredentialScope;
pub use dispatch::{CapabilityClassTable, ConnectorTable};
pub use envelope::{ENVELOPE_SCHEMA_VERSION, RequestEnvelope};
pub use error::{SidecarError, SidecarErrorKind};
pub use limits::Limits;
pub use poller::PollSource;
pub use provider::{ProviderClient, ProviderError};
pub use server::{SidecarConfig, SidecarServer};
pub use telemetry::{TelemetryEvent, TelemetrySink};
pub use tenant::TenantBinding;
pub use version::PROTOCOL_VERSION;
pub use webhook::{WebhookIngress, WebhookPolicy};
