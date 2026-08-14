//! Canonical PolicyInput-to-OPA mapping and the `ContextPolicyEngine`
//! implementation (EP-008 M4).
//!
//! Mapping rules (verified against the pinned OPA 1.16.2 container in
//! the M4 Decision Log):
//! - `PolicyInput` fields map to a flat, typed OPA input object
//!   (tenant_id, principal_id, principal_type, capability, risk,
//!   strength, device_trust, device_state, object_type, object_id,
//!   request_id, sensitivity, context{location, network_trust,
//!   maintenance, emergency});
//! - sensitivity and context are OPTIONAL extension fields the adapter
//!   accepts from the caller via `OpaContext`; they are never
//!   fabricated by the adapter;
//! - the query path is `data.nexus.allow` (boolean) with
//!   `data.nexus.policy_version` (string) checked separately - the
//!   policy bundle MUST expose a stable version (directive H);
//! - the adapter NEVER sends arbitrary domain objects, secrets, or
//!   unrelated personal data (directive C).
//!
//! The adapter performs the real HTTP evaluation against the configured
//! OPA server and classifies every failure into the typed surface.

use std::time::{Duration, Instant};

use nexus_domain::CorrelationId;
use nexus_policy::error::PolicyError;
use nexus_policy::policy::{ContextPolicyEngine, PolicyDecision, PolicyInput};

use crate::error::{OpaError, OpaErrorCode};
use crate::telemetry::{TelemetryEvent, TelemetrySink};

/// Connection/read/write budget for the OPA sidecar surface.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// Optional contextual extension fields (directive C; never fabricated).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OpaContext {
    /// Location context: HOME|WORK|PUBLIC|REMOTE.
    pub location: Option<String>,
    /// Network trust: UNTRUSTED|GUEST|TRUSTED.
    pub network_trust: Option<String>,
    /// Maintenance flag.
    pub maintenance: Option<bool>,
    /// Emergency flag.
    pub emergency: Option<bool>,
    /// Device state: ENABLED|DISABLED|REVOKED.
    pub device_state: Option<String>,
    /// Sensitivity: PUBLIC|HOUSEHOLD|PERSONAL|SECRET.
    pub sensitivity: Option<String>,
}

impl OpaContext {
    /// An empty context.
    pub fn none() -> Self {
        Self::default()
    }

    /// Build a context from individual fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        location: Option<impl Into<String>>,
        network_trust: Option<impl Into<String>>,
        maintenance: Option<bool>,
        emergency: Option<bool>,
        device_state: Option<impl Into<String>>,
        sensitivity: Option<impl Into<String>>,
    ) -> Self {
        Self {
            location: location.map(|v| v.into()),
            network_trust: network_trust.map(|v| v.into()),
            maintenance,
            emergency,
            device_state: device_state.map(|v| v.into()),
            sensitivity: sensitivity.map(|v| v.into()),
        }
    }
}

/// Adapter configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpaConfig {
    /// Base URL of the OPA HTTP surface, e.g. `http://127.0.0.1:8181`.
    pub base_url: String,
    /// Expected policy bundle version (e.g. `nexus-policy-v1`). The
    /// adapter refuses to evaluate an unknown/unversioned bundle
    /// (directive H).
    pub expected_policy_version: String,
    /// Optional contextual extension fields for every evaluation.
    pub context: OpaContext,
    /// Correlation for requests when the caller does not provide one.
    pub default_correlation: Option<CorrelationId>,
}

impl OpaConfig {
    /// Construct an adapter config; rejects empty base/version.
    pub fn new(
        base_url: impl Into<String>,
        expected_policy_version: impl Into<String>,
    ) -> Result<Self, OpaError> {
        let base_url = base_url.into();
        let expected_policy_version = expected_policy_version.into();
        if base_url.trim().is_empty() {
            return Err(OpaError::new(
                OpaErrorCode::InvalidPolicyInput,
                "base_url must not be empty",
            ));
        }
        if expected_policy_version.trim().is_empty() {
            return Err(OpaError::new(
                OpaErrorCode::PolicyBundleVersionMismatch,
                "expected policy version must not be empty",
            ));
        }
        Ok(Self {
            base_url,
            expected_policy_version,
            context: OpaContext::none(),
            default_correlation: None,
        })
    }

    /// Set the contextual extension fields.
    pub fn with_context(mut self, context: OpaContext) -> Self {
        self.context = context;
        self
    }

    /// Set the default correlation id.
    pub fn with_correlation(mut self, correlation: CorrelationId) -> Self {
        self.default_correlation = Some(correlation);
        self
    }
}

