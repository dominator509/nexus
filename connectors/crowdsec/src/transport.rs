//! EP-031 CrowdSec transport (M3): real HTTP transport over the
//! DOCUMENTED CrowdSec Local API (LAPI).
//!
//! CrowdSec is optional reputation enforcement (SPEC-013 behavior 3;
//! COMPONENT_REGISTRY external sensor, MIT). Nexus queries the
//! documented LAPI surface and normalizes provider payloads at this
//! infrastructure boundary - free-form CrowdSec JSON never becomes a
//! domain contract.
//!
//! Canonical transport surface (documented at
//! doc.crowdsec.net/docs/local_api and the crowdsecurity/crowdsec
//! apiserver source; anti-hallucination - no invented endpoints):
//! - POST {base}/v1/watchers/login
//!   watcher login; body {"machine_id": ..., "password": ...};
//!   response {"code": 200, "expire": "...", "token": "<jwt>"}
//! - GET  {base}/v1/decisions?ip=<addr>
//!   query active decisions for an IP; response {"decisions": [
//!   {"id": ..., "origin": ..., "type": "ban", "scope": "Ip",
//!   "value": "<addr>", "duration": "...", "scenario": "...",
//!   "action": "ban", "created_at": ...}, ...]}
//!
//! Authentication: the watcher login token is a JWT sent as a Bearer
//! Authorization header on the decisions request. The machine_id and
//! password are used ONLY for the login exchange and are registered as
//! redaction secrets; they never appear in errors or telemetry.
//!
//! HTTP status mapping follows SPEC-006: 400 -> Validation, 401/403 ->
//! Authorization, 404 -> NotFound, 409 -> Conflict, 429 -> RateLimit,
//! 5xx -> Unavailable, silent peer -> Timeout, refused -> Unavailable,
//! malformed JSON -> External (fail closed).

use std::time::Duration;

use nexus_sentinel::{SentinelError, SentinelErrorCode};

/// Normalized CrowdSec decision (documented GET /v1/decisions entry).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CrowdSecDecision {
    /// Provider-side decision id.
    pub id: i64,
    /// Decision origin (e.g. "cscli", "CAPI").
    pub origin: String,
    /// Decision action/type (documented "ban").
    pub action: String,
    /// Decision scope (documented "Ip").
    pub scope: String,
    /// Decision value (the IP address).
    pub value: String,
    /// Decision duration (documented duration string).
    pub duration: String,
    /// Triggering scenario (documented scenario name).
    pub scenario: String,
    /// RFC3339 timestamp of decision creation.
    pub created_at: String,
}

/// The CrowdSec transport port. Default implementations fail closed
/// (Unavailable) so an unbound transport never fabricates reputation.
pub trait CrowdSecTransport {
    /// Query active decisions for an indicator (documented
    /// GET /v1/decisions?ip=).
    fn decisions_for(&mut self, indicator: &str) -> Result<Vec<CrowdSecDecision>, SentinelError> {
        let _ = indicator;
        Err(SentinelError::unavailable(
            "crowdsec transport has no implementation bound",
        ))
    }
}

fn classify_status(status: reqwest::StatusCode) -> SentinelErrorCode {
    match status.as_u16() {
        400 => SentinelErrorCode::Validation,
        401 | 403 => SentinelErrorCode::Authorization,
        404 => SentinelErrorCode::NotFound,
        409 => SentinelErrorCode::Conflict,
        429 => SentinelErrorCode::RateLimit,
        500 | 502 | 503 | 504 => SentinelErrorCode::Unavailable,
        _ => SentinelErrorCode::ExternalProvider,
    }
}

/// Real blocking HTTP CrowdSec LAPI transport over the documented
/// watcher login + decisions surface.
pub struct HttpCrowdSecTransport {
    client: reqwest::blocking::Client,
    base_url: String,
    /// LAPI machine id (watcher login). Used ONLY for the login
    /// exchange; never logged, never embedded in errors.
    machine_id: String,
    /// LAPI password (watcher login). Used ONLY for the login
    /// exchange; never logged, never embedded in errors.
    password: String,
    /// Cached login token (JWT). Reused until the transport is
    /// re-created or the token is rejected (401 -> re-login).
    token: Option<String>,
}

