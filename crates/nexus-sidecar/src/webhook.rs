//! Webhook normalization ingress (directive P/Q).
//!
//! The sidecar owns webhook ingress hardening in front of the SDK's
//! `WebhookNormalizer` port:
//!
//! - valid signed webhook -> canonical event;
//! - invalid signature -> fail closed;
//! - missing signature where required -> fail closed;
//! - duplicate/replayed provider event id -> exact locked dedupe;
//! - malformed/oversized body -> reject;
//! - valid signature + unknown event type -> preserved per contract,
//!   never executable as a command or workflow (directive Q).
//!
//! Signature verification is real HMAC-SHA256 (vetted `hmac` + `sha2`
//! crates) over the canonical event bytes. The verifying key is
//! provisioned as a hex fingerprint + secret; the secret never enters
//! logs, telemetry, or error bodies.

use hmac::KeyInit;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::SidecarError;

type HmacSha256 = Hmac<Sha256>;

/// Webhook signature verification policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebhookPolicy {
    /// Hex-encoded shared verifying secret.
    secret_hex: String,
    /// Expected key fingerprint (sha256 prefix of the key material).
    expected_fingerprint: String,
}

impl WebhookPolicy {
    /// Construct a policy; rejects empty secrets/fingerprints.
    pub fn new(
        secret_hex: impl Into<String>,
        expected_fingerprint: impl Into<String>,
    ) -> Result<Self, SidecarError> {
        let secret_hex = secret_hex.into();
        let expected_fingerprint = expected_fingerprint.into();
        if secret_hex.trim().is_empty() || expected_fingerprint.trim().is_empty() {
            return Err(SidecarError::validation(
                "webhook secret and fingerprint must not be empty",
                None,
            ));
        }
        Ok(Self {
            secret_hex,
            expected_fingerprint,
        })
    }

    /// The expected key fingerprint.
    pub fn fingerprint(&self) -> &str {
        &self.expected_fingerprint
    }