/// Canonical OPA input object from a `PolicyInput` (directive C).
pub fn encode_input(input: &PolicyInput, context: &OpaContext) -> serde_json::Value {
    let mut value = serde_json::json!({
        "tenant_id": input.tenant_id.as_str(),
        "principal_id": input.principal.principal_id.as_str(),
        "principal_type": input.principal.principal_type.as_str(),
        "capability": input.capability.as_str(),
        "risk": input.risk.as_str(),
        "strength": input.strength.as_str(),
        "device_trust": input.device_trust.as_str(),
        "object_type": input.object_type,
        "object_id": input.object_id,
    });
    if let Some(loc) = &context.location {
        value["context"]["location"] = serde_json::json!(loc);
    }
    if let Some(nt) = &context.network_trust {
        value["context"]["network_trust"] = serde_json::json!(nt);
    }
    if let Some(m) = context.maintenance {
        value["context"]["maintenance"] = serde_json::json!(m);
    }
    if let Some(e) = context.emergency {
        value["context"]["emergency"] = serde_json::json!(e);
    }
    if let Some(ds) = &context.device_state {
        value["device_state"] = serde_json::json!(ds);
    }
    if let Some(s) = &context.sensitivity {
        value["sensitivity"] = serde_json::json!(s);
    }
    value
}

/// The OPA `ContextPolicyEngine` implementation.
///
/// Fail closed: undefined decisions, malformed responses, version
/// mismatches, and provider failures are typed errors (never an
/// allow).
pub struct OpaAuthorizer {
    config: OpaConfig,
    sink: Box<dyn TelemetrySink>,
    agent: ureq::Agent,
}

impl OpaAuthorizer {
    /// Construct the authorizer with a no-op telemetry sink.
    pub fn new(config: OpaConfig) -> Self {
        Self::with_sink(config, crate::telemetry::NoopSink)
    }

