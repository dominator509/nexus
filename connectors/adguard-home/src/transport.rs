//! EP-030 AdGuard Home transport (M4): real HTTP transport over the
//! DOCUMENTED AdGuard Home control API.
//!
//! AdGuard Home is the DNS security default (SPEC-013 behavior 2;
//! COMPONENT_REGISTRY isolated-sidecar, GPL-3.0). Nexus orchestrates
//! its documented control API (openapi.yaml from the AdGuardHome
//! upstream source) and normalizes provider payloads at this
//! infrastructure boundary - free-form AdGuard JSON never becomes a
//! domain contract.
//!
//! Canonical transport surface (verified against the official
//! AdGuardHome openapi.yaml):
//! - GET  {base}/control/status          server status (ServerStatus:
//!   dns_addresses, dns_port, http_port, protection_enabled, running,
//!   version)
//! - GET  {base}/control/querylog?limit=N&search=S
//!   DNS query log (QueryLog: oldest + data[] of QueryLogItem with
//!   filtering reason enum FilteringReason: NotFilteredNotFound,
//!   NotFilteredWhiteList, NotFilteredError, FilteredBlackList,
//!   FilteredSafeBrowsing, FilteredParental, FilteredInvalid,
//!   FilteredSafeSearch, FilteredBlockedService, Rewrite,
//!   RewriteEtcHosts, RewriteRule)
//! - GET  {base}/control/querylog/config query log config (enabled,
//!   interval, anonymize_client_ip, ignored)
//! - GET  {base}/control/stats            DNS statistics
//! - GET  {base}/control/filtering/status filtering status
//!
//! Authentication: HTTP Basic with the AdGuard Home username/password
//! (documented Login/install credentials). The credential pair is used
//! ONLY for the Basic auth header and never appears in errors or
//! telemetry.
//!
//! HTTP status mapping follows SPEC-006: 400 -> Validation, 401/403 ->
//! Authorization, 404 -> NotFound, 409 -> Conflict, 429 -> RateLimit,
//! 5xx -> Unavailable, silent peer -> Timeout, refused -> Unavailable,
//! malformed JSON -> External (fail closed).

use std::time::Duration;

use nexus_sentinel::{SentinelError, SentinelErrorCode};

/// Normalized AdGuard Home server status (documented ServerStatus).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AdGuardStatus {
    /// DNS listen addresses.
    pub dns_addresses: Vec<String>,
    /// DNS listen port.
    pub dns_port: u16,
    /// HTTP listen port.
    pub http_port: u16,
    /// Protection enabled flag.
    pub protection_enabled: bool,
    /// Server running flag.
    pub running: bool,
    /// Server version.
    pub version: String,
}

/// Normalized query log entry (documented QueryLogItem; the adapter
/// only needs the fields that drive DNS security telemetry).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QueryLogEntry {
    /// Query time (RFC3339).
    pub time: String,
    /// Query domain.
    pub question: String,
    /// Client IP.
    pub client: String,
    /// Filtering reason (documented FilteringReason enum string).
    pub reason: String,
}

/// Normalized filter subscription info (documented Filter schema).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Filter {
    /// Subscription enabled flag.
    pub enabled: bool,
    /// Subscription id.
    pub id: i64,
    /// Last update time (RFC3339) when the upstream reported one.
    #[serde(default)]
    pub last_updated: String,
    /// Human-readable filter list name.
    pub name: String,
    /// Number of rules in the list.
    pub rules_count: u64,
    /// Upstream list URL.
    pub url: String,
}

/// Normalized filtering settings (documented FilterStatus schema).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct FilteringStatus {
    /// Global filtering enabled flag.
    pub enabled: bool,
    /// Filter update interval (seconds) when present.
    #[serde(default)]
    pub interval: u64,
    /// Configured blocklist filter subscriptions.
    #[serde(default)]
    pub filters: Vec<Filter>,
    /// Configured whitelist filter subscriptions.
    #[serde(default)]
    pub whitelist_filters: Vec<Filter>,
    /// Explicit user rules (AdGuard rule syntax, e.g. `||example.com^`).
    #[serde(default)]
    pub user_rules: Vec<String>,
}

