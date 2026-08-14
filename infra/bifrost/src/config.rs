//! Bifrost adapter configuration (SPEC-009; EP-013 M2).
//!
//! Configuration is provider-neutral. Credentials are REFERENCED by
//! id, never stored here; the adapter never serializes a credential
//! value into requests, logs, or telemetry.

use serde::{Deserialize, Serialize};

/// Deterministic retry policy (SPEC-009 required behavior 7:
/// deterministic escalation and safe failover).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum total attempts (1 means no retry).
    pub max_attempts: u32,
    /// Base delay between attempts in milliseconds.
    pub base_delay_ms: u64,
    /// Multiplicative backoff factor applied per retry.
    pub backoff_factor: f64,
}

impl RetryPolicy {
    pub fn new(max_attempts: u32, base_delay_ms: u64, backoff_factor: f64) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            base_delay_ms,
            backoff_factor,
        }
    }

    /// Deterministic delay for attempt `n` (1-based). Attempt 1 is
    /// the first attempt and has zero delay; attempt 2 uses the base
    /// delay; each later attempt multiplies by the backoff factor.
    pub fn delay_for_attempt(&self, attempt: u32) -> u64 {
        if attempt <= 1 {
            return 0;
        }
        let retries = (attempt - 1) as u64;
        let mut delay = self.base_delay_ms;
        let mut i = 0u64;
        while i + 1 < retries {
            delay = (delay as f64 * self.backoff_factor).round() as u64;
            i += 1;
        }
        delay
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self::new(3, 100, 2.0)
    }
}

/// Deterministic rate limit policy per provider (SPEC-009 budgets,
/// retries, rate limits, fallbacks are consistent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateLimitPolicy {
    /// Maximum requests allowed in the window.
    pub max_requests: u32,
    /// Window length in seconds.
    pub window_seconds: u64,
}

impl RateLimitPolicy {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
        }
    }
}

impl Default for RateLimitPolicy {
    fn default() -> Self {
        Self::new(120, 60)
    }
}

/// Bifrost adapter configuration.
///
/// `preferred_provider` is the id of the Bifrost gateway provider
/// (registered in the registry under the `ModelProvider` port);
/// `fallback_order` lists direct provider ids tried in order when the
/// preferred provider is unhealthy or fails. Credentials are
/// referenced by `credential_ref`; the gateway resolves them through
/// the provider adapter and never exposes them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BifrostConfig {
    pub gateway_id: String,
    pub preferred_provider: String,
    pub fallback_order: Vec<String>,
    pub retry: RetryPolicy,
    pub rate_limit: RateLimitPolicy,
    /// Credential references by provider id (never values).
    pub credential_refs: Vec<CredentialRef>,
}

/// A credential reference: provider id plus the secret reference key.
/// The value lives in the secret store (EP-009 SecretReference), not
/// in this adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRef {
    pub provider_id: String,
    pub secret_ref: String,
}

impl BifrostConfig {
    pub fn new(
        gateway_id: impl Into<String>,
        preferred_provider: impl Into<String>,
        fallback_order: Vec<String>,
    ) -> Self {
        Self {
            gateway_id: gateway_id.into(),
            preferred_provider: preferred_provider.into(),
            fallback_order,
            retry: RetryPolicy::default(),
            rate_limit: RateLimitPolicy::default(),
            credential_refs: Vec::new(),
        }
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub fn with_rate_limit(mut self, rate_limit: RateLimitPolicy) -> Self {
        self.rate_limit = rate_limit;
        self
    }

    pub fn with_credential_ref(
        mut self,
        provider_id: impl Into<String>,
        secret_ref: impl Into<String>,
    ) -> Self {
        self.credential_refs.push(CredentialRef {
            provider_id: provider_id.into(),
            secret_ref: secret_ref.into(),
        });
        self
    }

    /// Resolve the credential reference for a provider id. Returns
    /// the ref (never the value).
    pub fn credential_ref_for(&self, provider_id: &str) -> Option<&CredentialRef> {
        self.credential_refs
            .iter()
            .find(|c| c.provider_id == provider_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep013_unit_retry_policy_deterministic_delays() {
        let p = RetryPolicy::new(4, 100, 2.0);
        assert_eq!(p.delay_for_attempt(1), 0);
        assert_eq!(p.delay_for_attempt(2), 100);
        assert_eq!(p.delay_for_attempt(3), 200);
        assert_eq!(p.delay_for_attempt(4), 400);
    }

    #[test]
    fn ep013_unit_retry_policy_never_zero_attempts() {
        let p = RetryPolicy::new(0, 100, 2.0);
        assert_eq!(p.max_attempts, 1);
    }

    #[test]
    fn ep013_unit_rate_limit_default_window() {
        let p = RateLimitPolicy::default();
        assert_eq!(p.max_requests, 120);
        assert_eq!(p.window_seconds, 60);
    }

    #[test]
    fn ep013_unit_config_credential_ref_never_value() {
        let cfg = BifrostConfig::new("gw-1", "bifrost", vec!["deepseek".to_string()])
            .with_credential_ref("bifrost", "secret/bifrost/key");
        let c = cfg.credential_ref_for("bifrost").unwrap();
        assert_eq!(c.secret_ref, "secret/bifrost/key");
        assert!(cfg.credential_ref_for("missing").is_none());
        // The serialized config carries the ref, never a value.
        let v = serde_json::to_value(&cfg).unwrap();
        assert_eq!(v["credential_refs"][0]["secret_ref"], "secret/bifrost/key");
    }

    #[test]
    fn ep013_unit_config_round_trip() {
        let cfg = BifrostConfig::new("gw-2", "bifrost", vec!["deepseek".to_string()])
            .with_retry(RetryPolicy::new(5, 50, 1.5));
        let v = serde_json::to_value(&cfg).unwrap();
        let back: BifrostConfig = serde_json::from_value(v).unwrap();
        assert_eq!(back.gateway_id, "gw-2");
        assert_eq!(back.retry.max_attempts, 5);
    }
}
