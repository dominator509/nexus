//! EP-028 Hydra connector (SPEC-015; M2).
//!
//! Real production Hydra adapter behind the nexus-hydra `HydraProvider`
//! port. Hydra is the CRM canonical source; Nexus orchestrates its
//! authenticated REST surface and normalizes provider payloads at the
//! infrastructure boundary - Hydra JSON never becomes a domain
//! contract.
//!
//! Permanent invariants (SPEC-015):
//! - Hydra remains canonical; Nexus stores references/projections.
//! - Authenticated MCP/REST/durable events only; no direct DB access.
//! - Dual authorization gates and end-to-end correlation preserved.
//! - Policy before mutation; paid-ad/crisis require human approval.
//! - Unbound providers fail closed (Reality rule).

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::HydraAdapter;
pub use observability::{HydraAuditEntry, HydraObservability};
pub use transport::{
    HttpHydraTransport, HydraActionEnvelope, HydraCapabilityAd, HydraProviderEvent, HydraTransport,
};
