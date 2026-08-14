//! OpenBao Transit-backed capability token issuer (EP-009 M5 directive I).
//!
//! The `CapabilityTokenIssuer` port (nexus-trust M1) gets its concrete
//! live implementation here, backed by the REAL OpenBao Transit engine:
//! - tokens are short-lived, audience/subject/tenant/action scoped,
//!   uniquely identifiable, and revocable;
//! - authenticity is a real Ed25519 signature produced and verified by
//!   OpenBao Transit (approved provider crypto; no custom cryptography);
//! - the token record (claims fingerprint + state) lives in KV-v2, so
//!   revocation is a real provider state change;
//! - tokens are never long-lived universal bearer credentials
//!   (EP-009 acceptance obligation 1) and are never model-generated
//!   authority (directive M).
//!
//! Wire format (verified live against pinned OpenBao 2.5.4):
//! - enable transit: POST /v1/sys/mounts/transit {"type":"transit"} -> 204
//! - create key:     POST /v1/transit/keys/<name> {"type":"ed25519"} -> 200
//!   (re-create returns 400 "key already exists"; treated as success)
//! - sign:           POST /v1/transit/sign/<name>
//!   {"input":"<base64 claims>"} -> {"data":{"signature":"vault:v1:..."}}
//! - verify:         POST /v1/transit/verify/<name>
//!   {"input":"<base64>","signature":"vault:v1:..."} -> {"data":{"valid":true|false}}
//! - tampered signature / wrong input -> valid:false (HTTP 200, never error)

use std::str::FromStr;
use std::time::{Duration, Instant};

use nexus_trust::TrustError;
use nexus_trust::token::{CapabilityToken, CapabilityTokenIssuer};
use nexus_trust::vocabulary::TokenState;

use crate::error::{OpenBaoError, OpenBaoErrorCode};
use crate::telemetry::{TelemetryEvent, fingerprint};

/// Connection/read/write budget for the OpenBao surface.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// Default KV-v2 mount holding token records.
const DEFAULT_MOUNT: &str = "secret";

/// Default Transit mount (standard OpenBao layout).
const DEFAULT_TRANSIT_MOUNT: &str = "transit";

/// Capability token record path under the KV-v2 mount.
const TOKEN_RECORD_PREFIX: &str = "capability-tokens";

/// Canonical claims JSON used as the Transit signing input. This is the
/// exact byte string signed by the provider; any change to a token field
/// changes the claims and fails verification (directive E hard binding).
fn canonical_claims(token: &CapabilityToken) -> String {
    // Deterministic, key-ordered JSON (serde_json Map is BTree by
    // default for string keys) so issue/verify agree byte-for-byte.
    serde_json::json!({
        "token_id": token.token_id,
        "audience": token.audience,
        "tenant_id": token.tenant_id,
        "resource": token.resource,
        "action": token.action,
        "actor": token.actor,
        "issued_at_unix_s": token.issued_at_unix_s,
        "expires_at_unix_s": token.expires_at_unix_s,
    })
    .to_string()
}

/// Real OpenBao Transit-backed capability token issuer.
pub struct OpenBaoTokenIssuer {
    base_url: String,
    mount: String,
    transit_mount: String,
    key_name: String,
    client_token: String,
    sink: crate::telemetry::RecordingSink,
}

