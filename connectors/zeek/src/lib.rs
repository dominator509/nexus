//! EP-031 Zeek connector (SPEC-013; M2).
//!
//! Real production Zeek adapter behind the nexus-sentinel-advanced
//! `NetworkDetectionProvider` port. Zeek is the Advanced profile
//! network detection sensor (SPEC-013 behavior 3; COMPONENT_REGISTRY
//! external sensor, GPL-2.0). Nexus consumes its documented JSON log
//! output and normalizes provider payloads at the infrastructure
//! boundary - Zeek JSON never becomes a domain contract.
//!
//! Permanent invariants (SPEC-013):
//! - Detection events are OBSERVED data with evidence references;
//!   unbound or failing providers never fabricate events.
//! - A provider advertises only capabilities it actually holds
//!   (Reality rule).
//! - Free-form provider payloads are normalized at the boundary and
//!   never become domain contracts.
//! - Unknown or malformed records fail closed (never guessed).

#![forbid(unsafe_code)]

pub mod adapter;
pub mod transport;

pub use adapter::ZeekNetworkDetectionProvider;
pub use transport::{JsonLinesZeekTransport, ZeekNotice, ZeekTransport};
