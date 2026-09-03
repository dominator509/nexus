//! EP-030 OPNsense transport (M2): real HTTP transport over the
//! DOCUMENTED OPNsense firewall automation API.
//!
//! OPNsense is the primary serious firewall (SPEC-013 behavior 2;
//! COMPONENT_REGISTRY external-appliance, BSD-2-Clause). Nexus
//! orchestrates its documented firewall automation API and normalizes
//! provider payloads at this infrastructure boundary - free-form
//! OPNsense JSON never becomes a domain contract.
//!
//! Canonical transport surface (verified against the official OPNsense
//! documentation, docs.opnsense.org/development/api/core/firewall.html,
//! and the OPNsense core source FilterController.php/Filter.xml):
//! - GET  {base}/api/firewall/filter/searchRule?current=1&rowCount=N&searchPhrase=S
//!   search firewall automation rules (response rows[].uuid,
//!   rows[].description, rows[].enabled, rows[].action)
//! - POST {base}/api/firewall/filter/addRule
//!   create a rule (body {"rule": {...}}; response {uuid})
//! - POST {base}/api/firewall/filter/toggleRule/{uuid}/{enabled}
//!   enable (1) or disable (0) a rule
//! - POST {base}/api/firewall/filter/apply
//!   apply/reload the firewall so staged rule changes become active
//!
//! Authentication: HTTP Basic with the OPNsense API key as username
//! and API secret as password (documented `$key`/`$secret`). The
//! credential pair is used ONLY for the Basic auth header and never
//! appears in errors or telemetry.
//!
//! Rule model (Filter.xml, verified): enabled (BooleanField, default
//! 1), action (OptionField pass|block|reject, default pass), quick,
//! direction (in|out|any), ipprotocol (inet|inet6|inet46), protocol
//! (ProtocolField, default any), source_net (NetworkAliasField),
//! destination_net (NetworkAliasField), description
//! (DescriptionField). Free-form provider payloads are normalized at
//! the boundary.
//!
//! HTTP status mapping follows SPEC-006: 400 -> Validation, 401/403 ->
//! Authorization, 404 -> NotFound, 409 -> Conflict, 429 -> RateLimit,
//! 5xx -> Unavailable, silent peer -> Timeout, refused -> Unavailable,
//! malformed JSON -> External (fail closed).

use std::time::Duration;

use nexus_sentinel::{SentinelError, SentinelErrorCode};

/// Normalized OPNsense firewall automation rule (documented searchRule
/// row shape).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpnsenseRule {
    /// Provider-side rule uuid.
    pub uuid: String,
    /// Provider-side description.
    pub description: String,
    /// Provider-side enabled flag (0/1 normalized to bool).
    pub enabled: bool,
    /// Provider-side action (pass|block|reject).
    pub action: String,
    /// Provider-side source network/alias (searchRule row
    /// source_net). AUD-026: verification reads this back to prove the
    /// rule binds the OBSERVED network identity, not just a rule id.
    pub source_net: Option<String>,
}

/// Normalized OPNsense rule creation payload (documented addRule body
/// `{"rule": {...}}`; Filter.xml fields).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpnsenseRulePayload {
    /// Rule description (used as the search key for readback).
    pub description: String,
    /// Action: pass|block|reject (Filter.xml OptionField).
    pub action: String,
    /// Direction: in|out|any (Filter.xml OptionField).
    pub direction: String,
    /// IP protocol: inet|inet6|inet46 (Filter.xml OptionField).
    pub ipprotocol: String,
    /// Protocol: any|tcp|udp|tcp/udp|icmp (Filter.xml ProtocolField).
    pub protocol: String,
    /// Source network/alias (Filter.xml NetworkAliasField).
    pub source_net: String,
    /// Destination network/alias (Filter.xml NetworkAliasField).
    pub destination_net: String,
}

impl OpnsenseRulePayload {
    /// Build a canonical quarantine/containment rule payload: block
    /// traffic for the given source network (device) in both
    /// directions, any protocol, IPv4+IPv6.
    pub fn containment_block(
        description: impl Into<String>,
        source_net: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            action: "block".into(),
            direction: "any".into(),
            ipprotocol: "inet46".into(),
            protocol: "any".into(),
            source_net: source_net.into(),
            destination_net: "any".into(),
        }
    }
}

