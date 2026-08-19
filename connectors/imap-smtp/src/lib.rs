//! EP-026 IMAP/SMTP connector (SPEC-014; M4).
//!
//! Real production IMAP read transport and SMTP submission transport
//! behind the nexus-email `EmailProvider` port. IMAP and SMTP are
//! separate authorities: SMTP credentials never imply IMAP read
//! permission and IMAP credentials never imply send permission
//! (directive C).
//!
//! Permanent invariants (owner directive, EP-026):
//! - READ != SEND != MODIFY: separate authorities, never widened.
//! - SENT != DELIVERED: SMTP acceptance proves submission only;
//!   DeliveryReceipt is the only delivery authority.
//! - DRAFT != SENT: drafting is local intent; sending is governed.
//! - Message in Sent folder != recipient received it.
//! - PROVIDER CLAIMS != NEXUS PROVED.
//! - Unknown targets are NotFound, never Verified.
//! - AMBIGUOUS OUTCOME -> VERIFY FIRST -> NO BLIND RETRY (a duplicate
//!   email has real consequences).

#![forbid(unsafe_code)]

pub mod adapter;
pub mod observability;
pub mod transport;

pub use adapter::ImapSmtpAdapter;
pub use observability::{MailAuditEntry, MailObservability};
pub use transport::{
    ImapAuthority, ImapEnvelope, ImapMessage, ImapSession, ImapTls, ImapTransport,
    RealImapTransport, RealSmtpTransport, SmtpAuthority, SmtpOutcome, SmtpTls, SmtpTransport,
};
