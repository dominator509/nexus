//! EP-026 Gmail transport (M2): real HTTP transport against the
//! DOCUMENTED Gmail REST API surface.
//!
//! Gmail is the external provider; Nexus orchestrates the provider API
//! and normalizes provider payloads at this infrastructure boundary -
//! free-form Gmail JSON never becomes a domain contract. Read and send
//! scopes are separate (acceptance obligation 2): the transport
//! carries a token whose granted scope is declared by the caller, and
//! a SEND operation refuses to run on a READ-only token.
//!
//! Documented surface used (Google Gmail API v1):
//! - GET  /gmail/v1/users/me/messages?q=...            list messages
//! - GET  /gmail/v1/users/me/messages/{id}             fetch message
//! - GET  /gmail/v1/users/me/messages/{id}/attachments/{attachmentId}
//! - POST /gmail/v1/users/me/drafts                    create draft
//! - POST /gmail/v1/users/me/messages/send             send raw message
//! - POST /gmail/v1/users/me/messages/{id}/modify      add/remove labels
//! - POST /gmail/v1/users/me/messages/{id}/trash       trash
//! - GET  /gmail/v1/users/me/messages/{id}             readback (verify)
//!
//! HTTP status mapping follows SPEC-006: 401/403 -> Authorization,
//! 404 -> NotFound, 429 -> RateLimit, 500/502/503 -> Unavailable,
//! silent peer -> Timeout, refused -> Unavailable, malformed JSON ->
//! External (fail closed). The Authorization bearer token is used
//! ONLY for the header and never appears in errors or telemetry.

use std::time::Duration;

use nexus_email::{MailError, MailErrorCode};

/// Canonical Gmail message envelope (documented Gmail API shape,
/// normalized at the boundary).
///
/// Matches the REAL Gmail wire format: `historyId` and `internalDate`
/// are returned as JSON strings (int64/uint64 serialized as strings),
/// and message headers live in `payload.headers` as `{name, value}`
/// pairs - not as flat top-level fields. Accessors resolve From/To/
/// Subject from the payload headers, so a real Gmail response
/// deserializes and normalizes without loss.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GmailMessage {
    pub id: String,
    #[serde(rename = "threadId")]
    pub thread_id: String,
    #[serde(rename = "labelIds")]
    pub label_ids: Vec<String>,
    pub snippet: String,
    #[serde(rename = "historyId")]
    pub history_id: String,
    #[serde(rename = "internalDate")]
    pub internal_date_ms: String,
    pub payload: GmailPayload,
    /// base64url-encoded RFC822 body bytes (read scope only).
    #[serde(default)]
    pub raw: Option<String>,
}

impl GmailMessage {
    /// Case-insensitive header lookup from payload.headers.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.payload
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.as_str())
    }

    pub fn from_header(&self) -> Option<String> {
        self.header("From").map(str::to_string)
    }

    pub fn to_headers(&self) -> Vec<String> {
        self.header("To")
            .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default()
    }

    pub fn subject(&self) -> Option<String> {
        self.header("Subject").map(str::to_string)
    }
}

/// Gmail payload envelope: the real wire format carries headers as a
/// name/value list here, never as flat message fields, and attachment
/// parts as a recursive part tree.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct GmailPayload {
    #[serde(default)]
    pub headers: Vec<GmailHeader>,
    #[serde(default)]
    pub parts: Vec<GmailPart>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GmailHeader {
    pub name: String,
    pub value: String,
}

/// One MIME part of a Gmail message payload. Attachment parts carry a
/// body with an attachmentId; inline/text parts have no attachmentId.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct GmailPart {
    #[serde(default, rename = "partId")]
    pub part_id: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default, rename = "mimeType")]
    pub mime_type: String,
    #[serde(default)]
    pub body: Option<GmailPartBody>,
    #[serde(default)]
    pub parts: Vec<GmailPart>,
}

impl GmailPart {
    /// Walk this part tree collecting every part that carries a real
    /// attachmentId (attachment metadata). Inline/text parts without
    /// an attachmentId are skipped - never fabricated as attachments.
    pub fn collect_attachments(&self, out: &mut Vec<GmailAttachmentMeta>) {
        if let Some(body) = &self.body {
            if let Some(attachment_id) = &body.attachment_id {
                out.push(GmailAttachmentMeta {
                    attachment_id: attachment_id.clone(),
                    size_bytes: body.size,
                    filename: self.filename.clone(),
                    mime_type: self.mime_type.clone(),
                });
            }
        }
        for child in &self.parts {
            child.collect_attachments(out);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct GmailPartBody {
    #[serde(default, rename = "attachmentId")]
    pub attachment_id: Option<String>,
    #[serde(default)]
    pub size: u64,
}

/// Canonical Gmail draft envelope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GmailDraft {
    pub id: String,
    pub message_id: Option<String>,
}

/// Canonical Gmail attachment metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GmailAttachmentMeta {
    pub attachment_id: String,
    pub size_bytes: u64,
    pub filename: String,
    pub mime_type: String,
}

/// OAuth scope granted to the bearer token (read and send are
/// separate; a token never widens its own scope).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmailScope {
    ReadOnly,
    Send,
    Full,
}

