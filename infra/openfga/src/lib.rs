//! Nexus OpenFGA adapter (EP-008 M3).
//!
//! Implements the provider-neutral `RelationshipAuthorizer` port from
//! `nexus-policy` against a real OpenFGA server (pinned 1.18.1). This
//! crate is the transport boundary: it owns the canonical
//! Nexus-to-OpenFGA tuple mapping, the typed provider failure surface,
//! and redacted telemetry. It does NOT encode contextual risk, time,
//! auth strength, or approval - those belong to OPA / nexus-policy /
//! action-gateway (EP-008 responsibility boundary).
//!
//! Fail closed: every provider error maps to a typed denial/error on
//! the policy surface; no error can become an allow.

#![forbid(unsafe_code)]

pub mod error;
pub mod mapping;
pub mod model;
pub mod telemetry;

pub use error::{OpenFgaError, OpenFgaErrorCode};
pub use mapping::{OpenFgaAuthorizer, OpenFgaConfig};
pub use model::{NEXUS_MODEL_SCHEMA_VERSION, nexus_model_type_definitions};
