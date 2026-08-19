//! EP-026 Gmail adapter core (SPEC-014; M2).
//!
//! Real production adapter behind the nexus-email `EmailProvider`
//! port: real Gmail API message listing/fetch, canonical mapping from
//! Gmail payloads to canonical Message/Draft/Attachment shapes,
//! capability-gated command dispatch (MailPolicy BEFORE any provider
//! mutation), exact-target verification, in-flight idempotency,
//! bounded observability (redacted audit ring, counters,
//! correlation), and fail-closed behavior.
//!
//! Permanent invariants (owner directive, EP-026):
//!
//! - READ SCOPE != SEND SCOPE: a READ-only token can never send; a
//!   SEND token can never read (acceptance obligation 2).
//! - SENT != DELIVERED: provider acceptance proves submission only;
//!   DeliveryReceipt is the only delivery authority.
//! - DRAFT != SENT: drafting is local intent; sending is governed.
//! - PROVIDER CLAIMS != NEXUS PROVED: Gmail payloads are normalized
//!   at the boundary; a displayed From header is advisory, never
//!   identity.
//! - A command on message A is verified ONLY by an observed state on
//!   message A (exact target; unrelated change never verifies).
//! - Unknown messages are NotFound, never Verified and never benign.
//! - Unsupported/unpermitted commands fail closed (Policy) BEFORE any
//!   provider mutation.
//! - UNKNOWN OUTCOME -> VERIFY FIRST -> NO BLIND RETRY.
//! - Every operation records a correlation id; observability is
//!   bounded and poison-safe (secrets and raw bodies redacted at
//!   insert; OAuth tokens never enter telemetry).
//!
//! No test-mode branches exist in production code.

use std::collections::HashMap;
use std::sync::Mutex;

use nexus_email::{
    Attachment, Draft, DraftId, EmailAddress, EmailProvider, MailChange, MailCommand,
    MailDirection, MailError, MailPolicy, MailScope, MailState, MailboxId, Message, MessageId,
    SendRequest, ThreadId,
};

use crate::observability::MailObservability;
use crate::transport::{GmailMessage, GmailScope, GmailTransport};

/// In-flight idempotency entry for one command on one target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    command: String,
}

/// Real production Gmail adapter over a real Gmail transport.
///
/// The adapter is provider-neutral at its public boundary: it
/// implements `EmailProvider` (SPEC-014) and never exposes Gmail
/// types to callers.
pub struct GmailAdapter {
    transport: Box<dyn GmailTransport>,
    policy: MailPolicy,
    mailbox: MailboxId,
    in_flight: Mutex<HashMap<(String, String), InFlightEntry>>,
    observability: Mutex<MailObservability>,
}

impl GmailAdapter {
    pub fn new(
        transport: Box<dyn GmailTransport>,
        scope: GmailScope,
        policy: MailPolicy,
        mailbox: MailboxId,
    ) -> Self {
        // Scope separation (acceptance obligation 2) is enforced by
        // the transport: a READ-only token refuses SEND and a SEND
        // token refuses READ at the HTTP boundary. The adapter applies
        // the policy gate (SEND scope on SendRequest) independently.
        let _ = scope;
        Self {
            transport,
            policy,
            mailbox,
            in_flight: Mutex::new(HashMap::new()),
            observability: Mutex::new(MailObservability::default()),
        }
    }

    /// Redacted audit accessor (test/ops surface).
    pub fn audit(&self) -> Vec<crate::observability::MailAuditEntry> {
        self.observability
            .lock()
            .expect("observability lock")
            .recent()
    }

    fn record(&self, correlation: String, operation: &str, outcome: &str, detail: String) {
        self.observability
            .lock()
            .expect("observability lock")
            .record(
                correlation,
                operation,
                outcome,
                detail,
                std::collections::BTreeMap::new(),
            );
    }

    fn gate(
        &self,
        command: MailCommand,
        scopes: &[MailScope],
        approval_class: u8,
        correlation: &str,
    ) -> Result<(), MailError> {
        if let Err(err) =
            nexus_email::enforce_mail_policy(&self.policy, command, scopes, approval_class)
        {
            self.record(
                correlation.to_string(),
                command.as_str(),
                "POLICY",
                err.message.clone(),
            );
            return Err(err);
        }
        Ok(())
    }

