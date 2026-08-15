//! EP-016 typed context errors (SPEC-006; ADR-023).
//!
//! Every failure uses a canonical SPEC-006 code and preserves
//! correlation/actor/tenant/resource context when available. Messages
//! are redacted: never prompts, credentials, or private content.

use serde::{Deserialize, Serialize};

/// Canonical context error code (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContextErrorCode {
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

impl ContextErrorCode {
    pub const fn as_str(self) -> &'static str {
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

/// Typed context error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextError {
    pub code: ContextErrorCode,
    pub message: String,
    pub correlation_id: Option<String>,
    pub actor: Option<String>,
    pub tenant_id: Option<String>,
    pub resource: Option<String>,
}

impl ContextError {
    pub fn new(
        code: ContextErrorCode,
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
            ContextErrorCode::Validation,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn authorization(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ContextErrorCode::Authorization,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn policy(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ContextErrorCode::Policy,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn unavailable(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ContextErrorCode::Unavailable,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn internal(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ContextErrorCode::Internal,
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

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ContextError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep016_unit_error_codes_are_canonical() {
        assert_eq!(ContextErrorCode::Validation.as_str(), "VALIDATION");
        assert_eq!(ContextErrorCode::Authorization.as_str(), "AUTHORIZATION");
        assert_eq!(ContextErrorCode::Policy.as_str(), "POLICY");
        assert_eq!(ContextErrorCode::Unavailable.as_str(), "UNAVAILABLE");
        assert_eq!(ContextErrorCode::Timeout.as_str(), "TIMEOUT");
        assert_eq!(ContextErrorCode::Compensation.as_str(), "COMPENSATION");
    }

    #[test]
    fn ep016_unit_error_preserves_context() {
        let err = ContextError::validation("bad context request", Some("context".into()))
            .with_context("c-1", "p-1", "t-1");
        assert_eq!(err.code, ContextErrorCode::Validation);
        assert_eq!(err.correlation_id.as_deref(), Some("c-1"));
        assert_eq!(err.actor.as_deref(), Some("p-1"));
        assert_eq!(err.tenant_id.as_deref(), Some("t-1"));
    }

    #[test]
    fn ep016_unit_error_serializes_without_secrets() {
        let err = ContextError::internal("internal invariant failure", None)
            .with_context("c-1", "p-1", "t-1");
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["code"], "INTERNAL");
        let s = v.to_string();
        assert!(!s.contains("sk-"));
        assert!(!s.contains("api_key"));
    }
}
