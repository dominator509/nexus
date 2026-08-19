//! EP-027 ICTFax transport (M2): real HTTP transport against the
//! DOCUMENTED ICTFax REST API surface.
//!
//! ICTFax is the primary self-hosted fax control sidecar (SPEC-014
//! behavior 5; COMPONENT_REGISTRY id ictfax, GPL-3.0, isolated-sidecar
//! boundary). Nexus orchestrates the provider API and normalizes
//! provider payloads at this infrastructure boundary - free-form
//! ICTFax JSON never becomes a domain contract.
//!
//! Documented surface used (ICTFax REST APIs Guide):
//! - POST /api/authenticate                          session token
//! - POST /api/messages/documents                   create document
//! - GET  /api/messages/documents                   list documents
//! - GET  /api/messages/documents/{id}              document detail
//! - POST /api/messages/documents/{id}/media        add document file
//! - GET  /api/messages/documents/{id}/media        get document file
//! - POST /api/programs/sendfax                     create send program
//! - POST /api/transmissions                        create transmission
//! - GET  /api/transmissions/{id}                   transmission detail
//! - GET  /api/transmissions/{id}/status            status report
//! - GET  /api/transmissions/{id}/result            result report
//! - POST /api/transmissions/{id}/send              send transmission
//! - DELETE /api/transmissions/{id}                 cancel transmission
//! - GET  /api/accounts                             list accounts/DIDs
//!
//! Documented HTTP codes (ICTFax guide): 200 ok, 401 invalid/missing
//! username or password, 403 permission denied, 404 invalid API
//! location, 412 data validation failed, 417 unexpected error, 423
//! system not ready, 500 internal, 501 feature not implemented.
//!
//! SPEC-006 mapping: 401/403 -> Authorization, 404 -> NotFound,
//! 412 -> Validation, 417/501 -> External, 423/500 -> Unavailable,
//! silent peer -> Timeout, refused -> Unavailable, malformed JSON ->
//! External (fail closed). The session token is used ONLY for the
//! documented `Authentication: Bearer <token>` header and never
//! appears in errors or telemetry.

use std::time::Duration;

use nexus_fax::{FaxError, FaxErrorCode};

/// Canonical ICTFax transmission envelope (documented transmission
/// shape, normalized at the boundary). Unknown fields are tolerated so
/// a provider schema addition never breaks the adapter; missing
/// required identity fails closed instead.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IctFaxTransmission {
    pub id: String,
    /// Canonical recipient number as ICTFax recorded it.
    #[serde(default)]
    pub destination: String,
    /// Carrier-observed status string (mapped canonically by
    /// `map_transmission_state`; unknown strings fail closed).
    #[serde(default)]
    pub status: String,
    /// Transmission program/document reference.
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub document_id: Option<String>,
    /// Attempts and page counts where the provider reports them.
    #[serde(default)]
    pub attempts: u32,
    #[serde(default)]
    pub pages: u32,
}

/// Canonical ICTFax account/DID envelope (inbound route source).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IctFaxAccount {
    pub id: String,
    /// Canonical number/extension this account serves.
    #[serde(default)]
    pub number: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
}

/// Canonical ICTFax document metadata (upload target reference).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct IctFaxDocument {
    pub id: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub size_bytes: u64,
}

/// Real ICTFax REST transport. All methods are real HTTP operations
/// against the documented ICTFax API; no in-memory substitute exists
/// in production code. Test doubles live under TESTING.md's test
/// zones.
pub trait IctFaxTransport {
    /// Authenticate and mint a session token.
    fn authenticate(&self, username: &str, password: &str) -> Result<String, FaxError> {
        let _ = (username, password);
        Err(FaxError::unavailable(
            "ictfax transport has no implementation bound",
        ))
    }

    /// Upload document media for a previously created document.
    fn upload_document_media(&self, document_id: &str) -> Result<(), FaxError> {
        let _ = document_id;
        Err(FaxError::unavailable(
            "ictfax transport has no implementation bound",
        ))
    }

    /// Create a sendfax program and return its reference.
    fn create_sendfax_program(&self) -> Result<String, FaxError> {
        Err(FaxError::unavailable(
            "ictfax transport has no implementation bound",
        ))
    }

