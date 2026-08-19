//! EP-026 canonical email vocabulary (SPEC-014 terms are vocabulary
//! locked: Mailbox, Thread, Message, Draft, DeliveryReceipt,
//! DisclosurePolicy; a new synonym requires an ADR and schema update).
//!
//! Permanent invariants (owner directive, EP-026):
//! - READ SCOPE != SEND SCOPE: reading mail never grants sending and
//!   sending never grants reading (acceptance obligation 2).
//! - SENT != DELIVERED: a provider accepting a message proves
//!   submission, never delivery. DeliveryReceipt is the only delivery
//!   authority.
//! - DRAFT != SENT: drafting is local intent; sending is a governed
//!   action through the provider boundary.
//! - PROVIDER CLAIMS != NEXUS PROVED: free-form provider payloads are
//!   normalized at the infrastructure boundary and never become domain
//!   contracts (SPEC-014 inputs/outputs).
//! - Attachments carry a sha256 digest, never raw content; malware
//!   scanning is a separate scope and a blocked attachment is never
//!   deliverable.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{MailError, MailErrorCode};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, MailError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(MailError::new(
                        MailErrorCode::Validation,
                        concat!(stringify!($name), " must be 1..=128 characters"),
                        None,
                        None,
                    ));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(MailboxId);
typed_id!(ThreadId);
typed_id!(MessageId);
typed_id!(AttachmentId);
typed_id!(DraftId);
typed_id!(DeliveryReceiptId);

/// A validated RFC 5322 email address (local@domain).
///
/// Provider-neutral: Gmail, Microsoft Graph, and generic IMAP/SMTP all
/// map to this canonical shape. Free-form display names are carried
/// separately and never participate in identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EmailAddress(String);

impl EmailAddress {
    pub fn new(value: impl Into<String>) -> Result<Self, MailError> {
        let value = value.into();
        let ok = !value.is_empty()
            && value.len() <= 254
            && value.contains('@')
            && value.split('@').count() == 2
            && !value.starts_with('@')
            && !value.ends_with('@')
            && value
                .split('@')
                .all(|part| !part.is_empty() && !part.contains(' '));
        if !ok {
            return Err(MailError::validation(format!(
                "invalid email address {value:?}"
            )));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EmailAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Mail scope: separate read and send scopes are a permanent
/// invariant (acceptance obligation 2). A scope grants exactly one
/// authority; providers must never widen it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MailScope {
    /// Read messages, threads, and attachments.
    Read,
    /// Send messages and drafts through the provider.
    Send,
    /// Create and edit drafts locally (never implies Send).
    Draft,
    /// Reply to a message.
    Reply,
    /// Forward a message.
    Forward,
    /// Archive a message.
    Archive,
    /// Apply labels to a message.
    Label,
    /// Access attachment artifacts through ArtifactStore.
    Attachments,
}

impl MailScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "READ",
            Self::Send => "SEND",
            Self::Draft => "DRAFT",
            Self::Reply => "REPLY",
            Self::Forward => "FORWARD",
            Self::Archive => "ARCHIVE",
            Self::Label => "LABEL",
            Self::Attachments => "ATTACHMENTS",
        }
    }

    pub fn parse(text: &str) -> Result<Self, MailError> {
        match text {
            "READ" => Ok(Self::Read),
            "SEND" => Ok(Self::Send),
            "DRAFT" => Ok(Self::Draft),
            "REPLY" => Ok(Self::Reply),
            "FORWARD" => Ok(Self::Forward),
            "ARCHIVE" => Ok(Self::Archive),
            "LABEL" => Ok(Self::Label),
            "ATTACHMENTS" => Ok(Self::Attachments),
            _ => Err(MailError::vocabulary(format!(
                "unknown mail scope {text:?}"
            ))),
        }
    }
}

/// Governed mail command (acceptance obligation 3/4: every action
/// audits correctly and policy gates before provider mutation).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MailCommand {
    List,
    Fetch,
    Draft,
    Send,
    Reply,
    Forward,
    Archive,
    Label,
    Delete,
}

