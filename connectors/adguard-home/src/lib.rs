//! EP-030 AdGuard Home connector (SPEC-013; M4).
//!
//! Real production AdGuard Home adapter behind the nexus-sentinel
//! `DnsSecurityProvider` port. AdGuard Home supplies DNS security and
//! telemetry (SPEC-013 behavior 2; COMPONENT_REGISTRY isolated-sidecar,
//! GPL-3.0). Nexus orchestrates its documented control API and
//! normalizes provider payloads at the infrastructure boundary -
//! AdGuard JSON never becomes a domain contract.
//!
//! Permanent invariants (SPEC-013):
//! - AdGuard Home supplies DNS security and telemetry (acceptance
//!   obligation 2).
//! - Telemetry is OBSERVED data, never fabricated; unknown filtering
//!   reasons are normalized at the boundary.
//! - Policy before mutation; denied actions make ZERO provider calls.
//! - Unbound providers fail closed (Reality rule).

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::AdGuardDnsSecurityProvider;
pub use observability::{SentinelAuditEntry, SentinelObservability};
pub use transport::{AdGuardStatus, AdGuardTransport, HttpAdGuardTransport, QueryLogEntry};
