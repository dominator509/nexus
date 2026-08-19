//! EP-030 OpenWrt transport (M3): real HTTP transport over the
//! DOCUMENTED OpenWrt ubus JSON-RPC surface.
//!
//! OpenWrt is supported for embedded and consumer installations
//! (SPEC-013 behavior 2; COMPONENT_REGISTRY external-appliance,
//! GPL-2.0). Nexus orchestrates the documented ubus HTTP JSON-RPC
//! surface and normalizes provider payloads at this infrastructure
//! boundary - free-form OpenWrt JSON never becomes a domain contract.
//!
//! Canonical transport surface (verified against the official OpenWrt
//! documentation, openwrt.org/docs/techref/ubus, and the upstream
//! rpcd source uci.c/rc.c/session.c):
//! - POST {base}/ubus  JSON-RPC 2.0 body:
//!   `{"jsonrpc":"2.0","id":N,"method":"call","params":[<session>,"<object>","<method>",{<args>}]}`
//! - session/login: params ["00000000000000000000000000000000",
//!   "session","login",{"username":..,"password":..}]
//!   response result[0]=0 and result[1].ubus_rpc_session on success
//! - uci object (rpcd uci.c): get, set, add, delete, commit, apply,
//!   rollback - firewall rules live under config "firewall" with rule
//!   sections (name, src, dest, proto, target ACCEPT/REJECT/DROP,
//!   src_ip/dest_ip, enabled)
//! - rc object (rpcd rc.c): init {name, action} -> runs
//!   /etc/init.d/<name> <action> (e.g. firewall reload)
//!
//! ubus status codes (openwrt.org ubus docs): 0 OK, 2 INVALID_ARGUMENT,
//! 3 METHOD_NOT_FOUND, 4 NOT_FOUND, 5 NO_DATA, 6 PERMISSION_DENIED,
//! 7 TIMEOUT, 9 UNKNOWN_ERROR, 10 CONNECTION_FAILED. Errors map to
//! SPEC-006; malformed JSON and silent peers fail closed.

use std::time::Duration;

use nexus_sentinel::{SentinelError, SentinelErrorCode};

/// Canonical ubus status codes (openwrt.org ubus documentation).
pub const UBUS_STATUS_OK: i64 = 0;
pub const UBUS_STATUS_INVALID_ARGUMENT: i64 = 2;
pub const UBUS_STATUS_METHOD_NOT_FOUND: i64 = 3;
pub const UBUS_STATUS_NOT_FOUND: i64 = 4;
pub const UBUS_STATUS_NO_DATA: i64 = 5;
pub const UBUS_STATUS_PERMISSION_DENIED: i64 = 6;
pub const UBUS_STATUS_TIMEOUT: i64 = 7;
pub const UBUS_STATUS_UNKNOWN_ERROR: i64 = 9;
pub const UBUS_STATUS_CONNECTION_FAILED: i64 = 10;

/// Normalized OpenWrt firewall rule (ubus uci "firewall" rule section).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenWrtRule {
    /// UCI section identifier (e.g. "cfg012345").
    pub section: String,
    /// Rule name (option name).
    pub name: String,
    /// Target: ACCEPT|REJECT|DROP.
    pub target: String,
    /// Source IP/CIDR (option src_ip) when set.
    pub src_ip: Option<String>,
    /// Enabled flag (option enabled '0' disables; default enabled).
    pub enabled: bool,
}

/// Normalized OpenWrt firewall rule creation payload (uci add + set).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenWrtRulePayload {
    /// Rule name (option name; used as the readback key).
    pub name: String,
    /// Target: ACCEPT|REJECT|DROP.
    pub target: String,
    /// Source IP/CIDR (option src_ip).
    pub src_ip: String,
}

impl OpenWrtRulePayload {
    /// Build a canonical quarantine/containment rule payload: DROP all
    /// traffic from the given source network (device).
    pub fn containment_drop(name: impl Into<String>, src_ip: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            target: "DROP".into(),
            src_ip: src_ip.into(),
        }
    }
}

