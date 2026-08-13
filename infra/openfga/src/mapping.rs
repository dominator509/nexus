//! Canonical Nexus-to-OpenFGA mapping and the `RelationshipAuthorizer`
//! implementation (EP-008 M3).
//!
//! Mapping rules (verified against the pinned OpenFGA 1.18.1 container
//! in the M3 Decision Log):
//! - principal -> `user:<principal_id>` (all canonical principal types
//!   map to the OpenFGA `user` type; the canonical actor type is kept
//!   in telemetry, not in the relationship model);
//! - object -> `<object_type>:<tenant_id>|<object_id>` - the tenant id
//!   is embedded in the object identifier so an identically named
//!   object in another tenant is a DIFFERENT OpenFGA object (tenant
//!   isolation; verified live, colon separator rejected by the
//!   provider, pipe separator accepted);
//! - relation -> canonical relation name (owner/member/admin/operator/
//!   viewer/editor/delegated).
//!
//! The adapter performs the real HTTP check against the configured
//! OpenFGA store and classifies every failure into the typed surface.
//! No local decision cache: every check hits the provider (directive
//! G - correctness over latency; revocation takes effect on the next
//! provider read).

use std::time::{Duration, Instant};

use nexus_domain::CorrelationId;
use nexus_identity::Principal;
use nexus_policy::error::PolicyError;
use nexus_policy::relationship::{RelationshipAuthorizer, RelationshipDecision, RelationshipTuple};

use crate::error::{OpenFgaError, OpenFgaErrorCode};
use crate::telemetry::{TelemetryEvent, TelemetrySink};

/// Connection/read/write budget for the OpenFGA sidecar surface.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// Adapter configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenFgaConfig {
    /// Base URL of the OpenFGA HTTP surface, e.g.
    /// `http://127.0.0.1:8080`.
    pub base_url: String,
    /// Store identifier (created at bootstrap).
    pub store_id: String,
    /// Authorization model identifier (created at bootstrap).
    pub model_id: String,
    /// Correlation for requests when the caller does not provide one.
    pub default_correlation: Option<CorrelationId>,
}

impl OpenFgaConfig {
    /// Construct an adapter config; rejects empty base/store/model.
    pub fn new(
        base_url: impl Into<String>,
        store_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Result<Self, OpenFgaError> {
        let base_url = base_url.into();
        let store_id = store_id.into();
        let model_id = model_id.into();
        if base_url.trim().is_empty() {
            return Err(OpenFgaError::new(
                OpenFgaErrorCode::InvalidRelationshipRequest,
                "base_url must not be empty",
            ));
        }
        if store_id.trim().is_empty() {
            return Err(OpenFgaError::new(
                OpenFgaErrorCode::ModelStoreMismatch,
                "store_id must not be empty",
            ));
        }
        if model_id.trim().is_empty() {
            return Err(OpenFgaError::new(
                OpenFgaErrorCode::ModelStoreMismatch,
                "model_id must not be empty",
            ));
        }
        Ok(Self {
            base_url,
            store_id,
            model_id,
            default_correlation: None,
        })
    }

    /// Set the default correlation id.
    pub fn with_correlation(mut self, correlation: CorrelationId) -> Self {
        self.default_correlation = Some(correlation);
        self
    }
}

/// Canonical principal encoding: `user:<principal_id>`.
pub fn encode_user(principal: &Principal) -> String {
    format!("user:{}", principal.principal_id.as_str())
}

/// Canonical object encoding: `<object_type>:<tenant_id>|<object_id>`.
pub fn encode_object(tuple: &RelationshipTuple) -> String {
    format!(
        "{}:{}|{}",
        tuple.object_type,
        tuple.tenant_id.as_str(),
        tuple.object_id
    )
}

/// The OpenFGA `RelationshipAuthorizer` implementation.
///
/// Fail closed: any provider failure maps to a typed `PolicyError`
/// (never an allow). No local cache.
pub struct OpenFgaAuthorizer {
    config: OpenFgaConfig,
    sink: Box<dyn TelemetrySink>,
    agent: ureq::Agent,
}

impl OpenFgaAuthorizer {
    /// Construct the authorizer with a no-op telemetry sink.
    pub fn new(config: OpenFgaConfig) -> Self {
        Self::with_sink(config, crate::telemetry::NoopSink)
    }

    /// Construct the authorizer with a telemetry sink.
    pub fn with_sink(config: OpenFgaConfig, sink: impl TelemetrySink + 'static) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(REQUEST_TIMEOUT)
            .timeout_read(REQUEST_TIMEOUT)
            .timeout_write(REQUEST_TIMEOUT)
            .build();
        Self {
            config,
            sink: Box::new(sink),
            agent,
        }
    }

