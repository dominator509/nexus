//! EP-026 Microsoft Graph transport (M3): real HTTP transport against
//! the DOCUMENTED Microsoft Graph mail REST surface.
//!
//! Microsoft Graph is the external provider; Nexus orchestrates the
//! provider API and normalizes provider payloads at this
//! infrastructure boundary - free-form Graph JSON never becomes a
//! domain contract. Read and send scopes are separate (acceptance
//! obligation 2): the transport carries a token whose granted scope is
//! declared by the caller, and a SEND operation refuses to run on a
//! READ-only token. Message update/delete additionally require a
//! Mail.ReadWrite-class authority (allows_modify), never plain read.
//!
//! Documented surface used (Microsoft Graph v1.0 mail):
//! - GET  /v1.0/me/messages?$top=...            list messages
//! - GET  /v1.0/me/messages/{id}                fetch message
//! - GET  /v1.0/me/messages/{id}/attachments/{id}
//! - POST /v1.0/me/messages                     create draft (201 JSON)
//! - POST /v1.0/me/sendMail                     send message (202, no body)
//! - POST /v1.0/me/messages/{id}/send           send existing draft (202, no body)
//! - POST /v1.0/me/messages/{id}/reply          reply (202, no body)
//! - POST /v1.0/me/messages/{id}/forward        forward (202, no body)
//! - PATCH /v1.0/me/messages/{id}               update (200 + updated message)
//! - DELETE /v1.0/me/messages/{id}              delete (204, no body)
//!
//! Documented response semantics honored:
//! - sendMail / draft-send / reply / forward return 202 Accepted with
//!   NO response body: a status-only helper is used, JSON is never
//!   parsed from a 202, and 202 proves SUBMISSION (SENT), never
//!   delivery (DELIVERED).
//! - update returns 200 OK with the updated message object (structured
//!   parse).
//! - delete returns 204 No Content with no body (status-only).
//!
//! HTTP status mapping follows SPEC-006: 401/403 -> Authorization,
//! 404 -> NotFound, 409 -> Conflict, 429 -> RateLimit, 500/502/503/504
//! -> Unavailable, silent peer -> Timeout, refused -> Unavailable,
//! malformed JSON -> External (fail closed). The OAuth bearer token is
//! used ONLY for the Authorization header and never appears in errors
//! or telemetry.

use std::time::Duration;

use nexus_email::{MailError, MailErrorCode};

/// Canonical Graph email address object (documented Graph shape:
/// `{"address": "..."}` inside recipient objects).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphEmailAddress {
    pub address: String,
}

/// Canonical Graph recipient object (documented Graph shape:
/// `{"emailAddress": {"address": "..."}}`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphRecipient {
    #[serde(rename = "emailAddress")]
    pub email_address: GraphEmailAddress,
}

/// Canonical Graph message envelope (documented Graph API shape,
/// normalized at the boundary). `from` and `toRecipients` are the REAL
/// Graph object shapes; a plain string payload is not a Graph message
/// and fails closed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphMessage {
    pub id: String,
    #[serde(default)]
    pub subject: Option<String>,
    /// From recipient (advisory; never trusted as identity).
    #[serde(default)]
    pub from: Option<GraphRecipient>,
    #[serde(default, rename = "toRecipients")]
    pub to_recipients: Vec<GraphRecipient>,
    #[serde(default, rename = "bodyPreview")]
    pub body_preview: Option<String>,
    #[serde(default, rename = "isRead")]
    pub is_read: bool,
    #[serde(default, rename = "hasAttachments")]
    pub has_attachments: bool,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default, rename = "folderId")]
    pub folder_id: Option<String>,
}

impl GraphMessage {
    /// The From address string, when present (advisory display only).
    pub fn from_address(&self) -> Option<&str> {
        self.from.as_ref().map(|r| r.email_address.address.as_str())
    }
}

/// Canonical Graph draft envelope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphDraft {
    pub id: String,
}

/// Canonical Graph attachment metadata.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GraphAttachmentMeta {
    pub id: String,
    pub size_bytes: u64,
    pub name: String,
    pub content_type: String,
}

/// OAuth scope granted to the bearer token.
///
/// Graph scopes are SEPARATE authorities (acceptance obligation 2 and
/// owner directive F):
/// - ReadOnly  = Mail.Read           (read only)
/// - ReadWrite = Mail.ReadWrite      (read + update/delete)
/// - Send      = Mail.Send           (sendMail/reply/forward)
/// - Full      = read + modify + send
///
/// A token never widens its own scope: read authority never implies
/// send, send authority never implies read, and update/delete require
/// a ReadWrite-class authority, never plain read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphScope {
    ReadOnly,
    ReadWrite,
    Send,
    Full,
}

