//! EP-038 M3 -- GlitchTip/Sentry-compatible incident sink
//! (SPEC-007 behavior 3; node contract).
//!
//! This crate is the real dependency + transport integration for
//! incidents: it consumes the M1 `IncidentSink` port and the M1
//! `RedactedEnvelope` export boundary, serializes the documented
//! Sentry envelope wire format, and POSTs it over `std::net` to a
//! GlitchTip (or any Sentry-protocol-compatible) endpoint.
//!
//! Invariants:
//! - RAW INCIDENT != EXPORTABLE INCIDENT: the boundary accepts only
//!   `RedactedEnvelope` and re-verifies `assert_exportable()`.
//! - The DSN public key is secret-shaped; never logged, never
//!   rendered, only used for the `X-Sentry-Auth` header and the
//!   envelope `dsn` field.
//! - Provider success is reported truthfully: `Accepted` means HTTP
//!   2xx; stronger semantic verification (readback) is a separate,
//!   explicitly labeled step.
//! - No vendor SDK: hand-rolled `std::net` HTTP/1.1 per the EP-037
//!   connector precedent.
//!
//! Component: GlitchTip 6.1.8 (glitchtip/glitchtip:6.1.8@sha256:
//! 7e497103d3694e95fce232e83193cc2ef6865569314c66eb577b97c7c651008b,
//! MIT, required-prod, sidecar; COMPONENT_REGISTRY.yaml).

pub mod diag;
pub mod dsn;
pub mod envelope;
pub mod event;
pub mod incident;
pub mod sink;
pub mod transport;

pub use dsn::{Dsn, DsnError};
pub use incident::{event_from_redacted, fingerprint_event_id, severity_to_level};
pub use sink::{FailureKind, GlitchTipIncidentSink};
pub use transport::{DeliveryOutcome, TransportFailure};
