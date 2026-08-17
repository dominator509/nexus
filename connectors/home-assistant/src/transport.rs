//! Home Assistant transport port and real REST implementation
//! (SPEC-011; ADR-027, EP-020 M2).
//!
//! The transport port is the infrastructure boundary between the
//! provider adapter and Home Assistant. The real implementation uses
//! the actual Home Assistant REST API. Controlled fixtures are
//! acceptable for deterministic unit rules; provider certification
//! requires the real instance (M3/M5).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use nexus_home::{HomeError, HomeErrorCode};

/// One Home Assistant entity state record (the real `/api/states`
/// response shape). Only the fields the adapter needs are bound; the
/// rest remain opaque JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaEntityState {
    pub entity_id: String,
    pub state: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, Value>,
    #[serde(default)]
    pub last_changed: Option<String>,
    #[serde(default)]
    pub last_updated: Option<String>,
}

/// One Home Assistant service descriptor (the real `/api/services`
/// response shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HaService {
    pub domain: String,
    pub services: BTreeMap<String, Value>,
}

/// Transport port for the Home Assistant provider.
///
/// Implementations are real infrastructure adapters. The adapter core
/// never parses free-form provider payloads directly; it consumes the
/// normalized types below.
pub trait HaTransport {
    /// Verify the credential against the real instance (GET /api/).
    fn auth_check(&mut self) -> Result<bool, HomeError>;

    /// Fetch all entity states (GET /api/states).
    fn get_states(&mut self) -> Result<Vec<HaEntityState>, HomeError>;

    /// Fetch all services (GET /api/services).
    fn get_services(&mut self) -> Result<Vec<HaService>, HomeError>;

    /// Read one entity state (GET /api/states/<entity_id>).
    fn get_state(&mut self, entity_id: &str) -> Result<HaEntityState, HomeError>;

    /// Call a real Home Assistant service (POST
    /// /api/services/<domain>/<service>). This is the ONLY command
    /// path for physical device control; state writes are never used.
    fn call_service(
        &mut self,
        domain: &str,
        service: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), HomeError>;

    /// Provision an automation through the provider's real supported
    /// automation config API (POST
    /// /api/config/automation/config/<automation_id>). The provider
    /// validates the automation config, persists it, and creates the
    /// runnable automation entity; creation is proven by readback, never
    /// by acceptance alone.
    fn create_automation(
        &mut self,
        automation_id: &str,
        config: &BTreeMap<String, Value>,
    ) -> Result<(), HomeError>;
}

/// Real Home Assistant REST transport over reqwest (blocking).
///
/// `base_url` is the HA instance base URL (e.g.
/// `http://127.0.0.1:8123`); `token` is a long-lived access token
/// routed through EP-009 SecretStore references by the caller. The
/// token is never logged or serialized.
pub struct RestTransport {
    base_url: String,
    token: String,
    client: reqwest::blocking::Client,
}

/// Map a reqwest transport error to the canonical HomeError code.
/// Timeout errors (silent peer, stalled connection) are TIMEOUT;
/// every other transport failure (refused, reset, DNS, TLS) is
/// UNAVAILABLE. A silent HTTP peer must never hang a caller and must
/// never be conflated with a refused endpoint.
fn map_send_error(e: reqwest::Error, message: String, resource: Option<Box<str>>) -> HomeError {
    if e.is_timeout() {
        HomeError::new(HomeErrorCode::Timeout, message, None, resource)
    } else {
        HomeError::new(HomeErrorCode::Unavailable, message, None, resource)
    }
}

impl RestTransport {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token: token.into(),
            client: reqwest::blocking::Client::new(),
        }
    }

    /// Construct with a bounded per-request timeout. Used by
    /// consequential physical-control connectors (irrigation) so a
    /// stalled provider can never hang a command indefinitely; the
    /// ambiguity is preserved as TIMEOUT/UNKNOWN rather than a
    /// fabricated outcome.
    pub fn with_timeout(
        base_url: impl Into<String>,
        token: impl Into<String>,
        timeout: std::time::Duration,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            token: token.into(),
            client: reqwest::blocking::Client::builder()
                .timeout(timeout)
                .build()
                .expect("reqwest client with timeout"),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    fn get_json(&self, path: &str) -> Result<Value, HomeError> {
        let resp = self
            .client
            .get(self.url(path))
            .bearer_auth(&self.token)
            .send()
            .map_err(|e| {
                let message = format!("HA request failed: {e}");
                map_send_error(e, message, Some(Box::from(path)))
            })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(HomeError::new(
                HomeErrorCode::External,
                format!("HA GET {path} returned {status}"),
                None,
                Some(Box::from(path)),
            ));
        }
        resp.json().map_err(|e| {
            HomeError::new(
                HomeErrorCode::External,
                format!("HA GET {path} returned malformed JSON: {e}"),
                None,
                Some(Box::from(path)),
            )
        })
    }
}

