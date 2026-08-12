//! Nexus presence behavior layer (EP-003 M2).
//!
//! Owns the deterministic behavior around presence and identity context:
//!
//! - `fusion`: combines voice, room, BLE, mobile, and camera evidence into
//!   `IdentityConfidence` without ever becoming cryptographic authentication
//!   (INV-003, SPEC-005 behavior 3, EP-003 acceptance obligation 2).
//! - `guest`: bounded local permissions for unknown and guest principals
//!   (EP-003 acceptance obligation 3).
//! - `tenant_guard`: cross-tenant and cross-business reads fail without
//!   existence disclosure (EP-003 acceptance obligation 4).
//!
//! This crate imports `nexus-domain` (typed IDs, vocabulary) and
//! `nexus-identity` (identity and presence types) only. No infrastructure
//! crate may be imported here; the dependency-direction tests enforce it.

#![forbid(unsafe_code)]

pub mod error;
pub mod fusion;
pub mod guest;
pub mod tenant_guard;

pub use error::PresenceError;
pub use fusion::PresenceFusionEngine;
pub use guest::{GuestPolicy, PrincipalAccess};
pub use tenant_guard::TenantGuard;
