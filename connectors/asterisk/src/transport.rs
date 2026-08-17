//! EP-025 Asterisk transport port and real ARI HTTP client (SPEC-014;
//! M2).
//!
//! Asterisk 22 LTS is the telephony gateway. Nexus orchestrates
//! Asterisk through the real ARI (Asterisk REST Interface) surface;
//! it does NOT implement SIP signaling, RTP, codecs, TLS, or SRTP
//! itself (directive 3). The transport is provider-neutral behind a
//! port so future gateways keep the same Nexus call semantics.
//!
//! Endpoints exercised are the DOCUMENTED Asterisk 22 ARI surface:
//! - GET  /ari/asterisk/info                     (health)
//! - GET  /ari/channels                          (list)
//! - GET  /ari/channels/{id}                     (channel state)
//! - POST /ari/channels                          (originate)
//! - POST /ari/channels/{id}/answer              (answer)
//! - DELETE /ari/channels/{id}                   (hangup)
//! - POST /ari/channels/{id}/bridge              (bridge to bridge id)
//! - POST /ari/channels/{id}/continue            (continue dialplan)
//! - POST /ari/channels/{id}/play                (playback/media)
//! - POST /ari/channels/{id}/dtmf                (send DTMF)
//!
//! Error classification (mirrors EP-023/EP-024 transport discipline):
//! - HTTP 401/403 -> Authorization
//! - HTTP 404     -> NotFound
//! - HTTP 500/502/503 -> Unavailable
//! - silent peer  -> Timeout (bounded request timeout)
//! - refused peer -> Unavailable
//! - malformed JSON -> External (fail closed)
//!
//! A fake Asterisk HTTP server may support PARSER FAILURE tests only
//! (directive 2); it never certifies Asterisk integration. Real
//! integration certification happens in M3 with a real Asterisk
//! container.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use nexus_telephony::{CallError, CallErrorCode, SipEndpointId};

/// Canonical ARI channel object (DOCUMENTED Asterisk 22 ARI model).
/// Only fields Nexus needs; unknown fields are ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AriChannel {
    pub id: String,
    pub name: String,
    /// Real Asterisk channel state (e.g. "Up", "Ring", "Down",
    /// "Ringing", "Busy").
    pub state: String,
    #[serde(default)]
    pub caller: Option<AriCallerId>,
    #[serde(default)]
    pub connected: Option<AriCallerId>,
    #[serde(default)]
    pub dialplan: Option<AriDialplan>,
    /// Real Asterisk bridge id when the channel is bridged (present
    /// in the ARI channel object only when actually bridged).
    #[serde(default)]
    pub bridge: Option<String>,
    #[serde(default)]
    pub creationtime: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
}

/// ARI caller id object (advisory identity - NEVER authorization;
/// directive 16).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AriCallerId {
    pub name: String,
    pub number: String,
}

/// ARI dialplan location object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AriDialplan {
    pub context: String,
    pub exten: String,
    pub priority: i64,
}

/// Canonical session/leg selector: the real Asterisk channel id bound
/// to a CallSession.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChannelSelector {
    channel_id: String,
}

impl ChannelSelector {
    pub fn new(channel_id: impl Into<String>) -> Result<Self, CallError> {
        let channel_id = channel_id.into();
        if channel_id.is_empty() || channel_id.len() > 256 {
            return Err(CallError::validation(
                "channel_id must be 1..=256 characters",
            ));
        }
        Ok(Self { channel_id })
    }

    pub fn as_str(&self) -> &str {
        &self.channel_id
    }
}

