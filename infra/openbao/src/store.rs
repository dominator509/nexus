//! OpenBao client and the `SecretStore`/`BootstrapSecretStore`
//! implementations (EP-009 M2 directives A-F, G, H, I, N, P).
//!
//! Wire format verified live against the pinned OpenBao 2.5.4 container
//! (sha256:436eaf9778cad75507ff70ea26ace30dcbe15606e619ac3823495663d7f7c115):
//! - KV-v2 mounted at `secret/` (dev default); write = PUT
//!   `/v1/secret/data/<path>` with `{"data": {...}}`; read = GET same
//!   path (optionally `?version=N`); metadata = GET
//!   `/v1/secret/metadata/<path>`; soft delete = DELETE
//!   `/v1/secret/data/<path>`; undelete = POST
//!   `/v1/secret/undelete/<path>` `{"versions":[...]}`; destroy = POST
//!   `/v1/secret/destroy/<path>` `{"versions":[...]}`; destroy returns
//!   204 with empty body.
//! - Response wrapping: any request with header `X-Vault-Wrap-TTL`
//!   returns `wrap_info.token` instead of the normal payload; the
//!   wrapping token is consumed by POST `/v1/sys/wrapping/unwrap` with
//!   `X-Vault-Token: <wrapping token>`; second unwrap and expired
//!   tokens return 400 `wrapping token is not valid or does not exist`.
//! - AppRole: enable `POST /v1/sys/auth/approle`; policy
//!   `PUT /v1/sys/policies/acl/<name>`; role
//!   `POST /v1/auth/approle/role/<name>`; role-id
//!   `GET /v1/auth/approle/role/<name>/role-id`; secret-id
//!   `POST /v1/auth/approle/role/<name>/secret-id`; login
//!   `POST /v1/auth/approle/login` `{"role_id","secret_id"}` -> 200 with
//!   `auth.client_token`, `auth.lease_duration`, `auth.renewable`;
//!   wrong secret-id -> 400 `invalid role or secret ID`.
//!
//! The adapter never exposes mount names, API payload shapes, token
//! formats, or KV-v2 metadata structures through the nexus-trust
//! contract surface; it maps everything to canonical Nexus types.

use std::time::{Duration, Instant};

use nexus_trust::secret::{SecretReference, SecretStore};
use nexus_trust::vocabulary::SecretState;
use nexus_trust::{SecretValue, TrustError};

use crate::error::{OpenBaoError, OpenBaoErrorCode};
use crate::telemetry::{RecordingSink, TelemetryEvent};

/// Connection/read/write budget for the OpenBao sidecar surface.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// Default KV-v2 mount for Nexus static secrets.
pub const DEFAULT_MOUNT: &str = "secret";

/// Least-privilege AppRole login result.
pub struct AppRoleLogin {
    /// Bounded client token (never persisted, never logged).
    pub(crate) client_token: String,
    /// Token TTL in seconds.
    pub lease_duration: u64,
    /// Whether the token is renewable.
    pub renewable: bool,
}

impl std::fmt::Debug for AppRoleLogin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppRoleLogin")
            .field("lease_duration", &self.lease_duration)
            .field("renewable", &self.renewable)
            .field("client_token", &"<redacted>")
            .finish()
    }
}

impl AppRoleLogin {
    /// Access to the client token for the transport layer only.
    // Used by the live-fire probe (infra/openbao/examples) and unit
    // tests; not part of the lib's public contract surface.
    #[allow(dead_code)]
    pub(crate) fn client_token(&self) -> &str {
        &self.client_token
    }
}

/// Result of a one-time response wrapping handoff.
pub struct WrappedHandoff {
    /// Single-use wrapping token (never logged; consumed by unwrap).
    pub(crate) wrapping_token: String,
    /// Safe metadata: accessor, creation path, TTL, creation time.
    pub accessor: String,
    pub creation_path: String,
    pub ttl: u64,
    pub creation_time: String,
}