impl HaTransport for RestTransport {
    fn auth_check(&mut self) -> Result<bool, HomeError> {
        // GET /api returns 200 when the token is valid.
        let resp = self
            .client
            .get(self.url("/api/"))
            .bearer_auth(&self.token)
            .send()
            .map_err(|e| {
                let message = format!("HA auth check failed: {e}");
                map_send_error(e, message, None)
            })?;
        Ok(resp.status().is_success())
    }

    fn get_states(&mut self) -> Result<Vec<HaEntityState>, HomeError> {
        let value = self.get_json("/api/states")?;
        serde_json::from_value(value).map_err(|e| {
            HomeError::new(
                HomeErrorCode::External,
                format!("HA /api/states malformed: {e}"),
                None,
                None,
            )
        })
    }

    fn get_services(&mut self) -> Result<Vec<HaService>, HomeError> {
        let value = self.get_json("/api/services")?;
        serde_json::from_value(value).map_err(|e| {
            HomeError::new(
                HomeErrorCode::External,
                format!("HA /api/services malformed: {e}"),
                None,
                None,
            )
        })
    }

    fn get_state(&mut self, entity_id: &str) -> Result<HaEntityState, HomeError> {
        let path = format!("/api/states/{entity_id}");
        let value = self.get_json(&path)?;
        serde_json::from_value(value).map_err(|e| {
            HomeError::new(
                HomeErrorCode::External,
                format!("HA {path} malformed: {e}"),
                None,
                Some(Box::from(entity_id)),
            )
        })
    }

    fn call_service(
        &mut self,
        domain: &str,
        service: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), HomeError> {
        let path = format!("/api/services/{domain}/{service}");
        let resp = self
            .client
            .post(self.url(&path))
            .bearer_auth(&self.token)
            .json(data)
            .send()
            .map_err(|e| {
                let message = format!("HA service call failed: {e}");
                map_send_error(e, message, Some(Box::from(path.clone())))
            })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(HomeError::new(
                HomeErrorCode::External,
                format!("HA service call {path} returned {status}"),
                None,
                Some(Box::from(path)),
            ));
        }
        Ok(())
    }

    fn create_automation(
        &mut self,
        automation_id: &str,
        config: &BTreeMap<String, Value>,
    ) -> Result<(), HomeError> {
        let path = format!("/api/config/automation/config/{automation_id}");
        let resp = self
            .client
            .post(self.url(&path))
            .bearer_auth(&self.token)
            .json(config)
            .send()
            .map_err(|e| {
                let message = format!("HA automation config request failed: {e}");
                map_send_error(e, message, Some(Box::from(path.clone())))
            })?;
        let status = resp.status();
        if !status.is_success() {
            return Err(HomeError::new(
                HomeErrorCode::External,
                format!("HA automation config {path} returned {status}"),
                None,
                Some(Box::from(path)),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep020_unit_ha_entity_state_parses_real_shape() {
        let json = r#"{"entity_id":"light.kitchen","state":"on","attributes":{"brightness":200},"last_changed":"2026-01-01T00:00:00Z","last_updated":"2026-01-01T00:00:00Z"}"#;
        let state: HaEntityState = serde_json::from_str(json).unwrap();
        assert_eq!(state.entity_id, "light.kitchen");
        assert_eq!(state.state, "on");
        assert_eq!(state.attributes["brightness"], Value::from(200));
    }

    #[test]
    fn ep020_unit_ha_entity_state_defaults_missing_fields() {
        let json = r#"{"entity_id":"sensor.temp","state":"21.5"}"#;
        let state: HaEntityState = serde_json::from_str(json).unwrap();
        assert!(state.attributes.is_empty());
        assert!(state.last_changed.is_none());
    }

    #[test]
    fn ep020_unit_rest_transport_url_joins_without_double_slash() {
        let t = RestTransport::new("http://127.0.0.1:8123/", "tok");
        assert_eq!(t.url("/api/states"), "http://127.0.0.1:8123/api/states");
    }
}
