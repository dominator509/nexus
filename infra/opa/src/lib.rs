//! Nexus OPA adapter (EP-008 M4).
//!
//! Implements the provider-neutral `ContextPolicyEngine` port from
//! `nexus-policy` against a real OPA server (pinned 1.16.2). This
//! crate is the transport boundary: it owns the canonical
//! PolicyInput-to-OPA input mapping, the typed provider failure
//! surface, and redacted telemetry. It does NOT encode relationship
//! truth (OpenFGA), risk-level calculation (nexus-policy owns it),
//! human approval, approval-digest binding, capability issuance, or
//! action execution (SPEC-005; directive B).
//!
//! Fail closed: undefined policy, malformed responses, provider
//! failures, and version mismatches are typed errors/denials - never
//! an allow.

#![forbid(unsafe_code)]

pub mod error;
pub mod mapping;
pub mod telemetry;

pub use error::{OpaError, OpaErrorCode};
pub use mapping::{OpaAuthorizer, OpaConfig};
pub use telemetry::{TelemetryEvent, TelemetrySink};