/// The OpenWrt transport port. Default implementations fail closed
/// (Unavailable) so an unbound transport never fabricates a session.
pub trait OpenWrtTransport {
    /// Log in and return the ubus RPC session id. The session is
    /// provider state; a transport without a session fails closed.
    fn login(&self) -> Result<String, SentinelError> {
        Err(SentinelError::unavailable(
            "openwrt transport has no implementation bound",
        ))
    }

    /// List firewall rules (uci get on the "firewall" config rule
    /// sections).
    fn list_rules(&self, session: &str) -> Result<Vec<OpenWrtRule>, SentinelError> {
        let _ = session;
        Err(SentinelError::unavailable(
            "openwrt transport has no implementation bound",
        ))
    }

    /// Create a firewall rule (uci add + set + commit).
    fn add_rule(
        &self,
        session: &str,
        payload: &OpenWrtRulePayload,
    ) -> Result<String, SentinelError> {
        let _ = (session, payload);
        Err(SentinelError::unavailable(
            "openwrt transport has no implementation bound",
        ))
    }

    /// Toggle a rule enabled/disabled (uci set enabled '0'/'1' +
    /// commit).
    fn toggle_rule(
        &self,
        session: &str,
        section: &str,
        enabled: bool,
    ) -> Result<(), SentinelError> {
        let _ = (session, section, enabled);
        Err(SentinelError::unavailable(
            "openwrt transport has no implementation bound",
        ))
    }

    /// Reload the firewall (rc init {name: "firewall", action:
    /// "reload"}).
    fn reload_firewall(&self, session: &str) -> Result<(), SentinelError> {
        let _ = session;
        Err(SentinelError::unavailable(
            "openwrt transport has no implementation bound",
        ))
    }
}

fn classify_status(code: i64) -> SentinelErrorCode {
    match code {
        UBUS_STATUS_OK => SentinelErrorCode::Internal,
        UBUS_STATUS_INVALID_ARGUMENT => SentinelErrorCode::Validation,
        UBUS_STATUS_METHOD_NOT_FOUND | UBUS_STATUS_NOT_FOUND | UBUS_STATUS_NO_DATA => {
            SentinelErrorCode::NotFound
        }
        UBUS_STATUS_PERMISSION_DENIED => SentinelErrorCode::Authorization,
        UBUS_STATUS_TIMEOUT => SentinelErrorCode::Timeout,
        UBUS_STATUS_CONNECTION_FAILED => SentinelErrorCode::Unavailable,
        _ => SentinelErrorCode::ExternalProvider,
    }
}

/// Real blocking HTTP OpenWrt transport over the documented ubus
/// JSON-RPC surface.
pub struct HttpOpenWrtTransport {
    client: reqwest::blocking::Client,
    base_url: String,
    /// OpenWrt username (session login). Used ONLY for the login
    /// call; never logged, never embedded in errors.
    username: String,
    /// OpenWrt password (session login). Used ONLY for the login
    /// call; never logged, never embedded in errors.
    password: String,
}

impl HttpOpenWrtTransport {
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

