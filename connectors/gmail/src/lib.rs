//! EP-026 Gmail connector (SPEC-014; M2).
//!
//! Real production Gmail adapter behind the nexus-email `EmailProvider`
//! port. Gmail is an external provider; Nexus orchestrates its
//! documented REST API and normalizes provider payloads at the
//! infrastructure boundary - Gmail JSON never becomes a domain
//! contract.
//!
//! Permanent invariants (owner directive, EP-026):
//! - READ SCOPE != SEND SCOPE (acceptance obligation 2).
//! - SENT != DELIVERED: DeliveryReceipt is the only delivery authority.
//! - DRAFT != SENT: drafting is local intent; sending is governed.
//! - PROVIDER CLAIMS != NEXUS PROVED: a displayed From header is
//!   advisory, never identity.
//! - Unbound providers fail closed and never fabricate mail state
//!   (Reality rule).

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::GmailAdapter;
pub use observability::{MailAuditEntry, MailObservability};
pub use transport::{GmailScope, GmailTransport, HttpGmailTransport};