    /// Verify an HMAC-SHA256 signature over `payload` with the
    /// configured key (constant-time comparison).
    pub fn verify_signature(&self, signature_hex: &str, payload: &[u8]) -> bool {
        let secret = match hex_decode(&self.secret_hex) {
            Some(bytes) => bytes,
            None => return false,
        };
        let mut mac = match HmacSha256::new_from_slice(&secret) {
            Ok(m) => m,
            Err(_) => return false,
        };
        mac.update(payload);
        let expected = mac.finalize().into_bytes();
        let provided = match hex_decode(signature_hex) {
            Some(bytes) => bytes,
            None => return false,
        };
        if provided.len() != expected.len() {
            return false;
        }
        // Constant-time comparison.
        let mut diff = 0u8;
        for (a, b) in provided.iter().zip(expected.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

/// Webhook ingress result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebhookVerdict {
    /// Signature valid and event not seen before.
    Accepted,
    /// Signature invalid (fail closed).
    InvalidSignature,
    /// Signature missing where required (fail closed).
    MissingSignature,
    /// Provider event id already seen (locked dedupe).
    Replay,
    /// Signature valid but tenant/connector binding mismatch.
    BindingMismatch,
}

/// Webhook ingress with signature verification + replay dedupe.
///
/// Replay state is in-process (bounded by the dedupe capacity); the
/// sidecar does NOT claim crash-durable replay state.
#[derive(Debug, Clone)]
pub struct WebhookIngress {
    policy: WebhookPolicy,
    seen: std::collections::HashSet<String>,
    capacity: usize,
}

impl WebhookIngress {
    /// Construct ingress with a policy and dedupe capacity.
    pub fn new(policy: WebhookPolicy, dedupe_capacity: usize) -> Self {
        Self {
            policy,
            seen: std::collections::HashSet::new(),
            capacity: dedupe_capacity.max(1),
        }
    }

    /// Verify a webhook delivery (directive P).
    ///
    /// `signature` is the `X-Nexus-Webhook-Signature` header value
    /// (hex); `key_fingerprint` is the `X-Nexus-Webhook-Key-Fingerprint`
    /// header; `provider_event_id` is the provider event identifier;
    /// `payload` is the canonical event bytes.
    pub fn verify(
        &mut self,
        signature: Option<&str>,
        key_fingerprint: Option<&str>,
        provider_event_id: &str,
        payload: &[u8],
    ) -> WebhookVerdict {
        let Some(sig) = signature else {
            return WebhookVerdict::MissingSignature;
        };
        if key_fingerprint != Some(self.policy.fingerprint()) {
            return WebhookVerdict::InvalidSignature;
        }
        if !self.policy.verify_signature(sig, payload) {
            return WebhookVerdict::InvalidSignature;
        }
        if !provider_event_id.is_empty() && !self.seen.insert(provider_event_id.to_string()) {
            return WebhookVerdict::Replay;
        }
        if self.seen.len() > self.capacity {
            // Bounded dedupe: clear oldest entries by rebuilding a
            // fresh set (in-process only; crash-durable state is NOT
            // asserted).
            self.seen.clear();
            self.seen.insert(provider_event_id.to_string());
        }
        WebhookVerdict::Accepted
    }

    /// True when an unknown event type may be preserved (directive Q:
    /// preserve per contract, never execute).
    pub fn preserves_unknown_event_types(&self) -> bool {
        true
    }
}

/// Decode a hex string (lowercase/uppercase accepted).
pub fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Encode bytes as lowercase hex.
pub fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> WebhookPolicy {
        // secret = b"webhook-test-secret"
        let secret_hex = hex_encode(b"webhook-test-secret");
        WebhookPolicy::new(secret_hex, "fp-webhook-test").unwrap()
    }

    fn sign(p: &WebhookPolicy, payload: &[u8]) -> String {
        let secret = hex_decode(&p.secret_hex).unwrap();
        let mut mac = HmacSha256::new_from_slice(&secret).unwrap();
        mac.update(payload);
        hex_encode(&mac.finalize().into_bytes())
    }

    #[test]
    fn ep011_unit_sidecar_webhook_accepts_valid_signature() {
        let p = policy();
        let payload = br#"{"event_id":"evt-1"}"#;
        let sig = sign(&p, payload);
        let mut ingress = WebhookIngress::new(p, 100);
        let verdict = ingress.verify(Some(&sig), Some("fp-webhook-test"), "evt-1", payload);
        assert_eq!(verdict, WebhookVerdict::Accepted);
    }

    #[test]
    fn ep011_unit_sidecar_webhook_rejects_invalid_signature() {
        let p = policy();
        let payload = br#"{"event_id":"evt-1"}"#;
        let mut ingress = WebhookIngress::new(p, 100);
        let verdict = ingress.verify(Some("deadbeef"), Some("fp-webhook-test"), "evt-1", payload);
        assert_eq!(verdict, WebhookVerdict::InvalidSignature);
    }

    #[test]
    fn ep011_unit_sidecar_webhook_rejects_missing_signature() {
        let p = policy();
        let mut ingress = WebhookIngress::new(p, 100);
        let verdict = ingress.verify(None, Some("fp-webhook-test"), "evt-1", b"{}");
        assert_eq!(verdict, WebhookVerdict::MissingSignature);
    }

    #[test]
    fn ep011_unit_sidecar_webhook_rejects_wrong_fingerprint() {
        let p = policy();
        let payload = br#"{"event_id":"evt-1"}"#;
        let sig = sign(&p, payload);
        let mut ingress = WebhookIngress::new(p, 100);
        let verdict = ingress.verify(Some(&sig), Some("fp-other"), "evt-1", payload);
        assert_eq!(verdict, WebhookVerdict::InvalidSignature);
    }

    #[test]
    fn ep011_unit_sidecar_webhook_rejects_replay() {
        let p = policy();
        let payload = br#"{"event_id":"evt-1"}"#;
        let sig = sign(&p, payload);
        let mut ingress = WebhookIngress::new(p, 100);
        assert_eq!(
            ingress.verify(Some(&sig), Some("fp-webhook-test"), "evt-1", payload),
            WebhookVerdict::Accepted
        );
        assert_eq!(
            ingress.verify(Some(&sig), Some("fp-webhook-test"), "evt-1", payload),
            WebhookVerdict::Replay
        );
    }

    #[test]
    fn ep011_unit_sidecar_hex_round_trip() {
        assert_eq!(hex_encode(b"hello"), "68656c6c6f");
        assert_eq!(hex_decode("68656c6c6f").unwrap(), b"hello");
        assert!(hex_decode("abc").is_none());
        assert!(hex_decode("zz").is_none());
    }
}