impl std::fmt::Debug for WrappedHandoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WrappedHandoff")
            .field("accessor", &self.accessor)
            .field("creation_path", &self.creation_path)
            .field("ttl", &self.ttl)
            .field("creation_time", &self.creation_time)
            .field("wrapping_token", &"<redacted>")
            .finish()
    }
}

impl WrappedHandoff {
    /// Access to the wrapping token for the transport layer only.
    pub(crate) fn wrapping_token(&self) -> &str {
        &self.wrapping_token
    }
}

/// Real OpenBao adapter implementing the nexus-trust secret contracts.
pub struct OpenBaoStore {
    base_url: String,
    mount: String,
    client_token: String,
    sink: RecordingSink,
}

impl OpenBaoStore {
    /// Construct from an already-issued bounded client token.
    pub fn with_token(
        base_url: impl Into<String>,
        client_token: impl Into<String>,
    ) -> Result<Self, TrustError> {
        let base_url = base_url.into();
        let client_token = client_token.into();
        if base_url.trim().is_empty() {
            return Err(TrustError::invalid("openbao base url must not be empty"));
        }
        if client_token.trim().is_empty() {
            return Err(TrustError::invalid(
                "openbao client token must not be empty",
            ));
        }
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            mount: DEFAULT_MOUNT.to_string(),
            client_token,
            sink: RecordingSink::new(),
        })
    }

    /// The redacted telemetry sink (tests and probe only).
    pub fn sink(&self) -> &RecordingSink {
        &self.sink
    }

    /// Live health probe (real HTTP to /v1/sys/health).
    pub fn health(&self) -> Result<(), TrustError> {
        let start = Instant::now();
        let result = self.request("GET", "/v1/sys/health", None::<serde_json::Value>);
        match result {
            Ok(_) => {
                self.sink.record(TelemetryEvent {
                    operation: "health".to_string(),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(())
            }
            Err(e) => {
                self.sink.record(TelemetryEvent {
                    operation: "health".to_string(),
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().to_string()),
                    ..Default::default()
                });
                Err(e.into_trust())
            }
        }
    }

    /// Low-level request against the OpenBao HTTP surface.
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, OpenBaoError> {
        let url = format!("{}{}", self.base_url, path);
        let result = ureq::request(method, &url)
            .timeout(REQUEST_TIMEOUT)
            .set("X-Vault-Token", &self.client_token)
            .set("Content-Type", "application/json");
        let resp = match body {
            Some(b) => result.send_json(b),
            None => result.call(),
        };
        match resp {
            Ok(r) => {
                let status = r.status();
                if (200..300).contains(&status) {
                    // Some OpenBao success responses (204) have no body.
                    let text = r.into_string().unwrap_or_default();
                    if text.trim().is_empty() {
                        return Ok(serde_json::Value::Null);
                    }
                    serde_json::from_str(&text).map_err(|_| {
                        OpenBaoError::new(
                            OpenBaoErrorCode::MalformedProviderResponse,
                            "malformed OpenBao success payload",
                        )
                    })
                } else {
                    let text = r.into_string().unwrap_or_default();
                    Err(classify_status(status, &text, &url))
                }
            }
            Err(ureq::Error::Status(status, r)) => {
                let text = r.into_string().unwrap_or_default();
                Err(classify_status(status, &text, &url))
            }
            Err(ureq::Error::Transport(t)) => {
                let msg = format!("cannot reach OpenBao at {}: {}", url, t);
                Err(OpenBaoError::new(OpenBaoErrorCode::Unavailable, msg))
            }
        }
    }
}