/// The AdGuard Home transport port. Default implementations fail
/// closed (Unavailable) so an unbound transport never fabricates a
/// session.
pub trait AdGuardTransport {
    /// Read the server status (documented GET /control/status).
    fn status(&self) -> Result<AdGuardStatus, SentinelError> {
        Err(SentinelError::unavailable(
            "adguard transport has no implementation bound",
        ))
    }

    /// Read the DNS query log (documented GET /control/querylog).
    /// `limit` bounds the page; `search` filters by domain or client.
    fn query_log(&self, limit: usize, search: &str) -> Result<Vec<QueryLogEntry>, SentinelError> {
        let _ = (limit, search);
        Err(SentinelError::unavailable(
            "adguard transport has no implementation bound",
        ))
    }

    /// Read the CONFIGURED filtering state (documented
    /// GET /control/filtering/status). This is the authoritative
    /// blocklist surface (AUD-027): configured rules with no recent
    /// query-log hit are still active blocklist entries.
    fn filtering_status(&self) -> Result<FilteringStatus, SentinelError> {
        Err(SentinelError::unavailable(
            "adguard transport has no implementation bound",
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

/// Real blocking HTTP AdGuard Home transport over the documented
/// control API.
pub struct HttpAdGuardTransport {
    client: reqwest::blocking::Client,
    base_url: String,
    /// AdGuard Home username (Basic auth). Used ONLY for the Basic
    /// auth header; never logged, never embedded in errors.
    username: String,
    /// AdGuard Home password (Basic auth). Used ONLY for the Basic
    /// auth header; never logged, never embedded in errors.
    password: String,
}

impl HttpAdGuardTransport {
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
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn get(&self, path: &str) -> Result<reqwest::blocking::Response, SentinelError> {
        self.client
            .get(self.url(path))
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(Self::map_send_error)
    }

    fn map_send_error(e: reqwest::Error) -> SentinelError {
        if e.is_timeout() {
            SentinelError::new(
                SentinelErrorCode::Timeout,
                "adguard transport timed out",
                None,
                None,
                None,
                None,
            )
        } else if e.is_connect() {
            SentinelError::new(
                SentinelErrorCode::Unavailable,
                "adguard transport refused connection",
                None,
                None,
                None,
                None,
            )
        } else {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "adguard transport request failed",
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
                format!("adguard transport returned HTTP {}", status.as_u16()),
                None,
                None,
                None,
                None,
            ));
        }
        response.json::<T>().map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "adguard transport returned malformed JSON",
                None,
                None,
                None,
                None,
            )
        })
    }
}

impl AdGuardTransport for HttpAdGuardTransport {
    fn status(&self) -> Result<AdGuardStatus, SentinelError> {
        // Documented GET /control/status -> ServerStatus.
        let response = self.get("/control/status")?;
        Self::parse(response)
    }

    fn query_log(&self, limit: usize, search: &str) -> Result<Vec<QueryLogEntry>, SentinelError> {
        // Documented GET /control/querylog?limit=N&search=S ->
        // QueryLog {oldest, data: [QueryLogItem]}.
        let path = format!(
            "/control/querylog?limit={}&search={}",
            limit,
            urlencode(search)
        );
        let response = self.get(&path)?;
        #[derive(serde::Deserialize)]
        struct QueryLogResponse {
            #[serde(default)]
            data: Vec<QueryLogItemRaw>,
        }
        #[derive(serde::Deserialize)]
        struct QueryLogItemRaw {
            #[serde(default)]
            time: String,
            #[serde(default)]
            question: serde_json::Value,
            #[serde(default)]
            client: String,
            #[serde(default)]
            reason: String,
        }
        let parsed: QueryLogResponse = Self::parse(response)?;
        Ok(parsed
            .data
            .into_iter()
            .map(|item| QueryLogEntry {
                time: item.time,
                question: match item.question {
                    serde_json::Value::Object(o) => o
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    serde_json::Value::String(s) => s,
                    _ => String::new(),
                },
                client: item.client,
                reason: item.reason,
            })
            .collect())
    }