/// Normalized OPNsense ARP table entry (documented
/// GET /api/diagnostics/interface/getArp; scripts/interfaces/list_arp.py
/// produces `{mac, ip, intf, expired, expires, permanent, type,
/// manufacturer, hostname}`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpnsenseArpEntry {
    /// MAC address reference.
    pub mac: String,
    /// IP address reference.
    pub ip: String,
    /// Interface name.
    pub intf: String,
    /// True when the kernel marked the entry expired.
    #[serde(default)]
    pub expired: bool,
    /// True for the firewall's own permanent interface entries.
    #[serde(default)]
    pub permanent: bool,
    /// Link type (e.g. "ethernet", "local").
    #[serde(default)]
    pub r#type: String,
    /// Manufacturer when the entry carries one.
    #[serde(default)]
    pub manufacturer: String,
    /// Hostname when the entry carries one.
    #[serde(default)]
    pub hostname: String,
}

/// The OPNsense transport port. Default implementations fail closed
/// (Unavailable) so an unbound transport never fabricates a session.
pub trait OpnsenseTransport {
    /// Search firewall automation rules (documented GET searchRule).
    fn search_rules(&self, phrase: &str) -> Result<Vec<OpnsenseRule>, SentinelError> {
        let _ = phrase;
        Err(SentinelError::unavailable(
            "opnsense transport has no implementation bound",
        ))
    }

    /// Read the ARP/neighbor table (documented GET
    /// /api/diagnostics/interface/getArp). The ARP table is the
    /// OBSERVED network inventory source (AUD-028): devices are
    /// discovered from what the router demonstrably sees.
    fn arp_table(&self) -> Result<Vec<OpnsenseArpEntry>, SentinelError> {
        Err(SentinelError::unavailable(
            "opnsense transport has no implementation bound",
        ))
    }

    /// Create a firewall automation rule (documented POST addRule).
    /// Returns the provider-side uuid.
    fn add_rule(&self, payload: &OpnsenseRulePayload) -> Result<String, SentinelError> {
        let _ = payload;
        Err(SentinelError::unavailable(
            "opnsense transport has no implementation bound",
        ))
    }

    /// Toggle a rule enabled/disabled (documented POST
    /// toggleRule/{uuid}/{enabled}).
    fn toggle_rule(&self, uuid: &str, enabled: bool) -> Result<(), SentinelError> {
        let _ = (uuid, enabled);
        Err(SentinelError::unavailable(
            "opnsense transport has no implementation bound",
        ))
    }

    /// Apply/reload the firewall (documented POST apply).
    fn apply(&self) -> Result<(), SentinelError> {
        Err(SentinelError::unavailable(
            "opnsense transport has no implementation bound",
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

/// Real blocking HTTP OPNsense transport over the documented firewall
/// automation API.
pub struct HttpOpnsenseTransport {
    client: reqwest::blocking::Client,
    base_url: String,
    /// OPNsense API key (Basic auth username). Used ONLY for the Basic
    /// auth header; never logged, never embedded in errors.
    api_key: String,
    /// OPNsense API secret (Basic auth password). Used ONLY for the
    /// Basic auth header; never logged, never embedded in errors.
    api_secret: String,
}

impl HttpOpnsenseTransport {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
        timeout: Duration,
    ) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client");
        Self {
            client,
            base_url: base_url.into(),
            api_key: api_key.into(),
            api_secret: api_secret.into(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn get(&self, path: &str) -> Result<reqwest::blocking::Response, SentinelError> {
        self.client
            .get(self.url(path))
            .basic_auth(&self.api_key, Some(&self.api_secret))
            .send()
            .map_err(Self::map_send_error)
    }

    fn post(&self, path: &str) -> Result<reqwest::blocking::Response, SentinelError> {
        self.client
            .post(self.url(path))
            .basic_auth(&self.api_key, Some(&self.api_secret))
            .send()
            .map_err(Self::map_send_error)
    }

    fn post_json(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<reqwest::blocking::Response, SentinelError> {
        self.client
            .post(self.url(path))
            .basic_auth(&self.api_key, Some(&self.api_secret))
            .json(body)
            .send()
            .map_err(Self::map_send_error)
    }

    fn map_send_error(e: reqwest::Error) -> SentinelError {
        if e.is_timeout() {
            SentinelError::new(
                SentinelErrorCode::Timeout,
                "opnsense transport timed out",
                None,
                None,
                None,
                None,
            )
        } else if e.is_connect() {
            SentinelError::new(
                SentinelErrorCode::Unavailable,
                "opnsense transport refused connection",
                None,
                None,
                None,
                None,
            )
        } else {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "opnsense transport request failed",
                None,
                None,
                None,
                None,
            )
        }
    }

    fn parse<T: serde::de::DeserializeOwned>(
        response: reqwest::blocking::Response,
    ) -> Result<T, SentinelError> {
        let status = response.status();
        if !status.is_success() {
            return Err(SentinelError::new(
                classify_status(status),
                format!("opnsense transport returned HTTP {}", status.as_u16()),
                None,
                None,
                None,
                None,
            ));
        }
        response.json::<T>().map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "opnsense transport returned malformed JSON",
                None,
                None,
                None,
                None,
            )
        })
    }

    fn parse_optional(
        response: reqwest::blocking::Response,
    ) -> Result<Option<serde_json::Value>, SentinelError> {
        let status = response.status();
        if !status.is_success() {
            return Err(SentinelError::new(
                classify_status(status),
                format!("opnsense transport returned HTTP {}", status.as_u16()),
                None,
                None,
                None,
                None,
            ));
        }
        let text = response.text().map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "opnsense transport returned unreadable body",
                None,
                None,
                None,
                None,
            )
        })?;
        if text.trim().is_empty() {
            return Ok(None);
        }
        serde_json::from_str(&text).map(Some).map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "opnsense transport returned malformed JSON",
                None,
                None,
                None,
                None,
            )
        })
    }
}

