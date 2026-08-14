//! Provider client (directive J).
//!
//! The sidecar dispatches validated requests to the provider process
//! over real HTTP. Provider failures are typed and fail closed:
//!
//! - provider absent/unreachable -> UNAVAILABLE
//! - provider dies before/during dispatch -> PROVIDER_ERROR
//! - provider hangs -> TIMEOUT
//! - provider returns partial/truncated payload -> PROVIDER_ERROR
//! - provider returns schema-invalid payload -> PROVIDER_ERROR
//! - provider returns oversized payload -> bounded rejection
//!
//! There is no empty-success fallback.

use std::time::Duration;

use crate::error::{SidecarError, SidecarErrorKind};
use crate::limits::Limits;

/// Typed provider failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// Provider unreachable/absent (connection refused, DNS, etc).
    Unavailable(String),
    /// Provider died or connection failed mid-dispatch.
    Transport(String),
    /// Provider exceeded the bounded timeout.
    Timeout(String),
    /// Provider payload was malformed, truncated, or schema-invalid.
    Malformed(String),
    /// Provider payload exceeded the bounded response size.
    Oversized(String),
    /// Provider returned a canonical error envelope.
    Envelope(serde_json::Value),
}

impl ProviderError {
    /// Map to a typed sidecar error (fail closed, no empty success).
    pub fn into_sidecar(
        self,
        correlation_id: Option<String>,
        resource: Option<String>,
    ) -> SidecarError {
        let kind = match &self {
            Self::Unavailable(_) => SidecarErrorKind::Unavailable,
            Self::Transport(_) | Self::Malformed(_) => SidecarErrorKind::ProviderError,
            Self::Timeout(_) => SidecarErrorKind::Timeout,
            Self::Oversized(_) => SidecarErrorKind::ResponseTooLarge,
            Self::Envelope(v) => {
                // Provider canonical envelope: prefer its code when it
                // is a known canonical code; otherwise fail closed as
                // a provider error.
                let code = v.get("code").and_then(|c| c.as_str()).unwrap_or("INTERNAL");
                match code {
                    "NOT_FOUND" => SidecarErrorKind::Unavailable,
                    "CONFLICT" => SidecarErrorKind::Validation,
                    "UNAVAILABLE" => SidecarErrorKind::Unavailable,
                    "TIMEOUT" => SidecarErrorKind::Timeout,
                    "AUTHORIZATION" => SidecarErrorKind::CredentialDenied,
                    _ => SidecarErrorKind::ProviderError,
                }
            }
        };
        let message = match &self {
            Self::Unavailable(m)
            | Self::Transport(m)
            | Self::Timeout(m)
            | Self::Malformed(m)
            | Self::Oversized(m) => m.clone(),
            Self::Envelope(v) => v
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("provider returned an error envelope")
                .to_string(),
        };
        SidecarError::new(kind, message, correlation_id, None, resource)
    }
}