/// ARI transport port (fail-closed defaults; Reality rule).
///
/// Unbound transports never fabricate channels, call states, or
/// command acceptance.
pub trait AriTransport {
    /// Asterisk ARI health (GET /ari/asterisk/info).
    fn health(&self) -> Result<(), CallError> {
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// List real channels (GET /ari/channels).
    fn list_channels(&self) -> Result<Vec<AriChannel>, CallError> {
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Channel state (GET /ari/channels/{id}).
    fn channel_state(&self, channel: &ChannelSelector) -> Result<AriChannel, CallError> {
        let _ = channel;
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Originate a call to a PJSIP endpoint (POST /ari/channels).
    fn originate(
        &self,
        endpoint: &SipEndpointId,
        context: &str,
        extension: &str,
        caller_id: Option<&str>,
    ) -> Result<AriChannel, CallError> {
        let _ = (endpoint, context, extension, caller_id);
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Answer a channel (POST /ari/channels/{id}/answer).
    fn answer(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        let _ = channel;
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Hangup a channel (DELETE /ari/channels/{id}).
    fn hangup(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        let _ = channel;
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Bridge a channel into an existing bridge
    /// (POST /ari/channels/{id}/bridge).
    fn bridge(&self, channel: &ChannelSelector, bridge_id: &str) -> Result<(), CallError> {
        let _ = (channel, bridge_id);
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Continue a channel in the dialplan
    /// (POST /ari/channels/{id}/continue).
    fn r#continue(
        &self,
        channel: &ChannelSelector,
        context: &str,
        extension: &str,
    ) -> Result<(), CallError> {
        let _ = (channel, context, extension);
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Send DTMF digits (POST /ari/channels/{id}/dtmf).
    fn send_dtmf(&self, channel: &ChannelSelector, digits: &str) -> Result<(), CallError> {
        let _ = (channel, digits);
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Start music on hold (POST /ari/channels/{id}/moh).
    fn start_moh(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        let _ = channel;
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Stop music on hold (DELETE /ari/channels/{id}/moh).
    fn stop_moh(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        let _ = channel;
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }

    /// Redirect a channel to a dialplan location
    /// (POST /ari/channels/{id}/redirect).
    fn redirect(
        &self,
        channel: &ChannelSelector,
        context: &str,
        extension: &str,
    ) -> Result<(), CallError> {
        let _ = (channel, context, extension);
        Err(CallError::unavailable(
            "ari transport has no implementation bound",
        ))
    }
}

/// Real ARI HTTP client over the DOCUMENTED Asterisk 22 REST surface.
///
/// Authentication: HTTP Basic with the ARI user/password (Asterisk
/// ari.conf). The credential is used ONLY for the Authorization
/// header and is never placed in errors, audit entries, or metrics.
#[derive(Debug, Clone)]
pub struct RestAriTransport {
    base_url: String,
    username: String,
    password: String,
    client: reqwest::blocking::Client,
}

impl RestAriTransport {
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, CallError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err(CallError::validation("base_url must not be empty"));
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
            .map_err(|e| CallError::external(format!("http client build failed: {e}")))?;
        Ok(Self {
            base_url,
            username: username.into(),
            password: password.into(),
            client,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn get_json(&self, path: &str) -> Result<Value, CallError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    CallError::timeout(format!("ari request timed out: {path}"))
                } else if e.is_connect() {
                    CallError::unavailable(format!("ari connect failed: {path}: {e}"))
                } else {
                    CallError::external(format!("ari request failed: {path}: {e}"))
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(classify_status(status.as_u16(), path));
        }
        response
            .json::<Value>()
            .map_err(|e| CallError::external(format!("ari malformed JSON response on {path}: {e}")))
    }

    fn post_json(&self, path: &str, params: &[(&str, String)]) -> Result<Value, CallError> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self
            .client
            .post(&url)
            .basic_auth(&self.username, Some(&self.password));
        for (key, value) in params {
            request = request.query(&[(key, value.as_str())]);
        }
        let response = request.send().map_err(|e| {
            if e.is_timeout() {
                CallError::timeout(format!("ari request timed out: {path}"))
            } else if e.is_connect() {
                CallError::unavailable(format!("ari connect failed: {path}: {e}"))
            } else {
                CallError::external(format!("ari request failed: {path}: {e}"))
            }
        })?;
        let status = response.status();
        if !status.is_success() {
            return Err(classify_status(status.as_u16(), path));
        }
        response
            .json::<Value>()
            .map_err(|e| CallError::external(format!("ari malformed JSON response on {path}: {e}")))
    }

    fn delete(&self, path: &str) -> Result<(), CallError> {
        let url = format!("{}{}", self.base_url, path);
        let response = self
            .client
            .delete(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    CallError::timeout(format!("ari request timed out: {path}"))
                } else if e.is_connect() {
                    CallError::unavailable(format!("ari connect failed: {path}: {e}"))
                } else {
                    CallError::external(format!("ari request failed: {path}: {e}"))
                }
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(classify_status(status.as_u16(), path));
        }
        Ok(())
    }
}

/// Classify ARI HTTP status into a typed telephony error.
fn classify_status(status: u16, path: &str) -> CallError {
    match status {
        401 | 403 => CallError::new(
            CallErrorCode::Authorization,
            format!("ari authentication rejected on {path}"),
            None,
            None,
        ),
        404 => CallError::new(
            CallErrorCode::NotFound,
            format!("ari resource not found on {path}"),
            None,
            None,
        ),
        500 | 502 | 503 => CallError::new(
            CallErrorCode::Unavailable,
            format!("ari unavailable (HTTP {status}) on {path}"),
            None,
            None,
        ),
        409 => CallError::new(
            CallErrorCode::Conflict,
            format!("ari conflict on {path}"),
            None,
            None,
        ),
        _ => CallError::new(
            CallErrorCode::External,
            format!("ari unexpected HTTP {status} on {path}"),
            None,
            None,
        ),
    }
}

impl AriTransport for RestAriTransport {
    fn health(&self) -> Result<(), CallError> {
        let _ = self.get_json("/ari/asterisk/info")?;
        Ok(())
    }

    fn list_channels(&self) -> Result<Vec<AriChannel>, CallError> {
        let value = self.get_json("/ari/channels")?;
        let channels: Vec<AriChannel> = serde_json::from_value(value)
            .map_err(|e| CallError::external(format!("ari channels schema invalid: {e}")))?;
        Ok(channels)
    }

    fn channel_state(&self, channel: &ChannelSelector) -> Result<AriChannel, CallError> {
        let value = self.get_json(&format!("/ari/channels/{}", channel.as_str()))?;
        serde_json::from_value(value)
            .map_err(|e| CallError::external(format!("ari channel schema invalid: {e}")))
    }

    fn originate(
        &self,
        endpoint: &SipEndpointId,
        context: &str,
        extension: &str,
        caller_id: Option<&str>,
    ) -> Result<AriChannel, CallError> {
        let mut params: Vec<(&str, String)> = vec![
            ("endpoint", format!("PJSIP/{}", endpoint.as_str())),
            ("context", context.to_string()),
            ("extension", extension.to_string()),
            ("priority", "1".to_string()),
        ];
        if let Some(cid) = caller_id {
            params.push(("callerId", cid.to_string()));
        }
        let value = self.post_json("/ari/channels", &params)?;
        serde_json::from_value(value)
            .map_err(|e| CallError::external(format!("ari originate schema invalid: {e}")))
    }

    fn answer(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        let path = format!("/ari/channels/{}/answer", channel.as_str());
        let _ = self.post_json(&path, &[])?;
        Ok(())
    }

    fn hangup(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        let path = format!("/ari/channels/{}", channel.as_str());
        self.delete(&path)
    }

    fn bridge(&self, channel: &ChannelSelector, bridge_id: &str) -> Result<(), CallError> {
        let path = format!("/ari/channels/{}/bridge", channel.as_str());
        let _ = self.post_json(&path, &[("bridge", bridge_id.to_string())])?;
        Ok(())
    }

    fn r#continue(
        &self,
        channel: &ChannelSelector,
        context: &str,
        extension: &str,
    ) -> Result<(), CallError> {
        let path = format!("/ari/channels/{}/continue", channel.as_str());
        let _ = self.post_json(
            &path,
            &[
                ("context", context.to_string()),
                ("extension", extension.to_string()),
                ("priority", "1".to_string()),
            ],
        )?;
        Ok(())
    }

    fn send_dtmf(&self, channel: &ChannelSelector, digits: &str) -> Result<(), CallError> {
        if digits.is_empty() {
            return Err(CallError::validation("dtmf digits must not be empty"));
        }
        let path = format!("/ari/channels/{}/dtmf", channel.as_str());
        let _ = self.post_json(&path, &[("dtmf", digits.to_string())])?;
        Ok(())
    }

    fn start_moh(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        let path = format!("/ari/channels/{}/moh", channel.as_str());
        let _ = self.post_json(&path, &[])?;
        Ok(())
    }

    fn stop_moh(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        let path = format!("/ari/channels/{}/moh", channel.as_str());
        self.delete(&path)
    }

    fn redirect(
        &self,
        channel: &ChannelSelector,
        context: &str,
        extension: &str,
    ) -> Result<(), CallError> {
        let path = format!("/ari/channels/{}/redirect", channel.as_str());
        let _ = self.post_json(
            &path,
            &[
                ("context", context.to_string()),
                ("extension", extension.to_string()),
                ("priority", "1".to_string()),
            ],
        )?;
        Ok(())
    }
}

/// Send-error classifier for unit tests (parser-failure tests only;
/// directive 2 allows a fake Asterisk HTTP server for parser failure
/// tests - it never certifies Asterisk integration).
pub fn classify_status_pub(status: u16, path: &str) -> CallError {
    classify_status(status, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep025_unit_channel_selector_validation() {
        assert!(ChannelSelector::new("PJSIP/a-00000001").is_ok());
        assert!(ChannelSelector::new("").is_err());
    }

    #[test]
    fn ep025_unit_ari_channel_json_mapping() {
        // DOCUMENTED ARI channel object shape (Asterisk 22 ARI model).
        let json = r#"{
            "id": "PJSIP/endpoint-a-00000001",
            "name": "PJSIP/endpoint-a-00000001",
            "state": "Up",
            "caller": {"name": "Nexus", "number": "100"},
            "connected": {"name": "endpoint-a", "number": "endpoint-a"},
            "dialplan": {"context": "internal", "exten": "100", "priority": 1},
            "creationtime": "2026-08-17T12:00:00.000+0000",
            "language": "en"
        }"#;
        let channel: AriChannel = serde_json::from_str(json).unwrap();
        assert_eq!(channel.state, "Up");
        assert_eq!(channel.caller.as_ref().unwrap().number, "100");
        assert_eq!(channel.dialplan.as_ref().unwrap().exten, "100");
    }

    #[test]
    fn ep025_unit_ari_channel_missing_optional_fields() {
        let json = r#"{"id": "abc", "name": "abc", "state": "Ring"}"#;
        let channel: AriChannel = serde_json::from_str(json).unwrap();
        assert_eq!(channel.state, "Ring");
        assert!(channel.caller.is_none());
        assert!(channel.dialplan.is_none());
    }

    #[test]
    fn ep025_unit_ari_status_classification() {
        assert_eq!(
            classify_status_pub(401, "/ari/channels").code,
            CallErrorCode::Authorization
        );
        assert_eq!(
            classify_status_pub(403, "/ari/channels").code,
            CallErrorCode::Authorization
        );
        assert_eq!(
            classify_status_pub(404, "/ari/channels/x").code,
            CallErrorCode::NotFound
        );
        assert_eq!(
            classify_status_pub(500, "/ari/channels").code,
            CallErrorCode::Unavailable
        );
        assert_eq!(
            classify_status_pub(503, "/ari/channels").code,
            CallErrorCode::Unavailable
        );
        assert_eq!(
            classify_status_pub(409, "/ari/channels").code,
            CallErrorCode::Conflict
        );
        assert_eq!(
            classify_status_pub(418, "/ari/channels").code,
            CallErrorCode::External
        );
    }

    #[test]
    fn ep025_unit_ari_transport_fails_closed() {
        struct Unbound;
        impl AriTransport for Unbound {}

        assert!(Unbound.health().is_err());
        assert!(Unbound.list_channels().is_err());
        let ch = ChannelSelector::new("x").unwrap();
        assert!(Unbound.channel_state(&ch).is_err());
        let ep = SipEndpointId::new("endpoint-a").unwrap();
        assert!(Unbound.originate(&ep, "internal", "100", None).is_err());
        assert!(Unbound.answer(&ch).is_err());
        assert!(Unbound.hangup(&ch).is_err());
        assert!(Unbound.bridge(&ch, "b1").is_err());
        assert!(Unbound.send_dtmf(&ch, "1").is_err());
    }

    #[test]
    fn ep025_unit_dtmf_empty_rejected() {
        // DTMF digits are validated before any transport call.
        let transport = RestAriTransport::new(
            "http://127.0.0.1:1",
            "user",
            "pass",
            Duration::from_millis(500),
        )
        .unwrap();
        let ch = ChannelSelector::new("x").unwrap();
        let err = transport.send_dtmf(&ch, "").unwrap_err();
        assert_eq!(err.code, CallErrorCode::Validation);
    }
}