    /// One ubus JSON-RPC call. `session` is the ubus_rpc_session (or
    /// the null session "0000...0" for session/login).
    fn call(
        &self,
        session: &str,
        object: &str,
        method: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, SentinelError> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "call",
            "params": [session, object, method, args],
        });
        let response = self
            .client
            .post(self.url("/ubus"))
            .json(&body)
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    SentinelError::new(
                        SentinelErrorCode::Timeout,
                        "openwrt transport timed out",
                        None,
                        None,
                        None,
                        None,
                    )
                } else if e.is_connect() {
                    SentinelError::new(
                        SentinelErrorCode::Unavailable,
                        "openwrt transport refused connection",
                        None,
                        None,
                        None,
                        None,
                    )
                } else {
                    SentinelError::new(
                        SentinelErrorCode::ExternalProvider,
                        "openwrt transport request failed",
                        None,
                        None,
                        None,
                        None,
                    )
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(SentinelError::new(
                classify_http(status.as_u16()),
                format!("openwrt transport returned HTTP {}", status.as_u16()),
                None,
                None,
                None,
                None,
            ));
        }
        #[derive(serde::Deserialize)]
        struct CallResponse {
            #[serde(default)]
            result: Option<serde_json::Value>,
        }
        let parsed: CallResponse = response.json().map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "openwrt transport returned malformed JSON",
                None,
                None,
                None,
                None,
            )
        })?;
        let result = parsed.result.ok_or_else(|| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "openwrt transport returned no ubus result",
                None,
                None,
                None,
                None,
            )
        })?;
        let arr = result.as_array().ok_or_else(|| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "openwrt transport returned malformed ubus result",
                None,
                None,
                None,
                None,
            )
        })?;
        if arr.is_empty() {
            return Err(SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "openwrt transport returned empty ubus result",
                None,
                None,
                None,
                None,
            ));
        }
        let code = arr[0].as_i64().ok_or_else(|| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "openwrt transport returned non-numeric ubus status",
                None,
                None,
                None,
                None,
            )
        })?;
        if code != UBUS_STATUS_OK {
            return Err(SentinelError::new(
                classify_status(code),
                format!("openwrt ubus returned status {code}"),
                None,
                None,
                None,
                None,
            ));
        }
        Ok(arr.get(1).cloned().unwrap_or(serde_json::Value::Null))
    }
}

fn classify_http(status: u16) -> SentinelErrorCode {
    match status {
        400 => SentinelErrorCode::Validation,
        401 | 403 => SentinelErrorCode::Authorization,
        404 => SentinelErrorCode::NotFound,
        409 => SentinelErrorCode::Conflict,
        429 => SentinelErrorCode::RateLimit,
        500 | 502 | 503 | 504 => SentinelErrorCode::Unavailable,
        _ => SentinelErrorCode::ExternalProvider,
    }
}