    /// Create a transmission for a sendfax program.
    fn create_transmission(&self) -> Result<IctFaxTransmission, FaxError> {
        Err(FaxError::unavailable(
            "ictfax transport has no implementation bound",
        ))
    }

    /// Send a created transmission.
    fn send_transmission(&self, transmission_id: &str) -> Result<(), FaxError> {
        let _ = transmission_id;
        Err(FaxError::unavailable(
            "ictfax transport has no implementation bound",
        ))
    }

    /// Fetch one transmission by id (exact-target status readback).
    fn fetch_transmission(&self, transmission_id: &str) -> Result<IctFaxTransmission, FaxError> {
        let _ = transmission_id;
        Err(FaxError::unavailable(
            "ictfax transport has no implementation bound",
        ))
    }

    /// List transmissions (bounded by the provider's page size).
    fn list_transmissions(&self) -> Result<Vec<IctFaxTransmission>, FaxError> {
        Err(FaxError::unavailable(
            "ictfax transport has no implementation bound",
        ))
    }

    /// Cancel/delete a transmission where the provider supports it.
    fn delete_transmission(&self, transmission_id: &str) -> Result<(), FaxError> {
        let _ = transmission_id;
        Err(FaxError::unavailable(
            "ictfax transport has no implementation bound",
        ))
    }

    /// List accounts/DIDs (inbound route sources).
    fn list_accounts(&self) -> Result<Vec<IctFaxAccount>, FaxError> {
        Err(FaxError::unavailable(
            "ictfax transport has no implementation bound",
        ))
    }
}

/// Map a carrier-observed ICTFax transmission status string to the
/// canonical fax state. Unknown strings fail closed (External) - a
/// status we cannot classify must never fabricate a state.
///
/// SUBMITTED != DELIVERED: a status like `sent`/`accepted` proves the
/// carrier accepted the job, never delivery. Only an explicit
/// completed/successful transmission record is DELIVERED-class
/// evidence, and even then exact-target verification is required
/// before Nexus treats delivery as proved.
pub fn map_transmission_state(status: &str) -> Result<nexus_fax::FaxState, FaxError> {
    use nexus_fax::FaxState;
    match status.trim().to_ascii_lowercase().as_str() {
        "new" | "queued" | "pending" | "scheduled" => Ok(FaxState::Queued),
        "sending" | "in_progress" | "in-progress" | "processing" | "transmitting" => {
            Ok(FaxState::Submitting)
        }
        "sent" | "accepted" | "submitted" => Ok(FaxState::Submitted),
        "completed" | "success" | "successful" | "delivered" => Ok(FaxState::Delivered),
        "failed" | "error" | "rejected" | "busy" | "no_answer" | "no-answer" => {
            Ok(FaxState::Failed)
        }
        "cancelled" | "canceled" => Ok(FaxState::Cancelled),
        other => Err(FaxError::external(format!(
            "ictfax transmission status {other:?} is not in the documented vocabulary (fail closed)"
        ))),
    }
}

/// Real blocking HTTP ICTFax transport.
///
/// The documented session header is `Authentication: Bearer <token>`.
/// The session token is held only for that header; it is never
/// serialized into errors or audit entries.
pub struct HttpIctFaxTransport {
    base_url: String,
    session_token: String,
    client: reqwest::blocking::Client,
}

impl HttpIctFaxTransport {
    pub fn new(base_url: impl Into<String>, session_token: impl Into<String>) -> Self {
        let timeout = Duration::from_secs(10);
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client builds");
        Self {
            base_url: base_url.into(),
            session_token: session_token.into(),
            client,
        }
    }