impl MailCommand {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::List => "LIST",
            Self::Fetch => "FETCH",
            Self::Draft => "DRAFT",
            Self::Send => "SEND",
            Self::Reply => "REPLY",
            Self::Forward => "FORWARD",
            Self::Archive => "ARCHIVE",
            Self::Label => "LABEL",
            Self::Delete => "DELETE",
        }
    }

    /// The scope required to execute this command.
    pub const fn required_scope(self) -> MailScope {
        match self {
            Self::List | Self::Fetch => MailScope::Read,
            Self::Draft => MailScope::Draft,
            Self::Send => MailScope::Send,
            Self::Reply => MailScope::Reply,
            Self::Forward => MailScope::Forward,
            Self::Archive => MailScope::Archive,
            Self::Label => MailScope::Label,
            Self::Delete => MailScope::Archive,
        }
    }
}

/// Mail direction (SPEC-014 Mailbox/Message).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MailDirection {
    Inbound,
    Outbound,
}

impl MailDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inbound => "INBOUND",
            Self::Outbound => "OUTBOUND",
        }
    }

    pub fn parse(text: &str) -> Result<Self, MailError> {
        match text {
            "INBOUND" => Ok(Self::Inbound),
            "OUTBOUND" => Ok(Self::Outbound),
            _ => Err(MailError::vocabulary(format!(
                "unknown mail direction {text:?}"
            ))),
        }
    }
}

/// Message lifecycle. The permanent hierarchy is:
/// DRAFT < QUEUED < SENDING < SENT < DELIVERED.
///
/// SENT != DELIVERED: provider acceptance proves submission only.
/// DELIVERED requires a DeliveryReceipt (the only delivery
/// authority). FAILED is terminal. ARCHIVED/DELETED are terminal
/// mailbox states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MailState {
    Draft,
    Queued,
    Sending,
    Sent,
    Delivered,
    Failed,
    Archived,
    Deleted,
}

impl MailState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Queued => "QUEUED",
            Self::Sending => "SENDING",
            Self::Sent => "SENT",
            Self::Delivered => "DELIVERED",
            Self::Failed => "FAILED",
            Self::Archived => "ARCHIVED",
            Self::Deleted => "DELETED",
        }
    }

    pub fn parse(text: &str) -> Result<Self, MailError> {
        match text {
            "DRAFT" => Ok(Self::Draft),
            "QUEUED" => Ok(Self::Queued),
            "SENDING" => Ok(Self::Sending),
            "SENT" => Ok(Self::Sent),
            "DELIVERED" => Ok(Self::Delivered),
            "FAILED" => Ok(Self::Failed),
            "ARCHIVED" => Ok(Self::Archived),
            "DELETED" => Ok(Self::Deleted),
            _ => Err(MailError::vocabulary(format!(
                "unknown mail state {text:?}"
            ))),
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Failed | Self::Archived | Self::Deleted)
    }
}

/// Mail privacy class (SPEC-014 privacy; SECURITY.md data
/// classification binding).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MailPrivacyClass {
    Public,
    Private,
    Sensitive,
}

impl MailPrivacyClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "PUBLIC",
            Self::Private => "PRIVATE",
            Self::Sensitive => "SENSITIVE",
        }
    }

    pub fn parse(text: &str) -> Result<Self, MailError> {
        match text {
            "PUBLIC" => Ok(Self::Public),
            "PRIVATE" => Ok(Self::Private),
            "SENSITIVE" => Ok(Self::Sensitive),
            _ => Err(MailError::vocabulary(format!(
                "unknown mail privacy class {text:?}"
            ))),
        }
    }
}

/// Attachment artifact. Carries a sha256 digest and storage reference;
/// NEVER raw content in the domain contract (SPEC-014 inputs/outputs).
/// Malware scanning status gates deliverability (acceptance
/// obligation 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: AttachmentId,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub storage_ref: String,
    pub scan_status: ScanStatus,
}

/// Malware scan status. A blocked or unscanned attachment is never
/// deliverable (fail closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanStatus {
    Pending,
    Clean,
    Quarantined,
    Blocked,
}

