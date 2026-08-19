//! EP-026 Microsoft Graph connector (SPEC-014; M3).
//!
//! Real production Microsoft Graph adapter behind the nexus-email
//! `EmailProvider` port. Microsoft Graph is an external provider;
//! Nexus orchestrates its documented v1.0 REST mail surface and
//! normalizes provider payloads at the infrastructure boundary -
//! free-form Graph JSON never becomes a domain contract.
//!
//! Permanent invariants (owner directive, EP-026):
//! - READ SCOPE != SEND SCOPE (acceptance obligation 2); update and
//!   delete additionally require a Mail.ReadWrite-class authority
//!   that is distinct from plain read.
//! - SENT != DELIVERED: Graph 202 means accepted for processing, NOT
//!   delivered. DeliveryReceipt is the only delivery authority.
//! - DRAFT != SENT: drafting is local intent; sending is governed.
//! - PROVIDER CLAIMS != NEXUS PROVED: a displayed From header is
//!   advisory, never identity.
//! - Unbound providers fail closed and never fabricate mail state
//!   (Reality rule).
//!
//! Certification boundary (owner directive, M): this connector is
//! IMPLEMENTED / TRANSPORT_CERTIFIED through real HTTP against
//! controlled Graph-shaped fixtures. Real Microsoft tenant/provider
//! certification is DEFERRED to the live-fire owner (M5/LF-011);
//! controlled fixtures never certify a real Microsoft account.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::MicrosoftGraphAdapter;
pub use observability::{MailAuditEntry, MailObservability};
pub use transport::{
    GraphAttachmentMeta, GraphDraft, GraphEmailAddress, GraphMessage, GraphRecipient, GraphScope,
    GraphTransport, HttpGraphTransport,
};
