//! Webhook normalizer (SPEC-022 behavior 2).
//!
//! The `WebhookNormalizer` port converts raw webhook deliveries into
//! canonical, versioned, correlated `WebhookEvent` records and
//! verifies signatures. Replay detection is part of the contract;
//! a delivery that fails signature or replay checks is rejected and
//! never becomes an event.

use serde::{Deserialize, Serialize};

use nexus_capabilities::context::InvocationContext;

use crate::error::SdkError;
use crate::vocabulary::{WebhookEvent, WebhookVerification};

/// Raw webhook delivery before normalization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawWebhook {
    /// Provider raw payload (normalized at this boundary).
    pub raw_payload: serde_json::Value,
    /// Optional signature header content.
    pub signature: Option<String>,
    /// Optional provider event id.
    pub provider_event_id: Option<String>,
    /// Optional provider event type.
    pub provider_event_type: Option<String>,
}

/// Normalized webhook outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedWebhook {
    /// Canonical event (present only when verification passed).
    pub event: Option<WebhookEvent>,
    /// Verification result.
    pub verification: WebhookVerification,
}

/// Error produced by the webhook normalizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookNormalizerError(pub SdkError);

impl std::fmt::Display for WebhookNormalizerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "webhook normalizer: {}", self.0)
    }
}

impl std::error::Error for WebhookNormalizerError {}

/// Port for normalizing and verifying webhook deliveries.
pub trait WebhookNormalizer: Send + Sync {
    /// Normalize a raw delivery into a canonical event, verifying the
    /// signature and replay state.
    fn normalize(
        &self,
        raw: RawWebhook,
        capability_id: String,
        context: InvocationContext,
    ) -> Result<NormalizedWebhook, WebhookNormalizerError>;
}

/// Deterministic normalizer that accepts a configured signature.
///
/// Test/verification zone: used by unit tests and the M5 conformance
/// corpus to prove the normalize contract; production webhook
/// providers implement their own `WebhookNormalizer` with real
/// signature verification.
#[derive(Debug, Clone)]
pub struct AcceptingWebhookNormalizer {
    /// Fingerprint the normalizer accepts.
    pub expected_fingerprint: String,
}

impl WebhookNormalizer for AcceptingWebhookNormalizer {
    fn normalize(
        &self,
        raw: RawWebhook,
        _capability_id: String,
        context: InvocationContext,
    ) -> Result<NormalizedWebhook, WebhookNormalizerError> {
        let signature = raw.signature.unwrap_or_default();
        let verification = if signature.contains(&self.expected_fingerprint) {
            WebhookVerification::Valid
        } else {
            WebhookVerification::Invalid
        };
        let event = if verification == WebhookVerification::Valid {
            Some(WebhookEvent {
                event_id: raw
                    .provider_event_id
                    .unwrap_or_else(|| format!("wh-{}", context.request_id)),
                event_type: raw
                    .provider_event_type
                    .unwrap_or_else(|| "webhook.received".to_string()),
                version: "1".to_string(),
                correlation_id: context.correlation_id.to_string(),
                payload: raw.raw_payload,
            })
        } else {
            None
        };
        Ok(NormalizedWebhook {
            event,
            verification,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::WebhookSignature;
    use nexus_domain::{CorrelationId, NexusId, PrincipalType, TenantId};

    fn ctx() -> InvocationContext {
        InvocationContext::new(
            NexusId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
            "test",
            "user:alice",
            PrincipalType::Human,
            TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap(),
            Some("mcp".to_string()),
            None,
            None,
            None,
        )
        .unwrap()
    }

    #[test]
    fn ep011_unit_webhook_normalizer_valid_signature() {
        let normalizer = AcceptingWebhookNormalizer {
            expected_fingerprint: "fp-test".to_string(),
        };
        let result = normalizer
            .normalize(
                RawWebhook {
                    raw_payload: serde_json::json!({ "ok": true }),
                    signature: Some("sha256=fp-test:abc".to_string()),
                    provider_event_id: Some("prov-1".to_string()),
                    provider_event_type: Some("invoice.paid".to_string()),
                },
                "cap.webhook".to_string(),
                ctx(),
            )
            .unwrap();
        assert_eq!(result.verification, WebhookVerification::Valid);
        let event = result.event.unwrap();
        assert_eq!(event.event_id, "prov-1");
        assert_eq!(event.event_type, "invoice.paid");
    }

    #[test]
    fn ep011_unit_webhook_normalizer_rejects_bad_signature() {
        let normalizer = AcceptingWebhookNormalizer {
            expected_fingerprint: "fp-test".to_string(),
        };
        let result = normalizer
            .normalize(
                RawWebhook {
                    raw_payload: serde_json::json!({ "ok": true }),
                    signature: Some("sha256=wrong:xyz".to_string()),
                    provider_event_id: None,
                    provider_event_type: None,
                },
                "cap.webhook".to_string(),
                ctx(),
            )
            .unwrap();
        assert_eq!(result.verification, WebhookVerification::Invalid);
        assert!(result.event.is_none());
    }

    #[test]
    fn ep011_unit_webhook_signature_never_contains_secret() {
        let sig = WebhookSignature {
            scheme: "hmac-sha256".to_string(),
            key_fingerprint: "fp-abc123".to_string(),
            value_hex: "deadbeef".to_string(),
        };
        let json = serde_json::to_value(&sig).unwrap();
        assert_eq!(json["key_fingerprint"], "fp-abc123");
        assert!(json["value_hex"].as_str().unwrap().len() <= 128);
        assert!(json.as_object().unwrap().get("key").is_none());
    }
}