impl OpnsenseTransport for HttpOpnsenseTransport {
    fn arp_table(&self) -> Result<Vec<OpnsenseArpEntry>, SentinelError> {
        // Documented GET /api/diagnostics/interface/getArp -> bare
        // JSON array (scripts/interfaces/list_arp.py shape).
        let response = self.get("/api/diagnostics/interface/getArp")?;
        Self::parse(response)
    }

    fn search_rules(&self, phrase: &str) -> Result<Vec<OpnsenseRule>, SentinelError> {
        // Documented GET searchRule with current/rowCount/searchPhrase
        // query parameters (docs example).
        let path = format!(
            "/api/firewall/filter/searchRule?current=1&rowCount=100&searchPhrase={}",
            urlencode(phrase)
        );
        let response = self.get(&path)?;
        // Documented response: {"total":N,"rowCount":N,"current":1,
        // "rows":[{"uuid":...,"description":...,"enabled":1,...}]}
        #[derive(serde::Deserialize)]
        struct SearchResponse {
            rows: Vec<SearchRow>,
        }
        #[derive(serde::Deserialize)]
        struct SearchRow {
            uuid: String,
            #[serde(default)]
            description: String,
            #[serde(default)]
            enabled: Option<serde_json::Value>,
            #[serde(default)]
            action: Option<String>,
            #[serde(default)]
            source_net: Option<String>,
        }
        let parsed: SearchResponse = Self::parse(response)?;
        Ok(parsed
            .rows
            .into_iter()
            .map(|row| OpnsenseRule {
                uuid: row.uuid,
                description: row.description,
                enabled: match row.enabled {
                    Some(serde_json::Value::String(s)) => s == "1",
                    Some(serde_json::Value::Number(n)) => n.as_i64() == Some(1),
                    Some(serde_json::Value::Bool(b)) => b,
                    _ => false,
                },
                action: row.action.unwrap_or_default(),
                source_net: row.source_net,
            })
            .collect())
    }