/// Classify an HTTP status + provider error body into a typed class.
fn classify_status(status: u16, body: &str, path: &str) -> OpenBaoError {
    let errors = extract_errors(body);
    let joined = errors.join("; ");
    match status {
        400 => {
            if joined.contains("wrapping token is not valid or does not exist") {
                OpenBaoError::with_status(
                    OpenBaoErrorCode::NotFound,
                    "wrapping token already consumed, expired, or invalid",
                    status,
                )
            } else if joined.contains("invalid role or secret ID")
                || joined.contains("failed to verify credentials")
            {
                OpenBaoError::with_status(
                    OpenBaoErrorCode::AuthenticationFailed,
                    "OpenBao authentication failed",
                    status,
                )
            } else if joined.contains("invalid version") {
                OpenBaoError::with_status(
                    OpenBaoErrorCode::VersionMismatch,
                    "OpenBao version mismatch",
                    status,
                )
            } else {
                OpenBaoError::with_status(
                    OpenBaoErrorCode::MalformedProviderResponse,
                    format!("OpenBao rejected request for {}", path),
                    status,
                )
            }
        }
        403 => {
            if joined.contains("permission denied") {
                OpenBaoError::with_status(
                    OpenBaoErrorCode::PermissionDenied,
                    "OpenBao policy denied the operation",
                    status,
                )
            } else {
                OpenBaoError::with_status(
                    OpenBaoErrorCode::AuthenticationFailed,
                    "OpenBao rejected the client credential",
                    status,
                )
            }
        }
        404 => {
            if joined.contains("not found") || joined.is_empty() {
                OpenBaoError::with_status(
                    OpenBaoErrorCode::NotFound,
                    "OpenBao secret or path not found",
                    status,
                )
            } else {
                OpenBaoError::with_status(
                    OpenBaoErrorCode::NotFound,
                    "OpenBao resource not found",
                    status,
                )
            }
        }
        _ => OpenBaoError::with_status(
            OpenBaoErrorCode::PolicyViolation,
            format!("OpenBao returned status {} for {}", status, path),
            status,
        ),
    }
}