/// Real HTTP provider client (directive J).
///
/// Uses reqwest (already locked) to talk to the provider process over
/// plain localhost HTTP with a bounded timeout.
#[derive(Debug, Clone)]
pub struct ProviderClient {
    base_url: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl ProviderClient {
    /// Construct a provider client for a base URL (e.g.
    /// `http://127.0.0.1:PORT`).
    pub fn new(base_url: impl Into<String>, limits: Limits) -> Result<Self, SidecarError> {
        let base_url = base_url.into();
        if base_url.trim().is_empty() {
            return Err(SidecarError::validation(
                "provider URL must not be empty",
                None,
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(limits.provider_timeout)
            .build()
            .map_err(|e| {
                SidecarError::new(
                    SidecarErrorKind::Internal,
                    format!("provider client build failed: {e}"),
                    None,
                    None,
                    None,
                )
            })?;
        Ok(Self {
            base_url,
            client,
            timeout: limits.provider_timeout,
        })
    }

    /// Base URL the client targets.
    pub fn base(&self) -> &str {
        &self.base_url
    }

    /// Dispatch one canonical request to the provider endpoint.
    ///
    /// `path` is the provider's canonical endpoint (e.g. `/v1/query`);
    /// `body` is the canonical wire payload. The provider's canonical
    /// success/error envelope is returned.
    pub async fn dispatch(
        &self,
        path: &str,
        body: serde_json::Value,
        _correlation_id: Option<&str>,
        limits: &Limits,
    ) -> Result<serde_json::Value, ProviderError> {
        let url = format!("{}{}", self.base_url, path);
        let result = tokio::time::timeout(
            self.timeout,
            self.client
                .post(&url)
                .header("x-nexus-protocol-version", crate::version::PROTOCOL_VERSION)
                .json(&body)
                .send(),
        )
        .await;
        let response = match result {
            Err(_) => {
                return Err(ProviderError::Timeout(format!(
                    "provider did not respond within {}ms",
                    self.timeout.as_millis()
                )));
            }
            Ok(Err(e)) => {
                if e.is_timeout() {
                    return Err(ProviderError::Timeout(format!(
                        "provider request timed out: {e}"
                    )));
                }
                if e.is_connect() {
                    return Err(ProviderError::Unavailable(format!(
                        "provider unavailable: {e}"
                    )));
                }
                return Err(ProviderError::Transport(format!(
                    "provider transport failure: {e}"
                )));
            }
            Ok(Ok(r)) => r,
        };

        let status = response.status();
        let bytes_result = tokio::time::timeout(self.timeout, response.bytes()).await;
        let bytes = match bytes_result {
            Err(_) => {
                return Err(ProviderError::Timeout(
                    "provider response body timed out".to_string(),
                ));
            }
            Ok(Err(e)) => {
                return Err(ProviderError::Transport(format!(
                    "provider response read failed: {e}"
                )));
            }
            Ok(Ok(b)) => b,
        };

        if bytes.len() as u64 > limits.max_response_bytes {
            return Err(ProviderError::Oversized(format!(
                "provider response exceeds bounded size ({} bytes)",
                bytes.len()
            )));
        }

        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            ProviderError::Malformed(format!("provider returned malformed JSON: {e}"))
        })?;

        if !status.is_success() {
            return Err(ProviderError::Envelope(value));
        }
        // Schema validation (directive J.7): a syntactically valid but
        // schema-invalid provider payload is rejected, never relayed
        // as success.
        Self::validate_response_shape(path, &value).map_err(ProviderError::Malformed)?;
        Ok(value)
    }

    /// Canonical response-shape validation (directive J.7).
    ///
    /// Each endpoint has a locked canonical success shape. A payload that
    /// parses as JSON but does not match the shape is a malformed provider
    /// response, never a success.
    fn validate_response_shape(path: &str, value: &serde_json::Value) -> Result<(), String> {
        let Some(obj) = value.as_object() else {
            return Err("provider response must be a JSON object".to_string());
        };
        match path {
            "/v1/discover" => {
                if !obj.contains_key("capabilities") {
                    return Err("discover response missing `capabilities`".to_string());
                }
            }
            "/v1/query" | "/v1/command" | "/v1/execute" => {
                if !obj.contains_key("capability_id") || !obj.contains_key("output") {
                    return Err(format!(
                        "{path} response missing `capability_id` or `output`"
                    ));
                }
            }
            "/v1/workflow" => {
                if !obj.contains_key("handle") || !obj.contains_key("status") {
                    return Err("workflow response missing `handle` or `status`".to_string());
                }
            }
            "/v1/health" => {
                if !obj.contains_key("target_id") || !obj.contains_key("state") {
                    return Err("health response missing `target_id` or `state`".to_string());
                }
            }
            "/v1/changefeed" | "/v1/poll"
                if !obj.contains_key("events") || !obj.contains_key("next_cursor") =>
            {
                return Err(format!("{path} response missing `events` or `next_cursor`"));
            }
            "/v1/changefeed" | "/v1/poll" => {}
            _ => {}
        }
        Ok(())
    }

    /// Provider availability probe (directive J.1).
    pub async fn available(&self) -> bool {
        let url = format!("{}/v1/fixture/healthz", self.base_url);
        tokio::time::timeout(self.timeout, self.client.get(&url).send())
            .await
            .map(|r| r.map(|resp| resp.status().is_success()).unwrap_or(false))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep011_unit_sidecar_provider_error_maps_fail_closed() {
        let err = ProviderError::Unavailable("refused".to_string()).into_sidecar(None, None);
        assert_eq!(err.kind, SidecarErrorKind::Unavailable);
        let err = ProviderError::Timeout("slow".to_string()).into_sidecar(None, None);
        assert_eq!(err.kind, SidecarErrorKind::Timeout);
        let err = ProviderError::Malformed("bad".to_string()).into_sidecar(None, None);
        assert_eq!(err.kind, SidecarErrorKind::ProviderError);
        let err = ProviderError::Oversized("big".to_string()).into_sidecar(None, None);
        assert_eq!(err.kind, SidecarErrorKind::ResponseTooLarge);
    }

    #[test]
    fn ep011_unit_sidecar_provider_client_rejects_empty_url() {
        assert!(ProviderClient::new("", Limits::default()).is_err());
    }
}