impl ScanStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Clean => "CLEAN",
            Self::Quarantined => "QUARANTINED",
            Self::Blocked => "BLOCKED",
        }
    }

    /// Only clean attachments are deliverable; everything else fails
    /// closed (acceptance obligation 3).
    pub const fn is_deliverable(self) -> bool {
        matches!(self, Self::Clean)
    }
}

/// A canonical email message. Free-form provider payloads never leak
/// into this shape; body is carried as a digest reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub mailbox: MailboxId,
    pub thread: ThreadId,
    pub direction: MailDirection,
    pub from: EmailAddress,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub subject: String,
    pub body_digest: String,
    pub attachments: Vec<Attachment>,
    pub state: MailState,
    pub privacy_class: MailPrivacyClass,
}

/// A draft is local intent, never a sent message (DRAFT != SENT).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub id: DraftId,
    pub mailbox: MailboxId,
    pub thread: Option<ThreadId>,
    pub to: Vec<EmailAddress>,
    pub cc: Vec<EmailAddress>,
    pub bcc: Vec<EmailAddress>,
    pub subject: String,
    pub body_digest: String,
    pub attachments: Vec<Attachment>,
}

/// A governed send request. Carries an idempotency key so retryable
/// commands never double-send (SPEC-014 error states: Conflict for
/// duplicate same-target commands).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SendRequest {
    pub draft: DraftId,
    pub idempotency_key: String,
    pub approval_class: u8,
    pub scopes_granted: Vec<MailScope>,
}

impl SendRequest {
    /// The SEND scope is required to send; a request that only holds
    /// READ (or DRAFT) scopes must be refused before any provider
    /// mutation (acceptance obligation 2).
    pub fn has_send_scope(&self) -> bool {
        self.scopes_granted.contains(&MailScope::Send)
    }
}

/// DeliveryReceipt is the ONLY delivery authority (SENT != DELIVERED).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    pub id: DeliveryReceiptId,
    pub message: MessageId,
    pub delivered: bool,
    pub provider_timestamp_ms: Option<u64>,
}

/// Mail policy: allowed scopes/capabilities, approval threshold,
/// retention bound, and attachment policy. Policy gates BEFORE
/// provider mutation (SPEC-014 behavior 8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailPolicy {
    pub allowed_scopes: Vec<MailScope>,
    pub allowed_commands: Vec<MailCommand>,
    pub min_approval_class: u8,
    pub max_retention_seconds: u64,
    pub max_attachment_bytes: u64,
    pub require_scan: bool,
}

impl MailPolicy {
    pub fn allows_scope(&self, scope: MailScope) -> bool {
        self.allowed_scopes.contains(&scope)
    }

    pub fn allows_command(&self, command: MailCommand) -> bool {
        self.allowed_commands.contains(&command)
    }

    /// Policy gate: approval class must meet the threshold.
    pub fn approval_allows(&self, approval_class: u8) -> bool {
        approval_class >= self.min_approval_class
    }

    /// Attachment policy gate: size bound and mandatory scan.
    pub fn attachment_allows(&self, attachment: &Attachment) -> bool {
        if attachment.size_bytes > self.max_attachment_bytes {
            return false;
        }
        if self.require_scan && !attachment.scan_status.is_deliverable() {
            return false;
        }
        true
    }
}

/// Provider-neutral mail change event (MailChangeFeed). One canonical
/// event shape regardless of provider push/poll transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailChange {
    pub mailbox: MailboxId,
    pub message: Option<MessageId>,
    pub thread: Option<ThreadId>,
    pub change: MailChangeKind,
    pub sequence: u64,
}

/// Change kinds for MailChangeFeed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MailChangeKind {
    NewMessage,
    StateChanged,
    Archived,
    Deleted,
    Labeled,
}

impl MailChangeKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NewMessage => "NEW_MESSAGE",
            Self::StateChanged => "STATE_CHANGED",
            Self::Archived => "ARCHIVED",
            Self::Deleted => "DELETED",
            Self::Labeled => "LABELED",
        }
    }
}
