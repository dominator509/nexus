//! EP-026 IMAP/SMTP transport (M4): real protocol clients.
//!
//! IMAP (read) and SMTP (submission) are SEPARATE authorities
//! (directive C): SMTP credentials never imply IMAP read permission,
//! IMAP credentials never imply send permission, and IMAP modify
//! operations require a Modify-class authority, never read-only.
//!
//! Real protocol stacks:
//! - IMAP: the mature `imap` crate (RFC 3501) over a caller-owned
//!   `TcpStream` with OS-level read/write timeouts (deterministic
//!   Timeout classification); TLS via native-tls with an optional
//!   custom root certificate.
//! - SMTP: the mature `lettre` crate's `SmtpConnection` driven
//!   phase-exactly (EHLO, AUTH, MAIL FROM, RCPT TO, DATA, message).
//!   Phase knowledge is what makes the ambiguous-outcome
//!   classification honest: if the final response after DATA is lost,
//!   the submission MAY have been accepted and must NOT be blindly
//!   retried (directive M).
//!
//! SMTP server acceptance is SUBMISSION (SENT), never DELIVERY
//! (DELIVERED). A message in the Sent folder is not proof the
//! recipient received it (directive D).

use std::error::Error as StdError;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use imap::types::{Flag, Mailbox as ImapMailbox};
use nexus_email::{MailError, MailState};

/// Unified stream for the IMAP session (plaintext or TLS).
trait Stream: Read + Write + Send {}
impl<T: Read + Write + Send> Stream for T {}

/// IMAP authority: read-only never implies modify (directive C).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImapAuthority {
    /// Read/list/fetch only. Modify operations are refused.
    ReadOnly,
    /// Read + mailbox modification (flags, archive, label, delete).
    Modify,
    /// Read + modify (alias for clarity; no additional authority).
    Full,
}

impl ImapAuthority {
    pub const fn allows_read(self) -> bool {
        matches!(self, Self::ReadOnly | Self::Modify | Self::Full)
    }

    pub const fn allows_modify(self) -> bool {
        matches!(self, Self::Modify | Self::Full)
    }
}

/// SMTP authority: submission only (directive C). A submit token
/// never grants read access to any mailbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpAuthority {
    Submit,
}

impl SmtpAuthority {
    pub const fn allows_submit(self) -> bool {
        matches!(self, Self::Submit)
    }
}

/// IMAP TLS mode.
#[derive(Debug, Clone)]
pub enum ImapTls {
    /// Plaintext (controlled fixture / localhost only).
    Plain,
    /// TLS with the default trust store.
    Tls,
    /// TLS trusting ONLY the supplied PEM root certificate.
    TlsWithCa(Vec<u8>),
}

/// SMTP TLS mode.
#[derive(Debug, Clone)]
pub enum SmtpTls {
    /// Plaintext (controlled fixture / localhost only).
    Plain,
    /// TLS with the default trust store.
    Tls,
    /// TLS trusting ONLY the supplied PEM root certificate.
    TlsWithCa(Vec<u8>),
}

/// IMAP message envelope (headers only; no body bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImapEnvelope {
    pub uid: u32,
    pub message_id: String,
    pub subject: String,
    pub from: Option<String>,
    pub to: Vec<String>,
    pub body_preview: Option<String>,
    pub flags: Vec<String>,
    pub mailbox: String,
}

/// Full IMAP message (headers + body).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImapMessage {
    pub uid: u32,
    pub message_id: String,
    pub subject: String,
    pub from: Option<String>,
    pub to: Vec<String>,
    pub body: String,
    pub flags: Vec<String>,
}

/// Canonical IMAP attachment metadata derived from the REAL
/// BODYSTRUCTURE: only parts with an attachment disposition (or a
/// filename parameter) are attachments; inline/text parts are not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImapAttachmentMeta {
    pub part_number: String,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: u64,
}

/// Outcome of an SMTP submission (directive M).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmtpOutcome {
    /// The submission server returned its final acceptance (250).
    /// This is SUBMISSION (SENT), never recipient delivery.
    Accepted(String),
    /// The client wrote the message body but the connection
    /// disappeared before the authoritative final response. The
    /// provider MAY have accepted the message. MUST NOT be blindly
    /// retried; verification is required.
    Ambiguous,
}

