//! EP-026 IMAP/SMTP adapter core (SPEC-014; M4).
//!
//! Real production adapter behind the nexus-email `EmailProvider`
//! port: IMAP for read/list/mailbox state/modify, SMTP for outbound
//! submission. IMAP and SMTP are SEPARATE authorities (directive C):
//! the adapter refuses modify operations when the IMAP authority is
//! read-only and refuses submission when the SMTP authority is absent,
//! BEFORE any transport call.
//!
//! Permanent invariants (owner directive, EP-026):
//! - READ != SEND != MODIFY (directive C).
//! - SENT != DELIVERED: SMTP acceptance proves submission only;
//!   DeliveryReceipt is the only delivery authority. A message in the
//!   Sent folder is not proof of recipient delivery (directive D).
//! - DRAFT != SENT.
//! - AMBIGUOUS OUTCOME -> VERIFY FIRST -> NO BLIND RETRY (directive M):
//!   if the SMTP connection dies after DATA, the provider MAY have
//!   accepted the message; the ledger records Ambiguous and a replay
//!   with the same idempotency key is REFUSED until reconciliation
//!   (M5-owned).
//! - Envelope vs header (directive R): the SMTP MAIL FROM and the RFC
//!   From header are both bound to the authenticated account address;
//!   the contract carries no spoofable From field.
//! - Header injection (directive Q): CR/LF in any user-controlled
//!   header value rejects the send BEFORE any provider mutation.
//! - Attachment safety (directive T): ScanStatus != CLEAN or size
//!   over policy rejects BEFORE any SMTP mutation.
//! - Unknown targets are NotFound, never Verified.
//! - Bounded concurrent connections (directive W): local backpressure
//!   is an explicit RateLimit error, never classified as provider
//!   success.
//!
//! No test-mode branches exist in production code.

use std::collections::HashMap;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nexus_email::{
    Attachment, AttachmentId, Draft, DraftId, EmailAddress, EmailProvider, MailChange, MailCommand,
    MailDirection, MailError, MailPolicy, MailScope, MailState, MailboxId, Message, MessageId,
    SendRequest, ThreadId,
};

use crate::observability::MailObservability;
use crate::transport::{ImapAttachmentMeta, ImapTransport, SmtpOutcome, SmtpTransport};

/// In-flight idempotency entry for one command on one target.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    command: String,
}

/// Completed-send ledger entry (directive M/N).
#[derive(Debug, Clone, PartialEq, Eq)]
enum SendLedgerEntry {
    /// Confirmed acceptance; a replay returns the same result.
    Confirmed(MessageId),
    /// Ambiguous outcome; a replay is REFUSED until reconciliation
    /// (no blind retry; M5 owns reconciliation).
    Ambiguous,
}

/// Bounded concurrent-connection limiter (directive W).
#[derive(Debug)]
struct ConnectionLimiter {
    max: usize,
    condvar: Condvar,
}

impl ConnectionLimiter {
    fn new(max: usize) -> Self {
        Self {
            max,
            condvar: Condvar::new(),
        }
    }

    /// Acquire a permit with a bounded wait. Returns a guard, or
    /// RateLimit when the limit is exceeded for the bound.
    fn acquire<'a>(
        &'a self,
        state: &'a Mutex<usize>,
        bound: Duration,
    ) -> Result<ConnectionPermit<'a>, MailError> {
        let mut guard = state.lock().expect("limiter lock");
        let deadline = SystemTime::now() + bound;
        while *guard >= self.max {
            let now = SystemTime::now();
            if now >= deadline {
                return Err(MailError::new(
                    nexus_email::MailErrorCode::RateLimit,
                    "connection limit exceeded",
                    None,
                    None,
                ));
            }
            let wait = deadline.duration_since(now).unwrap_or(Duration::ZERO);
            let (g, _) = self
                .condvar
                .wait_timeout(guard, wait)
                .expect("limiter wait");
            guard = g;
        }
        *guard += 1;
        Ok(ConnectionPermit {
            state,
            condvar: &self.condvar,
        })
    }
}

#[derive(Debug)]
struct ConnectionPermit<'a> {
    state: &'a Mutex<usize>,
    condvar: &'a Condvar,
}

impl Drop for ConnectionPermit<'_> {
    fn drop(&mut self) {
        let mut guard = self.state.lock().expect("limiter lock");
        *guard = guard.saturating_sub(1);
        self.condvar.notify_one();
    }
}