/// Extract the provider `errors` array (redacted; only stable strings).
fn extract_errors(body: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    v.get("errors")
        .and_then(|e| e.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// SecretStore implementation (KV-v2 semantics; directives D, F)
// ---------------------------------------------------------------------------

impl SecretStore for OpenBaoStore {
    fn get(&self, reference: &SecretReference) -> Result<SecretValue, TrustError> {
        let start = Instant::now();
        let key = reference.key.trim_start_matches('/').to_string();
        if key.is_empty() {
            return Err(TrustError::invalid("empty secret key"));
        }
        let mut path = format!("/v1/{}/data/{}", self.mount, key);
        if let Some(v) = &reference.version {
            path.push_str(&format!("?version={}", v));
        }
        let result = self.request("GET", &path, None::<serde_json::Value>);
        match result {
            Ok(v) => {
                let data = v
                    .get("data")
                    .and_then(|d| d.get("data"))
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let destroyed = v
                    .get("data")
                    .and_then(|d| d.get("metadata"))
                    .and_then(|m| m.get("destroyed"))
                    .and_then(|d| d.as_bool())
                    .unwrap_or(false);
                if destroyed {
                    return Err(OpenBaoError::new(
                        OpenBaoErrorCode::Destroyed,
                        "secret version permanently destroyed",
                    )
                    .into_trust());
                }
                if data.is_null() {
                    return Err(OpenBaoError::new(
                        OpenBaoErrorCode::NotFound,
                        "secret has no data",
                    )
                    .into_trust());
                }
                let bytes = serde_json::to_vec(&data).map_err(|_| {
                    OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "cannot encode secret payload",
                    )
                })?;
                self.sink.record(TelemetryEvent {
                    operation: "get".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(SecretValue::new(bytes))
            }
            Err(e) => {
                self.sink.record(TelemetryEvent {
                    operation: "get".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().to_string()),
                    ..Default::default()
                });
                Err(e.into_trust())
            }
        }
    }

    fn put(&self, reference: &SecretReference, value: SecretValue) -> Result<(), TrustError> {
        let start = Instant::now();
        let key = reference.key.trim_start_matches('/').to_string();
        let body = serde_json::json!({ "data": serde_json::from_slice::<serde_json::Value>(value.as_bytes()).map_err(|_| OpenBaoError::new(OpenBaoErrorCode::MalformedProviderResponse, "secret payload must be valid JSON"))? });
        let path = format!("/v1/{}/data/{}", self.mount, key);
        let result = self.request("PUT", &path, Some(body));
        match result {
            Ok(_) => {
                self.sink.record(TelemetryEvent {
                    operation: "put".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(())
            }
            Err(e) => {
                self.sink.record(TelemetryEvent {
                    operation: "put".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().to_string()),
                    ..Default::default()
                });
                Err(e.into_trust())
            }
        }
    }

    fn rotate(&self, reference: &SecretReference, value: SecretValue) -> Result<(), TrustError> {
        // KV-v2 writes always create the next version; rotate == put.
        self.put(reference, value)
    }

    fn revoke(&self, reference: &SecretReference) -> Result<(), TrustError> {
        let start = Instant::now();
        let key = reference.key.trim_start_matches('/').to_string();
        let path = format!("/v1/{}/data/{}", self.mount, key);
        let result = self.request("DELETE", &path, None::<serde_json::Value>);
        match result {
            Ok(_) => {
                self.sink.record(TelemetryEvent {
                    operation: "revoke".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(())
            }
            Err(e) => {
                self.sink.record(TelemetryEvent {
                    operation: "revoke".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().to_string()),
                    ..Default::default()
                });
                Err(e.into_trust())
            }
        }
    }

    fn state(&self, reference: &SecretReference) -> Result<SecretState, TrustError> {
        let start = Instant::now();
        let key = reference.key.trim_start_matches('/').to_string();
        let path = format!("/v1/{}/metadata/{}", self.mount, key);
        let result = self.request("GET", &path, None::<serde_json::Value>);
        match result {
            Ok(v) => {
                let versions = v
                    .get("data")
                    .and_then(|d| d.get("versions"))
                    .and_then(|x| x.as_object())
                    .cloned()
                    .unwrap_or_default();
                let latest = versions
                    .get(
                        &versions
                            .keys()
                            .max()
                            .cloned()
                            .unwrap_or_default()
                            .to_string(),
                    )
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let deleted = latest
                    .get("deletion_time")
                    .and_then(|d| d.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);
                let destroyed = latest
                    .get("destroyed")
                    .and_then(|d| d.as_bool())
                    .unwrap_or(false);
                self.sink.record(TelemetryEvent {
                    operation: "state".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    state: Some(
                        if deleted || destroyed {
                            "REVOKED"
                        } else {
                            "ACTIVE"
                        }
                        .to_string(),
                    ),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                if deleted || destroyed {
                    Ok(SecretState::Revoked)
                } else {
                    Ok(SecretState::Active)
                }
            }
            Err(e) => {
                self.sink.record(TelemetryEvent {
                    operation: "state".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().to_string()),
                    ..Default::default()
                });
                Err(e.into_trust())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Response wrapping (directive G): single-use one-time handoff
// ---------------------------------------------------------------------------

impl OpenBaoStore {
    /// Wrap a secret read response: the caller receives a single-use
    /// wrapping token, never the plaintext.
    pub fn wrap_read(
        &self,
        reference: &SecretReference,
        ttl: &str,
    ) -> Result<WrappedHandoff, TrustError> {
        let start = Instant::now();
        let key = reference.key.trim_start_matches('/').to_string();
        let path = format!("/v1/{}/data/{}", self.mount, key);
        let url = format!("{}{}", self.base_url, path);
        let result = ureq::get(&url)
            .timeout(REQUEST_TIMEOUT)
            .set("X-Vault-Token", &self.client_token)
            .set("X-Vault-Wrap-TTL", ttl)
            .call();
        match result {
            Ok(r) => {
                let text = r.into_string().unwrap_or_default();
                let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| {
                    OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "malformed wrapped response",
                    )
                })?;
                let wrap = v
                    .get("wrap_info")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                let token = wrap
                    .get("token")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| {
                        OpenBaoError::new(
                            OpenBaoErrorCode::MalformedProviderResponse,
                            "wrapped response missing wrap token",
                        )
                    })?
                    .to_string();
                // The wrapped response must NOT contain plaintext data.
                let has_data = v.get("data").is_some();
                if has_data {
                    return Err(OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "wrapped response unexpectedly contains plaintext data",
                    )
                    .into_trust());
                }
                self.sink.record(TelemetryEvent {
                    operation: "wrap_read".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    wrapping: true,
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(WrappedHandoff {
                    wrapping_token: token,
                    accessor: wrap
                        .get("accessor")
                        .and_then(|t| t.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    creation_path: wrap
                        .get("creation_path")
                        .and_then(|t| t.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    ttl: wrap.get("ttl").and_then(|t| t.as_u64()).unwrap_or_default(),
                    creation_time: wrap
                        .get("creation_time")
                        .and_then(|t| t.as_str())
                        .unwrap_or_default()
                        .to_string(),
                })
            }
            Err(ureq::Error::Status(status, r)) => {
                let text = r.into_string().unwrap_or_default();
                let e = classify_status(status, &text, &path);
                self.sink.record(TelemetryEvent {
                    operation: "wrap_read".to_string(),
                    reference_fingerprint: fingerprint(&reference.to_string()),
                    wrapping: true,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().to_string()),
                    ..Default::default()
                });
                Err(e.into_trust())
            }
            Err(ureq::Error::Transport(t)) => {
                let msg = format!("cannot reach OpenBao at {}: {}", url, t);
                let code = if format!("{}", t).to_lowercase().contains("timeout") {
                    OpenBaoErrorCode::Timeout
                } else {
                    OpenBaoErrorCode::Unavailable
                };
                Err(OpenBaoError::new(code, msg).into_trust())
            }
        }
    }

    /// Consume a single-use wrapping token exactly once.
    ///
    /// The wrapping token is presented as the request credential to
    /// `POST /v1/sys/wrapping/unwrap`. A second call with the same token
    /// (or an expired token) fails with 400 -> typed NotFound.
    pub fn unwrap_once(&self, handoff: &WrappedHandoff) -> Result<SecretValue, TrustError> {
        let start = Instant::now();
        let url = format!("{}/v1/sys/wrapping/unwrap", self.base_url);
        let token = handoff.wrapping_token();
        let result = ureq::post(&url)
            .timeout(REQUEST_TIMEOUT)
            .set("X-Vault-Token", token)
            .call();
        match result {
            Ok(r) => {
                let text = r.into_string().unwrap_or_default();
                let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| {
                    OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "malformed unwrap response",
                    )
                })?;
                let data = v.get("data").cloned().unwrap_or(serde_json::Value::Null);
                let bytes = serde_json::to_vec(&data).map_err(|_| {
                    OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "cannot encode unwrapped payload",
                    )
                })?;
                self.sink.record(TelemetryEvent {
                    operation: "unwrap_once".to_string(),
                    wrapping: true,
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(SecretValue::new(bytes))
            }
            Err(ureq::Error::Status(status, r)) => {
                let text = r.into_string().unwrap_or_default();
                let e = classify_status(status, &text, "/v1/sys/wrapping/unwrap");
                self.sink.record(TelemetryEvent {
                    operation: "unwrap_once".to_string(),
                    wrapping: true,
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().to_string()),
                    ..Default::default()
                });
                Err(e.into_trust())
            }
            Err(ureq::Error::Transport(t)) => {
                let msg = format!("cannot reach OpenBao at {}: {}", url, t);
                let code = if format!("{}", t).to_lowercase().contains("timeout") {
                    OpenBaoErrorCode::Timeout
                } else {
                    OpenBaoErrorCode::Unavailable
                };
                Err(OpenBaoError::new(code, msg).into_trust())
            }
        }
    }
}

/// Fingerprint helper re-export for telemetry users.
pub use crate::telemetry::fingerprint;
