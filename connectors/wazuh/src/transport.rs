//! EP-031 Wazuh transport (M4): real HTTP transport over the
//! DOCUMENTED Wazuh server API.
//!
//! Wazuh is the Endpoint profile security sensor (SPEC-013 behavior 3;
//! COMPONENT_REGISTRY external sensor, GPL-2.0). Nexus consumes its
//! documented server API and normalizes provider payloads at this
//! infrastructure boundary - free-form Wazuh JSON never becomes a
//! domain contract.
//!
//! Canonical transport surface (documented at
//! documentation.wazuh.com/current/user-manual/api; anti-hallucination
//! - no invented endpoints):
//! - POST {base}/security/user/authenticate
//!   Basic auth (username/password); response {"data": {"token": "<jwt>"}}
//! - GET  {base}/alerts?limit=N
//!   Bearer auth; response {"data": {"affected_items": [{
//!   "id": ..., "timestamp": ..., "rule": {"level": ..., "description":
//!   ...}, "agent": {"id": ..., "name": ..., "ip": ...}, ...}],
//!   "total_affected_items": N, "total_failed_items": 0,
//!   "failed_items": []}, "message": "...", "error": 0}
//!
//! Authentication: the authenticate token is a JWT sent as a Bearer
//! Authorization header on every subsequent request. The username and
//! password are used ONLY for the authenticate exchange and are
//! registered as redaction secrets; they never appear in errors or
//! telemetry.
//!
//! HTTP status mapping follows SPEC-006: 400 -> Validation, 401/403 ->
//! Authorization, 404 -> NotFound, 409 -> Conflict, 429 -> RateLimit,
//! 5xx -> Unavailable, silent peer -> Timeout, refused -> Unavailable,
//! malformed JSON -> External (fail closed).

use std::time::Duration;

use nexus_sentinel::{SentinelError, SentinelErrorCode};

/// Normalized Wazuh alert (documented GET /alerts affected_items entry).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WazuhAlert {
    /// Provider-side alert id.
    pub id: String,
    /// Rule level (documented `rule.level`).
    pub rule_level: u32,
    /// Rule description (documented `rule.description`).
    pub rule_description: String,
    /// Agent id (documented `agent.id`).
    pub agent_id: Option<String>,
    /// Agent name (documented `agent.name`).
    pub agent_name: Option<String>,
    /// Agent ip (documented `agent.ip`).
    pub agent_ip: Option<String>,
    /// RFC3339 alert timestamp (documented `timestamp`).
    pub timestamp: String,
}

/// The Wazuh transport port. Default implementations fail closed
/// (Unavailable) so an unbound transport never fabricates alerts.
pub trait WazuhTransport {
    /// Read observed endpoint alerts (documented GET /alerts).
    fn read_alerts(&mut self, limit: usize) -> Result<Vec<WazuhAlert>, SentinelError> {
        let _ = limit;
        Err(SentinelError::unavailable(
            "wazuh transport has no implementation bound",
        ))
    }
}

/// Unit transport: always fails closed (used for the unbound case).
impl WazuhTransport for () {}

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

/// Real blocking HTTP Wazuh server API transport over the documented
/// authenticate + alerts surface.
pub struct HttpWazuhTransport {
    client: reqwest::blocking::Client,
    base_url: String,
    /// Wazuh API username (Basic auth for authenticate). Used ONLY for
    /// the authenticate exchange; never logged, never embedded in
    /// errors.
    username: String,
    /// Wazuh API password (Basic auth for authenticate). Used ONLY for
    /// the authenticate exchange; never logged, never embedded in
    /// errors.
    password: String,
    /// Cached JWT token. Reused until rejected (401 -> bounded
    /// re-authenticate once).
    token: Option<String>,
}

impl HttpWazuhTransport {
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
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
            username: username.into(),
            password: password.into(),
            token: None,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn authenticate(&mut self) -> Result<String, SentinelError> {
        let resp = self
            .client
            .post(self.url("/security/user/authenticate"))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(map_send_error)?;
        if !resp.status().is_success() {
            return Err(SentinelError::new(
                classify_status(resp.status()),
                "wazuh authenticate failed",
                None,
                None,
                None,
                None,
            ));
        }
        let value: serde_json::Value = resp.json().map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "malformed wazuh authenticate response",
                None,
                None,
                None,
                None,
            )
        })?;
        let token = value
            .get("data")
            .and_then(|d| d.get("token"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "wazuh authenticate response missing token",
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

    fn alerts_with_token(&self, limit: usize) -> Result<Vec<WazuhAlert>, SentinelError> {
        let token = self
            .token
            .clone()
            .ok_or_else(|| SentinelError::unavailable("wazuh transport not authenticated"))?;
        let resp = self
            .client
            .get(self.url(&format!("/alerts?limit={limit}")))
            .bearer_auth(&token)
            .send()
            .map_err(map_send_error)?;
        if !resp.status().is_success() {
            return Err(SentinelError::new(
                classify_status(resp.status()),
                "wazuh alerts request failed",
                None,
                None,
                None,
                None,
            ));
        }
        let value: serde_json::Value = resp.json().map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "malformed wazuh alerts response",
                None,
                None,
                None,
                None,
            )
        })?;
        let items = value
            .get("data")
            .and_then(|d| d.get("affected_items"))
            .and_then(|a| a.as_array())
            .ok_or_else(|| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "wazuh alerts response missing affected_items",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
        let mut out = Vec::new();
        for item in items {
            let parsed: WazuhAlertRaw = serde_json::from_value(item.clone()).map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "wazuh alert missing documented fields",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            out.push(parsed.into());
        }
        Ok(out)
    }
}

