//! EP-029 Postiz connector (SPEC-015; M2).
//!
//! Real production Postiz adapter behind the nexus-social
//! `SocialProvider` and `PostizProvider` ports. Postiz is the isolated
//! AGPL sidecar for scheduling and connector breadth (SPEC-015
//! behavior 4); Nexus orchestrates its documented public API and
//! normalizes provider payloads at the infrastructure boundary -
//! Postiz JSON never becomes a domain contract.
//!
//! Permanent invariants (SPEC-015):
//! - Postiz remains an isolated replaceable sidecar (behavior 4).
//! - Platform-native variants preserve ONE campaign objective.
//! - Publishing, replies, spend, and crisis statements use SEPARATE
//!   approval classes (behavior 5); spend/crisis require human
//!   approval (behavior 8).
//! - Policy before mutation; denied actions make ZERO provider calls.
//! - Unbound providers fail closed (Reality rule).

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::PostizAdapter;
pub use observability::{SocialAuditEntry, SocialObservability};
pub use transport::{HttpPostizTransport, PostizIntegration, PostizPostRef, PostizTransport};
