//! EP-015 typed router errors (SPEC-006; ADR-022).
//!
//! Every failure uses a canonical SPEC-006 code and preserves
//! correlation/actor/tenant/resource context when available. Messages
//! are redacted: never prompts, credentials, or private content.

use serde::{Deserialize, Serialize};

/// Canonical router error code (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RouterErrorCode {
    Validation,
    Authentication,
    Authorization,
    Policy,
    Unavailable,
    Timeout,
    NotFound,
    Conflict,
    RateLimited,
    ExternalProvider,
    Verification,
    Compensation,
    Internal,
}

impl RouterErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authentication => "AUTHENTICATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::RateLimited => "RATE_LIMITED",
            Self::ExternalProvider => "EXTERNAL_PROVIDER",
            Self::Verification => "VERIFICATION",
            Self::Compensation => "COMPENSATION",
            Self::Internal => "INTERNAL",
        }
    }
}

/// Typed router error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouterError {
    pub code: RouterErrorCode,
    pub message: String,
    pub correlation_id: Option<String>,
    pub actor: Option<String>,
    pub tenant_id: Option<String>,
    pub resource: Option<String>,
}

impl RouterError {
    pub fn new(
        code: RouterErrorCode,
        message: impl Into<String>,
        correlation_id: Option<String>,
        actor: Option<String>,
        tenant_id: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id,
            actor,
            tenant_id,
            resource,
        }
    }

    pub fn validation(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            RouterErrorCode::Validation,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn policy(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(RouterErrorCode::Policy, message, None, None, None, resource)
    }

    pub fn unavailable(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            RouterErrorCode::Unavailable,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn internal(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            RouterErrorCode::Internal,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    /// Attach correlation/actor/tenant context.
    pub fn with_context(
        mut self,
        correlation_id: impl Into<String>,
        actor: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self.actor = Some(actor.into());
        self.tenant_id = Some(tenant_id.into());
        self
    }
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for RouterError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep015_unit_error_codes_are_canonical() {
        assert_eq!(RouterErrorCode::Validation.as_str(), "VALIDATION");
        assert_eq!(RouterErrorCode::Unavailable.as_str(), "UNAVAILABLE");
        assert_eq!(RouterErrorCode::Policy.as_str(), "POLICY");
        assert_eq!(
            RouterErrorCode::ExternalProvider.as_str(),
            "EXTERNAL_PROVIDER"
        );
        assert_eq!(RouterErrorCode::Compensation.as_str(), "COMPENSATION");
    }

    #[test]
    fn ep015_unit_error_preserves_context() {
        let err = RouterError::validation("bad features", Some("routing-features".into()))
            .with_context("c-1", "p-1", "t-1");
        assert_eq!(err.code, RouterErrorCode::Validation);
        assert_eq!(err.correlation_id.as_deref(), Some("c-1"));
        assert_eq!(err.actor.as_deref(), Some("p-1"));
        assert_eq!(err.tenant_id.as_deref(), Some("t-1"));
    }

    #[test]
    fn ep015_unit_error_serializes_without_secrets() {
        // The error type serializes exactly the redacted fields it is
        // given; it never fabricates or appends context that could carry
        // secrets. Credential values are never placed into error messages
        // (redaction is proven at the adapter boundary, EP-014 M4 pattern).
        let err = RouterError::internal("internal invariant failure", None)
            .with_context("c-1", "p-1", "t-1");
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "INTERNAL");
        assert_eq!(v["message"], "internal invariant failure");
        assert_eq!(v["correlation_id"], "c-1");
        let s = v.to_string();
        assert!(!s.contains("sk-"));
        assert!(!s.contains("api_key"));
    }
}
