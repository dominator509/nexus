//! EP-027 provider-neutral fax contracts (SPEC-014).
//!
//! ICTFax (primary self-hosted control sidecar), HylaFAX (compatibility
//! backend), and CloudFax (Telnyx/Phaxio-class external carrier
//! fallback) all map to the canonical objects here without
//! vendor-specific domain logic (SPEC-014 behavior 5). Fax jobs
//! preserve source artifact hash, number normalization, pages, carrier,
//! retries, status, inbound route, and archive (behavior 6). External
//! sends are governed by policy and approval (behavior 8).
//!
//! Permanent invariants (owner directive, EP-027):
//! - SUBMITTED != DELIVERED: carrier acceptance proves submission,
//!   never delivery. Delivery confirmation requires independent
//!   carrier/recipient-side evidence.
//! - PROVIDER CLAIMS != NEXUS PROVED: carrier payloads are normalized
//!   at the infrastructure boundary, never domain contracts.
//! - Fax documents carry a sha256 digest, never raw content; only
//!   CLEAN-scanned documents are transmittable (fail closed).
//! - Numbers are normalized before any provider boundary; raw dial
//!   strings are never compared as identity.

#![forbid(unsafe_code)]

pub mod error;
pub mod provider;
pub mod vocabulary;

pub use error::{FaxError, FaxErrorCode};
pub use provider::{
    enforce_fax_policy, submit_governed, validate_send_request, verify_delivery, FaxProvider,
};
pub use vocabulary::{
    FaxCarrierJobId, FaxDirection, FaxDocument, FaxDocumentId, FaxJob, FaxJobId, FaxNumber,
    FaxProviderKind, FaxRouteId, FaxScanStatus, FaxSendRequest, FaxState, FaxStatus,
    InboundFaxRoute,
};
