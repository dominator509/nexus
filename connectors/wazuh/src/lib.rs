//! EP-031 Wazuh connector (SPEC-013; M4).
//!
//! Real production Wazuh adapter behind the nexus-sentinel-advanced
//! `EndpointTelemetryProvider` port. Wazuh is the Endpoint profile
//! security sensor (SPEC-013 behavior 3; COMPONENT_REGISTRY external
//! sensor, GPL-2.0). Nexus consumes its documented server API and
//! normalizes provider payloads at the infrastructure boundary -
//! Wazuh JSON never becomes a domain contract.
//!
//! Permanent invariants (SPEC-013):
//! - Endpoint telemetry is OBSERVED data with evidence references;
//!   unbound or failing providers never fabricate events.
//! - A provider advertises only capabilities it actually holds
//!   (Reality rule).
//! - Credentials (username/password) are used ONLY for the
//!   authenticate exchange and never appear in errors or telemetry.
//! - Every public operation records a bounded redacted audit entry
//!   with correlation (fail-closed error path included).
//! - Malformed or unknown provider payloads fail closed.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::WazuhEndpointTelemetryProvider;
pub use observability::{SentinelAuditEntry, SentinelObservability};
pub use transport::{HttpWazuhTransport, WazuhAlert, WazuhTransport};