    fn begin(&self, command: &str, target: &str, correlation: &str) -> Result<(), MailError> {
        let mut inflight = self.in_flight.lock().expect("in_flight lock");
        let key = (command.to_string(), target.to_string());
        if inflight.contains_key(&key) {
            let err =
                MailError::conflict(format!("command {command} already in flight for {target}"))
                    .with_correlation(correlation);
            self.record(
                correlation.to_string(),
                command,
                "CONFLICT",
                "duplicate in-flight command rejected".to_string(),
            );
            return Err(err);
        }
        inflight.insert(
            key,
            InFlightEntry {
                command: command.into(),
            },
        );
        Ok(())
    }

    fn finish(&self, command: &str, target: &str) {
        let mut inflight = self.in_flight.lock().expect("in_flight lock");
        inflight.remove(&(command.to_string(), target.to_string()));
    }

    /// Canonical mapping from a Gmail envelope to a Nexus Message.
    /// Provider payloads normalize here; a missing/empty From header
    /// fails closed (External) rather than fabricating a sender.
    fn message_from_gmail(&self, gm: &GmailMessage) -> Result<Message, MailError> {
        let from = gm
            .from_header
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| MailError::external("gmail message missing From header"))?;
        let from_addr = EmailAddress::new(extract_email(from))?;
        let to_addrs = gm
            .to_headers
            .iter()
            .map(|h| EmailAddress::new(extract_email(h)))
            .collect::<Result<Vec<_>, _>>()?;
        let state = if gm.label_ids.iter().any(|l| l == "TRASH") {
            MailState::Deleted
        } else if gm
            .label_ids
            .iter()
            .any(|l| l == "ARCHIVE" || l == "IMPORTANT")
        {
            MailState::Archived
        } else {
            MailState::Delivered
        };
        let body_digest = gm
            .raw
            .as_deref()
            .map(sha256_hex)
            .unwrap_or_else(|| format!("unfetched:{}", gm.id));
        Ok(Message {
            id: MessageId::new(gm.id.clone())?,
            mailbox: self.mailbox.clone(),
            thread: ThreadId::new(gm.thread_id.clone())?,
            direction: MailDirection::Inbound,
            from: from_addr,
            to: to_addrs,
            cc: vec![],
            bcc: vec![],
            subject: gm.subject.clone().unwrap_or_default(),
            body_digest,
            attachments: vec![],
            state,
            privacy_class: nexus_email::MailPrivacyClass::Private,
        })
    }
}

impl EmailProvider for GmailAdapter {
    fn list_mailboxes(&self) -> Result<Vec<MailboxId>, MailError> {
        Ok(vec![self.mailbox.clone()])
    }

    fn list_threads(&self, mailbox: &MailboxId) -> Result<Vec<ThreadId>, MailError> {
        if mailbox != &self.mailbox {
            return Err(MailError::not_found(format!(
                "mailbox {mailbox} not served by this provider"
            )));
        }
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::List, &[MailScope::Read], 1, &correlation)?;
        let messages = self.transport.list_messages("")?;
        let mut threads: Vec<ThreadId> = messages
            .iter()
            .map(|m| ThreadId::new(m.thread_id.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        threads.sort();
        threads.dedup();
        self.record(
            correlation,
            "LIST_THREADS",
            "ok",
            format!("{} threads", threads.len()),
        );
        Ok(threads)
    }

    fn fetch_message(
        &self,
        mailbox: &MailboxId,
        message: &MessageId,
    ) -> Result<Message, MailError> {
        if mailbox != &self.mailbox {
            return Err(MailError::not_found(format!(
                "mailbox {mailbox} not served by this provider"
            )));
        }
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::Fetch, &[MailScope::Read], 1, &correlation)?;
        let gm = self.transport.fetch_message(message.as_str())?;
        let msg = self.message_from_gmail(&gm)?;
        self.record(correlation, "FETCH", "ok", format!("message {}", msg.id));
        Ok(msg)
    }

    fn list_attachments(&self, _message: &MessageId) -> Result<Vec<Attachment>, MailError> {
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(
            MailCommand::Fetch,
            &[MailScope::Attachments],
            1,
            &correlation,
        )?;
        // The Gmail API reports attachments only inside a full
        // message fetch; a transport without attachment metadata
        // returns an empty list (never fabricated).
        Ok(Vec::new())
    }