/// A live IMAP session over one authenticated connection.
pub trait ImapSession: Send {
    fn uid_list(&mut self, mailbox: &str, top: u32) -> Result<Vec<ImapEnvelope>, MailError>;
    fn uid_fetch_by_message_id(
        &mut self,
        mailbox: &str,
        message_id: &str,
    ) -> Result<ImapMessage, MailError>;
    fn uid_fetch(&mut self, mailbox: &str, uid: u32) -> Result<ImapMessage, MailError>;
    /// Fetch the BODYSTRUCTURE for a message and return its real
    /// attachment parts (disposition=attachment or filename present).
    fn uid_fetch_attachments(
        &mut self,
        mailbox: &str,
        uid: u32,
    ) -> Result<Vec<ImapAttachmentMeta>, MailError>;
    fn uid_state(&mut self, mailbox: &str, uid: u32) -> Result<MailState, MailError>;
    fn uid_archive(&mut self, mailbox: &str, uid: u32) -> Result<(), MailError>;
    fn uid_label(&mut self, mailbox: &str, uid: u32, label: &str) -> Result<(), MailError>;
    fn uid_delete(&mut self, mailbox: &str, uid: u32) -> Result<(), MailError>;
    fn append_draft(&mut self, mailbox: &str, content: &[u8]) -> Result<(), MailError>;
    /// Create a mailbox (folder) if it does not already exist. Used to
    /// re-provision mailbox topology after a provider restart, which
    /// wipes in-memory folders (real IMAP CREATE).
    fn create_mailbox(&mut self, mailbox: &str) -> Result<(), MailError>;
    fn logout(&mut self);
}

/// IMAP transport factory. Each operation opens a fresh
/// authenticated session; the configured authority is enforced at the
/// boundary (directive C).
pub trait ImapTransport: Send + Sync {
    fn authority(&self) -> ImapAuthority;
    fn open(&self) -> Result<Box<dyn ImapSession>, MailError>;
    fn health_check(&self) -> Result<(), MailError>;
}

/// SMTP submission transport.
pub trait SmtpTransport: Send + Sync {
    fn authority(&self) -> SmtpAuthority;
    /// Submit a pre-built RFC 5322 message. `envelope_from` is the
    /// SMTP MAIL FROM (bounce) address; `message_id` is the canonical
    /// Message-ID header value generated by the caller.
    fn submit(
        &self,
        envelope_from: &str,
        to: &[String],
        message: &[u8],
        message_id: &str,
    ) -> Result<SmtpOutcome, MailError>;
    fn health_check(&self) -> Result<(), MailError>;
}

fn classify_io(err: &std::io::Error, what: &str) -> MailError {
    use std::io::ErrorKind;
    match err.kind() {
        ErrorKind::WouldBlock | ErrorKind::TimedOut => {
            MailError::timeout(format!("{what} timed out"))
        }
        ErrorKind::ConnectionRefused
        | ErrorKind::ConnectionReset
        | ErrorKind::ConnectionAborted
        | ErrorKind::NotConnected
        | ErrorKind::BrokenPipe => MailError::unavailable(format!("{what}: connection lost")),
        _ => MailError::unavailable(format!("{what}: network error")),
    }
}

fn classify_imap(err: &imap::Error, what: &str) -> MailError {
    match err {
        imap::Error::Io(io) => classify_io(io, what),
        imap::Error::No(resp) => {
            let text = resp.to_string();
            if text.contains("LOGIN")
                || text.contains("AUTHENTICATE")
                || text.contains("auth")
                || text.contains("credentials")
            {
                MailError::authorization(format!("{what}: IMAP authentication rejected"))
            } else {
                MailError::not_found(format!("{what}: IMAP no such mailbox/uid: {text}"))
            }
        }
        imap::Error::Bad(resp) => MailError::external(format!("{what}: IMAP BAD response: {resp}")),
        imap::Error::ConnectionLost => MailError::unavailable(format!("{what}: connection lost")),
        imap::Error::TlsHandshake(_) => {
            MailError::authorization(format!("{what}: IMAP TLS handshake failed"))
        }
        imap::Error::Tls(_) => MailError::authorization(format!("{what}: IMAP TLS failure")),
        _ => MailError::external(format!("{what}: IMAP protocol error")),
    }
}

// ------------------------------------------------------------------
// Real IMAP transport
// ------------------------------------------------------------------

/// Real IMAP transport over the `imap` crate.
pub struct RealImapTransport {
    host: String,
    port: u16,
    username: String,
    password: String,
    authority: ImapAuthority,
    timeout: Duration,
    tls: ImapTls,
}