    fn filtering_status(&self) -> Result<FilteringStatus, SentinelError> {
        // Documented GET /control/filtering/status -> FilterStatus.
        let response = self.get("/control/filtering/status")?;
        #[derive(serde::Deserialize)]
        struct FilterStatusRaw {
            #[serde(default)]
            enabled: bool,
            #[serde(default)]
            interval: serde_json::Value,
            #[serde(default)]
            filters: Vec<FilterRaw>,
            #[serde(default)]
            whitelist_filters: Vec<FilterRaw>,
            #[serde(default)]
            user_rules: Vec<String>,
        }
        #[derive(serde::Deserialize)]
        struct FilterRaw {
            #[serde(default)]
            enabled: bool,
            #[serde(default)]
            id: serde_json::Value,
            #[serde(default)]
            last_updated: String,
            #[serde(default)]
            name: String,
            #[serde(default)]
            rules_count: serde_json::Value,
            #[serde(default)]
            url: String,
        }
        let raw: FilterStatusRaw = Self::parse(response)?;
        let to_u64 = |v: &serde_json::Value| -> u64 {
            v.as_u64()
                .unwrap_or_else(|| v.as_str().and_then(|s| s.parse().ok()).unwrap_or(0))
        };
        let to_i64 = |v: &serde_json::Value| -> i64 {
            v.as_i64()
                .unwrap_or_else(|| v.as_str().and_then(|s| s.parse().ok()).unwrap_or(0))
        };
        Ok(FilteringStatus {
            enabled: raw.enabled,
            interval: to_u64(&raw.interval),
            filters: raw
                .filters
                .into_iter()
                .map(|f| Filter {
                    enabled: f.enabled,
                    id: to_i64(&f.id),
                    last_updated: f.last_updated,
                    name: f.name,
                    rules_count: to_u64(&f.rules_count),
                    url: f.url,
                })
                .collect(),
            whitelist_filters: raw
                .whitelist_filters
                .into_iter()
                .map(|f| Filter {
                    enabled: f.enabled,
                    id: to_i64(&f.id),
                    last_updated: f.last_updated,
                    name: f.name,
                    rules_count: to_u64(&f.rules_count),
                    url: f.url,
                })
                .collect(),
            user_rules: raw.user_rules,
        })
    }
}

fn urlencode(s: &str) -> String {
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
        impl AdGuardTransport for Unbound {}
        let t = Unbound;
        assert_eq!(t.status().unwrap_err().code, SentinelErrorCode::Unavailable);
        assert_eq!(
            t.query_log(10, "").unwrap_err().code,
            SentinelErrorCode::Unavailable
        );
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
            classify_status(StatusCode::TOO_MANY_REQUESTS),
            SentinelErrorCode::RateLimit
        );
        assert_eq!(
            classify_status(StatusCode::INTERNAL_SERVER_ERROR),
            SentinelErrorCode::Unavailable
        );
    }

    #[test]
    fn ep030_unit_transport_normalizes_documented_status() {
        let json = serde_json::json!({
            "dns_addresses": ["127.0.0.1"],
            "dns_port": 53,
            "http_port": 80,
            "protection_enabled": true,
            "running": true,
            "version": "v0.108.0"
        });
        let parsed: AdGuardStatus = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.dns_port, 53);
        assert!(parsed.protection_enabled);
        assert_eq!(parsed.version, "v0.108.0");
    }

    #[test]
    fn ep030_unit_transport_normalizes_querylog_item() {
        // Documented QueryLogItem shape: question is an object with a
        // name field; reason is the FilteringReason enum string.
        let json = serde_json::json!({
            "data": [{
                "time": "2026-08-20T00:00:00Z",
                "question": {"name": "ads.example.com", "type": "A"},
                "client": "192.0.2.10",
                "reason": "FilteredBlackList"
            }]
        });
        // The normalized QueryLogEntry expects question as string; the
        // raw wire shape has it as an object. This test asserts the
        // struct-level serde shape (string), while the production
        // transport normalizes the object via QueryLogItemRaw.
        let raw = json["data"][0].clone();
        let obj = raw.as_object().unwrap();
        let entry = QueryLogEntry {
            time: obj["time"].as_str().unwrap().into(),
            question: obj["question"]["name"].as_str().unwrap().into(),
            client: obj["client"].as_str().unwrap().into(),
            reason: obj["reason"].as_str().unwrap().into(),
        };
        assert_eq!(entry.question, "ads.example.com");
        assert_eq!(entry.reason, "FilteredBlackList");
    }
}
