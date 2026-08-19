//! EP-026 provider ports (fail-closed defaults; SPEC-014).
//!
//! Gmail, Microsoft Graph, generic IMAP/SMTP, and self-hosted mail all
//! map to this provider-neutral boundary (SPEC-014 behavior 1). Nexus
//! orchestrates providers; it never replaces SMTP/IMAP transport or
//! vendor APIs with a home-grown stack. Unbound providers fail closed
//! and never fabricate mail state (Reality rule). Provider-specific
//! payloads are normalized at the infrastructure boundary and never
//! become domain contracts.

use crate::error::MailError;
use crate::vocabulary::{
    Attachment, Draft, DraftId, MailChange, MailCommand, MailPolicy, MailScope, MailState,
    MailboxId, Message, MessageId, SendRequest, ThreadId,
};

/// Email provider port (provider-neutral; Gmail / Microsoft Graph /
/// IMAP+SMTP / self-hosted all implement this boundary).
pub trait EmailProvider {
    fn list_mailboxes(&self) -> Result<Vec<MailboxId>, MailError> {
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn list_threads(&self, mailbox: &MailboxId) -> Result<Vec<ThreadId>, MailError> {
        let _ = mailbox;
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn fetch_message(
        &self,
        mailbox: &MailboxId,
        message: &MessageId,
    ) -> Result<Message, MailError> {
        let _ = (mailbox, message);
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn list_attachments(&self, message: &MessageId) -> Result<Vec<Attachment>, MailError> {
        let _ = message;
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn save_draft(&self, draft: &Draft) -> Result<DraftId, MailError> {
        let _ = draft;
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn send(&self, request: &SendRequest) -> Result<MessageId, MailError> {
        let _ = request;
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn reply(&self, original: &MessageId, draft: &Draft) -> Result<MessageId, MailError> {
        let _ = (original, draft);
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn forward(&self, original: &MessageId, draft: &Draft) -> Result<MessageId, MailError> {
        let _ = (original, draft);
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn archive(&self, mailbox: &MailboxId, message: &MessageId) -> Result<(), MailError> {
        let _ = (mailbox, message);
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn label(
        &self,
        mailbox: &MailboxId,
        message: &MessageId,
        label: &str,
    ) -> Result<(), MailError> {
        let _ = (mailbox, message, label);
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn delete(&self, mailbox: &MailboxId, message: &MessageId) -> Result<(), MailError> {
        let _ = (mailbox, message);
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    fn message_state(
        &self,
        mailbox: &MailboxId,
        message: &MessageId,
    ) -> Result<MailState, MailError> {
        let _ = (mailbox, message);
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }

    /// Change feed: push events when the provider supports them, else
    /// controlled polling. The canonical event shape is
    /// provider-neutral (SPEC-014 replacement/fallback).
    fn changes(
        &self,
        mailbox: &MailboxId,
        after_sequence: u64,
    ) -> Result<Vec<MailChange>, MailError> {
        let _ = (mailbox, after_sequence);
        Err(MailError::unavailable(
            "email provider has no implementation bound",
        ))
    }
}

/// Mail policy gate: commands and scopes are checked BEFORE any
/// provider mutation (SPEC-014 behavior 8; acceptance obligations
/// 2-4). Denials are recorded by the caller's audit path.
pub fn enforce_mail_policy(
    policy: &MailPolicy,
    command: MailCommand,
    scopes: &[MailScope],
    approval_class: u8,
) -> Result<(), MailError> {
    if !policy.allows_command(command) {
        return Err(MailError::policy(format!(
            "command {} not allowed by mail policy",
            command.as_str()
        )));
    }
    let required = command.required_scope();
    if !scopes.contains(&required) {
        return Err(MailError::policy(format!(
            "command {} requires scope {} (not granted)",
            command.as_str(),
            required.as_str()
        )));
    }
    if !policy.approval_allows(approval_class) {
        return Err(MailError::policy(format!(
            "approval class {approval_class} below policy minimum {}",
            policy.min_approval_class
        )));
    }
    Ok(())
}
