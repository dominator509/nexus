//! EP-027 M3 HylaFAX connector (SPEC-014).
//!
//! Real HylaFAX adapter behind the nexus-fax `FaxProvider` port:
//! hfaxd client-server protocol transport (FTP-like control channel,
//! EPRT data channel, MODE Z zlib document transfer) against a real
//! HylaFAX server, canonical mapping (SUBMITTED != DELIVERED),
//! governed submission (policy BEFORE provider mutation),
//! exact-target verification, ambiguity-safe idempotency, bounded
//! redacted observability.
//!
//! Certification boundary (M3, controlled fixture):
//! - nexus-hylafax: IMPLEMENTED
//! - hfaxd client/server transport: PROTOCOL_CERTIFIED (observed wire)
//! - HylaFAX 6.0.6-8.1: PROVIDER_CERTIFIED against controlled fixture
//! - container: CONTROLLED_TEST_FIXTURE
//! - faxq submission: PROVIDER_CERTIFIED
//! - physical modem / PSTN fax delivery / remote fax machine receipt /
//!   DELIVERED: NOT ASSERTED

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::{build_hylafax_provider, HylaFaxProvider};
pub use transport::{HylaFaxTcpTransport, HylaFaxTransport};