impl RealImapTransport {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        authority: ImapAuthority,
        tls: ImapTls,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            authority,
            timeout: Duration::from_secs(4),
            tls,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn connect_client(&self) -> Result<imap::Client<Box<dyn Stream>>, MailError> {
        match &self.tls {
            ImapTls::Plain => {
                let stream = TcpStream::connect((self.host.as_str(), self.port))
                    .map_err(|e| classify_io(&e, "imap connect"))?;
                stream
                    .set_read_timeout(Some(self.timeout))
                    .map_err(|e| classify_io(&e, "imap read timeout set"))?;
                stream
                    .set_write_timeout(Some(self.timeout))
                    .map_err(|e| classify_io(&e, "imap write timeout set"))?;
                let mut client = imap::Client::new(Box::new(stream) as Box<dyn Stream>);
                client
                    .read_greeting()
                    .map_err(|e| classify_imap(&e, "imap greeting"))?;
                Ok(client)
            }
            ImapTls::Tls | ImapTls::TlsWithCa(_) => {
                let mut builder = native_tls::TlsConnector::builder();
                if let ImapTls::TlsWithCa(pem) = &self.tls {
                    let cert = native_tls::Certificate::from_pem(pem)
                        .map_err(|e| MailError::external(format!("imap CA parse: {e}")))?;
                    builder.add_root_certificate(cert);
                }
                let connector = builder
                    .build()
                    .map_err(|e| MailError::external(format!("imap tls builder: {e}")))?;
                let tcp = TcpStream::connect((self.host.as_str(), self.port))
                    .map_err(|e| classify_io(&e, "imap tls connect"))?;
                let tls_stream = connector
                    .connect(self.host.as_str(), tcp)
                    .map_err(|_e| MailError::authorization("imap TLS handshake failed"))?;
                let mut client = imap::Client::new(Box::new(tls_stream) as Box<dyn Stream>);
                client
                    .read_greeting()
                    .map_err(|e| classify_imap(&e, "imap tls greeting"))?;
                Ok(client)
            }
        }
    }
}

impl ImapTransport for RealImapTransport {
    fn authority(&self) -> ImapAuthority {
        self.authority
    }

    fn open(&self) -> Result<Box<dyn ImapSession>, MailError> {
        let client = self.connect_client()?;
        let session = match client.login(&self.username, &self.password) {
            Ok(session) => session,
            Err((e, _orig)) => return Err(classify_imap(&e, "imap login")),
        };
        Ok(Box::new(RealImapSession { session }))
    }

    fn health_check(&self) -> Result<(), MailError> {
        let mut session = self.open()?;
        session.logout();
        Ok(())
    }
}

struct RealImapSession {
    session: imap::Session<Box<dyn Stream>>,
}

fn select_mailbox(
    session: &mut imap::Session<Box<dyn Stream>>,
    mailbox: &str,
) -> Result<ImapMailbox, MailError> {
    session
        .select(mailbox)
        .map_err(|e| classify_imap(&e, "imap select"))
}

fn lossy(bytes: Option<&[u8]>) -> String {
    bytes
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default()
}

fn lossy_cow(bytes: Option<std::borrow::Cow<'_, [u8]>>) -> String {
    lossy(bytes.as_deref())
}

