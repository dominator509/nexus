//! EP-027 ICTFax connector (SPEC-014; M2).
//!
//! Real production ICTFax adapter behind the nexus-fax `FaxProvider`
//! port. ICTFax is the primary self-hosted fax control sidecar
//! (SPEC-014 behavior 5); Nexus orchestrates its documented REST API
//! and normalizes provider payloads at the infrastructure boundary -
//! ICTFax JSON never becomes a domain contract.
//!
//! Permanent invariants (owner directive, EP-027):
//! - SUBMITTED != DELIVERED: carrier acceptance proves submission,
//!   never delivery. Delivery confirmation requires independent
//!   carrier/recipient-side evidence.
//! - PROVIDER CLAIMS != NEXUS PROVED: carrier payloads are normalized
//!   at the infrastructure boundary, never domain contracts.
//! - Policy gates run BEFORE any provider mutation; denied sends
//!   never reach the carrier.
//! - Exact-target verification: a status for carrier job X never
//!   verifies fax job Y.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::{submit_ictfax_governed, verify_ictfax_delivery, IctFaxProvider};
pub use observability::{FaxAuditEntry, FaxObservability};
pub use transport::{
    map_transmission_state, HttpIctFaxTransport, IctFaxAccount, IctFaxDocument, IctFaxTransmission,
    IctFaxTransport,
};