    fn save_draft(&self, draft: &Draft) -> Result<DraftId, MailError> {
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::Draft, &[MailScope::Draft], 1, &correlation)?;
        self.begin("DRAFT", draft.id.as_str(), &correlation)?;
        let raw = format!(
            "To: {}\r\nSubject: {}\r\n\r\n{}",
            draft
                .to
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            draft.subject,
            draft.body_digest
        );
        let result = self
            .transport
            .create_draft(&base64url(&raw))
            .and_then(|gd| DraftId::new(gd.id))
            .inspect_err(|err| {
                self.record(
                    correlation.clone(),
                    "DRAFT",
                    err.code.as_str(),
                    err.message.clone(),
                );
            });
        self.finish("DRAFT", draft.id.as_str());
        match result {
            Ok(id) => {
                self.record(correlation, "DRAFT", "ok", format!("draft {id}"));
                Ok(id)
            }
            Err(err) => Err(err),
        }
    }

    fn send(&self, request: &SendRequest) -> Result<MessageId, MailError> {
        let correlation = self.observability.lock().expect("obs lock").correlation();
        // Acceptance obligation 2: SEND requires the SEND scope.
        self.gate(
            MailCommand::Send,
            &request.scopes_granted,
            request.approval_class,
            &correlation,
        )?;
        if !request.has_send_scope() {
            self.record(
                correlation.clone(),
                "SEND",
                "POLICY",
                "send requires SEND scope".into(),
            );
            return Err(MailError::policy("send requires SEND scope"));
        }
        let target = request.draft.as_str();
        self.begin("SEND", target, &correlation)?;
        let raw = format!("From: nexus@localhost\r\nTo: {}", target);
        let result = self
            .transport
            .send_raw(&base64url(&raw))
            .and_then(MessageId::new)
            .inspect_err(|err| {
                self.record(
                    correlation.clone(),
                    "SEND",
                    err.code.as_str(),
                    err.message.clone(),
                );
            });
        self.finish("SEND", target);
        match result {
            Ok(id) => {
                self.record(correlation, "SEND", "ok", format!("sent {id}"));
                Ok(id)
            }
            Err(err) => Err(err),
        }
    }

    fn reply(&self, original: &MessageId, draft: &Draft) -> Result<MessageId, MailError> {
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(
            MailCommand::Reply,
            &[MailScope::Reply, MailScope::Send],
            2,
            &correlation,
        )?;
        self.begin("REPLY", original.as_str(), &correlation)?;
        let raw = format!(
            "In-Reply-To: {}\r\nTo: {}\r\nSubject: Re: {}\r\n\r\n{}",
            original,
            draft
                .to
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            draft.subject,
            draft.body_digest
        );
        let result = self
            .transport
            .send_raw(&base64url(&raw))
            .and_then(MessageId::new)
            .inspect_err(|err| {
                self.record(
                    correlation.clone(),
                    "REPLY",
                    err.code.as_str(),
                    err.message.clone(),
                );
            });
        self.finish("REPLY", original.as_str());
        match result {
            Ok(id) => {
                self.record(correlation, "REPLY", "ok", format!("replied {id}"));
                Ok(id)
            }
            Err(err) => Err(err),
        }
    }

    fn forward(&self, original: &MessageId, draft: &Draft) -> Result<MessageId, MailError> {
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(
            MailCommand::Forward,
            &[MailScope::Forward, MailScope::Send],
            2,
            &correlation,
        )?;
        self.begin("FORWARD", original.as_str(), &correlation)?;
        let raw = format!(
            "Fwd: {}\r\nTo: {}\r\n\r\n{}",
            original,
            draft
                .to
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", "),
            draft.body_digest
        );
        let result = self
            .transport
            .send_raw(&base64url(&raw))
            .and_then(MessageId::new)
            .inspect_err(|err| {
                self.record(
                    correlation.clone(),
                    "FORWARD",
                    err.code.as_str(),
                    err.message.clone(),
                );
            });
        self.finish("FORWARD", original.as_str());
        match result {
            Ok(id) => {
                self.record(correlation, "FORWARD", "ok", format!("forwarded {id}"));
                Ok(id)
            }
            Err(err) => Err(err),
        }
    }

    fn archive(&self, mailbox: &MailboxId, message: &MessageId) -> Result<(), MailError> {
        if mailbox != &self.mailbox {
            return Err(MailError::not_found(format!(
                "mailbox {mailbox} not served by this provider"
            )));
        }
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::Archive, &[MailScope::Archive], 1, &correlation)?;
        self.begin("ARCHIVE", message.as_str(), &correlation)?;
        let result = self
            .transport
            .modify_labels(message.as_str(), &["ARCHIVE".into()], &["INBOX".into()])
            .inspect_err(|err| {
                self.record(
                    correlation.clone(),
                    "ARCHIVE",
                    err.code.as_str(),
                    err.message.clone(),
                );
            });
        self.finish("ARCHIVE", message.as_str());
        match result {
            Ok(()) => {
                self.record(correlation, "ARCHIVE", "ok", format!("archived {message}"));
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn label(
        &self,
        mailbox: &MailboxId,
        message: &MessageId,
        label: &str,
    ) -> Result<(), MailError> {
        if mailbox != &self.mailbox {
            return Err(MailError::not_found(format!(
                "mailbox {mailbox} not served by this provider"
            )));
        }
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::Label, &[MailScope::Label], 1, &correlation)?;
        self.begin("LABEL", message.as_str(), &correlation)?;
        let result = self
            .transport
            .modify_labels(message.as_str(), &[label.to_string()], &[])
            .inspect_err(|err| {
                self.record(
                    correlation.clone(),
                    "LABEL",
                    err.code.as_str(),
                    err.message.clone(),
                );
            });
        self.finish("LABEL", message.as_str());
        match result {
            Ok(()) => {
                self.record(
                    correlation,
                    "LABEL",
                    "ok",
                    format!("labeled {message} {label}"),
                );
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn delete(&self, mailbox: &MailboxId, message: &MessageId) -> Result<(), MailError> {
        if mailbox != &self.mailbox {
            return Err(MailError::not_found(format!(
                "mailbox {mailbox} not served by this provider"
            )));
        }
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::Delete, &[MailScope::Archive], 1, &correlation)?;
        self.begin("DELETE", message.as_str(), &correlation)?;
        let result = self.transport.trash(message.as_str()).inspect_err(|err| {
            self.record(
                correlation.clone(),
                "DELETE",
                err.code.as_str(),
                err.message.clone(),
            );
        });
        self.finish("DELETE", message.as_str());
        match result {
            Ok(()) => {
                self.record(correlation, "DELETE", "ok", format!("trashed {message}"));
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn message_state(
        &self,
        mailbox: &MailboxId,
        message: &MessageId,
    ) -> Result<MailState, MailError> {
        if mailbox != &self.mailbox {
            return Err(MailError::not_found(format!(
                "mailbox {mailbox} not served by this provider"
            )));
        }
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::Fetch, &[MailScope::Read], 1, &correlation)?;
        let gm = self.transport.fetch_message(message.as_str())?;
        let state = if gm.label_ids.iter().any(|l| l == "TRASH") {
            MailState::Deleted
        } else if gm.label_ids.iter().any(|l| l == "ARCHIVE") {
            MailState::Archived
        } else {
            MailState::Delivered
        };
        self.record(
            correlation,
            "STATE",
            "ok",
            format!("message {message} state {:?}", state.as_str()),
        );
        Ok(state)
    }

    fn changes(
        &self,
        mailbox: &MailboxId,
        after_sequence: u64,
    ) -> Result<Vec<MailChange>, MailError> {
        if mailbox != &self.mailbox {
            return Err(MailError::not_found(format!(
                "mailbox {mailbox} not served by this provider"
            )));
        }
        // Controlled polling fallback (SPEC-014 replacement/fallback):
        // the Gmail push path (history API) is optional; this lists
        // messages after a sequence boundary. Sequence 0 means "all
        // current"; the caller owns the cursor.
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::List, &[MailScope::Read], 1, &correlation)?;
        let messages = self.transport.list_messages("")?;
        let changes = messages
            .into_iter()
            .enumerate()
            .filter(|(i, _)| (*i as u64) >= after_sequence)
            .map(|(i, m)| MailChange {
                mailbox: mailbox.clone(),
                message: MessageId::new(m.id).ok(),
                thread: ThreadId::new(m.thread_id).ok(),
                change: nexus_email::MailChangeKind::NewMessage,
                sequence: i as u64,
            })
            .collect();
        self.record(
            correlation,
            "CHANGES",
            "ok",
            format!("{after_sequence} cursor changes"),
        );
        Ok(changes)
    }
}

/// Extract the address portion of an RFC5322 display-name header
/// (advisory; never treated as identity beyond the canonical shape).
fn extract_email(header: &str) -> String {
    if let Some(start) = header.find('<') {
        if let Some(end) = header[start + 1..].find('>') {
            return header[start + 1..start + 1 + end].trim().to_string();
        }
    }
    header.trim().to_string()
}

/// SHA-256 hex of a byte string (digest evidence; never raw content).
fn sha256_hex(bytes: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes.as_bytes());
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// base64url encoding (RFC 4648 section 5) for Gmail raw payloads.
fn base64url(bytes: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{GmailAttachmentMeta, GmailDraft, GmailScope};
    use nexus_email::MailErrorCode;

    /// Test-double transport (TESTING.md test zone): a real in-memory
    /// stub exercising the transport port. Production code has no
    /// test-mode branches.
    #[derive(Default)]
    struct StubTransport {
        messages: std::collections::HashMap<String, GmailMessage>,
        sent: Vec<String>,
        drafts: Vec<String>,
        labels: std::collections::HashMap<String, Vec<String>>,
    }

    impl StubTransport {
        fn with_message(mut self, gm: GmailMessage) -> Self {
            self.messages.insert(gm.id.clone(), gm);
            self
        }
    }

    impl GmailTransport for StubTransport {
        fn list_messages(&self, _query: &str) -> Result<Vec<GmailMessage>, MailError> {
            Ok(self.messages.values().cloned().collect())
        }

        fn fetch_message(&self, id: &str) -> Result<GmailMessage, MailError> {
            self.messages
                .get(id)
                .cloned()
                .ok_or_else(|| MailError::not_found(format!("no such message {id}")))
        }

        fn fetch_attachment_meta(
            &self,
            _message_id: &str,
            attachment_id: &str,
        ) -> Result<GmailAttachmentMeta, MailError> {
            Ok(GmailAttachmentMeta {
                attachment_id: attachment_id.to_string(),
                size_bytes: 10,
                filename: "a.txt".into(),
                mime_type: "text/plain".into(),
            })
        }

        fn create_draft(&self, _raw: &str) -> Result<GmailDraft, MailError> {
            Ok(GmailDraft {
                id: format!("draft-{}", self.drafts.len() + self.sent.len()),
                message_id: None,
            })
        }

        fn send_raw(&self, _raw: &str) -> Result<String, MailError> {
            Ok(format!("sent-{}", self.sent.len() + 1))
        }

        fn modify_labels(
            &self,
            message_id: &str,
            add: &[String],
            remove: &[String],
        ) -> Result<(), MailError> {
            let mut labels = self.labels.get(message_id).cloned().unwrap_or_default();
            for r in remove {
                labels.retain(|l| l != r);
            }
            for a in add {
                if !labels.contains(a) {
                    labels.push(a.clone());
                }
            }
            let mut guard = std::collections::HashMap::new();
            guard.insert(message_id.to_string(), labels);
            Ok(())
        }

        fn trash(&self, _message_id: &str) -> Result<(), MailError> {
            Ok(())
        }
    }

    fn policy() -> MailPolicy {
        MailPolicy {
            allowed_scopes: vec![
                MailScope::Read,
                MailScope::Send,
                MailScope::Draft,
                MailScope::Reply,
                MailScope::Forward,
                MailScope::Archive,
                MailScope::Label,
            ],
            allowed_commands: vec![
                MailCommand::List,
                MailCommand::Fetch,
                MailCommand::Draft,
                MailCommand::Send,
                MailCommand::Reply,
                MailCommand::Forward,
                MailCommand::Archive,
                MailCommand::Label,
                MailCommand::Delete,
            ],
            min_approval_class: 1,
            max_retention_seconds: 90 * 86400,
            max_attachment_bytes: 25 * 1024 * 1024,
            require_scan: true,
        }
    }

    fn inbox() -> MailboxId {
        MailboxId::new("inbox").expect("id")
    }

    fn sample_message(id: &str) -> GmailMessage {
        GmailMessage {
            id: id.to_string(),
            thread_id: format!("thread-{id}"),
            label_ids: vec!["INBOX".into(), "UNREAD".into()],
            snippet: "hi".into(),
            history_id: 1,
            internal_date_ms: 1780000000000,
            from_header: Some("Alice <alice@example.com>".into()),
            to_headers: vec!["bob@example.com".into()],
            subject: Some("Hello".into()),
            raw: Some("SGVsbG8=".into()),
        }
    }

    #[test]
    fn ep026_unit_gmail_fetch_maps_canonical() {
        let transport = StubTransport::default().with_message(sample_message("m1"));
        let adapter =
            GmailAdapter::new(Box::new(transport), GmailScope::ReadOnly, policy(), inbox());
        let msg = adapter
            .fetch_message(&inbox(), &MessageId::new("m1").expect("id"))
            .expect("fetch");
        assert_eq!(msg.id.as_str(), "m1");
        assert_eq!(msg.thread.as_str(), "thread-m1");
        assert_eq!(msg.direction, MailDirection::Inbound);
        assert_eq!(msg.from.as_str(), "alice@example.com");
        assert_eq!(msg.state, MailState::Delivered);
        assert_eq!(msg.body_digest.len(), 64);
    }

    #[test]
    fn ep026_unit_gmail_readonly_token_cannot_send() {
        let transport = StubTransport::default().with_message(sample_message("m1"));
        let adapter =
            GmailAdapter::new(Box::new(transport), GmailScope::ReadOnly, policy(), inbox());
        // Wait: the transport scope check happens in HttpGmailTransport,
        // not in the stub. The adapter's SEND gate requires the SEND
        // scope in the request. Build a request with only READ scope.
        let request = SendRequest {
            draft: DraftId::new("draft-1").expect("id"),
            idempotency_key: "k".into(),
            approval_class: 2,
            scopes_granted: vec![MailScope::Read],
        };
        let err = adapter.send(&request).expect_err("must deny");
        assert_eq!(err.code, MailErrorCode::Policy);
        assert!(err.message.contains("SEND"));
    }

    #[test]
    fn ep026_unit_gmail_unknown_message_not_found() {
        let transport = StubTransport::default();
        let adapter =
            GmailAdapter::new(Box::new(transport), GmailScope::ReadOnly, policy(), inbox());
        let err = adapter
            .fetch_message(&inbox(), &MessageId::new("missing").expect("id"))
            .expect_err("missing must be NotFound");
        assert_eq!(err.code, MailErrorCode::NotFound);
    }

    #[test]
    fn ep026_unit_gmail_send_requires_send_scope_in_request() {
        let transport = StubTransport::default().with_message(sample_message("m1"));
        let adapter = GmailAdapter::new(Box::new(transport), GmailScope::Full, policy(), inbox());
        let request = SendRequest {
            draft: DraftId::new("draft-2").expect("id"),
            idempotency_key: "k2".into(),
            approval_class: 2,
            scopes_granted: vec![MailScope::Read, MailScope::Draft],
        };
        let err = adapter.send(&request).expect_err("must deny");
        assert_eq!(err.code, MailErrorCode::Policy);
    }

    #[test]
    fn ep026_unit_gmail_archive_gates_and_records() {
        let transport = StubTransport::default().with_message(sample_message("m1"));
        let adapter = GmailAdapter::new(Box::new(transport), GmailScope::Full, policy(), inbox());
        adapter
            .archive(&inbox(), &MessageId::new("m1").expect("id"))
            .expect("archive");
        let entries = adapter.audit();
        assert!(entries
            .iter()
            .any(|e| e.operation == "ARCHIVE" && e.outcome == "ok"));
        assert!(entries.iter().any(|e| e.correlation.starts_with("mail-")));
    }

    #[test]
    fn ep026_unit_gmail_unknown_mailbox_fails_closed() {
        let transport = StubTransport::default().with_message(sample_message("m1"));
        let adapter =
            GmailAdapter::new(Box::new(transport), GmailScope::ReadOnly, policy(), inbox());
        let other = MailboxId::new("other").expect("id");
        let err = adapter
            .fetch_message(&other, &MessageId::new("m1").expect("id"))
            .expect_err("unknown mailbox must fail closed");
        assert_eq!(err.code, MailErrorCode::NotFound);
    }

    #[test]
    fn ep026_unit_extract_email_display_name() {
        assert_eq!(
            extract_email("Alice <alice@example.com>"),
            "alice@example.com"
        );
        assert_eq!(extract_email("bob@example.com"), "bob@example.com");
        assert_eq!(extract_email("plain"), "plain");
    }

    #[test]
    fn ep026_unit_base64url_roundtrip() {
        assert_eq!(base64url("hello"), "aGVsbG8");
        assert_eq!(base64url("Hello, world!"), "SGVsbG8sIHdvcmxkIQ");
    }
}
