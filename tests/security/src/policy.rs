//! Security policy behavior: authorization decisions fail closed,
//! insecure configurations are rejected, and a denied permission is
//! never silently granted. POLICY DENIED != POLICY ACCEPTED.

use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};

/// Authorization decision for a capability request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthDecision {
    /// Principal id requesting the capability.
    pub principal: String,
    /// Capability id being requested.
    pub capability: String,
    /// Whether the decision was actually evaluated.
    pub evaluated: bool,
    /// Whether the request was granted.
    pub granted: bool,
}

impl AuthDecision {
    pub fn new(principal: impl Into<String>, capability: impl Into<String>) -> Self {
        Self {
            principal: principal.into(),
            capability: capability.into(),
            evaluated: false,
            granted: false,
        }
    }
}

/// Insecure configuration marker. Any of these present means the config
/// must be rejected (fail closed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InsecureConfig {
    /// TLS verification disabled.
    InsecureTls,
    /// Authentication disabled.
    Unauthenticated,
    /// Authorization bypass enabled.
    AuthorizationBypass,
    /// Secrets embedded in configuration.
    SecretInConfig,
}

/// Real security policy: deny by default. Only an explicit allow rule
/// grants a capability; anything unknown or denied fails closed.
#[derive(Debug, Clone, Default)]
pub struct SecurityPolicy {
    /// Explicit allow rules: (principal, capability).
    allow: Vec<(String, String)>,
}

impl SecurityPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow(mut self, principal: impl Into<String>, capability: impl Into<String>) -> Self {
        self.allow.push((principal.into(), capability.into()));
        self
    }

    /// Evaluate a capability request. The decision is recorded as
    /// evaluated and granted only when an explicit allow rule matches.
    pub fn authorize(&self, principal: &str, capability: &str) -> AuthDecision {
        let granted = self
            .allow
            .iter()
            .any(|(p, c)| p == principal && c == capability);
        AuthDecision {
            principal: principal.into(),
            capability: capability.into(),
            evaluated: true,
            granted,
        }
    }

    /// Fail closed on denial: a denied permission is a typed
    /// authorization failure, never a silent success.
    pub fn require(&self, principal: &str, capability: &str) -> TestingResult<()> {
        let decision = self.authorize(principal, capability);
        if !decision.evaluated {
            return Err(TestingError::internal(
                "authorization decision was not evaluated",
            ));
        }
        if !decision.granted {
            return Err(TestingError::new(
                TestingErrorCode::Authorization,
                format!("principal {principal} denied capability {capability}"),
            ));
        }
        Ok(())
    }

    /// Reject insecure configuration. Any insecure marker fails closed.
    pub fn reject_insecure(&self, configs: &[InsecureConfig]) -> TestingResult<()> {
        if let Some(bad) = configs.first() {
            return Err(TestingError::policy(format!(
                "insecure configuration rejected: {bad:?}"
            )));
        }
        Ok(())
    }
}
