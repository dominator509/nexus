//! EP-031 CrowdSec connector (SPEC-013; M3).
//!
//! Real production CrowdSec adapter behind the nexus-sentinel-advanced
//! `ThreatIntelProvider` port. CrowdSec is optional reputation
//! enforcement (SPEC-013 behavior 3; COMPONENT_REGISTRY external
//! sensor, MIT). Nexus queries its documented Local API (LAPI) and
//! normalizes provider payloads at the infrastructure boundary -
//! CrowdSec JSON never becomes a domain contract.
//!
//! Permanent invariants (SPEC-013):
//! - Reputation is OBSERVED data: a ban decision is evidence, absence
//!   of a decision is absence of evidence (never a fabricated
//!   verdict).
//! - A provider advertises only capabilities it actually holds
//!   (Reality rule).
//! - Credentials (machine_id/password) are used ONLY for the watcher
//!   login exchange and never appear in errors or telemetry.
//! - Malformed or unknown provider payloads fail closed.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod transport;

pub use adapter::CrowdSecThreatIntelProvider;
pub use transport::{CrowdSecDecision, CrowdSecTransport, HttpCrowdSecTransport};