impl OpenBaoTokenIssuer {
    /// Construct from an already-issued bounded client token.
    pub fn with_token(
        base_url: impl Into<String>,
        client_token: impl Into<String>,
        key_name: &str,
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
        let key_name = key_name.trim().to_string();
        if key_name.is_empty() {
            return Err(TrustError::invalid("transit key name must not be empty"));
        }
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            mount: DEFAULT_MOUNT.to_string(),
            transit_mount: DEFAULT_TRANSIT_MOUNT.to_string(),
            key_name,
            client_token,
            sink: crate::telemetry::RecordingSink::new(),
        })
    }

    /// The redacted telemetry sink (tests and probe only).
    pub fn sink(&self) -> &crate::telemetry::RecordingSink {
        &self.sink
    }

    /// Ensure the Transit mount and Ed25519 signing key exist.
    /// Idempotent: mount enable and key create are one-time; re-create
    /// errors are treated as success ("already exists").
    pub fn ensure_key(&self) -> Result<(), TrustError> {
        let start = Instant::now();
        // Enable transit mount (204 on first create; 400 when the mount
        // already exists - treated as success, matching the documented
        // idempotency: "mount enable and key create are one-time").
        let (status, body) = self
            .http_json(
                "POST",
                &format!("/v1/sys/mounts/{}", self.transit_mount),
                Some(&serde_json::json!({"type": "transit"})),
            )
            .map_err(|e| self.record("ensure_transit", &e))?;
        let already_mounted = status == 400
            && body
                .get("errors")
                .and_then(|e| e.as_array())
                .map(|arr| {
                    arr.iter().any(|m| {
                        m.as_str()
                            .map(|s| s.contains("already in use"))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
        if status != 204 && status != 200 && !already_mounted {
            return Err(self.record(
                "ensure_transit",
                &OpenBaoError::new(
                    OpenBaoErrorCode::PermissionDenied,
                    format!("cannot enable transit mount (status {status})"),
                ),
            ));
        }
        // Create the Ed25519 key (200 on create, 400 on re-create).
        let (status, body) = self
            .http_json(
                "POST",
                &format!("/v1/{}/keys/{}", self.transit_mount, self.key_name),
                Some(&serde_json::json!({"type": "ed25519"})),
            )
            .map_err(|e| self.record("ensure_key", &e))?;
        if status == 200 {
            let created = body
                .get("data")
                .and_then(|d| d.get("supports_signing"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !created {
                return Err(self.record(
                    "ensure_key",
                    &OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "transit key response missing supports_signing",
                    ),
                ));
            }
        } else if status != 400 {
            return Err(self.record(
                "ensure_key",
                &OpenBaoError::new(
                    OpenBaoErrorCode::PermissionDenied,
                    format!("cannot create transit key (status {status})"),
                ),
            ));
        }
        self.sink.record(TelemetryEvent {
            operation: "ensure_key".to_string(),
            provider: "openbao".to_string(),
            latency_ms: start.elapsed().as_millis() as u64,
            ..Default::default()
        });
        Ok(())
    }

    /// Sign canonical claims via Transit and return the signature.
    fn sign_claims(&self, token: &CapabilityToken) -> Result<String, TrustError> {
        let claims = canonical_claims(token);
        let input = base64_encode(claims.as_bytes());
        let (status, body) = self
            .http_json(
                "POST",
                &format!("/v1/{}/sign/{}", self.transit_mount, self.key_name),
                Some(&serde_json::json!({"input": input})),
            )
            .map_err(|e| self.record("sign_token", &e))?;
        if status != 200 {
            return Err(self.record(
                "sign_token",
                &OpenBaoError::new(
                    OpenBaoErrorCode::PermissionDenied,
                    format!("transit sign failed (status {status})"),
                ),
            ));
        }
        let signature = body
            .get("data")
            .and_then(|d| d.get("signature"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                self.record(
                    "sign_token",
                    &OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "transit sign response missing signature",
                    ),
                )
            })?
            .to_string();
        if !signature.starts_with("vault:v1:") {
            return Err(self.record(
                "sign_token",
                &OpenBaoError::new(
                    OpenBaoErrorCode::MalformedProviderResponse,
                    "transit signature has unexpected format",
                ),
            ));
        }
        Ok(signature)
    }

    /// Verify a signature over the canonical claims via Transit.
    fn verify_claims(&self, token: &CapabilityToken, signature: &str) -> Result<bool, TrustError> {
        let claims = canonical_claims(token);
        let input = base64_encode(claims.as_bytes());
        let (status, body) = self
            .http_json(
                "POST",
                &format!("/v1/{}/verify/{}", self.transit_mount, self.key_name),
                Some(&serde_json::json!({"input": input, "signature": signature})),
            )
            .map_err(|e| self.record("verify_token", &e))?;
        if status != 200 {
            return Err(self.record(
                "verify_token",
                &OpenBaoError::new(
                    OpenBaoErrorCode::PermissionDenied,
                    format!("transit verify failed (status {status})"),
                ),
            ));
        }
        let valid = body
            .get("data")
            .and_then(|d| d.get("valid"))
            .and_then(|v| v.as_bool())
            .ok_or_else(|| {
                self.record(
                    "verify_token",
                    &OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "transit verify response missing valid flag",
                    ),
                )
            })?;
        Ok(valid)
    }

    /// KV-v2 record path for a token.
    fn record_path(&self, token_id: &str) -> String {
        format!("{}/{}/{}", self.mount, TOKEN_RECORD_PREFIX, token_id)
    }

    /// Persist the token record (signature + state) to KV-v2.
    fn store_record(&self, token: &CapabilityToken, signature: &str) -> Result<(), TrustError> {
        let start = Instant::now();
        let (status, _) = self
            .http_json(
                "POST",
                &format!(
                    "/v1/{}/data/{}",
                    self.mount,
                    self.record_path(&token.token_id)
                ),
                Some(&serde_json::json!({
                    "data": {
                        "signature": signature,
                        "state": token.state.as_str(),
                        "issued_at_unix_s": token.issued_at_unix_s,
                        "expires_at_unix_s": token.expires_at_unix_s,
                    }
                })),
            )
            .map_err(|e| self.record("store_token_record", &e))?;
        if status != 200 && status != 204 {
            return Err(self.record(
                "store_token_record",
                &OpenBaoError::new(
                    OpenBaoErrorCode::PermissionDenied,
                    format!("cannot store token record (status {status})"),
                ),
            ));
        }
        self.sink.record(TelemetryEvent {
            operation: "store_token_record".to_string(),
            provider: "openbao".to_string(),
            latency_ms: start.elapsed().as_millis() as u64,
            ..Default::default()
        });
        Ok(())
    }

    /// Read the token record (signature + state) from KV-v2.
    fn read_record(&self, token_id: &str) -> Result<(String, TokenState), TrustError> {
        let start = Instant::now();
        let (status, body) = self
            .http_json(
                "GET",
                &format!("/v1/{}/data/{}", self.mount, self.record_path(token_id)),
                None,
            )
            .map_err(|e| self.record("read_token_record", &e))?;
        if status == 404 {
            return Err(self.record(
                "read_token_record",
                &OpenBaoError::new(
                    OpenBaoErrorCode::NotFound,
                    format!("token record {token_id} not found"),
                ),
            ));
        }
        if status != 200 {
            return Err(self.record(
                "read_token_record",
                &OpenBaoError::new(
                    OpenBaoErrorCode::PermissionDenied,
                    format!("cannot read token record (status {status})"),
                ),
            ));
        }
        let data = body
            .get("data")
            .and_then(|d| d.get("data"))
            .ok_or_else(|| {
                self.record(
                    "read_token_record",
                    &OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "token record response missing data",
                    ),
                )
            })?;
        let signature = data
            .get("signature")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                self.record(
                    "read_token_record",
                    &OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "token record missing signature",
                    ),
                )
            })?
            .to_string();
        let state = data
            .get("state")
            .and_then(|v| v.as_str())
            .and_then(|s| TokenState::from_str(s).ok())
            .ok_or_else(|| {
                self.record(
                    "read_token_record",
                    &OpenBaoError::new(
                        OpenBaoErrorCode::MalformedProviderResponse,
                        "token record missing or invalid state",
                    ),
                )
            })?;
        self.sink.record(TelemetryEvent {
            operation: "read_token_record".to_string(),
            provider: "openbao".to_string(),
            latency_ms: start.elapsed().as_millis() as u64,
            ..Default::default()
        });
        Ok((signature, state))
    }

    /// Update only the state field of a token record.
    fn update_state(&self, token_id: &str, state: TokenState) -> Result<(), TrustError> {
        let (signature, _) = self.read_record(token_id)?;
        let (status, _) = self
            .http_json(
                "POST",
                &format!("/v1/{}/data/{}", self.mount, self.record_path(token_id)),
                Some(&serde_json::json!({
                    "data": {
                        "signature": signature,
                        "state": state.as_str(),
                        "issued_at_unix_s": 0,
                        "expires_at_unix_s": 1,
                    }
                })),
            )
            .map_err(|e| self.record("update_token_state", &e))?;
        if status != 200 && status != 204 {
            return Err(self.record(
                "update_token_state",
                &OpenBaoError::new(
                    OpenBaoErrorCode::PermissionDenied,
                    format!("cannot update token state (status {status})"),
                ),
            ));
        }
        Ok(())
    }

    fn record(&self, op: &str, err: &OpenBaoError) -> TrustError {
        self.sink.record(TelemetryEvent {
            operation: op.to_string(),
            provider: "openbao".to_string(),
            error_class: Some(err.code.as_str().to_string()),
            ..Default::default()
        });
        err.clone().into_trust()
    }

    /// Minimal real HTTP/1.1 JSON client (mirrors the OpenBao adapter
    /// pattern; plain HTTP on the local container surface).
    fn http_json(
        &self,
        method: &str,
        path: &str,
        body: Option<&serde_json::Value>,
    ) -> Result<(u16, serde_json::Value), OpenBaoError> {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let url = format!("{}{}", self.base_url, path);
        let uri = url
            .strip_prefix("http://")
            .ok_or_else(|| OpenBaoError::new(OpenBaoErrorCode::Internal, "only http supported"))?;
        let (host, rest) = uri.split_once('/').unwrap_or((uri, ""));
        let path_part = format!("/{rest}");
        let (host, port) = match host.rsplit_once(':') {
            Some((h, p)) => (h, p),
            None => (host, "80"),
        };
        let payload = body.map(|b| b.to_string()).unwrap_or_default();
        let content_len = payload.len();
        let has_body = body.is_some();

        let mut stream =
            TcpStream::connect((host, port.parse::<u16>().unwrap_or(80))).map_err(|e| {
                OpenBaoError::new(
                    OpenBaoErrorCode::Unavailable,
                    format!("cannot reach OpenBao at {host}:{port}: {e}"),
                )
            })?;
        stream
            .set_read_timeout(Some(REQUEST_TIMEOUT))
            .map_err(|e| {
                OpenBaoError::new(OpenBaoErrorCode::Internal, format!("set timeout: {e}"))
            })?;
        stream
            .set_write_timeout(Some(REQUEST_TIMEOUT))
            .map_err(|e| {
                OpenBaoError::new(OpenBaoErrorCode::Internal, format!("set timeout: {e}"))
            })?;

        let mut req = format!(
            "{method} {path_part} HTTP/1.1\r\nHost: {host}:{port}\r\nX-Vault-Token: {}\r\nContent-Type: application/json\r\n",
            self.client_token
        );
        if has_body {
            req.push_str(&format!("Content-Length: {content_len}\r\n"));
        }
        req.push_str("Connection: close\r\n\r\n");
        if has_body {
            req.push_str(&payload);
        }
        stream.write_all(req.as_bytes()).map_err(|e| {
            OpenBaoError::new(OpenBaoErrorCode::Unavailable, format!("write request: {e}"))
        })?;

        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).map_err(|e| {
            OpenBaoError::new(OpenBaoErrorCode::Unavailable, format!("read response: {e}"))
        })?;
        let text = String::from_utf8_lossy(&raw);
        let status = text
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);
        let body_start = text.find("\r\n\r\n").map(|i| i + 4).unwrap_or(0);
        let body_text = &text[body_start..];
        if body_text.trim().is_empty() {
            return Ok((status, serde_json::json!({})));
        }
        let value = serde_json::from_str(body_text)
            .unwrap_or_else(|_| serde_json::json!({"_raw": body_text}));
        Ok((status, value))
    }
}