    /// Run a real check against OpenFGA and classify the result.
    fn check_with(&self, tuple: &RelationshipTuple) -> Result<RelationshipDecision, OpenFgaError> {
        let user = encode_user(&tuple.principal);
        let object = encode_object(tuple);
        let url = format!(
            "{}/stores/{}/check",
            self.config.base_url.trim_end_matches('/'),
            self.config.store_id
        );
        let body = serde_json::json!({
            "tuple_key": {
                "user": user,
                "relation": tuple.relation,
                "object": object,
            },
            "authorization_model_id": self.config.model_id,
        });

        let started = Instant::now();
        let result = self
            .agent
            .post(&url)
            .send_json(body)
            .map_err(|err| self.classify_transport(&url, &err));

        let latency_ms = started.elapsed().as_millis() as u64;
        match result {
            Ok(resp) => {
                let status = resp.status();
                if !(200..300).contains(&status) {
                    // HTTP error from the provider surface.
                    let detail = resp.into_string().unwrap_or_default();
                    return Err(self.classify_http(status, &detail));
                }
                let json: serde_json::Value = resp
                    .into_json()
                    .map_err(|_| OpenFgaError::malformed("check response was not JSON"))?;
                let allowed = json
                    .get("allowed")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| {
                        OpenFgaError::malformed("check response missing boolean `allowed`")
                    })?;
                let decision = if allowed {
                    RelationshipDecision::Allowed
                } else {
                    RelationshipDecision::Denied {
                        reason: "relationship does not hold".to_string(),
                    }
                };
                self.emit_ok(tuple, &decision, latency_ms);
                Ok(decision)
            }
            Err(err) => {
                self.emit_err(tuple, &err, latency_ms);
                Err(err)
            }
        }
    }

    /// Classify a transport-level failure.
    fn classify_transport(&self, url: &str, err: &ureq::Error) -> OpenFgaError {
        match err {
            ureq::Error::Status(code, _) => {
                // Should not happen: status errors are handled on the
                // Ok path above; guard anyway (fail closed).
                OpenFgaError::new(
                    OpenFgaErrorCode::MalformedProviderResponse,
                    format!("unexpected HTTP status {code} from {url}"),
                )
            }
            ureq::Error::Transport(t) => match t.kind() {
                ureq::ErrorKind::ConnectionFailed | ureq::ErrorKind::Dns | ureq::ErrorKind::Io => {
                    OpenFgaError::unavailable(format!(
                        "cannot reach OpenFGA at {url}: {}",
                        t.message().unwrap_or("connection failed")
                    ))
                }
                ureq::ErrorKind::TooManyRedirects => {
                    OpenFgaError::malformed("provider sent too many redirects")
                }
                _ => OpenFgaError::unavailable(format!(
                    "provider transport error at {url}: {}",
                    t.message().unwrap_or("unknown")
                )),
            },
        }
    }

    /// Classify an HTTP status response.
    fn classify_http(&self, status: u16, detail: &str) -> OpenFgaError {
        let redacted = redact_detail(detail);
        match status {
            400 => {
                // Validation failures: invalid relationship shape OR
                // unknown model/store. Distinguish by message (case
                // insensitive; the provider uses `AuthorizationModelId`
                // in camelCase, verified live).
                let lower = redacted.to_lowercase();
                if lower.contains("authorizationmodel") || lower.contains(" store ") {
                    OpenFgaError::mismatch(redacted)
                } else {
                    OpenFgaError::invalid_request(redacted)
                }
            }
            401 | 403 => OpenFgaError::authorization(redacted),
            404 => OpenFgaError::mismatch(redacted),
            429 => OpenFgaError::new(
                OpenFgaErrorCode::Unavailable,
                format!("provider rate limited: {redacted}"),
            ),
            500..=599 => OpenFgaError::unavailable(redacted),
            _ => OpenFgaError::new(
                OpenFgaErrorCode::MalformedProviderResponse,
                format!("unexpected HTTP status {status}: {redacted}"),
            ),
        }
    }

    fn emit_ok(&self, tuple: &RelationshipTuple, decision: &RelationshipDecision, latency_ms: u64) {
        self.sink.emit(
            TelemetryEvent::decision(tuple, decision.is_allowed(), latency_ms, None, None)
                .with_fingerprint(&self.config.store_id, &self.config.model_id)
                .with_correlation(self.config.default_correlation.clone()),
        );
    }