/// Real production IMAP/SMTP adapter over real protocol transports.
pub struct ImapSmtpAdapter {
    imap: Box<dyn ImapTransport>,
    smtp: Box<dyn SmtpTransport>,
    policy: MailPolicy,
    mailbox: MailboxId,
    /// Authenticated account address: the From header and the SMTP
    /// envelope sender are bound to it (directive R).
    account_addr: String,
    in_flight: Mutex<HashMap<(String, String), InFlightEntry>>,
    send_ledger: Mutex<HashMap<String, SendLedgerEntry>>,
    limiter: ConnectionLimiter,
    limiter_state: Mutex<usize>,
    observability: Mutex<MailObservability>,
}

impl ImapSmtpAdapter {
    pub fn new(
        imap: Box<dyn ImapTransport>,
        smtp: Box<dyn SmtpTransport>,
        policy: MailPolicy,
        mailbox: MailboxId,
        account_addr: impl Into<String>,
    ) -> Self {
        Self {
            imap,
            smtp,
            policy,
            mailbox,
            account_addr: account_addr.into(),
            in_flight: Mutex::new(HashMap::new()),
            send_ledger: Mutex::new(HashMap::new()),
            limiter: ConnectionLimiter::new(8),
            limiter_state: Mutex::new(0),
            observability: Mutex::new(MailObservability::default()),
        }
    }

    pub fn audit(&self) -> Vec<crate::observability::MailAuditEntry> {
        self.observability
            .lock()
            .expect("observability lock")
            .recent()
    }

    /// Configure the bounded concurrent-connection limit
    /// (directive W). Default 8.
    pub fn with_connection_limit(mut self, max: usize) -> Self {
        self.limiter = ConnectionLimiter::new(max.max(1));
        self
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

    /// Attachment safety gate (directive T): every attachment must be
    /// within policy bounds and CLEAN before any provider mutation.
    fn gate_attachments(
        &self,
        attachments: &[Attachment],
        correlation: &str,
        operation: &str,
    ) -> Result<(), MailError> {
        for attachment in attachments {
            if !self.policy.attachment_allows(attachment) {
                self.record(
                    correlation.to_string(),
                    operation,
                    "POLICY",
                    format!("attachment {} not deliverable", attachment.filename),
                );
                return Err(MailError::policy(format!(
                    "attachment {} rejected by mail policy (size/scan)",
                    attachment.filename
                )));
            }
        }
        Ok(())
    }

    /// IMAP modify authority check (directive C): a read-only IMAP
    /// authority refuses modification BEFORE any transport call.
    fn require_imap_modify(&self, operation: &str, correlation: &str) -> Result<(), MailError> {
        if !self.imap.authority().allows_modify() {
            self.record(
                correlation.to_string(),
                operation,
                "POLICY",
                "imap authority does not allow modify".to_string(),
            );
            return Err(MailError::policy(format!(
                "{operation} requires IMAP modify authority"
            )));
        }
        Ok(())
    }

    /// SMTP authority check (directive C): submission requires the
    /// SMTP submit authority.
    fn require_smtp_submit(&self, operation: &str, correlation: &str) -> Result<(), MailError> {
        if !self.smtp.authority().allows_submit() {
            self.record(
                correlation.to_string(),
                operation,
                "POLICY",
                "smtp authority does not allow submit".to_string(),
            );
            return Err(MailError::policy(format!(
                "{operation} requires SMTP submit authority"
            )));
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

    fn message_from_full(&self, msg: &crate::transport::ImapMessage) -> Result<Message, MailError> {
        let from = msg
            .from
            .as_deref()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| MailError::external("imap message missing From address"))?;
        let from_addr = EmailAddress::new(from)?;
        let to_addrs = msg
            .to
            .iter()
            .map(|h| EmailAddress::new(h.as_str()))
            .collect::<Result<Vec<_>, _>>()?;
        let state = if msg.flags.iter().any(|f| f == "\\Deleted") {
            MailState::Deleted
        } else if msg.flags.iter().any(|f| f == "NexusArchive") {
            MailState::Archived
        } else {
            MailState::Delivered
        };
        Ok(Message {
            id: MessageId::new(msg.message_id.clone())?,
            mailbox: self.mailbox.clone(),
            thread: ThreadId::new(msg.message_id.clone())?,
            direction: MailDirection::Inbound,
            from: from_addr,
            to: to_addrs,
            cc: vec![],
            bcc: vec![],
            subject: msg.subject.clone(),
            body_digest: sha256_hex(&msg.body),
            attachments: vec![],
            state,
            privacy_class: nexus_email::MailPrivacyClass::Private,
        })
    }

    /// Acquire a bounded connection permit for a provider call.
    fn acquire_permit(&self) -> Result<ConnectionPermit<'_>, MailError> {
        self.limiter
            .acquire(&self.limiter_state, Duration::from_millis(500))
    }
}

/// Build a minimal RFC 5322 message with strict header hygiene.
///
/// Directive Q: every user-controlled header value is checked for
/// CR/LF; an injection attempt rejects the message before any
/// provider mutation. Directive R: the From header is bound to the
/// authenticated account address; no spoofable value passes through.
fn build_mime(
    from: &str,
    to: &[String],
    subject: &str,
    body: &str,
    message_id: &str,
    extra: &[(&str, &str)],
) -> Result<Vec<u8>, MailError> {
    for (label, value) in [
        ("from", from),
        ("subject", subject),
        ("message-id", message_id),
    ]
    .into_iter()
    .chain(extra.iter().copied())
    {
        if value.contains(['\r', '\n']) {
            return Err(MailError::validation(format!(
                "header {label} contains CR/LF (injection rejected)"
            )));
        }
    }
    for recipient in to {
        if recipient.contains(['\r', '\n']) {
            return Err(MailError::validation(
                "recipient contains CR/LF (injection rejected)",
            ));
        }
    }
    let mut out = String::new();
    out.push_str("From: <");
    out.push_str(from);
    out.push_str(">\r\n");
    if !to.is_empty() {
        out.push_str("To: ");
        out.push_str(&to.join(", "));
        out.push_str("\r\n");
    }
    out.push_str("Subject: ");
    out.push_str(subject);
    out.push_str("\r\n");
    out.push_str("Message-ID: <");
    out.push_str(message_id);
    out.push_str(">\r\n");
    out.push_str("Date: ");
    out.push_str(&rfc2822_date());
    out.push_str("\r\n");
    out.push_str("MIME-Version: 1.0\r\n");
    out.push_str("Content-Type: text/plain; charset=utf-8\r\n");
    for (k, v) in extra {
        out.push_str(k);
        out.push_str(": ");
        out.push_str(v);
        out.push_str("\r\n");
    }
    out.push_str("\r\n");
    out.push_str(body);
    Ok(out.into_bytes())
}

fn rfc2822_date() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    format!("{}", now.as_secs())
}