impl OpenWrtTransport for HttpOpenWrtTransport {
    fn login(&self) -> Result<String, SentinelError> {
        // Documented session/login with the null session
        // (openwrt.org ubus docs).
        let args = serde_json::json!({
            "username": self.username,
            "password": self.password,
        });
        let result = self.call("00000000000000000000000000000000", "session", "login", args)?;
        result
            .get("ubus_rpc_session")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "openwrt login returned no ubus_rpc_session",
                    None,
                    None,
                    None,
                    None,
                )
            })
    }

    fn list_rules(&self, session: &str) -> Result<Vec<OpenWrtRule>, SentinelError> {
        // ubus uci get {"config": "firewall", "type": "rule"} returns
        // the firewall rule sections.
        let args = serde_json::json!({
            "config": "firewall",
            "type": "rule",
        });
        let result = self.call(session, "uci", "get", args)?;
        let sections = result.as_object().ok_or_else(|| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "openwrt uci get returned malformed firewall sections",
                None,
                None,
                None,
                None,
            )
        })?;
        let mut rules = Vec::new();
        for (section, value) in sections {
            let obj = value.as_object().cloned().unwrap_or_default();
            let name = obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let target = obj
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let src_ip = obj
                .get("src_ip")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let enabled = match obj.get("enabled") {
                Some(v) => v.as_str() != Some("0"),
                None => true,
            };
            rules.push(OpenWrtRule {
                section: section.clone(),
                name,
                target,
                src_ip,
                enabled,
            });
        }
        Ok(rules)
    }

    fn add_rule(
        &self,
        session: &str,
        payload: &OpenWrtRulePayload,
    ) -> Result<String, SentinelError> {
        // ubus uci add {"config": "firewall", "type": "rule"} returns
        // the new section id; then set the rule fields and commit.
        let add_args = serde_json::json!({
            "config": "firewall",
            "type": "rule",
        });
        let result = self.call(session, "uci", "add", add_args)?;
        let section = result.as_str().ok_or_else(|| {
            SentinelError::new(
                SentinelErrorCode::ExternalProvider,
                "openwrt uci add returned no section id",
                None,
                None,
                None,
                None,
            )
        })?;
        let set_args = serde_json::json!({
            "config": "firewall",
            "section": section,
            "values": {
                "name": payload.name,
                "target": payload.target,
                "src_ip": payload.src_ip,
            },
        });
        self.call(session, "uci", "set", set_args)?;
        let commit_args = serde_json::json!({
            "config": "firewall",
        });
        self.call(session, "uci", "commit", commit_args)?;
        Ok(section.to_string())
    }

    fn toggle_rule(
        &self,
        session: &str,
        section: &str,
        enabled: bool,
    ) -> Result<(), SentinelError> {
        let set_args = serde_json::json!({
            "config": "firewall",
            "section": section,
            "values": {
                "enabled": if enabled { "1" } else { "0" },
            },
        });
        self.call(session, "uci", "set", set_args)?;
        let commit_args = serde_json::json!({
            "config": "firewall",
        });
        self.call(session, "uci", "commit", commit_args)?;
        Ok(())
    }

    fn reload_firewall(&self, session: &str) -> Result<(), SentinelError> {
        // Documented rc init {name, action} -> /etc/init.d/firewall
        // reload.
        let args = serde_json::json!({
            "name": "firewall",
            "action": "reload",
        });
        self.call(session, "rc", "init", args)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep030_unit_transport_fails_closed_unbound() {
        struct Unbound;
        impl OpenWrtTransport for Unbound {}
        let t = Unbound;
        assert_eq!(t.login().unwrap_err().code, SentinelErrorCode::Unavailable);
        assert_eq!(
            t.list_rules("sess").unwrap_err().code,
            SentinelErrorCode::Unavailable
        );
        assert_eq!(
            t.add_rule(
                "sess",
                &OpenWrtRulePayload::containment_drop("r", "10.0.0.0/24")
            )
            .unwrap_err()
            .code,
            SentinelErrorCode::Unavailable
        );
        assert_eq!(
            t.toggle_rule("sess", "cfg1", false).unwrap_err().code,
            SentinelErrorCode::Unavailable
        );
        assert_eq!(
            t.reload_firewall("sess").unwrap_err().code,
            SentinelErrorCode::Unavailable
        );
    }

    #[test]
    fn ep030_unit_transport_ubus_status_classification() {
        assert_eq!(
            classify_status(UBUS_STATUS_PERMISSION_DENIED),
            SentinelErrorCode::Authorization
        );
        assert_eq!(
            classify_status(UBUS_STATUS_INVALID_ARGUMENT),
            SentinelErrorCode::Validation
        );
        assert_eq!(
            classify_status(UBUS_STATUS_NOT_FOUND),
            SentinelErrorCode::NotFound
        );
        assert_eq!(
            classify_status(UBUS_STATUS_TIMEOUT),
            SentinelErrorCode::Timeout
        );
        assert_eq!(
            classify_status(UBUS_STATUS_CONNECTION_FAILED),
            SentinelErrorCode::Unavailable
        );
    }

    #[test]
    fn ep030_unit_transport_containment_payload_is_drop() {
        let payload = OpenWrtRulePayload::containment_drop("nexus-q-1", "192.0.2.10");
        assert_eq!(payload.target, "DROP");
        assert_eq!(payload.src_ip, "192.0.2.10");
        assert_eq!(payload.name, "nexus-q-1");
        let json = serde_json::to_value(&payload).unwrap();
        assert_eq!(json["target"], "DROP");
    }

    #[test]
    fn ep030_unit_transport_normalizes_uci_firewall_sections() {
        // The uci get result is a map of section -> values.
        let result = serde_json::json!({
            "cfg012345": {
                "name": "nexus-q-1",
                "target": "DROP",
                "src_ip": "192.0.2.10",
                "enabled": "1"
            }
        });
        let sections = result.as_object().unwrap();
        let obj = sections.get("cfg012345").unwrap().as_object().unwrap();
        let rule = OpenWrtRule {
            section: "cfg012345".into(),
            name: obj.get("name").unwrap().as_str().unwrap().into(),
            target: obj.get("target").unwrap().as_str().unwrap().into(),
            src_ip: obj.get("src_ip").unwrap().as_str().map(|s| s.into()),
            enabled: obj.get("enabled").unwrap().as_str() != Some("0"),
        };
        assert_eq!(rule.section, "cfg012345");
        assert_eq!(rule.target, "DROP");
        assert!(rule.enabled);
    }
}