impl GraphScope {
    pub const fn allows_read(self) -> bool {
        matches!(self, Self::ReadOnly | Self::ReadWrite | Self::Full)
    }

    /// Update/delete (PATCH/DELETE) require Mail.ReadWrite-class
    /// authority; plain read (Mail.Read) is not enough.
    pub const fn allows_modify(self) -> bool {
        matches!(self, Self::ReadWrite | Self::Full)
    }

    pub const fn allows_send(self) -> bool {
        matches!(self, Self::Send | Self::Full)
    }
}

/// Real Microsoft Graph REST transport. All methods are real HTTP
/// operations against the documented Graph API; no in-memory
/// substitute exists in production code. Test doubles live under
/// TESTING.md's test zones. The trait is `Send + Sync` so a shared
/// adapter can be driven concurrently (in-flight idempotency).
pub trait GraphTransport: Send + Sync {
    fn list_messages(&self, top: u32) -> Result<Vec<GraphMessage>, MailError> {
        let _ = top;
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    fn fetch_message(&self, id: &str) -> Result<GraphMessage, MailError> {
        let _ = id;
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    fn fetch_attachment_meta(
        &self,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<GraphAttachmentMeta, MailError> {
        let _ = (message_id, attachment_id);
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    /// List attachment metadata via GET /v1.0/me/messages/{id}/attachments.
    /// A message with no attachments yields an empty list - never
    /// fabricated.
    fn fetch_attachments(&self, message_id: &str) -> Result<Vec<GraphAttachmentMeta>, MailError> {
        let _ = message_id;
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    fn create_draft(
        &self,
        subject: &str,
        to: &[String],
        body: &str,
    ) -> Result<GraphDraft, MailError> {
        let _ = (subject, to, body);
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    /// Send an inline message via `POST /me/sendMail`.
    ///
    /// Microsoft returns 202 Accepted with NO body. 202 proves the
    /// request was accepted for processing (SUBMITTED / SENT), never
    /// delivery. No JSON is parsed from the 202.
    fn send_mail(&self, subject: &str, to: &[String], body: &str) -> Result<(), MailError> {
        let _ = (subject, to, body);
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    /// Send an existing draft via `POST /me/messages/{id}/send`.
    ///
    /// Microsoft returns 202 Accepted with NO body; the draft id is
    /// the message handle (Graph moves the draft to Sent Items with
    /// the same id). 202 proves submission, never delivery. The
    /// returned id is the caller-owned draft id - never a fabricated
    /// provider id.
    fn send_draft(&self, draft_id: &str) -> Result<String, MailError> {
        let _ = draft_id;
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    /// Reply action via `POST /me/messages/{id}/reply`.
    ///
    /// 202 Accepted, no body. The original message id is returned as
    /// the correlation handle (the reply id is generated server-side
    /// and not observable from the 202).
    fn reply(&self, original_id: &str, body: &str) -> Result<String, MailError> {
        let _ = (original_id, body);
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    /// Forward action via `POST /me/messages/{id}/forward`.
    ///
    /// 202 Accepted, no body. Documented payload rule: `toRecipients`
    /// is TOP-LEVEL (never `message.toRecipients`), and content is
    /// carried as `comment` (never both `comment` and
    /// `message.body`).
    fn forward(&self, original_id: &str, to: &[String], body: &str) -> Result<String, MailError> {
        let _ = (original_id, to, body);
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    /// Update message via `PATCH /me/messages/{id}`.
    ///
    /// Microsoft returns 200 OK with the updated message object
    /// (structured response). The updated envelope is returned so the
    /// adapter can perform exact-target verification.
    fn update_message(
        &self,
        message_id: &str,
        update: &serde_json::Value,
    ) -> Result<GraphMessage, MailError> {
        let _ = (message_id, update);
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }

    /// Delete message via `DELETE /me/messages/{id}`.
    ///
    /// Microsoft returns 204 No Content with no body (status-only;
    /// JSON is never parsed from a 204).
    fn delete_message(&self, message_id: &str) -> Result<(), MailError> {
        let _ = message_id;
        Err(MailError::unavailable(
            "graph transport has no implementation bound",
        ))
    }
}

/// Real blocking HTTP Microsoft Graph transport.
///
/// `scope` is the granted OAuth scope (separate read/send/read-write
/// authorities). The bearer token is held only for the Authorization
/// header; it is never serialized into errors or audit entries.
pub struct HttpGraphTransport {
    base_url: String,
    token: String,
    scope: GraphScope,
    client: reqwest::blocking::Client,
}

impl HttpGraphTransport {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>, scope: GraphScope) -> Self {
        Self::with_timeout(base_url, token, scope, Duration::from_secs(10))
    }

    pub fn with_timeout(
        base_url: impl Into<String>,
        token: impl Into<String>,
        scope: GraphScope,
        timeout: Duration,
    ) -> Self {
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
            500 | 502 | 503 | 504 => MailErrorCode::Unavailable,
            _ => MailErrorCode::External,
        }
    }

    fn transport_error(verb: &str, err: reqwest::Error) -> MailError {
        if err.is_timeout() {
            MailError::timeout(format!("graph {verb} timed out"))
        } else if err.is_connect() {
            MailError::unavailable(format!("graph endpoint refused connection ({verb})"))
        } else {
            MailError::external(format!("graph {verb} transport error"))
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
            .map_err(|e| Self::transport_error("GET", e))?;
        if !response.status().is_success() {
            return Err(MailError::new(
                Self::map_status(response.status()),
                format!("graph GET {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        response.json().map_err(|_e| {
            MailError::new(
                MailErrorCode::External,
                format!("graph GET {path} malformed JSON"),
                None,
                None,
            )
        })
    }

    /// Structured POST: requires a JSON body in the response (e.g.
    /// create draft -> 201 + message object).
    fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, MailError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.post(&url).json(body))
            .send()
            .map_err(|e| Self::transport_error("POST", e))?;
        if !response.status().is_success() {
            return Err(MailError::new(
                Self::map_status(response.status()),
                format!("graph POST {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        response.json().map_err(|_e| {
            MailError::new(
                MailErrorCode::External,
                format!("graph POST {path} malformed JSON"),
                None,
                None,
            )
        })
    }

    /// Status-only POST (documented 202 + empty body action endpoints:
    /// sendMail, draft-send, reply, forward). Never parses JSON from
    /// the response; an empty body is the documented success shape.
    fn post_status_only(&self, path: &str, body: &serde_json::Value) -> Result<(), MailError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.post(&url).json(body))
            .send()
            .map_err(|e| Self::transport_error("POST", e))?;
        if !response.status().is_success() {
            return Err(MailError::new(
                Self::map_status(response.status()),
                format!("graph POST {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        Ok(())
    }

    /// Structured PATCH: returns the updated message object (200 +
    /// JSON).
    fn patch_json(&self, path: &str, body: &serde_json::Value) -> Result<GraphMessage, MailError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.patch(&url).json(body))
            .send()
            .map_err(|e| Self::transport_error("PATCH", e))?;
        if !response.status().is_success() {
            return Err(MailError::new(
                Self::map_status(response.status()),
                format!("graph PATCH {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        response.json().map_err(|_e| {
            MailError::new(
                MailErrorCode::External,
                format!("graph PATCH {path} malformed JSON"),
                None,
                None,
            )
        })
    }

    /// Status-only DELETE (documented 204 No Content + empty body).
    /// Never parses JSON from a 204.
    fn delete_status_only(&self, path: &str) -> Result<(), MailError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.delete(&url))
            .send()
            .map_err(|e| Self::transport_error("DELETE", e))?;
        if !response.status().is_success() {
            return Err(MailError::new(
                Self::map_status(response.status()),
                format!("graph DELETE {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        Ok(())
    }
}

impl GraphTransport for HttpGraphTransport {
    fn list_messages(&self, top: u32) -> Result<Vec<GraphMessage>, MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "graph token scope does not allow read",
            ));
        }
        let value: serde_json::Value =
            self.get_json("/v1.0/me/messages", &[("$top", &top.to_string())])?;
        let messages = value
            .get("value")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| serde_json::from_value::<GraphMessage>(m.clone()).ok())
                    .collect()
            })
            .unwrap_or_default();
        Ok(messages)
    }

    fn fetch_message(&self, id: &str) -> Result<GraphMessage, MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "graph token scope does not allow read",
            ));
        }
        self.get_json(&format!("/v1.0/me/messages/{id}"), &[])
    }

    fn fetch_attachment_meta(
        &self,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<GraphAttachmentMeta, MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "graph token scope does not allow read",
            ));
        }
        self.get_json(
            &format!("/v1.0/me/messages/{message_id}/attachments/{attachment_id}"),
            &[],
        )
    }

    fn fetch_attachments(&self, message_id: &str) -> Result<Vec<GraphAttachmentMeta>, MailError> {
        if !self.scope.allows_read() {
            return Err(MailError::authorization(
                "graph token scope does not allow read",
            ));
        }
        // Documented Graph surface: GET /me/messages/{id}/attachments
        // returns { "value": [ {id, name, contentType, size, ...} ] }.
        let list: serde_json::Value =
            self.get_json(&format!("/v1.0/me/messages/{message_id}/attachments"), &[])?;
        let mut out = Vec::new();
        if let Some(arr) = list.get("value").and_then(|v| v.as_array()) {
            for item in arr {
                let id = match item.get("id").and_then(|v| v.as_str()) {
                    Some(v) => v.to_string(),
                    None => continue,
                };
                out.push(GraphAttachmentMeta {
                    id,
                    size_bytes: item.get("size").and_then(|v| v.as_u64()).unwrap_or(0),
                    name: item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    content_type: item
                        .get("contentType")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                });
            }
        }
        Ok(out)
    }

    fn create_draft(
        &self,
        subject: &str,
        to: &[String],
        body: &str,
    ) -> Result<GraphDraft, MailError> {
        if !self.scope.allows_send() {
            return Err(MailError::authorization(
                "graph token scope does not allow draft/send",
            ));
        }
        let body = serde_json::json!({
            "subject": subject,
            "toRecipients": to.iter().map(|t| serde_json::json!({"emailAddress": {"address": t}})).collect::<Vec<_>>(),
            "body": {"contentType": "text", "content": body}
        });
        self.post_json("/v1.0/me/messages", &body)
    }

    fn send_mail(&self, subject: &str, to: &[String], body: &str) -> Result<(), MailError> {
        if !self.scope.allows_send() {
            return Err(MailError::authorization(
                "graph token scope does not allow send",
            ));
        }
        let payload = serde_json::json!({
            "message": {
                "subject": subject,
                "toRecipients": to.iter().map(|t| serde_json::json!({"emailAddress": {"address": t}})).collect::<Vec<_>>(),
                "body": {"contentType": "text", "content": body}
            },
            "saveToSentItems": true
        });
        self.post_status_only("/v1.0/me/sendMail", &payload)
    }

    fn send_draft(&self, draft_id: &str) -> Result<String, MailError> {
        if !self.scope.allows_send() {
            return Err(MailError::authorization(
                "graph token scope does not allow send",
            ));
        }
        let payload = serde_json::json!({});
        self.post_status_only(&format!("/v1.0/me/messages/{draft_id}/send"), &payload)?;
        Ok(draft_id.to_string())
    }

    fn reply(&self, original_id: &str, body: &str) -> Result<String, MailError> {
        if !self.scope.allows_send() {
            return Err(MailError::authorization(
                "graph token scope does not allow reply",
            ));
        }
        // Documented reply action shape: `comment` OR `message.body`
        // (mutually exclusive). The comment-only form is used.
        let payload = serde_json::json!({ "comment": body });
        self.post_status_only(&format!("/v1.0/me/messages/{original_id}/reply"), &payload)?;
        Ok(original_id.to_string())
    }

    fn forward(&self, original_id: &str, to: &[String], body: &str) -> Result<String, MailError> {
        if !self.scope.allows_send() {
            return Err(MailError::authorization(
                "graph token scope does not allow forward",
            ));
        }
        // Documented forward action shape: `toRecipients` is TOP-LEVEL
        // (never message.toRecipients); content is `comment` (never
        // both comment and message.body).
        let payload = serde_json::json!({
            "comment": body,
            "toRecipients": to.iter().map(|t| serde_json::json!({"emailAddress": {"address": t}})).collect::<Vec<_>>()
        });
        self.post_status_only(
            &format!("/v1.0/me/messages/{original_id}/forward"),
            &payload,
        )?;
        Ok(original_id.to_string())
    }

    fn update_message(
        &self,
        message_id: &str,
        update: &serde_json::Value,
    ) -> Result<GraphMessage, MailError> {
        // Update/delete are Mail.ReadWrite-class operations; plain
        // Mail.Read authority is NOT enough (owner directive F).
        if !self.scope.allows_modify() {
            return Err(MailError::authorization(
                "graph token scope does not allow update (requires ReadWrite)",
            ));
        }
        self.patch_json(&format!("/v1.0/me/messages/{message_id}"), update)
    }

    fn delete_message(&self, message_id: &str) -> Result<(), MailError> {
        if !self.scope.allows_modify() {
            return Err(MailError::authorization(
                "graph token scope does not allow delete (requires ReadWrite)",
            ));
        }
        self.delete_status_only(&format!("/v1.0/me/messages/{message_id}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep026_unit_graph_scope_separation() {
        // Acceptance obligation 2 + directive F: read/send/read-write
        // authorities are separate.
        assert!(GraphScope::ReadOnly.allows_read());
        assert!(!GraphScope::ReadOnly.allows_modify());
        assert!(!GraphScope::ReadOnly.allows_send());
        assert!(GraphScope::ReadWrite.allows_read());
        assert!(GraphScope::ReadWrite.allows_modify());
        assert!(!GraphScope::ReadWrite.allows_send());
        assert!(!GraphScope::Send.allows_read());
        assert!(!GraphScope::Send.allows_modify());
        assert!(GraphScope::Send.allows_send());
        assert!(GraphScope::Full.allows_read());
        assert!(GraphScope::Full.allows_modify());
        assert!(GraphScope::Full.allows_send());
    }

    #[test]
    fn ep026_unit_graph_status_mapping() {
        assert_eq!(
            HttpGraphTransport::map_status(reqwest::StatusCode::UNAUTHORIZED),
            MailErrorCode::Authorization
        );
        assert_eq!(
            HttpGraphTransport::map_status(reqwest::StatusCode::FORBIDDEN),
            MailErrorCode::Authorization
        );
        assert_eq!(
            HttpGraphTransport::map_status(reqwest::StatusCode::NOT_FOUND),
            MailErrorCode::NotFound
        );
        assert_eq!(
            HttpGraphTransport::map_status(reqwest::StatusCode::CONFLICT),
            MailErrorCode::Conflict
        );
        assert_eq!(
            HttpGraphTransport::map_status(reqwest::StatusCode::TOO_MANY_REQUESTS),
            MailErrorCode::RateLimit
        );
        for code in [
            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
            reqwest::StatusCode::BAD_GATEWAY,
            reqwest::StatusCode::SERVICE_UNAVAILABLE,
            reqwest::StatusCode::GATEWAY_TIMEOUT,
        ] {
            assert_eq!(
                HttpGraphTransport::map_status(code),
                MailErrorCode::Unavailable,
                "status {code} must map to Unavailable"
            );
        }
        assert_eq!(
            HttpGraphTransport::map_status(reqwest::StatusCode::IM_A_TEAPOT),
            MailErrorCode::External
        );
    }

    #[test]
    fn ep026_unit_graph_message_serde_real_shape() {
        // The REAL Graph message shape: from/toRecipients are recipient
        // OBJECTS, isRead/hasAttachments are camelCase.
        let json = r#"{
            "id": "msg-1",
            "subject": "Hello",
            "from": {"emailAddress": {"address": "alice@example.com"}},
            "toRecipients": [{"emailAddress": {"address": "bob@example.com"}}],
            "bodyPreview": "hi",
            "isRead": false,
            "hasAttachments": true,
            "categories": ["Work"],
            "folderId": "folder-9"
        }"#;
        let msg: GraphMessage = serde_json::from_str(json).expect("parse");
        assert_eq!(msg.id, "msg-1");
        assert_eq!(msg.subject.as_deref(), Some("Hello"));
        assert_eq!(msg.from_address(), Some("alice@example.com"));
        assert_eq!(msg.to_recipients.len(), 1);
        assert_eq!(
            msg.to_recipients[0].email_address.address,
            "bob@example.com"
        );
        assert_eq!(msg.body_preview.as_deref(), Some("hi"));
        assert!(!msg.is_read);
        assert!(msg.has_attachments);
        assert_eq!(msg.folder_id.as_deref(), Some("folder-9"));
    }

    #[test]
    fn ep026_unit_graph_message_serde_plain_string_from_fails_closed() {
        // A plain-string from field is NOT the documented Graph shape;
        // deserialization must fail closed rather than fabricate a
        // sender.
        let json = r#"{"id": "m", "from": "alice@example.com"}"#;
        assert!(serde_json::from_str::<GraphMessage>(json).is_err());
    }
}