fn envelope_header_fields(
    fetch: &imap::types::Fetch,
) -> (String, String, Option<String>, Vec<String>) {
    let envelope = fetch.envelope();
    let message_id = lossy_cow(envelope.and_then(|e| e.message_id.clone()));
    // RFC 3501 ENVELOPE message-id carries the RFC 5322 value with
    // angle brackets; the canonical identifier is the bare value.
    let message_id = message_id
        .trim_start_matches('<')
        .trim_end_matches('>')
        .to_string();
    let subject = lossy_cow(envelope.and_then(|e| e.subject.clone()));
    let from = envelope
        .and_then(|e| e.from.as_ref())
        .and_then(|addrs| addrs.first())
        .map(|a| {
            format!(
                "{}@{}",
                lossy_cow(a.mailbox.clone()),
                lossy_cow(a.host.clone())
            )
        });
    let to = envelope
        .and_then(|e| e.to.as_ref())
        .map(|addrs| {
            addrs
                .iter()
                .filter_map(|a| {
                    let mb = lossy_cow(a.mailbox.clone());
                    let host = lossy_cow(a.host.clone());
                    if mb.is_empty() && host.is_empty() {
                        None
                    } else {
                        Some(format!("{mb}@{host}"))
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    (message_id, subject, from, to)
}

fn envelope_from_fetch(
    fetch: &imap::types::Fetch,
    mailbox: &str,
) -> Result<ImapEnvelope, MailError> {
    let uid = fetch
        .uid
        .ok_or_else(|| MailError::external("imap fetch missing UID"))?;
    let (message_id, subject, from, to) = envelope_header_fields(fetch);
    let message_id = if message_id.is_empty() {
        format!("imap-{uid}@{mailbox}")
    } else {
        message_id
    };
    let flags: Vec<String> = fetch.flags().iter().map(|f| f.to_string()).collect();
    Ok(ImapEnvelope {
        uid,
        message_id,
        subject,
        from,
        to,
        body_preview: None,
        flags,
        mailbox: mailbox.to_string(),
    })
}

/// Recursively walk a BODYSTRUCTURE tree collecting attachment parts.
/// A part is an attachment only when its disposition is attachment or
/// it carries a filename parameter; inline/text parts are skipped -
/// never fabricated as attachments.
fn walk_bodystructure(
    bs: &imap_proto::types::BodyStructure<'_>,
    prefix: &str,
    out: &mut Vec<ImapAttachmentMeta>,
) {
    use imap_proto::types::BodyStructure;
    match bs {
        BodyStructure::Multipart { bodies, .. } => {
            for (i, child) in bodies.iter().enumerate() {
                let child_prefix = if prefix.is_empty() {
                    (i + 1).to_string()
                } else {
                    format!("{prefix}.{}", i + 1)
                };
                walk_bodystructure(child, &child_prefix, out);
            }
        }
        BodyStructure::Basic { common, other, .. }
        | BodyStructure::Text { common, other, .. }
        | BodyStructure::Message { common, other, .. } => {
            let mut filename = None;
            let mut is_attachment = false;
            if let Some(disp) = &common.disposition {
                if disp.ty.eq_ignore_ascii_case("attachment") {
                    is_attachment = true;
                }
                if let Some(params) = &disp.params {
                    for (k, v) in params {
                        if k.eq_ignore_ascii_case("filename") {
                            filename = Some(v.to_string());
                        }
                    }
                }
            }
            if filename.is_none() {
                if let Some(params) = &common.ty.params {
                    for (k, v) in params {
                        if k.eq_ignore_ascii_case("name") {
                            filename = Some(v.to_string());
                        }
                    }
                }
            }
            if !is_attachment && filename.is_none() {
                return;
            }
            let mime_type = format!("{}/{}", common.ty.ty, common.ty.subtype);
            out.push(ImapAttachmentMeta {
                part_number: if prefix.is_empty() {
                    "1".to_string()
                } else {
                    prefix.to_string()
                },
                filename: filename.unwrap_or_default(),
                mime_type,
                size_bytes: other.octets as u64,
            });
        }
    }
}

fn message_from_fetch(fetch: &imap::types::Fetch, mailbox: &str) -> Result<ImapMessage, MailError> {
    let uid = fetch
        .uid
        .ok_or_else(|| MailError::external("imap fetch missing UID"))?;
    let (message_id, subject, from, to) = envelope_header_fields(fetch);
    let message_id = if message_id.is_empty() {
        format!("imap-{uid}@{mailbox}")
    } else {
        message_id
    };
    let body = fetch
        .body()
        .map(|b| String::from_utf8_lossy(b).to_string())
        .unwrap_or_default();
    let flags: Vec<String> = fetch.flags().iter().map(|f| f.to_string()).collect();
    Ok(ImapMessage {
        uid,
        message_id,
        subject,
        from,
        to,
        body,
        flags,
    })
}

fn state_from_flags(flags: &[Flag], mailbox: &str) -> MailState {
    if flags.iter().any(|f| f == &Flag::Deleted) {
        MailState::Deleted
    } else if flags.iter().any(|f| f.to_string() == "NexusArchive") {
        MailState::Archived
    } else if mailbox.eq_ignore_ascii_case("trash") {
        MailState::Deleted
    } else {
        MailState::Delivered
    }
}

impl ImapSession for RealImapSession {
    fn uid_list(&mut self, mailbox: &str, top: u32) -> Result<Vec<ImapEnvelope>, MailError> {
        select_mailbox(&mut self.session, mailbox)?;
        let uids = self
            .session
            .uid_search("ALL")
            .map_err(|e| classify_imap(&e, "imap search"))?;
        let mut uids: Vec<u32> = uids.into_iter().collect();
        uids.sort_unstable();
        if top > 0 && uids.len() as u32 > top {
            uids = uids[uids.len() - top as usize..].to_vec();
        }
        if uids.is_empty() {
            return Ok(Vec::new());
        }
        let set = uids
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let fetches = self
            .session
            .uid_fetch(
                set.as_str(),
                "(UID FLAGS ENVELOPE BODY.PEEK[HEADER.FIELDS (MESSAGE-ID SUBJECT FROM TO)])",
            )
            .map_err(|e| classify_imap(&e, "imap fetch list"))?;
        let mut out = Vec::new();
        for fetch in fetches.iter() {
            out.push(envelope_from_fetch(fetch, mailbox)?);
        }
        Ok(out)
    }

    fn uid_fetch_by_message_id(
        &mut self,
        mailbox: &str,
        message_id: &str,
    ) -> Result<ImapMessage, MailError> {
        select_mailbox(&mut self.session, mailbox)?;
        let query = format!("HEADER Message-ID \"{message_id}\"");
        let uids = self
            .session
            .uid_search(query.as_str())
            .map_err(|e| classify_imap(&e, "imap search message-id"))?;
        let mut uids: Vec<u32> = uids.into_iter().collect();
        uids.sort_unstable();
        let uid = uids
            .first()
            .copied()
            .ok_or_else(|| MailError::not_found(format!("no such message {message_id}")))?;
        self.uid_fetch(mailbox, uid)
    }

    fn uid_fetch(&mut self, mailbox: &str, uid: u32) -> Result<ImapMessage, MailError> {
        select_mailbox(&mut self.session, mailbox)?;
        let fetches = self
            .session
            .uid_fetch(uid.to_string().as_str(), "(UID FLAGS ENVELOPE BODY[])")
            .map_err(|e| classify_imap(&e, "imap fetch"))?;
        let fetch = fetches
            .iter()
            .find(|f| f.uid == Some(uid))
            .ok_or_else(|| MailError::not_found(format!("no such uid {uid} in {mailbox}")))?;
        message_from_fetch(fetch, mailbox)
    }

    fn uid_fetch_attachments(
        &mut self,
        mailbox: &str,
        uid: u32,
    ) -> Result<Vec<ImapAttachmentMeta>, MailError> {
        select_mailbox(&mut self.session, mailbox)?;
        let fetches = self
            .session
            .uid_fetch(uid.to_string().as_str(), "(UID BODYSTRUCTURE)")
            .map_err(|e| classify_imap(&e, "imap bodystructure fetch"))?;
        let fetch = fetches
            .iter()
            .find(|f| f.uid == Some(uid))
            .ok_or_else(|| MailError::not_found(format!("no such uid {uid} in {mailbox}")))?;
        let mut out = Vec::new();
        if let Some(bs) = fetch.bodystructure() {
            walk_bodystructure(bs, "", &mut out);
        }
        Ok(out)
    }

    fn uid_state(&mut self, mailbox: &str, uid: u32) -> Result<MailState, MailError> {
        select_mailbox(&mut self.session, mailbox)?;
        let fetches = self
            .session
            .uid_fetch(uid.to_string().as_str(), "(UID FLAGS)")
            .map_err(|e| classify_imap(&e, "imap state fetch"))?;
        let fetch = fetches
            .iter()
            .find(|f| f.uid == Some(uid))
            .ok_or_else(|| MailError::not_found(format!("no such uid {uid} in {mailbox}")))?;
        let flags: Vec<Flag> = fetch.flags().to_vec();
        Ok(state_from_flags(&flags, mailbox))
    }

    fn uid_archive(&mut self, mailbox: &str, uid: u32) -> Result<(), MailError> {
        select_mailbox(&mut self.session, mailbox)?;
        self.session
            .uid_store(uid.to_string().as_str(), "+FLAGS.SILENT (NexusArchive)")
            .map_err(|e| classify_imap(&e, "imap archive"))?;
        Ok(())
    }

    fn uid_label(&mut self, mailbox: &str, uid: u32, label: &str) -> Result<(), MailError> {
        select_mailbox(&mut self.session, mailbox)?;
        let clean: String = label
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .take(32)
            .collect();
        if clean.is_empty() {
            return Err(MailError::validation("imap label must be alphanumeric"));
        }
        self.session
            .uid_store(
                uid.to_string().as_str(),
                format!("+FLAGS.SILENT ({clean})").as_str(),
            )
            .map_err(|e| classify_imap(&e, "imap label"))?;
        Ok(())
    }

    fn uid_delete(&mut self, mailbox: &str, uid: u32) -> Result<(), MailError> {
        select_mailbox(&mut self.session, mailbox)?;
        self.session
            .uid_store(uid.to_string().as_str(), "+FLAGS.SILENT (\\Deleted)")
            .map_err(|e| classify_imap(&e, "imap delete"))?;
        self.session
            .uid_expunge(uid.to_string().as_str())
            .map_err(|e| classify_imap(&e, "imap expunge"))?;
        Ok(())
    }

    fn append_draft(&mut self, mailbox: &str, content: &[u8]) -> Result<(), MailError> {
        self.session
            .append(mailbox, content)
            .finish()
            .map_err(|e| classify_imap(&e, "imap append"))?;
        Ok(())
    }

    fn create_mailbox(&mut self, mailbox: &str) -> Result<(), MailError> {
        // Real IMAP CREATE; "already exists" (ALREADYEXISTS / NO) is
        // treated as success so topology provisioning is idempotent.
        match self.session.create(mailbox) {
            Ok(_) => Ok(()),
            Err(e) => {
                let text = e.to_string();
                if text.contains("ALREADYEXISTS") || text.contains("already exists") {
                    Ok(())
                } else {
                    Err(classify_imap(&e, "imap create"))
                }
            }
        }
    }

    fn logout(&mut self) {
        let _ = self.session.logout();
    }
}

// ------------------------------------------------------------------
// Real SMTP transport (phase-exact via lettre SmtpConnection)
// ------------------------------------------------------------------

fn classify_lettre(err: &lettre::transport::smtp::Error, phase: &str) -> MailError {
    // lettre's is_timeout only recognizes io::ErrorKind::TimedOut;
    // Linux socket timeouts surface as WouldBlock, so walk the source
    // chain for either kind (bounded timeout classification).
    let mut source = err.source();
    while let Some(cause) = source {
        if let Some(io) = cause.downcast_ref::<std::io::Error>() {
            if matches!(
                io.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) {
                return MailError::timeout(format!("smtp {phase}: timed out"));
            }
        }
        source = cause.source();
    }
    if err.is_timeout() {
        return MailError::timeout(format!("smtp {phase}: timed out"));
    }
    if let Some(code) = err.status() {
        let severity = code.severity;
        if matches!(
            severity,
            lettre::transport::smtp::response::Severity::PermanentNegativeCompletion
        ) {
            if phase == "auth" {
                return MailError::authorization(format!("smtp {phase}: authentication rejected"));
            }
            return MailError::policy(format!(
                "smtp {phase}: rejected {}",
                code.severity.to_string() + &code.category.to_string() + &code.detail.to_string()
            ));
        }
        return MailError::unavailable(format!("smtp {phase}: transient {severity}"));
    }
    let text = err.to_string();
    if err.is_tls()
        || text.contains("tls")
        || text.contains("certificate")
        || text.contains("handshake")
    {
        return MailError::authorization(format!("smtp {phase}: TLS failure"));
    }
    if err.is_response() {
        return MailError::external(format!("smtp {phase}: response error"));
    }
    MailError::unavailable(format!("smtp {phase}: {err}"))
}

fn parse_address(text: &str) -> Result<lettre::address::Address, MailError> {
    text.parse::<lettre::address::Address>()
        .map_err(|e| MailError::validation(format!("invalid address {text:?}: {e}")))
}

/// Real SMTP submission transport over the `lettre` crate, driven
/// phase-exactly for honest ambiguous-outcome classification.
pub struct RealSmtpTransport {
    host: String,
    port: u16,
    username: String,
    password: String,
    authority: SmtpAuthority,
    timeout: Duration,
    tls: SmtpTls,
    hello_name: String,
}

impl RealSmtpTransport {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        authority: SmtpAuthority,
        tls: SmtpTls,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            authority,
            timeout: Duration::from_secs(4),
            tls,
            hello_name: "nexus.local".to_string(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn tls_parameters(
        &self,
    ) -> Result<Option<lettre::transport::smtp::client::TlsParameters>, MailError> {
        use lettre::transport::smtp::client::TlsParameters;
        match &self.tls {
            SmtpTls::Plain => Ok(None),
            SmtpTls::Tls => {
                let params = TlsParameters::new(self.host.clone())
                    .map_err(|e| MailError::external(format!("smtp tls params: {e}")))?;
                Ok(Some(params))
            }
            SmtpTls::TlsWithCa(pem) => {
                let cert = lettre::transport::smtp::client::Certificate::from_pem(pem)
                    .map_err(|e| MailError::external(format!("smtp CA parse: {e}")))?;
                let params = TlsParameters::builder(self.host.clone())
                    .add_root_certificate(cert)
                    .build()
                    .map_err(|e| MailError::external(format!("smtp tls build: {e}")))?;
                Ok(Some(params))
            }
        }
    }

    fn connect(&self) -> Result<lettre::transport::smtp::client::SmtpConnection, MailError> {
        use lettre::transport::smtp::client::SmtpConnection;
        use lettre::transport::smtp::extension::ClientId;
        let tls = self.tls_parameters()?;
        SmtpConnection::connect(
            (self.host.as_str(), self.port),
            Some(self.timeout),
            &ClientId::Domain(self.hello_name.clone()),
            tls.as_ref(),
            None,
        )
        .map_err(|e| classify_lettre(&e, "connect"))
    }
}

impl SmtpTransport for RealSmtpTransport {
    fn authority(&self) -> SmtpAuthority {
        self.authority
    }

    fn submit(
        &self,
        envelope_from: &str,
        to: &[String],
        message: &[u8],
        message_id: &str,
    ) -> Result<SmtpOutcome, MailError> {
        use lettre::transport::smtp::commands;

        if !self.authority.allows_submit() {
            return Err(MailError::authorization(
                "smtp token scope does not allow submit",
            ));
        }
        if envelope_from.contains(['\r', '\n']) || to.iter().any(|t| t.contains(['\r', '\n'])) {
            return Err(MailError::validation("smtp envelope contains CR/LF"));
        }
        if message_id.contains(['\r', '\n']) {
            return Err(MailError::validation("message-id contains CR/LF"));
        }

        let mut conn = self.connect()?;

        // AUTH (submission credentials; separate from IMAP authority).
        let credentials = lettre::transport::smtp::authentication::Credentials::new(
            self.username.clone(),
            self.password.clone(),
        );
        conn.auth(
            &[lettre::transport::smtp::authentication::Mechanism::Plain],
            &credentials,
        )
        .map_err(|e| classify_lettre(&e, "auth"))?;

        // MAIL FROM
        let mail_cmd = commands::Mail::new(Some(parse_address(envelope_from)?), vec![]);
        let mail_resp = conn
            .command(mail_cmd)
            .map_err(|e| classify_lettre(&e, "mail from"))?;
        if !mail_resp.is_positive() {
            return Err(MailError::policy(format!(
                "smtp MAIL FROM rejected: {}",
                mail_resp.code().severity
            )));
        }

        // RCPT TO (one per recipient)
        for recipient in to {
            let rcpt_cmd = commands::Rcpt::new(parse_address(recipient)?, vec![]);
            let rcpt_resp = conn
                .command(rcpt_cmd)
                .map_err(|e| classify_lettre(&e, "rcpt to"))?;
            if !rcpt_resp.is_positive() {
                return Err(MailError::policy(format!(
                    "smtp RCPT TO rejected: {}",
                    rcpt_resp.code().severity
                )));
            }
        }

        // DATA
        let data_resp = conn
            .command(commands::Data)
            .map_err(|e| classify_lettre(&e, "data"))?;
        if !data_resp.is_positive() {
            return Err(MailError::external(format!(
                "smtp DATA rejected: {}",
                data_resp.code().severity
            )));
        }

        // Message body + terminator. If the final response is lost
        // here, the provider MAY have accepted the submission: the
        // outcome is Ambiguous, never blindly retried (directive M).
        match conn.message(message) {
            Ok(final_resp) => {
                let _ = final_resp;
                conn.quit().ok();
                Ok(SmtpOutcome::Accepted(message_id.to_string()))
            }
            Err(e) => {
                conn.abort();
                match &e {
                    e if e.is_timeout() => Ok(SmtpOutcome::Ambiguous),
                    e if e.is_transient() => Ok(SmtpOutcome::Ambiguous),
                    e if e.is_response() => Err(classify_lettre(e, "message")),
                    _ => Ok(SmtpOutcome::Ambiguous),
                }
            }
        }
    }

    fn health_check(&self) -> Result<(), MailError> {
        self.connect()?
            .quit()
            .map_err(|e| classify_lettre(&e, "quit"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_email::MailErrorCode;

    #[test]
    fn ep026_unit_m4_authority_matrix() {
        // Directive C: read/send/modify authorities never imply each
        // other.
        assert!(ImapAuthority::ReadOnly.allows_read());
        assert!(!ImapAuthority::ReadOnly.allows_modify());
        assert!(ImapAuthority::Modify.allows_read());
        assert!(ImapAuthority::Modify.allows_modify());
        assert!(ImapAuthority::Full.allows_read());
        assert!(ImapAuthority::Full.allows_modify());
        assert!(SmtpAuthority::Submit.allows_submit());
    }

    #[test]
    fn ep026_unit_m4_state_from_flags() {
        assert_eq!(
            state_from_flags(&[Flag::Deleted], "INBOX"),
            MailState::Deleted
        );
        assert_eq!(
            state_from_flags(&[Flag::Custom("NexusArchive".into())], "INBOX"),
            MailState::Archived
        );
        assert_eq!(
            state_from_flags(&[Flag::Seen], "INBOX"),
            MailState::Delivered
        );
        assert_eq!(state_from_flags(&[Flag::Seen], "Trash"), MailState::Deleted);
    }

    #[test]
    fn ep026_unit_m4_ambiguous_outcome_is_not_accepted() {
        // Ambiguous is distinct from Accepted: no success state may be
        // derived from it.
        let outcome = SmtpOutcome::Ambiguous;
        assert_ne!(outcome, SmtpOutcome::Accepted("x".to_string()));
    }

    #[test]
    fn ep026_unit_m4_smtp_envelope_crlf_rejected() {
        // Directive Q: CR/LF in the SMTP envelope rejects before any
        // provider mutation (unit-level; the production send surface
        // also rejects via build_mime).
        let transport = RealSmtpTransport::new(
            "127.0.0.1",
            1,
            "u",
            "p",
            SmtpAuthority::Submit,
            SmtpTls::Plain,
        );
        let err = transport
            .submit(
                "a@example.com",
                &["b@example.com\r\nX-Evil: 1".into()],
                b"data",
                "mid@nexus.local",
            )
            .expect_err("CRLF recipient must reject");
        assert_eq!(err.code, MailErrorCode::Validation);
    }

    #[test]
    fn ep026_unit_m4_bodystructure_lists_only_attachment_parts() {
        // AUD-010: BODYSTRUCTURE walking must report only parts with an
        // attachment disposition (or filename); inline text parts are
        // never fabricated as attachments.
        use imap_proto::types::{
            BodyContentCommon, BodyContentSinglePart, BodyStructure, ContentType,
        };
        let inline = BodyStructure::Text {
            common: BodyContentCommon {
                ty: ContentType {
                    ty: "text".into(),
                    subtype: "plain".into(),
                    params: None,
                },
                disposition: None,
                language: None,
                location: None,
            },
            other: BodyContentSinglePart {
                id: None,
                md5: None,
                description: None,
                transfer_encoding: imap_proto::types::ContentEncoding::SevenBit,
                octets: 12,
            },
            lines: 1,
            extension: None,
        };
        let attachment = BodyStructure::Basic {
            common: BodyContentCommon {
                ty: ContentType {
                    ty: "application".into(),
                    subtype: "pdf".into(),
                    params: None,
                },
                disposition: Some(imap_proto::types::ContentDisposition {
                    ty: "attachment".into(),
                    params: Some(vec![("filename".into(), "scan.pdf".into())]),
                }),
                language: None,
                location: None,
            },
            other: BodyContentSinglePart {
                id: None,
                md5: None,
                description: None,
                transfer_encoding: imap_proto::types::ContentEncoding::Base64,
                octets: 2048,
            },
            extension: None,
        };
        let multipart = BodyStructure::Multipart {
            common: BodyContentCommon {
                ty: ContentType {
                    ty: "multipart".into(),
                    subtype: "mixed".into(),
                    params: None,
                },
                disposition: None,
                language: None,
                location: None,
            },
            bodies: vec![inline, attachment],
            extension: None,
        };
        let mut out = Vec::new();
        walk_bodystructure(&multipart, "", &mut out);
        assert_eq!(out.len(), 1, "only the attachment part counts");
        assert_eq!(out[0].filename, "scan.pdf");
        assert_eq!(out[0].mime_type, "application/pdf");
        assert_eq!(out[0].size_bytes, 2048);
        assert_eq!(out[0].part_number, "2");
    }
}