    fn add_rule(&self, payload: &OpnsenseRulePayload) -> Result<String, SentinelError> {
        // Documented POST addRule with body {"rule": {...}}; response
        // {"uuid": "..."}.
        let body = serde_json::json!({
            "rule": {
                "description": payload.description,
                "action": payload.action,
                "direction": payload.direction,
                "ipprotocol": payload.ipprotocol,
                "protocol": payload.protocol,
                "source_net": payload.source_net,
                "destination_net": payload.destination_net,
            }
        });
        let response = self.post_json("/api/firewall/filter/addRule", &body)?;
        #[derive(serde::Deserialize)]
        struct AddResponse {
            uuid: Option<String>,
        }
        let parsed: AddResponse = Self::parse(response)?;
        parsed.uuid.ok_or_else(|| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "opnsense transport returned no uuid for created rule",
                None,
                None,
                None,
                None,
            )
        })
    }

    fn toggle_rule(&self, uuid: &str, enabled: bool) -> Result<(), SentinelError> {
        // Documented POST toggleRule/{uuid}/{enabled} (docs example
        // uses /0 to disable).
        let path = format!(
            "/api/firewall/filter/toggleRule/{}/{}",
            uuid,
            if enabled { "1" } else { "0" }
        );
        let response = self.post(&path)?;
        let _ = Self::parse_optional(response)?;
        Ok(())
    }

    fn apply(&self) -> Result<(), SentinelError> {
        // Documented POST apply (docs example reloads the firewall
        // after a change so the new ruleset becomes active).
        let response = self.post("/api/firewall/filter/apply")?;
        let _ = Self::parse_optional(response)?;
        Ok(())
    }
}

fn urlencode(s: &str) -> String {
    // Minimal RFC3986 percent-encoding for query values (spaces and
    // reserved characters). Kept small and dependency-free.
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep030_unit_transport_fails_closed_unbound() {
        struct Unbound;
        impl OpnsenseTransport for Unbound {}
        let t = Unbound;
        assert_eq!(
            t.search_rules("").unwrap_err().code,
            SentinelErrorCode::Unavailable
        );
        assert_eq!(
            t.add_rule(&OpnsenseRulePayload::containment_block("r", "10.0.0.0/24"))
                .unwrap_err()
                .code,
            SentinelErrorCode::Unavailable
        );
        assert_eq!(
            t.toggle_rule("uuid-1", false).unwrap_err().code,
            SentinelErrorCode::Unavailable
        );
        assert_eq!(t.apply().unwrap_err().code, SentinelErrorCode::Unavailable);
    }

    #[test]
    fn ep030_unit_transport_status_classification() {
        use reqwest::StatusCode;
        assert_eq!(
            classify_status(StatusCode::BAD_REQUEST),
            SentinelErrorCode::Validation
        );
        assert_eq!(
            classify_status(StatusCode::UNAUTHORIZED),
            SentinelErrorCode::Authorization
        );
        assert_eq!(
            classify_status(StatusCode::FORBIDDEN),
            SentinelErrorCode::Authorization
        );
        assert_eq!(
            classify_status(StatusCode::NOT_FOUND),
            SentinelErrorCode::NotFound
        );
        assert_eq!(
            classify_status(StatusCode::CONFLICT),
            SentinelErrorCode::Conflict
        );
        assert_eq!(
            classify_status(StatusCode::TOO_MANY_REQUESTS),
            SentinelErrorCode::RateLimit
        );
        assert_eq!(
            classify_status(StatusCode::INTERNAL_SERVER_ERROR),
            SentinelErrorCode::Unavailable
        );
    }

    #[test]
    fn ep030_unit_transport_containment_payload_is_block_both_directions() {
        // A containment rule must block traffic in both directions for
        // any protocol, IPv4+IPv6, from the device source network.
        let payload = OpnsenseRulePayload::containment_block("nexus-q-1", "192.0.2.10");
        assert_eq!(payload.action, "block");
        assert_eq!(payload.direction, "any");
        assert_eq!(payload.ipprotocol, "inet46");
        assert_eq!(payload.protocol, "any");
        assert_eq!(payload.source_net, "192.0.2.10");
        assert_eq!(payload.destination_net, "any");
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["action"], "block");
    }

    #[test]
    fn ep030_unit_transport_normalizes_documented_search_shape() {
        // The documented searchRule response rows are normalized at
        // the boundary into OpnsenseRule.
        let json = serde_json::json!({
            "total": 1,
            "rowCount": 1,
            "current": 1,
            "rows": [{
                "uuid": "abc-123",
                "description": "nexus-q-1",
                "enabled": true,
                "action": "block"
            }]
        });
        #[derive(serde::Deserialize)]
        struct Wrap {
            rows: Vec<OpnsenseRule>,
        }
        let parsed: Wrap = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.rows.len(), 1);
        assert_eq!(parsed.rows[0].uuid, "abc-123");
        assert!(parsed.rows[0].enabled);
        assert_eq!(parsed.rows[0].action, "block");
    }
}