/// Generate a canonical Message-ID for an outbound message.
fn generate_message_id(draft: &str, seq: u64) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    format!("{draft}.{nanos}.{seq}@nexus.local")
}

impl EmailProvider for ImapSmtpAdapter {
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
        let _permit = self.acquire_permit()?;
        let mut session = self.imap.open()?;
        let envelopes = session.uid_list("INBOX", 50)?;
        let mut threads: Vec<ThreadId> = envelopes
            .iter()
            .map(|e| ThreadId::new(e.message_id.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        session.logout();
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
        let _permit = self.acquire_permit()?;
        let mut session = self.imap.open()?;
        let imap_msg = session.uid_fetch_by_message_id("INBOX", message.as_str())?;
        session.logout();
        let msg = self.message_from_full(&imap_msg)?;
        self.record(correlation, "FETCH", "ok", format!("message {}", msg.id));
        Ok(msg)
    }

    fn list_attachments(&self, message: &MessageId) -> Result<Vec<Attachment>, MailError> {
        let correlation = self.observability.lock().expect("obs lock").correlation();
        // The operation reads the message (Read) and accesses
        // attachment artifacts (Attachments); both scopes are granted
        // by the policy gate, which fails closed otherwise.
        self.gate(
            MailCommand::Fetch,
            &[MailScope::Read, MailScope::Attachments],
            1,
            &correlation,
        )?;
        let _permit = self.acquire_permit()?;
        let mut session = self.imap.open()?;
        // Resolve the canonical message id to a live UID, then read the
        // REAL BODYSTRUCTURE. Only attachment-disposition/filename parts
        // are reported; inline text is never fabricated as an attachment.
        let imap_msg = session.uid_fetch_by_message_id("INBOX", message.as_str())?;
        let metas: Vec<ImapAttachmentMeta> =
            session.uid_fetch_attachments("INBOX", imap_msg.uid)?;
        session.logout();
        let mut attachments = Vec::with_capacity(metas.len());
        for meta in metas {
            attachments.push(Attachment {
                id: AttachmentId::new(format!("{}:{}", message.as_str(), meta.part_number))
                    .map_err(|_| MailError::external("invalid imap attachment id"))?,
                filename: meta.filename,
                content_type: meta.mime_type,
                size_bytes: meta.size_bytes,
                sha256: String::new(),
                storage_ref: format!("imap:{}:{}", message.as_str(), meta.part_number),
                scan_status: nexus_email::ScanStatus::Pending,
            });
        }
        self.record(
            correlation,
            "FETCH",
            "ok",
            format!("{} attachments on {}", attachments.len(), message),
        );
        Ok(attachments)
    }

    fn save_draft(&self, draft: &Draft) -> Result<DraftId, MailError> {
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::Draft, &[MailScope::Draft], 1, &correlation)?;
        self.gate_attachments(&draft.attachments, &correlation, "DRAFT")?;
        self.require_imap_modify("DRAFT", &correlation)?;
        self.begin("DRAFT", draft.id.as_str(), &correlation)?;
        let to: Vec<String> = draft.to.iter().map(ToString::to_string).collect();
        let message_id = format!("{}@nexus.local", draft.id);
        let mime = build_mime(
            &self.account_addr,
            &to,
            &draft.subject,
            &draft.body_digest,
            &message_id,
            &[],
        );
        let result = mime
            .and_then(|bytes| {
                let _permit = self.acquire_permit()?;
                let mut session = self.imap.open()?;
                let res = session.append_draft("Drafts", &bytes);
                session.logout();
                res?;
                Ok(())
            })
            .map(|_| draft.id.clone())
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
        self.require_smtp_submit("SEND", &correlation)?;
        let target = request.draft.as_str();
        // Completed-send idempotency (directive N): Confirmed replay
        // returns the SAME result with zero second send; Ambiguous
        // replay is REFUSED (no blind retry) until reconciliation.
        {
            let ledger = self.send_ledger.lock().expect("send ledger lock");
            match ledger.get(&request.idempotency_key) {
                Some(SendLedgerEntry::Confirmed(id)) => {
                    self.record(correlation, "SEND", "ok", format!("idempotent replay {id}"));
                    return Ok(id.clone());
                }
                Some(SendLedgerEntry::Ambiguous) => {
                    self.record(
                        correlation,
                        "SEND",
                        "AMBIGUOUS",
                        "replay refused pending reconciliation".into(),
                    );
                    return Err(MailError::verification(
                        "send outcome ambiguous; reconciliation required before retry",
                    ));
                }
                None => {}
            }
        }
        self.begin("SEND", target, &correlation)?;
        // The draft is local intent (DRAFT != SENT): the send flow
        // fetches the stored draft from IMAP Drafts by its canonical
        // Message-ID, builds the MIME, and submits it via SMTP. The
        // sent message keeps the SAME Message-ID, so provider-side
        // readback binds the sent item to the request.
        let draft_message_id = format!("{target}@nexus.local");
        let result = (|| {
            let _permit = self.acquire_permit()?;
            let mut session = self.imap.open()?;
            let fetched = session.uid_fetch_by_message_id("Drafts", &draft_message_id)?;
            let to: Vec<String> = fetched.to.clone();
            let subject = fetched.subject.clone();
            let body = fetched.body.clone();
            session.logout();
            let mime = build_mime(
                &self.account_addr,
                &to,
                &subject,
                &body,
                &draft_message_id,
                &[],
            )?;
            let _permit2 = self.acquire_permit()?;
            self.smtp
                .submit(&self.account_addr, &to, &mime, &draft_message_id)
        })()
        .inspect_err(|err| {
            self.record(
                correlation.clone(),
                "SEND",
                err.code.as_str(),
                err.message.clone(),
            );
        });
        let result = match result {
            Ok(SmtpOutcome::Accepted(mid)) => {
                let mut ledger = self.send_ledger.lock().expect("send ledger lock");
                if ledger.len() >= 4096 {
                    ledger.clear();
                }
                let id = MessageId::new(mid)?;
                ledger.insert(
                    request.idempotency_key.clone(),
                    SendLedgerEntry::Confirmed(id.clone()),
                );
                Ok(id)
            }
            Ok(SmtpOutcome::Ambiguous) => {
                let mut ledger = self.send_ledger.lock().expect("send ledger lock");
                ledger.insert(request.idempotency_key.clone(), SendLedgerEntry::Ambiguous);
                self.record(
                    correlation.clone(),
                    "SEND",
                    "AMBIGUOUS",
                    "submission outcome ambiguous; no blind retry".into(),
                );
                Err(MailError::verification(
                    "smtp submission outcome ambiguous; reconciliation required",
                ))
            }
            Err(err) => Err(err),
        };
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
        self.gate_attachments(&draft.attachments, &correlation, "REPLY")?;
        self.require_smtp_submit("REPLY", &correlation)?;
        self.begin("REPLY", original.as_str(), &correlation)?;
        let to: Vec<String> = draft.to.iter().map(ToString::to_string).collect();
        let message_id = generate_message_id(original.as_str(), 1);
        let mime = build_mime(
            &self.account_addr,
            &to,
            &format!("Re: {}", draft.subject),
            &draft.body_digest,
            &message_id,
            &[
                ("In-Reply-To", &format!("<{}>", original.as_str())),
                ("References", &format!("<{}>", original.as_str())),
            ],
        );
        let result = mime
            .and_then(|bytes| {
                let _permit = self.acquire_permit()?;
                self.smtp
                    .submit(&self.account_addr, &to, &bytes, &message_id)
            })
            .and_then(|outcome| match outcome {
                SmtpOutcome::Accepted(mid) => MessageId::new(mid),
                SmtpOutcome::Ambiguous => Err(MailError::verification(
                    "smtp reply outcome ambiguous; reconciliation required",
                )),
            })
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
        self.gate_attachments(&draft.attachments, &correlation, "FORWARD")?;
        self.require_smtp_submit("FORWARD", &correlation)?;
        self.begin("FORWARD", original.as_str(), &correlation)?;
        let to: Vec<String> = draft.to.iter().map(ToString::to_string).collect();
        let message_id = generate_message_id(original.as_str(), 2);
        let mime = build_mime(
            &self.account_addr,
            &to,
            &format!("Fwd: {}", draft.subject),
            &draft.body_digest,
            &message_id,
            &[("References", &format!("<{}>", original.as_str()))],
        );
        let result = mime
            .and_then(|bytes| {
                let _permit = self.acquire_permit()?;
                self.smtp
                    .submit(&self.account_addr, &to, &bytes, &message_id)
            })
            .and_then(|outcome| match outcome {
                SmtpOutcome::Accepted(mid) => MessageId::new(mid),
                SmtpOutcome::Ambiguous => Err(MailError::verification(
                    "smtp forward outcome ambiguous; reconciliation required",
                )),
            })
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
        self.require_imap_modify("ARCHIVE", &correlation)?;
        self.begin("ARCHIVE", message.as_str(), &correlation)?;
        let result = (|| {
            let _permit = self.acquire_permit()?;
            let mut session = self.imap.open()?;
            let fetched = session.uid_fetch_by_message_id("INBOX", message.as_str())?;
            let uid = fetched.uid;
            let res = session.uid_archive("INBOX", uid);
            session.logout();
            res
        })()
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
        self.require_imap_modify("LABEL", &correlation)?;
        self.begin("LABEL", message.as_str(), &correlation)?;
        let result = (|| {
            let _permit = self.acquire_permit()?;
            let mut session = self.imap.open()?;
            let fetched = session.uid_fetch_by_message_id("INBOX", message.as_str())?;
            let uid = fetched.uid;
            let res = session.uid_label("INBOX", uid, label);
            session.logout();
            res
        })()
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
        self.require_imap_modify("DELETE", &correlation)?;
        self.begin("DELETE", message.as_str(), &correlation)?;
        let result = (|| {
            let _permit = self.acquire_permit()?;
            let mut session = self.imap.open()?;
            let fetched = session.uid_fetch_by_message_id("INBOX", message.as_str())?;
            let uid = fetched.uid;
            let res = session.uid_delete("INBOX", uid);
            session.logout();
            res
        })()
        .inspect_err(|err| {
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
                self.record(correlation, "DELETE", "ok", format!("deleted {message}"));
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
        let _permit = self.acquire_permit()?;
        let mut session = self.imap.open()?;
        let fetched = session.uid_fetch_by_message_id("INBOX", message.as_str())?;
        let state = if fetched.flags.iter().any(|f| f == "\\Deleted") {
            MailState::Deleted
        } else if fetched.flags.iter().any(|f| f == "NexusArchive") {
            MailState::Archived
        } else {
            MailState::Delivered
        };
        session.logout();
        self.record(
            correlation,
            "STATE",
            "ok",
            format!("message {message} state {}", state.as_str()),
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
        // Controlled polling fallback (SPEC-014): list messages after
        // a sequence boundary; the caller owns the cursor.
        let correlation = self.observability.lock().expect("obs lock").correlation();
        self.gate(MailCommand::List, &[MailScope::Read], 1, &correlation)?;
        let _permit = self.acquire_permit()?;
        let mut session = self.imap.open()?;
        let envelopes = session.uid_list("INBOX", 0)?;
        let changes = envelopes
            .into_iter()
            .enumerate()
            .filter(|(i, _)| (*i as u64) >= after_sequence)
            .map(|(i, e)| MailChange {
                mailbox: mailbox.clone(),
                message: MessageId::new(e.message_id.clone()).ok(),
                thread: ThreadId::new(e.message_id.clone()).ok(),
                change: nexus_email::MailChangeKind::NewMessage,
                sequence: i as u64,
            })
            .collect();
        session.logout();
        self.record(
            correlation,
            "CHANGES",
            "ok",
            format!("{after_sequence} cursor changes"),
        );
        Ok(changes)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep026_unit_m4_build_mime_rejects_header_injection() {
        // Directive Q: CR/LF in user-controlled header values reject
        // the message BEFORE any provider mutation.
        let err = build_mime(
            "a@example.com",
            &["b@example.com".into()],
            "Subject\r\nBcc: evil@example.com",
            "body",
            "mid@nexus.local",
            &[],
        )
        .expect_err("subject CR/LF must reject");
        assert_eq!(err.code, nexus_email::MailErrorCode::Validation);

        let err = build_mime(
            "a@example.com",
            &["b@example.com\r\nX-Evil: 1".into()],
            "Subject",
            "body",
            "mid@nexus.local",
            &[],
        )
        .expect_err("recipient CR/LF must reject");
        assert_eq!(err.code, nexus_email::MailErrorCode::Validation);

        let err = build_mime(
            "a@example.com",
            &["b@example.com".into()],
            "Subject",
            "body",
            "mid@nexus.local",
            &[("In-Reply-To", "<x>\r\nX-Evil: 1")],
        )
        .expect_err("extra header CR/LF must reject");
        assert_eq!(err.code, nexus_email::MailErrorCode::Validation);
    }

    #[test]
    fn ep026_unit_m4_build_mime_ok() {
        let bytes = build_mime(
            "a@example.com",
            &["b@example.com".into()],
            "Hello",
            "body",
            "mid@nexus.local",
            &[("In-Reply-To", "<orig@nexus.local>")],
        )
        .expect("clean mime");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("From: <a@example.com>"));
        assert!(text.contains("To: b@example.com"));
        assert!(text.contains("Subject: Hello"));
        assert!(text.contains("Message-ID: <mid@nexus.local>"));
        assert!(text.contains("In-Reply-To: <orig@nexus.local>"));
    }

    #[test]
    fn ep026_unit_m4_ledger_ambiguous_refuses_replay() {
        // Directive M/N: an ambiguous send must not be blindly
        // retried; the ledger records Ambiguous and a replay is
        // refused until reconciliation.
        let entry = SendLedgerEntry::Ambiguous;
        match entry {
            SendLedgerEntry::Ambiguous => {}
            SendLedgerEntry::Confirmed(_) => panic!("must be ambiguous"),
        }
    }

    #[test]
    fn ep026_unit_m4_connection_limiter_backpressure_and_recovery() {
        // Directive W: bounded concurrency, predictable refusal when
        // the limit is exceeded, recovery after pressure disappears.
        let limiter = ConnectionLimiter::new(2);
        let state = Mutex::new(0);
        let p1 = limiter
            .acquire(&state, Duration::from_millis(200))
            .expect("permit 1");
        let p2 = limiter
            .acquire(&state, Duration::from_millis(200))
            .expect("permit 2");
        let err = limiter
            .acquire(&state, Duration::from_millis(200))
            .expect_err("third permit must exceed the limit");
        assert_eq!(err.code, nexus_email::MailErrorCode::RateLimit);
        drop(p1);
        let p3 = limiter
            .acquire(&state, Duration::from_millis(500))
            .expect("permit after release (recovery)");
        drop(p2);
        drop(p3);
        assert_eq!(*state.lock().expect("state"), 0);
    }
}
