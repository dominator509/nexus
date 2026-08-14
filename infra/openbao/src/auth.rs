//! AppRole machine authentication (EP-009 M2 directive C).
//!
//! The production adapter authenticates with a narrowly scoped AppRole
//! (least privilege): role_id + one-time SecretID -> bounded client
//! token. The SecretID is never persisted in the repository; the token
//! TTL is bounded and renewable only if the role explicitly permits it.
//! Root/bootstrap credentials are used only to configure the ephemeral
//! test instance, never by the adapter.

use std::time::{Duration, Instant};

use crate::error::{OpenBaoError, OpenBaoErrorCode};
use crate::store::AppRoleLogin;
use crate::telemetry::{RecordingSink, TelemetryEvent};

/// Connection budget for login.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// AppRole credential pair (never persisted; caller owns lifecycle).
#[derive(Debug, Clone)]
pub struct AppRoleCredentials {
    /// Stable role identifier.
    pub role_id: String,
    /// One-time secret identifier.
    pub secret_id: String,
}

/// Perform an AppRole login against the real OpenBao server.
pub fn approle_login(
    base_url: &str,
    credentials: &AppRoleCredentials,
    sink: &RecordingSink,
) -> Result<AppRoleLogin, OpenBaoError> {
    let start = Instant::now();
    let url = format!("{}/v1/auth/approle/login", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "role_id": credentials.role_id,
        "secret_id": credentials.secret_id,
    });
    let result = ureq::post(&url)
        .timeout(REQUEST_TIMEOUT)
        .set("Content-Type", "application/json")
        .send_json(body);
    match result {
        Ok(r) => {
            let status = r.status();
            let text = r.into_string().unwrap_or_default();
            if (200..300).contains(&status) {
                let v: serde_json::Value = serde_json::from_str(&text).map_err(|_| {
                    OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "malformed AppRole login response",
                    )
                })?;
                let auth = v.get("auth").cloned().unwrap_or(serde_json::Value::Null);
                let client_token = auth
                    .get("client_token")
                    .and_then(|t| t.as_str())
                    .ok_or_else(|| {
                        OpenBaoError::new(
                            OpenBaoErrorCode::MalformedProviderResponse,
                            "AppRole login missing client token",
                        )
                    })?
                    .to_string();
                let lease_duration = auth
                    .get("lease_duration")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0);
                let renewable = auth
                    .get("renewable")
                    .and_then(|t| t.as_bool())
                    .unwrap_or(false);
                sink.record(TelemetryEvent {
                    operation: "login".to_string(),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(AppRoleLogin {
                    client_token,
                    lease_duration,
                    renewable,
                })
            } else {
                let msg = "OpenBao AppRole login rejected";
                let e = if status == 400 || status == 401 || status == 403 {
                    OpenBaoError::with_status(OpenBaoErrorCode::AuthenticationFailed, msg, status)
                } else {
                    OpenBaoError::with_status(OpenBaoErrorCode::PolicyViolation, msg, status)
                };
                sink.record(TelemetryEvent {
                    operation: "login".to_string(),
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().to_string()),
                    ..Default::default()
                });
                Err(e)
            }
        }
        Err(ureq::Error::Status(status, r)) => {
            let _ = r.into_string();
            let e = OpenBaoError::with_status(
                OpenBaoErrorCode::AuthenticationFailed,
                "OpenBao AppRole login rejected",
                status,
            );
            sink.record(TelemetryEvent {
                operation: "login".to_string(),
                latency_ms: start.elapsed().as_millis() as u64,
                error_class: Some(e.code.as_str().to_string()),
                ..Default::default()
            });
            Err(e)
        }
        Err(ureq::Error::Transport(t)) => Err(OpenBaoError::new(
            OpenBaoErrorCode::Unavailable,
            format!("cannot reach OpenBao: {}", t),
        )),
    }
}
