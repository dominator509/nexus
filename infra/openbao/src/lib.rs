//! Nexus OpenBao adapter (EP-009 M2).
//!
//! Implements the nexus-trust `SecretStore` contract on the real pinned
//! OpenBao server (2.5.4) with least-privilege AppRole machine auth,
//! KV-v2 secret semantics, one-time response wrapping, typed fail-closed
//! errors, and redacted telemetry (SPEC-005, SPEC-020).
//!
//! RESPONSIBILITY BOUNDARY (directive N): OpenBao is the ONLINE SECRET
//! AUTHORITY. SOPS+age bootstrap configuration lives beside it under
//! `config/sops` and is only reachable through `BootstrapSecretStore`
//! operations; OpenBao unavailability NEVER silently falls back to
//! decrypting local SOPS files for runtime secrets.

#![forbid(unsafe_code)]

pub mod auth;
pub mod error;
pub mod sops;
pub mod store;
pub mod telemetry;
pub mod token;

pub use error::{OpenBaoError, OpenBaoErrorCode};
pub use sops::SopsBootstrapStore;
pub use store::{AppRoleLogin, OpenBaoStore, WrappedHandoff};
pub use token::OpenBaoTokenIssuer;

/// Re-export the canonical trust surface for adapter callers.
pub use nexus_trust::{SecretReference, SecretStore, SecretValue, TrustError};

#[cfg(test)]
mod lib_tests;