impl HttpCrowdSecTransport {
    pub fn new(
        base_url: impl Into<String>,
        machine_id: impl Into<String>,
        password: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client");
        Self {
            client,
            base_url: base_url.into(),
            machine_id: machine_id.into(),
            password: password.into(),
            token: None,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn login(&mut self) -> Result<String, SentinelError> {
        let body = serde_json::json!({
            "machine_id": self.machine_id,
            "password": self.password,
        });
        let resp = self
            .client
            .post(self.url("/v1/watchers/login"))
            .json(&body)
            .send()
            .map_err(map_send_error)?;
        if !resp.status().is_success() {
            return Err(SentinelError::new(
                classify_status(resp.status()),
                "crowdsec watcher login failed",
                None,
                None,
                None,
                None,
            ));
        }
        let value: serde_json::Value = resp.json().map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "malformed crowdsec login response",
                None,
                None,
                None,
                None,
            )
        })?;
        let token = value.get("token").and_then(|t| t.as_str()).ok_or_else(|| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "crowdsec login response missing token",
                None,
                None,
                None,
                None,
            )
        })?;
        let token = token.to_string();
        self.token = Some(token.clone());
        Ok(token)
    }

    fn decisions_with_token(
        &self,
        indicator: &str,
    ) -> Result<Vec<CrowdSecDecision>, SentinelError> {
        let token = self
            .token
            .clone()
            .ok_or_else(|| SentinelError::unavailable("crowdsec transport not logged in"))?;
        let resp = self
            .client
            .get(self.url(&format!("/v1/decisions?ip={indicator}")))
            .bearer_auth(&token)
            .send()
            .map_err(map_send_error)?;
        if !resp.status().is_success() {
            return Err(SentinelError::new(
                classify_status(resp.status()),
                "crowdsec decisions request failed",
                None,
                None,
                None,
                None,
            ));
        }
        let value: serde_json::Value = resp.json().map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "malformed crowdsec decisions response",
                None,
                None,
                None,
                None,
            )
        })?;
        let decisions = value
            .get("decisions")
            .and_then(|d| d.as_array())
            .ok_or_else(|| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "crowdsec decisions response missing decisions array",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
        let mut out = Vec::new();
        for d in decisions {
            let parsed: CrowdSecDecision = serde_json::from_value(d.clone()).map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "crowdsec decision missing documented fields",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            out.push(parsed);
        }
        Ok(out)
    }
}

impl CrowdSecTransport for HttpCrowdSecTransport {
    fn decisions_for(&mut self, indicator: &str) -> Result<Vec<CrowdSecDecision>, SentinelError> {
        if self.token.is_none() {
            self.login()?;
        }
        match self.decisions_with_token(indicator) {
            Ok(d) => Ok(d),
            // Token may be expired; a single bounded re-login retry is
            // allowed (never an unbounded retry loop).
            Err(e) if e.code == SentinelErrorCode::Authorization => {
                self.token = None;
                self.login()?;
                self.decisions_with_token(indicator)
            }
            Err(e) => Err(e),
        }
    }
}

fn map_send_error(e: reqwest::Error) -> SentinelError {
    if e.is_timeout() {
        SentinelError::new(
            SentinelErrorCode::Timeout,
            "crowdsec transport timed out",
            None,
            None,
            None,
            None,
        )
    } else if e.is_connect() {
        SentinelError::new(
            SentinelErrorCode::Unavailable,
            "crowdsec transport unreachable",
            None,
            None,
            None,
            None,
        )
    } else {
        SentinelError::new(
            SentinelErrorCode::ExternalProvider,
            "crowdsec transport request failed",
            None,
            None,
            None,
            None,
        )
    }
}

/// Unit transport: always fails closed (used for the unbound case).
impl CrowdSecTransport for () {}

/// Test-double transport that returns a fixed decision list. This is
/// a PEER control only - the adapter under test is never mocked.
#[cfg(test)]
pub struct StubCrowdSecTransport {
    pub decisions: Vec<CrowdSecDecision>,
}

#[cfg(test)]
impl CrowdSecTransport for StubCrowdSecTransport {
    fn decisions_for(&mut self, _indicator: &str) -> Result<Vec<CrowdSecDecision>, SentinelError> {
        Ok(self.decisions.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep031_unit_crowdsec_decision_parse_documented_shape() {
        let json = r#"{"decisions":[{"id":1,"origin":"cscli","type":"ban","scope":"Ip","value":"1.2.3.4","duration":"4h0m0s","scenario":"ssh-bf","action":"ban","created_at":"2026-08-20T00:00:00Z"}]}"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let d: CrowdSecDecision =
            serde_json::from_value(value.get("decisions").unwrap().as_array().unwrap()[0].clone())
                .unwrap();
        assert_eq!(d.action, "ban");
        assert_eq!(d.scope, "Ip");
        assert_eq!(d.value, "1.2.3.4");
        assert_eq!(d.scenario, "ssh-bf");
    }

    #[test]
    fn ep031_unit_crowdsec_decision_missing_fields_fails_closed() {
        let json = r#"{"decisions":[{"id":1}]}"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let res: Result<CrowdSecDecision, _> =
            serde_json::from_value(value.get("decisions").unwrap().as_array().unwrap()[0].clone());
        assert!(res.is_err());
    }
}