impl WazuhTransport for HttpWazuhTransport {
    fn read_alerts(&mut self, limit: usize) -> Result<Vec<WazuhAlert>, SentinelError> {
        if self.token.is_none() {
            self.authenticate()?;
        }
        match self.alerts_with_token(limit) {
            Ok(a) => Ok(a),
            // Token may be expired; a single bounded re-authenticate
            // retry is allowed (never an unbounded retry loop).
            Err(e) if e.code == SentinelErrorCode::Authorization => {
                self.token = None;
                self.authenticate()?;
                self.alerts_with_token(limit)
            }
            Err(e) => Err(e),
        }
    }
}

/// Raw wire shape for the documented Wazuh alert JSON record.
#[derive(Debug, Clone, serde::Deserialize)]
struct WazuhAlertRaw {
    id: String,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    rule: Option<WazuhRuleRaw>,
    #[serde(default)]
    agent: Option<WazuhAgentRaw>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct WazuhRuleRaw {
    #[serde(default)]
    level: Option<u32>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct WazuhAgentRaw {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    ip: Option<String>,
}

impl From<WazuhAlertRaw> for WazuhAlert {
    fn from(raw: WazuhAlertRaw) -> Self {
        Self {
            id: raw.id,
            rule_level: raw.rule.as_ref().and_then(|r| r.level).unwrap_or(0),
            rule_description: raw
                .rule
                .as_ref()
                .and_then(|r| r.description.clone())
                .unwrap_or_default(),
            agent_id: raw.agent.as_ref().and_then(|a| a.id.clone()),
            agent_name: raw.agent.as_ref().and_then(|a| a.name.clone()),
            agent_ip: raw.agent.as_ref().and_then(|a| a.ip.clone()),
            timestamp: raw.timestamp.unwrap_or_default(),
        }
    }
}

fn map_send_error(e: reqwest::Error) -> SentinelError {
    if e.is_timeout() {
        SentinelError::new(
            SentinelErrorCode::Timeout,
            "wazuh transport timed out",
            None,
            None,
            None,
            None,
        )
    } else if e.is_connect() {
        SentinelError::new(
            SentinelErrorCode::Unavailable,
            "wazuh transport unreachable",
            None,
            None,
            None,
            None,
        )
    } else {
        SentinelError::new(
            SentinelErrorCode::ExternalProvider,
            "wazuh transport request failed",
            None,
            None,
            None,
            None,
        )
    }
}

/// Test-double transport that returns a fixed alert list. This is a
/// PEER control only - the adapter under test is never mocked.
#[cfg(test)]
pub struct StubWazuhTransport {
    pub alerts: Vec<WazuhAlert>,
}

#[cfg(test)]
impl WazuhTransport for StubWazuhTransport {
    fn read_alerts(&mut self, _limit: usize) -> Result<Vec<WazuhAlert>, SentinelError> {
        Ok(self.alerts.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep031_unit_wazuh_alert_parse_documented_shape() {
        let json = r#"{"id":"1234","timestamp":"2026-08-20T00:00:00Z","rule":{"level":5,"description":"PAM: Login session opened."},"agent":{"id":"001","name":"laptop","ip":"192.0.2.5"}}"#;
        let raw: WazuhAlertRaw = serde_json::from_str(json).unwrap();
        let alert: WazuhAlert = raw.into();
        assert_eq!(alert.id, "1234");
        assert_eq!(alert.rule_level, 5);
        assert_eq!(alert.rule_description, "PAM: Login session opened.");
        assert_eq!(alert.agent_id.as_deref(), Some("001"));
        assert_eq!(alert.agent_ip.as_deref(), Some("192.0.2.5"));
    }

    #[test]
    fn ep031_unit_wazuh_alert_partial_agent_fields_default() {
        let json = r#"{"id":"5678","rule":{"level":3,"description":"x"}}"#;
        let raw: WazuhAlertRaw = serde_json::from_str(json).unwrap();
        let alert: WazuhAlert = raw.into();
        assert_eq!(alert.id, "5678");
        assert_eq!(alert.rule_level, 3);
        assert!(alert.agent_id.is_none());
        assert!(alert.timestamp.is_empty());
    }
}
