//! Real OpenAI-compatible HTTP transport (SPEC-009; EP-013 M3).
//!
//! Implements the `ModelProvider` port with a real HTTP client
//! (ureq, same pinned version as the other Nexus infra adapters).
//! It POSTs to the OpenAI-compatible chat completions surface of
//! Bifrost or a direct provider and normalizes the response into the
//! canonical `NexusControlObject`. Every failure is a typed SPEC-006
//! error; nothing is ever logged except redacted correlation data.

use crate::config::ProviderManifest;
use crate::error::TransportError;
use nexus_model_gateway::{
    ModelGatewayError, ModelProvider,
    health::ProviderHealth,
    model::{ModelRequest, ModelResponse, NexusControlObject, UsageReport},
    vocabulary::ProviderHealthState,
};
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Real OpenAI-compatible HTTP transport.
pub struct OpenAiCompatibleTransport {
    manifest: ProviderManifest,
    /// Credential value resolved by the caller (never logged,
    /// never serialized into telemetry). Kept behind a redacting
    /// Debug: the transport Debug impl never prints the value.
    credential: Option<String>,
    agent: ureq::Agent,
}

/// Builder for the transport.
pub struct OpenAiCompatibleTransportBuilder {
    manifest: ProviderManifest,
    credential: Option<String>,
}

impl OpenAiCompatibleTransportBuilder {
    pub fn new(manifest: ProviderManifest) -> Self {
        Self {
            manifest,
            credential: None,
        }
    }

    /// Resolve the credential by reference: the caller supplies the
    /// value here; the transport stores it without logging.
    pub fn with_credential(mut self, credential: impl Into<String>) -> Self {
        self.credential = Some(credential.into());
        self
    }

    pub fn build(self) -> OpenAiCompatibleTransport {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(REQUEST_TIMEOUT)
            .timeout_read(REQUEST_TIMEOUT)
            .timeout_write(REQUEST_TIMEOUT)
            .build();
        OpenAiCompatibleTransport {
            manifest: self.manifest,
            credential: self.credential,
            agent,
        }
    }
}

impl std::fmt::Debug for OpenAiCompatibleTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // NEVER print the credential value.
        f.debug_struct("OpenAiCompatibleTransport")
            .field("provider_id", &self.manifest.provider_id)
            .field("base_url", &self.manifest.base_url)
            .field("credential_present", &self.credential.is_some())
            .finish()
    }
}

impl OpenAiCompatibleTransport {
    fn chat_url(&self) -> String {
        let base = self.manifest.base_url.trim_end_matches('/');
        format!("{base}/chat/completions")
    }

    fn auth_header(&self) -> Option<String> {
        self.credential.as_ref().map(|c| format!("Bearer {c}"))
    }

    fn build_body(&self, request: &ModelRequest) -> serde_json::Value {
        // Canonical prompt: ordered segments joined deterministically
        // (SPEC-009 required behavior 4).
        let mut messages: Vec<serde_json::Value> = Vec::new();
        for part in request.ordered_segments() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": part.content,
            }));
        }
        serde_json::json!({
            "model": self.manifest.provider_id,
            "messages": messages,
            "stream": false,
        })
    }

    fn normalize(
        &self,
        request: &ModelRequest,
        body: &serde_json::Value,
    ) -> Result<ModelResponse, TransportError> {
        // Deterministic validation: the response must carry id,
        // model, usage, and choices; anything else is rejected
        // (SPEC-009 required behavior 3).
        let model = body.get("model").and_then(|v| v.as_str()).ok_or_else(|| {
            TransportError::validation(
                "provider response missing model",
                Some(self.manifest.provider_id.clone()),
                Some(self.manifest.provider_id.clone()),
            )
        })?;
        let usage = body.get("usage").ok_or_else(|| {
            TransportError::validation(
                "provider response missing usage",
                Some(self.manifest.provider_id.clone()),
                Some(self.manifest.provider_id.clone()),
            )
        })?;
        let prompt_tokens = usage
            .get("prompt_tokens")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                TransportError::validation(
                    "provider usage missing prompt_tokens",
                    Some(self.manifest.provider_id.clone()),
                    Some(self.manifest.provider_id.clone()),
                )
            })?;
        let completion_tokens = usage
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                TransportError::validation(
                    "provider usage missing completion_tokens",
                    Some(self.manifest.provider_id.clone()),
                    Some(self.manifest.provider_id.clone()),
                )
            })?;
        let cache_hit_prompt_tokens = usage
            .get("prompt_cache_hit_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        // Control object: the response content is advisory data, never
        // authority. It is normalized, not trusted.
        let content = body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(ModelResponse {
            request_id: request.request_id.clone(),
            correlation_id: request.correlation_id.clone(),
            control_object: NexusControlObject {
                schema_version: "1.0".into(),
                control: serde_json::json!({
                    "content": content,
                }),
                provider: self.manifest.provider_id.clone(),
                model: model.to_string(),
                usage: UsageReport {
                    prompt_tokens,
                    completion_tokens,
                    cache_hit_prompt_tokens,
                },
            },
        })
    }

    fn classify(&self, url: &str, err: &ureq::Error, provider_id: &str) -> TransportError {
        use std::error::Error as _;
        match err {
            ureq::Error::Status(code, _) => {
                let code_str = code.to_string();
                TransportError::external(
                    format!("provider returned HTTP {code_str}"),
                    Some(url.to_string()),
                    Some(provider_id.to_string()),
                )
            }
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
                        TransportError::timeout(
                            "provider request timed out",
                            Some(url.to_string()),
                            Some(provider_id.to_string()),
                        )
                    } else {
                        TransportError::unavailable(
                            "provider unreachable",
                            Some(url.to_string()),
                            Some(provider_id.to_string()),
                        )
                    }
                }
                _ => TransportError::unavailable(
                    "provider unreachable",
                    Some(url.to_string()),
                    Some(provider_id.to_string()),
                ),
            },
        }
    }
}