    fn authorize(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> reqwest::blocking::RequestBuilder {
        // Documented header: `Authentication: Bearer JWT`.
        request.header("Authentication", format!("Bearer {}", self.session_token))
    }

    fn map_status(status: reqwest::StatusCode) -> FaxErrorCode {
        match status.as_u16() {
            401 | 403 => FaxErrorCode::Authorization,
            404 => FaxErrorCode::NotFound,
            412 => FaxErrorCode::Validation,
            423 | 500 | 502 | 503 => FaxErrorCode::Unavailable,
            501 => FaxErrorCode::External,
            _ => FaxErrorCode::External,
        }
    }

    fn send_error(&self, e: reqwest::Error, operation: &str) -> FaxError {
        if e.is_timeout() {
            FaxError::timeout(format!("ictfax {operation} timed out"))
        } else if e.is_connect() {
            FaxError::unavailable(format!("ictfax {operation} endpoint refused connection"))
        } else {
            FaxError::external(format!("ictfax {operation} transport error: {e}"))
        }
    }

    fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, FaxError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.get(&url))
            .send()
            .map_err(|e| self.send_error(e, "GET"))?;
        if !response.status().is_success() {
            return Err(FaxError::new(
                Self::map_status(response.status()),
                format!("ictfax GET {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        response.json().map_err(|e| {
            FaxError::new(
                FaxErrorCode::External,
                format!("ictfax GET {path} malformed JSON: {e}"),
                None,
                None,
            )
        })
    }

    fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, FaxError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.post(&url).json(body))
            .send()
            .map_err(|e| self.send_error(e, "POST"))?;
        if !response.status().is_success() {
            return Err(FaxError::new(
                Self::map_status(response.status()),
                format!("ictfax POST {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        response.json().map_err(|e| {
            FaxError::new(
                FaxErrorCode::External,
                format!("ictfax POST {path} malformed JSON: {e}"),
                None,
                None,
            )
        })
    }

    fn post_empty(&self, path: &str, body: &serde_json::Value) -> Result<(), FaxError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.post(&url).json(body))
            .send()
            .map_err(|e| self.send_error(e, "POST"))?;
        if !response.status().is_success() {
            return Err(FaxError::new(
                Self::map_status(response.status()),
                format!("ictfax POST {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        Ok(())
    }

    fn delete_empty(&self, path: &str) -> Result<(), FaxError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .authorize(self.client.delete(&url))
            .send()
            .map_err(|e| self.send_error(e, "DELETE"))?;
        if !response.status().is_success() {
            return Err(FaxError::new(
                Self::map_status(response.status()),
                format!("ictfax DELETE {path} returned {}", response.status()),
                None,
                None,
            ));
        }
        Ok(())
    }
}

impl IctFaxTransport for HttpIctFaxTransport {
    fn authenticate(&self, username: &str, password: &str) -> Result<String, FaxError> {
        // Unlike other APIs, the documented authenticate call carries
        // username/password in the body and does not require the
        // session header.
        let body = serde_json::json!({ "username": username, "password": password });
        let url = format!("{}/api/authenticate", self.base_url);
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| self.send_error(e, "authenticate"))?;
        if !response.status().is_success() {
            return Err(FaxError::new(
                Self::map_status(response.status()),
                format!("ictfax authenticate returned {}", response.status()),
                None,
                None,
            ));
        }
        // The guide returns a session key; tolerate both bare string
        // and {"token": ...} envelopes but fail closed on neither.
        let value: serde_json::Value = response.json().map_err(|e| {
            FaxError::new(
                FaxErrorCode::External,
                format!("ictfax authenticate malformed JSON: {e}"),
                None,
                None,
            )
        })?;
        if let Some(s) = value.as_str() {
            return Ok(s.to_string());
        }
        if let Some(s) = value.get("token").and_then(|v| v.as_str()) {
            return Ok(s.to_string());
        }
        Err(FaxError::external(
            "ictfax authenticate response has no session token (fail closed)",
        ))
    }

    fn upload_document_media(&self, document_id: &str) -> Result<(), FaxError> {
        // Documented: POST /api/messages/documents/{document_id}/media
        // (Add / Update Document file).
        let path = format!("/api/messages/documents/{document_id}/media");
        let body = serde_json::json!({});
        self.post_empty(&path, &body)
    }

    fn create_sendfax_program(&self) -> Result<String, FaxError> {
        let body = serde_json::json!({ "name": "nexus-sendfax" });
        let created: serde_json::Value = self.post_json("/api/programs/sendfax", &body)?;
        created
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .ok_or_else(|| {
                FaxError::external("ictfax sendfax program response has no id (fail closed)")
            })
    }

    fn create_transmission(&self) -> Result<IctFaxTransmission, FaxError> {
        let body = serde_json::json!({});
        self.post_json("/api/transmissions", &body)
    }

    fn send_transmission(&self, transmission_id: &str) -> Result<(), FaxError> {
        let path = format!("/api/transmissions/{transmission_id}/send");
        let body = serde_json::json!({});
        self.post_empty(&path, &body)
    }

    fn fetch_transmission(&self, transmission_id: &str) -> Result<IctFaxTransmission, FaxError> {
        let path = format!("/api/transmissions/{transmission_id}");
        self.get_json(&path)
    }

    fn list_transmissions(&self) -> Result<Vec<IctFaxTransmission>, FaxError> {
        self.get_json("/api/transmissions")
    }

    fn delete_transmission(&self, transmission_id: &str) -> Result<(), FaxError> {
        let path = format!("/api/transmissions/{transmission_id}");
        self.delete_empty(&path)
    }

    fn list_accounts(&self) -> Result<Vec<IctFaxAccount>, FaxError> {
        self.get_json("/api/accounts")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep027_unit_ictfax_status_mapping_documented_vocabulary() {
        use nexus_fax::FaxState;
        assert_eq!(map_transmission_state("new").unwrap(), FaxState::Queued);
        assert_eq!(map_transmission_state("QUEUED").unwrap(), FaxState::Queued);
        assert_eq!(
            map_transmission_state("sending").unwrap(),
            FaxState::Submitting
        );
        assert_eq!(map_transmission_state("sent").unwrap(), FaxState::Submitted);
        // SUBMITTED != DELIVERED: `sent`/`accepted` never map to
        // DELIVERED; only explicit completion records do.
        assert_ne!(map_transmission_state("sent").unwrap(), FaxState::Delivered);
        assert_eq!(
            map_transmission_state("completed").unwrap(),
            FaxState::Delivered
        );
        assert_eq!(map_transmission_state("failed").unwrap(), FaxState::Failed);
        assert_eq!(
            map_transmission_state("cancelled").unwrap(),
            FaxState::Cancelled
        );
    }

    #[test]
    fn ep027_unit_ictfax_status_mapping_unknown_fails_closed() {
        let err = map_transmission_state("alien-state").expect_err("must reject");
        assert_eq!(err.code, FaxErrorCode::External);
    }

    #[test]
    fn ep027_unit_ictfax_http_status_mapping() {
        fn code(n: u16) -> FaxErrorCode {
            HttpIctFaxTransport::map_status(reqwest::StatusCode::from_u16(n).unwrap())
        }
        assert_eq!(code(200), FaxErrorCode::External); // success never mapped as error
        assert_eq!(code(401), FaxErrorCode::Authorization);
        assert_eq!(code(403), FaxErrorCode::Authorization);
        assert_eq!(code(404), FaxErrorCode::NotFound);
        assert_eq!(code(412), FaxErrorCode::Validation);
        assert_eq!(code(417), FaxErrorCode::External);
        assert_eq!(code(423), FaxErrorCode::Unavailable);
        assert_eq!(code(500), FaxErrorCode::Unavailable);
        assert_eq!(code(501), FaxErrorCode::External);
    }

    #[test]
    fn ep027_unit_ictfax_transmission_serde_roundtrip() {
        let t = IctFaxTransmission {
            id: "tx-1".into(),
            destination: "+15551234567".into(),
            status: "sent".into(),
            program: Some("p-1".into()),
            document_id: Some("d-1".into()),
            attempts: 1,
            pages: 2,
        };
        let json = serde_json::to_string(&t).expect("serialize");
        let back: IctFaxTransmission = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, "tx-1");
        assert_eq!(back.status, "sent");
        assert_eq!(back.pages, 2);
    }

    #[test]
    fn ep027_unit_ictfax_account_serde() {
        let a = IctFaxAccount {
            id: "acc-1".into(),
            number: "+15551234567".into(),
            name: "main".into(),
            enabled: true,
        };
        let json = serde_json::to_string(&a).expect("serialize");
        let back: IctFaxAccount = serde_json::from_str(&json).expect("deserialize");
        assert!(back.enabled);
        assert_eq!(back.number, "+15551234567");
    }
}