impl GmailScope {
    pub const fn allows_read(self) -> bool {
        matches!(self, Self::ReadOnly | Self::Full)
    }

    pub const fn allows_send(self) -> bool {
        matches!(self, Self::Send | Self::Full)
    }
}

/// Real Gmail REST transport. All methods are real HTTP operations
/// against the documented Gmail API; no in-memory substitute exists in
/// production code. Test doubles live under TESTING.md's test zones.
pub trait GmailTransport {
    fn list_messages(&self, query: &str) -> Result<Vec<GmailMessage>, MailError> {
        let _ = query;
        Err(MailError::unavailable(
            "gmail transport has no implementation bound",
        ))
    }

    fn fetch_message(&self, id: &str) -> Result<GmailMessage, MailError> {
        let _ = id;
        Err(MailError::unavailable(
            "gmail transport has no implementation bound",
        ))
    }

    fn fetch_attachment_meta(
        &self,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<GmailAttachmentMeta, MailError> {
        let _ = (message_id, attachment_id);
        Err(MailError::unavailable(
            "gmail transport has no implementation bound",
        ))
    }

    /// Fetch a message with format=full and return its attachment
    /// metadata (parts with an attachmentId). A message with no
    /// attachments yields an empty list - never fabricated.
    fn fetch_attachments(&self, message_id: &str) -> Result<Vec<GmailAttachmentMeta>, MailError> {
        let _ = message_id;
        Err(MailError::unavailable(
            "gmail transport has no implementation bound",
        ))
    }

    fn create_draft(&self, raw: &str) -> Result<GmailDraft, MailError> {
        let _ = raw;
        Err(MailError::unavailable(
            "gmail transport has no implementation bound",
        ))
    }

    fn send_raw(&self, raw: &str) -> Result<String, MailError> {
        let _ = raw;
        Err(MailError::unavailable(
            "gmail transport has no implementation bound",
        ))
    }

    /// Send an EXISTING draft via POST /gmail/v1/users/me/drafts/send.
    /// The draft id is the handle; Gmail resolves the stored
    /// recipients/document from the draft server-side. The draft id is
    /// NEVER a recipient address.
    fn send_draft(&self, draft_id: &str) -> Result<String, MailError> {
        let _ = draft_id;
        Err(MailError::unavailable(
            "gmail transport has no implementation bound",
        ))
    }

    fn modify_labels(
        &self,
        message_id: &str,
        add: &[String],
        remove: &[String],
    ) -> Result<(), MailError> {
        let _ = (message_id, add, remove);
        Err(MailError::unavailable(
            "gmail transport has no implementation bound",
        ))
    }

    fn trash(&self, message_id: &str) -> Result<(), MailError> {
        let _ = message_id;
        Err(MailError::unavailable(
            "gmail transport has no implementation bound",
        ))
    }
}

/// Real blocking HTTP Gmail transport.
///
/// `scope` is the granted OAuth scope (separate read/send). The bearer
/// token is held only for the Authorization header; it is never
/// serialized into errors or audit entries.
pub struct HttpGmailTransport {
    base_url: String,
    token: String,
    scope: GmailScope,
    client: reqwest::blocking::Client,
}

impl HttpGmailTransport {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>, scope: GmailScope) -> Self {
        let timeout = Duration::from_secs(10);
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client builds");
        Self {
            base_url: base_url.into(),
            token: token.into(),
            scope,
            client,
        }
    }

    fn authorize(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        request.bearer_auth(&self.token)
    }

    fn map_status(status: reqwest::StatusCode) -> MailErrorCode {
        match status.as_u16() {
            401 | 403 => MailErrorCode::Authorization,
            404 => MailErrorCode::NotFound,
            409 => MailErrorCode::Conflict,
            429 => MailErrorCode::RateLimit,
            500 | 502 | 503 => MailErrorCode::Unavailable,
            _ => MailErrorCode::External,
        }
    }

    fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, MailError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.get(&url).query(query))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    MailError::timeout("gmail GET timed out")
                } else if e.is_connect() {
                    MailError::unavailable("gmail endpoint refused connection")
                } else {
                    MailError::external(format!("gmail GET transport error: {e}"))
                }
            })?;
        if !response.status().is_success() {
            return Err(MailError::new(
                Self::map_status(response.status()),
                format!("gmail GET {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        response.json().map_err(|e| {
            MailError::new(
                MailErrorCode::External,
                format!("gmail GET {path} malformed JSON: {e}"),
                None,
                None,
            )
        })
    }

    fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, MailError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.post(&url).json(body))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    MailError::timeout("gmail POST timed out")
                } else if e.is_connect() {
                    MailError::unavailable("gmail endpoint refused connection")
                } else {
                    MailError::external(format!("gmail POST transport error: {e}"))
                }
            })?;
        if !response.status().is_success() {
            return Err(MailError::new(
                Self::map_status(response.status()),
                format!("gmail POST {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        response.json().map_err(|e| {
            MailError::new(
                MailErrorCode::External,
                format!("gmail POST {path} malformed JSON: {e}"),
                None,
                None,
            )
        })
    }

    fn post_empty(&self, path: &str, body: &serde_json::Value) -> Result<(), MailError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.post(&url).json(body))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    MailError::timeout("gmail POST timed out")
                } else if e.is_connect() {
                    MailError::unavailable("gmail endpoint refused connection")
                } else {
                    MailError::external(format!("gmail POST transport error: {e}"))
                }
            })?;
        if !response.status().is_success() {
            return Err(MailError::new(
                Self::map_status(response.status()),
                format!("gmail POST {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        Ok(())
    }
}

impl GmailTransport for HttpGmailTransport {
    fn list_messages(&self, query: &str) -> Result<Vec<GmailMessage>, MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "gmail token scope does not allow read",
            ));
        }
        let list: serde_json::Value = self.get_json(
            "/gmail/v1/users/me/messages",
            &[("q", query), ("maxResults", "50")],
        )?;
        let messages = list
            .get("messages")
            .and_then(|m| m.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        let id = m.get("id")?.as_str()?.to_string();
                        let thread_id = m
                            .get("threadId")
                            .and_then(|t| t.as_str())
                            .unwrap_or(&id)
                            .to_string();
                        Some(GmailMessage {
                            id,
                            thread_id,
                            label_ids: vec![],
                            snippet: String::new(),
                            history_id: String::new(),
                            internal_date_ms: String::new(),
                            payload: GmailPayload::default(),
                            raw: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(messages)
    }

    fn fetch_message(&self, id: &str) -> Result<GmailMessage, MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "gmail token scope does not allow read",
            ));
        }
        self.get_json(
            &format!("/gmail/v1/users/me/messages/{id}"),
            &[("format", "metadata")],
        )
    }

    fn fetch_attachment_meta(
        &self,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<GmailAttachmentMeta, MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "gmail token scope does not allow read",
            ));
        }
        self.get_json(
            &format!("/gmail/v1/users/me/messages/{message_id}/attachments/{attachment_id}"),
            &[],
        )
    }

    fn fetch_attachments(&self, message_id: &str) -> Result<Vec<GmailAttachmentMeta>, MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "gmail token scope does not allow read",
            ));
        }
        // format=full returns payload.parts with attachmentId/filename/
        // mimeType/size - the REAL Gmail attachment enumeration surface.
        let msg: GmailMessage = self.get_json(
            &format!("/gmail/v1/users/me/messages/{message_id}"),
            &[("format", "full")],
        )?;
        let mut out = Vec::new();
        for part in &msg.payload.parts {
            part.collect_attachments(&mut out);
        }
        Ok(out)
    }

    fn create_draft(&self, raw: &str) -> Result<GmailDraft, MailError> {
        if !self.scope.allows_send() {
            return Err(MailError::authorization(
                "gmail token scope does not allow draft/send",
            ));
        }
        let body = serde_json::json!({ "message": { "raw": raw } });
        self.post_json("/gmail/v1/users/me/drafts", &body)
    }

    fn send_raw(&self, raw: &str) -> Result<String, MailError> {
        if !self.scope.allows_send() {
            return Err(MailError::authorization(
                "gmail token scope does not allow send",
            ));
        }
        let body = serde_json::json!({ "raw": raw });
        let sent: serde_json::Value = self.post_json("/gmail/v1/users/me/messages/send", &body)?;
        sent.get("id")
            .and_then(|id| id.as_str())
            .map(str::to_string)
            .ok_or_else(|| MailError::external("gmail send response missing id"))
    }

    fn send_draft(&self, draft_id: &str) -> Result<String, MailError> {
        if !self.scope.allows_send() {
            return Err(MailError::authorization(
                "gmail token scope does not allow send",
            ));
        }
        // Documented draft-send surface: POST /gmail/v1/users/me/drafts/send
        // with the draft id. Gmail sends the STORED draft (its To/Cc/Bcc
        // resolve server-side); the id is never placed in a To header.
        let body = serde_json::json!({ "id": draft_id });
        let sent: serde_json::Value = self.post_json("/gmail/v1/users/me/drafts/send", &body)?;
        sent.get("id")
            .and_then(|id| id.as_str())
            .map(str::to_string)
            .ok_or_else(|| MailError::external("gmail draft send response missing id"))
    }

    fn modify_labels(
        &self,
        message_id: &str,
        add: &[String],
        remove: &[String],
    ) -> Result<(), MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "gmail token scope does not allow label modification",
            ));
        }
        let body = serde_json::json!({ "addLabelIds": add, "removeLabelIds": remove });
        self.post_empty(
            &format!("/gmail/v1/users/me/messages/{message_id}/modify"),
            &body,
        )
    }

    fn trash(&self, message_id: &str) -> Result<(), MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "gmail token scope does not allow trash",
            ));
        }
        let body = serde_json::json!({});
        self.post_empty(
            &format!("/gmail/v1/users/me/messages/{message_id}/trash"),
            &body,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep026_unit_gmail_scope_separation() {
        // Acceptance obligation 2: read and send scopes are separate.
        assert!(GmailScope::ReadOnly.allows_read());
        assert!(!GmailScope::ReadOnly.allows_send());
        assert!(!GmailScope::Send.allows_read());
        assert!(GmailScope::Send.allows_send());
        assert!(GmailScope::Full.allows_read());
        assert!(GmailScope::Full.allows_send());
    }

    #[test]
    fn ep026_unit_gmail_status_mapping() {
        assert_eq!(
            HttpGmailTransport::map_status(reqwest::StatusCode::UNAUTHORIZED),
            MailErrorCode::Authorization
        );
        assert_eq!(
            HttpGmailTransport::map_status(reqwest::StatusCode::FORBIDDEN),
            MailErrorCode::Authorization
        );
        assert_eq!(
            HttpGmailTransport::map_status(reqwest::StatusCode::NOT_FOUND),
            MailErrorCode::NotFound
        );
        assert_eq!(
            HttpGmailTransport::map_status(reqwest::StatusCode::TOO_MANY_REQUESTS),
            MailErrorCode::RateLimit
        );
        assert_eq!(
            HttpGmailTransport::map_status(reqwest::StatusCode::SERVICE_UNAVAILABLE),
            MailErrorCode::Unavailable
        );
        assert_eq!(
            HttpGmailTransport::map_status(reqwest::StatusCode::BAD_GATEWAY),
            MailErrorCode::Unavailable
        );
        assert_eq!(
            HttpGmailTransport::map_status(reqwest::StatusCode::INTERNAL_SERVER_ERROR),
            MailErrorCode::Unavailable
        );
        assert_eq!(
            HttpGmailTransport::map_status(reqwest::StatusCode::IM_A_TEAPOT),
            MailErrorCode::External
        );
    }

    #[test]
    fn ep026_unit_gmail_message_serde() {
        // REAL Gmail wire format: historyId/internalDate are JSON
        // strings, headers live in payload.headers as name/value pairs.
        let json = r#"{
            "id": "msg-1",
            "threadId": "thread-1",
            "labelIds": ["INBOX", "UNREAD"],
            "snippet": "Hello",
            "historyId": "1234567890123",
            "internalDate": "1780000000000",
            "payload": {
                "headers": [
                    {"name": "From", "value": "Alice <alice@example.com>"},
                    {"name": "To", "value": "bob@example.com, carol@example.com"},
                    {"name": "Subject", "value": "Hello"}
                ]
            }
        }"#;
        let msg: GmailMessage = serde_json::from_str(json).expect("parse");
        assert_eq!(msg.id, "msg-1");
        assert_eq!(msg.thread_id, "thread-1");
        assert_eq!(msg.history_id, "1234567890123");
        assert_eq!(msg.internal_date_ms, "1780000000000");
        assert!(msg.label_ids.contains(&"INBOX".to_string()));
        // Headers resolve from payload.headers, not flat fields.
        assert_eq!(
            msg.from_header().as_deref(),
            Some("Alice <alice@example.com>")
        );
        assert_eq!(
            msg.to_headers(),
            vec!["bob@example.com", "carol@example.com"]
        );
        assert_eq!(msg.subject().as_deref(), Some("Hello"));
    }
}