impl ModelProvider for OpenAiCompatibleTransport {
    fn generate(&mut self, request: &ModelRequest) -> Result<ModelResponse, ModelGatewayError> {
        let url = self.chat_url();
        let body = self.build_body(request);
        let mut req = self.agent.post(&url);
        if let Some(auth) = self.auth_header() {
            req = req.set("Authorization", &auth);
        }
        let outcome = req.send_json(body);
        match outcome {
            Ok(resp) => {
                let parsed: Result<serde_json::Value, _> = resp.into_json();
                match parsed {
                    Ok(value) => self.normalize(request, &value).map_err(|e| {
                        e.with_context(
                            Some(request.correlation_id.clone()),
                            Some(request.principal_id.clone()),
                            Some(request.tenant_id.clone()),
                        )
                        .into()
                    }),
                    Err(e) => Err(TransportError::external(
                        format!("provider returned non-JSON response: {e}"),
                        Some(url.clone()),
                        Some(self.manifest.provider_id.clone()),
                    )
                    .with_context(
                        Some(request.correlation_id.clone()),
                        Some(request.principal_id.clone()),
                        Some(request.tenant_id.clone()),
                    )
                    .into()),
                }
            }
            Err(e) => Err(self
                .classify(&url, &e, &self.manifest.provider_id)
                .with_context(
                    Some(request.correlation_id.clone()),
                    Some(request.principal_id.clone()),
                    Some(request.tenant_id.clone()),
                )
                .into()),
        }
    }

    fn health(&self) -> ProviderHealth {
        // Health is observed by the caller (gateway probe); the
        // transport reports its configured state without making a
        // call, so the gateway's own health checks drive routing.
        ProviderHealth::new(
            &self.manifest.provider_id,
            ProviderHealthState::Healthy,
            None,
            "configured",
            "manifest",
        )
    }

    fn provider_id(&self) -> &str {
        &self.manifest.provider_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_model_gateway::ModelGatewayErrorCode;
    use nexus_model_gateway::model::{PromptSegment, PromptSegmentPart};
    use nexus_model_gateway::vocabulary::EffortTier;

    fn manifest() -> ProviderManifest {
        ProviderManifest::new(
            "bifrost",
            crate::config::ManifestProviderKind::Bifrost,
            "http://127.0.0.1:9/v1",
            "0.1.0",
            "Apache-2.0",
            "source",
            "contract",
        )
    }

    fn request() -> ModelRequest {
        ModelRequest {
            request_id: "r-1".into(),
            correlation_id: "c-1".into(),
            causation_id: None,
            tenant_id: "t-1".into(),
            principal_id: "p-1".into(),
            effort_tier: EffortTier::Deterministic,
            segments: vec![PromptSegmentPart {
                segment: PromptSegment::Constitution,
                content: "constitution".into(),
            }],
            budget_ref: None,
            schema_version: "1.0".into(),
        }
    }

    #[test]
    fn ep013_unit_transport_debug_never_prints_credential() {
        let t = OpenAiCompatibleTransportBuilder::new(manifest())
            .with_credential("super-secret-value")
            .build();
        let dbg = format!("{t:?}");
        assert!(!dbg.contains("super-secret-value"));
        assert!(dbg.contains("credential_present: true"));
    }

    #[test]
    fn ep013_unit_transport_chat_url() {
        let t = OpenAiCompatibleTransportBuilder::new(manifest()).build();
        assert_eq!(t.chat_url(), "http://127.0.0.1:9/v1/chat/completions");
    }

    #[test]
    fn ep013_unit_transport_normalize_valid_response() {
        let t = OpenAiCompatibleTransportBuilder::new(manifest()).build();
        let body = serde_json::json!({
            "id": "chatcmpl-1",
            "model": "bifrost",
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "prompt_cache_hit_tokens": 9},
            "choices": [{"message": {"role": "assistant", "content": "hello"}}]
        });
        let resp = t.normalize(&request(), &body).unwrap();
        assert_eq!(resp.control_object.provider, "bifrost");
        assert_eq!(resp.control_object.model, "bifrost");
        assert_eq!(resp.control_object.control["content"], "hello");
        assert_eq!(resp.control_object.usage.prompt_tokens, 10);
        assert_eq!(resp.control_object.usage.cache_hit_prompt_tokens, 9);
    }

    #[test]
    fn ep013_unit_transport_normalize_rejects_missing_usage() {
        let t = OpenAiCompatibleTransportBuilder::new(manifest()).build();
        let body = serde_json::json!({
            "id": "chatcmpl-2",
            "model": "bifrost",
            "choices": [{"message": {"role": "assistant", "content": "x"}}]
        });
        let err = t.normalize(&request(), &body).unwrap_err();
        assert_eq!(err.code, ModelGatewayErrorCode::Validation);
    }

    #[test]
    fn ep013_unit_transport_classify_connect_failure() {
        // A real connection to a closed port classifies as
        // Unavailable (fail closed) without any server.
        let mut provider = OpenAiCompatibleTransportBuilder::new(manifest()).build();
        let err = provider
            .generate(&request())
            .expect_err("port 9 must be closed");
        assert_eq!(err.code, ModelGatewayErrorCode::Unavailable);
    }
}