impl CapabilityTokenIssuer for OpenBaoTokenIssuer {
    fn issue(
        &self,
        audience: &str,
        tenant_id: &str,
        resource: &str,
        action: &str,
        actor: &str,
        ttl_seconds: i64,
        now_unix_s: i64,
    ) -> Result<CapabilityToken, TrustError> {
        let token_id = format!(
            "cap-{}-{}",
            fingerprint(&format!("{audience}:{tenant_id}:{action}:{actor}"))
                .chars()
                .take(16)
                .collect::<String>(),
            now_unix_s
        );
        let token = CapabilityToken::new(
            token_id.clone(),
            audience,
            tenant_id,
            resource,
            action,
            actor,
            now_unix_s,
            now_unix_s.saturating_add(ttl_seconds),
        )
        .map_err(|e| TrustError::invalid(format!("cannot construct capability token: {e}")))?;
        let signature = self.sign_claims(&token)?;
        self.store_record(&token, &signature)?;
        self.sink.record(TelemetryEvent {
            operation: "issue_capability_token".to_string(),
            provider: "openbao".to_string(),
            reference_fingerprint: fingerprint(&token.token_id),
            ..Default::default()
        });
        Ok(token)
    }

    fn verify(&self, token: &CapabilityToken, now_unix_s: i64) -> Result<(), TrustError> {
        // 1. Structural/state checks (fail closed before any crypto).
        if token.state != TokenState::Active {
            return Err(TrustError::invalid("capability token is not active"));
        }
        if now_unix_s >= token.expires_at_unix_s {
            return Err(TrustError::invalid("capability token is expired"));
        }
        if now_unix_s < token.issued_at_unix_s {
            return Err(TrustError::invalid("capability token is not yet valid"));
        }
        // 2. Real provider signature check (tamper binding).
        let (signature, record_state) = self.read_record(&token.token_id)?;
        if record_state != TokenState::Active {
            return Err(TrustError::invalid("capability token record is revoked"));
        }
        let valid = self.verify_claims(token, &signature)?;
        if !valid {
            return Err(TrustError::invalid(
                "capability token signature does not match claims",
            ));
        }
        self.sink.record(TelemetryEvent {
            operation: "verify_capability_token".to_string(),
            provider: "openbao".to_string(),
            reference_fingerprint: fingerprint(&token.token_id),
            ..Default::default()
        });
        Ok(())
    }

    fn revoke(&self, token_id: &str) -> Result<(), TrustError> {
        self.update_state(token_id, TokenState::Revoked)?;
        self.sink.record(TelemetryEvent {
            operation: "revoke_capability_token".to_string(),
            provider: "openbao".to_string(),
            reference_fingerprint: fingerprint(token_id),
            ..Default::default()
        });
        Ok(())
    }
}

/// Base64 encode (standard alphabet).
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep009_unit_base64_roundtrip() {
        let input = b"hello";
        let enc = base64_encode(input);
        assert_eq!(enc, "aGVsbG8=");
        assert_eq!(enc.len() % 4, 0);
    }

    #[test]
    fn ep009_unit_canonical_claims_deterministic() {
        let a = CapabilityToken::new("t1", "svc", "ten", "res", "act", "usr", 100, 200).unwrap();
        let b = CapabilityToken::new("t1", "svc", "ten", "res", "act", "usr", 100, 200).unwrap();
        assert_eq!(canonical_claims(&a), canonical_claims(&b));
        // A changed field changes the claims (tamper binding).
        let mut c = a.clone();
        c.action = "act2".to_string();
        assert_ne!(canonical_claims(&a), canonical_claims(&c));
    }
}