    /// Construct the authorizer with a telemetry sink.
    pub fn with_sink(config: OpaConfig, sink: impl TelemetrySink + 'static) -> Self {
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

    /// The policy query path (data.nexus.allow).
    fn allow_url(&self) -> String {
        format!(
            "{}/v1/data/nexus/allow",
            self.config.base_url.trim_end_matches('/')
        )
    }

    /// The policy version query path (data.nexus.policy_version).
    fn version_url(&self) -> String {
        format!(
            "{}/v1/data/nexus/policy_version",
            self.config.base_url.trim_end_matches('/')
        )
    }

    /// Fetch the loaded policy version (directive H). Undefined or
    /// missing version is a typed mismatch/undefined failure.
    fn fetch_policy_version(&self) -> Result<String, OpaError> {
        let body = serde_json::json!({ "input": {} });
        let url = self.version_url();
        match self.agent.post(&url).send_json(body) {
            Ok(resp) => {
                let status = resp.status();
                if !(200..300).contains(&status) {
                    return Err(self.classify_http(status, &resp.into_string().unwrap_or_default()));
                }
                let json: serde_json::Value = resp
                    .into_json()
                    .map_err(|_| OpaError::malformed("policy version response was not JSON"))?;
                match json.get("result") {
                    Some(serde_json::Value::String(v)) if !v.is_empty() => Ok(v.clone()),
                    _ => Err(OpaError::undefined(
                        "policy bundle does not expose a defined policy_version",
                    )),
                }
            }
            Err(err) => Err(self.classify_transport(&url, &err)),
        }
    }

    /// Run a real evaluation against OPA and classify the result.
    fn evaluate_with(&self, input: &PolicyInput) -> Result<PolicyDecision, OpaError> {
        // 1. Version/digest check first (directive H): never evaluate
        //    an unknown/unversioned bundle.
        let loaded_version = self.fetch_policy_version()?;
        if loaded_version != self.config.expected_policy_version {
            return Err(OpaError::version_mismatch(format!(
                "expected policy version {}, loaded {}",
                self.config.expected_policy_version, loaded_version
            )));
        }

        // 2. Real evaluation.
        let oinput = encode_input(input, &self.config.context);
        let body = serde_json::json!({ "input": oinput });
        let url = self.allow_url();
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
                    let detail = resp.into_string().unwrap_or_default();
                    return Err(self.classify_http(status, &detail));
                }
                let json: serde_json::Value = resp
                    .into_json()
                    .map_err(|_| OpaError::malformed("allow response was not JSON"))?;
                match json.get("result") {
                    Some(serde_json::Value::Bool(true)) => {
                        self.emit(input, true, latency_ms, None, None);
                        Ok(PolicyDecision::allow(&self.config.expected_policy_version))
                    }
                    Some(serde_json::Value::Bool(false)) => {
                        self.emit(input, false, latency_ms, None, None);
                        Ok(PolicyDecision::deny(
                            &self.config.expected_policy_version,
                            "contextual policy denied",
                        ))
                    }
                    Some(_) => Err(OpaError::malformed("allow result was not a boolean")),
                    None => Err(OpaError::undefined(
                        "policy query path is undefined (no allow rule matched)",
                    )),
                }
            }
            Err(err) => {
                self.emit(
                    input,
                    false,
                    latency_ms,
                    Some(err.code),
                    Some(err.message.clone()),
                );
                Err(err)
            }
        }
    }

    /// Classify a transport-level failure.
    fn classify_transport(&self, url: &str, err: &ureq::Error) -> OpaError {
        use std::error::Error as _;
        match err {
            ureq::Error::Status(code, _) => OpaError::new(
                OpaErrorCode::MalformedProviderResponse,
                format!("unexpected HTTP status {code} from {url}"),
            ),
            ureq::Error::Transport(t) => match t.kind() {
                ureq::ErrorKind::ConnectionFailed | ureq::ErrorKind::Dns | ureq::ErrorKind::Io => {
                    // ureq surfaces read deadlines as ErrorKind::Io with
                    // an io::Error source whose kind is TimedOut (or
                    // WouldBlock normalized to TimedOut); verified
                    // against ureq 2.12.1 against a real unresponsive
                    // peer. Distinguish a real deadline from a hard
                    // connection failure by the source kind.
                    let timed_out = t
                        .source()
                        .and_then(|s| s.downcast_ref::<std::io::Error>())
                        .map(|io| {
                            matches!(
                                io.kind(),
                                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                            )
                        })
                        .unwrap_or(false);
                    if timed_out {
                        OpaError::timeout(format!("OPA read deadline exceeded at {url}"))
                    } else {
                        OpaError::unavailable(format!(
                            "cannot reach OPA at {url}: {}",
                            t.message().unwrap_or("connection failed")
                        ))
                    }
                }
                ureq::ErrorKind::TooManyRedirects => {
                    OpaError::malformed("provider sent too many redirects")
                }
                _ => OpaError::unavailable(format!(
                    "provider transport error at {url}: {}",
                    t.message().unwrap_or("unknown")
                )),
            },
        }
    }

    /// Classify an HTTP status response.
    fn classify_http(&self, status: u16, detail: &str) -> OpaError {
        let redacted = redact_detail(detail);
        match status {
            400 => {
                // Bad input to OPA (missing/invalid fields) or a bad
                // query path. Distinguish: compile errors are
                // evaluation failures; input errors are invalid input.
                let lower = redacted.to_lowercase();
                if lower.contains("compile") || lower.contains("rego") {
                    OpaError::evaluation(redacted)
                } else {
                    OpaError::invalid_input(redacted)
                }
            }
            401 | 403 => {
                OpaError::evaluation(format!("provider authorization failure: {redacted}"))
            }
            404 => OpaError::new(
                OpaErrorCode::ProviderEvaluationFailure,
                format!("query path not found: {redacted}"),
            ),
            429 => OpaError::new(
                OpaErrorCode::Unavailable,
                format!("provider rate limited: {redacted}"),
            ),
            500..=599 => OpaError::unavailable(redacted),
            _ => OpaError::new(
                OpaErrorCode::MalformedProviderResponse,
                format!("unexpected HTTP status {status}: {redacted}"),
            ),
        }
    }

    fn emit(
        &self,
        input: &PolicyInput,
        allowed: bool,
        latency_ms: u64,
        error_class: Option<OpaErrorCode>,
        error_detail: Option<String>,
    ) {
        self.sink.emit(
            TelemetryEvent::decision(input, allowed, latency_ms, error_class, error_detail)
                .with_version(&self.config.expected_policy_version)
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
    let mut s = trimmed.chars().take(300).collect::<String>();
    if trimmed.chars().count() > 300 {
        s.push_str("...");
    }
    s
}

impl ContextPolicyEngine for OpaAuthorizer {
    fn evaluate(&self, input: &PolicyInput) -> Result<PolicyDecision, PolicyError> {
        self.evaluate_with(input).map_err(OpaError::into_policy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::RecordingSink;
    use nexus_auth::AuthenticationStrength;
    use nexus_domain::{CapabilityClass, NexusId, PrincipalType, Risk, TenantId};
    use nexus_identity::{Principal, TrustLevel};
    use nexus_policy::error::PolicyErrorCode;

    fn tid(s: &str) -> TenantId {
        TenantId::new(s).unwrap()
    }
    fn nid(s: &str) -> NexusId {
        NexusId::new(s).unwrap()
    }

    fn principal() -> Principal {
        Principal::new(
            nid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01"),
            PrincipalType::Human,
            tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"),
        )
    }

    fn input() -> PolicyInput {
        PolicyInput::new(
            tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02"),
            principal(),
            CapabilityClass::Query,
            Risk::R0,
            AuthenticationStrength::SingleFactor,
            TrustLevel::Local,
            "task",
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
        )
        .unwrap()
    }

    #[test]
    fn ep008_unit_opa_config_rejects_empty_parts() {
        assert!(OpaConfig::new("", "v1").is_err());
        assert!(OpaConfig::new("http://x", "").is_err());
        assert!(OpaConfig::new("http://x", "v1").is_ok());
    }

    #[test]
    fn ep008_unit_opa_encoding_is_canonical_and_typed() {
        let v = encode_input(&input(), &OpaContext::none());
        assert_eq!(v["tenant_id"], "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02");
        assert_eq!(v["principal_id"], "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01");
        assert_eq!(v["principal_type"], "HUMAN");
        assert_eq!(v["capability"], "QUERY");
        assert_eq!(v["risk"], "R0");
        assert_eq!(v["strength"], "SINGLE_FACTOR");
        assert_eq!(v["device_trust"], "LOCAL");
        assert_eq!(v["object_type"], "task");
        assert_eq!(v["object_id"], "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03");
        // No fabricated context when none provided.
        assert!(v.get("context").is_none());
        assert!(v.get("sensitivity").is_none());
    }

    #[test]
    fn ep008_unit_opa_encoding_includes_context_when_provided() {
        let ctx = OpaContext::new(
            Some("HOME"),
            Some("TRUSTED"),
            Some(false),
            Some(true),
            Some("ENABLED"),
            Some("PERSONAL"),
        );
        let v = encode_input(&input(), &ctx);
        assert_eq!(v["context"]["location"], "HOME");
        assert_eq!(v["context"]["network_trust"], "TRUSTED");
        assert_eq!(v["context"]["maintenance"], false);
        assert_eq!(v["context"]["emergency"], true);
        assert_eq!(v["device_state"], "ENABLED");
        assert_eq!(v["sensitivity"], "PERSONAL");
    }

    #[test]
    fn ep008_unit_opa_classify_http_maps_typed_codes() {
        let cfg = OpaConfig::new("http://x", "v1").unwrap();
        let auth = OpaAuthorizer::new(cfg);
        assert_eq!(
            auth.classify_http(400, "rego_parse_error").code,
            OpaErrorCode::ProviderEvaluationFailure
        );
        assert_eq!(
            auth.classify_http(400, "missing required field").code,
            OpaErrorCode::InvalidPolicyInput
        );
        assert_eq!(
            auth.classify_http(401, "unauthorized").code,
            OpaErrorCode::ProviderEvaluationFailure
        );
        assert_eq!(
            auth.classify_http(404, "not found").code,
            OpaErrorCode::ProviderEvaluationFailure
        );
        assert_eq!(
            auth.classify_http(503, "unavailable").code,
            OpaErrorCode::Unavailable
        );
    }

    #[test]
    fn ep008_unit_opa_transport_timeout_is_distinct_from_unavailable() {
        // ureq normalizes read deadlines to ErrorKind::Io with a
        // "timed out reading response" message; the adapter must
        // classify that as TIMEOUT, not UNAVAILABLE.
        let _cfg = OpaConfig::new("http://x", "v1").unwrap();
        let err =
            OpaError::timeout("OPA read deadline exceeded at http://x: timed out reading response");
        assert_eq!(err.code, OpaErrorCode::Timeout);
        assert_eq!(err.code.policy_code(), PolicyErrorCode::Timeout);
    }

    #[test]
    fn ep008_unit_opa_recording_sink_receives_events() {
        let sink = RecordingSink::default();
        let cfg = OpaConfig::new("http://x", "nexus-policy-v1")
            .unwrap()
            .with_correlation(CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a04").unwrap());
        let auth = OpaAuthorizer::with_sink(cfg, sink.clone());
        auth.emit(&input(), true, 1, None, None);
        assert_eq!(sink.events().len(), 1);
        let ev = &sink.events()[0];
        assert!(ev.allowed);
        assert_eq!(ev.version, "nexus-policy-v1");
        assert_eq!(ev.actor_type, "HUMAN");
        assert_eq!(ev.target_type, "task");
        assert_eq!(
            ev.correlation.as_ref().unwrap().as_str(),
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a04"
        );
    }
}