    fn emit_err(&self, tuple: &RelationshipTuple, err: &OpenFgaError, latency_ms: u64) {
        self.sink.emit(
            TelemetryEvent::decision(
                tuple,
                false,
                latency_ms,
                Some(err.code),
                Some(err.message.clone()),
            )
            .with_fingerprint(&self.config.store_id, &self.config.model_id)
            .with_correlation(self.config.default_correlation.clone()),
        );
    }
}

/// Redact provider detail strings: never preserve secrets/tokens; keep
/// only safe, short explanations.
fn redact_detail(detail: &str) -> String {
    let trimmed = detail.trim();
    if trimmed.is_empty() {
        return "no detail".to_string();
    }
    // Provider messages are safe (error codes + short text) but cap the
    // length so no full payload can leak into telemetry.
    let mut s = trimmed.chars().take(300).collect::<String>();
    if trimmed.chars().count() > 300 {
        s.push_str("...");
    }
    s
}

impl RelationshipAuthorizer for OpenFgaAuthorizer {
    fn check(&self, tuple: &RelationshipTuple) -> Result<RelationshipDecision, PolicyError> {
        self.check_with(tuple).map_err(OpenFgaError::into_policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::RecordingSink;
    use nexus_domain::{CorrelationId, NexusId, PrincipalType, TenantId};

    fn id(s: &str) -> NexusId {
        NexusId::new(s).expect("valid nexus id")
    }

    fn tid(s: &str) -> TenantId {
        TenantId::new(s).expect("valid tenant id")
    }

    fn principal(pid: &str, tenant: &str) -> Principal {
        Principal::new(id(pid), PrincipalType::Human, tid(tenant))
    }

    fn tuple(principal: Principal, relation: &str, otype: &str, oid: &str) -> RelationshipTuple {
        RelationshipTuple::new(principal.tenant_id.clone(), principal, relation, otype, oid)
            .expect("valid tuple")
    }

    #[test]
    fn ep008_unit_mapping_user_and_object_are_canonical() {
        let p = principal(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02",
        );
        let t = tuple(
            p,
            "owner",
            "household",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
        );
        assert_eq!(
            encode_user(&t.principal),
            "user:0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"
        );
        assert_eq!(
            encode_object(&t),
            "household:0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02|0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03"
        );
    }

    #[test]
    fn ep008_unit_config_rejects_empty_parts() {
        assert!(OpenFgaConfig::new("", "s", "m").is_err());
        assert!(OpenFgaConfig::new("http://x", "", "m").is_err());
        assert!(OpenFgaConfig::new("http://x", "s", "").is_err());
        assert!(OpenFgaConfig::new("http://x", "s", "m").is_ok());
    }

    #[test]
    fn ep008_unit_classify_http_maps_typed_codes() {
        let cfg = OpenFgaConfig::new("http://x", "s", "m").unwrap();
        let auth = OpenFgaAuthorizer::new(cfg);
        assert_eq!(
            auth.classify_http(400, "invalid CheckRequest.AuthorizationModelId: ...")
                .code,
            OpenFgaErrorCode::ModelStoreMismatch
        );
        assert_eq!(
            auth.classify_http(
                400,
                "invalid relation: relation 'household#nonexistent' not found"
            )
            .code,
            OpenFgaErrorCode::InvalidRelationshipRequest
        );
        assert_eq!(
            auth.classify_http(401, "unauthorized").code,
            OpenFgaErrorCode::ProviderAuthorizationFailure
        );
        assert_eq!(
            auth.classify_http(404, "store not found").code,
            OpenFgaErrorCode::ModelStoreMismatch
        );
        assert_eq!(
            auth.classify_http(503, "unavailable").code,
            OpenFgaErrorCode::Unavailable
        );
    }

    #[test]
    fn ep008_unit_noop_sink_is_constructible() {
        let cfg = OpenFgaConfig::new("http://x", "s", "m").unwrap();
        let _auth = OpenFgaAuthorizer::new(cfg);
    }

    #[test]
    fn ep008_unit_recording_sink_receives_events() {
        let sink = RecordingSink::default();
        let cfg = OpenFgaConfig::new("http://x", "s", "m")
            .unwrap()
            .with_correlation(CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01").unwrap());
        let auth = OpenFgaAuthorizer::with_sink(cfg, sink.clone());
        let p = principal(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02",
        );
        let t = tuple(
            p,
            "owner",
            "household",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
        );
        auth.emit_ok(&t, &RelationshipDecision::Allowed, 1);
        assert_eq!(sink.events().len(), 1);
        let ev = &sink.events()[0];
        assert!(ev.allowed);
        assert_eq!(ev.relation, "owner");
        assert_eq!(ev.target_type, "household");
        assert_eq!(
            ev.correlation.as_ref().unwrap().as_str(),
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"
        );
    }
}
