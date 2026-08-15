//! EP-014 reflex errors (SPEC-006 typed errors).
//!
//! Canonical SPEC-006 code set, same shape as the model gateway error
//! but owned by the reflex plane so the provider boundary can carry
//! reflex-specific context without coupling to the gateway crate's
//! error type. Messages are REDACTED by default: never prompt text,
//! model output, provider credentials, or private content.

use serde::{Deserialize, Serialize};

/// Canonical reflex error codes (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReflexErrorCode {
    Validation,
    NotFound,
    Authorization,
    Unavailable,
    Timeout,
    Conflict,
    RateLimited,
    ExternalProvider,
    Verification,
    Internal,
}

impl ReflexErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::NotFound => "NOT_FOUND",
            Self::Authorization => "AUTHORIZATION",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::Conflict => "CONFLICT",
            Self::RateLimited => "RATE_LIMITED",
            Self::ExternalProvider => "EXTERNAL_PROVIDER",
            Self::Verification => "VERIFICATION",
            Self::Internal => "INTERNAL",
        }
    }
}

impl std::fmt::Display for ReflexErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Typed reflex error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflexError {
    pub code: ReflexErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl ReflexError {
    pub fn new(
        code: ReflexErrorCode,
        message: impl Into<String>,
        correlation_id: Option<String>,
        actor: Option<String>,
        tenant_id: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id: correlation_id.map(Into::into),
            actor: actor.map(Into::into),
            tenant_id: tenant_id.map(Into::into),
            resource: resource.map(Into::into),
        }
    }

    pub fn validation(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ReflexErrorCode::Validation,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn not_found(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ReflexErrorCode::NotFound,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn authorization(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ReflexErrorCode::Authorization,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn unavailable(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ReflexErrorCode::Unavailable,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn timeout(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ReflexErrorCode::Timeout,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn rate_limited(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ReflexErrorCode::RateLimited,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn external(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ReflexErrorCode::ExternalProvider,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn verification(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ReflexErrorCode::Verification,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn internal(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ReflexErrorCode::Internal,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    /// Attach request context (correlation, actor, tenant).
    pub fn with_context(
        mut self,
        correlation_id: Option<String>,
        actor: Option<String>,
        tenant_id: Option<String>,
    ) -> Self {
        self.correlation_id = correlation_id.map(Into::into);
        self.actor = actor.map(Into::into);
        self.tenant_id = tenant_id.map(Into::into);
        self
    }
}

impl std::fmt::Display for ReflexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ReflexError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep014_unit_error_codes_are_canonical() {
        assert_eq!(ReflexErrorCode::Validation.as_str(), "VALIDATION");
        assert_eq!(ReflexErrorCode::NotFound.as_str(), "NOT_FOUND");
        assert_eq!(ReflexErrorCode::Authorization.as_str(), "AUTHORIZATION");
        assert_eq!(ReflexErrorCode::Unavailable.as_str(), "UNAVAILABLE");
        assert_eq!(ReflexErrorCode::Timeout.as_str(), "TIMEOUT");
        assert_eq!(ReflexErrorCode::Conflict.as_str(), "CONFLICT");
        assert_eq!(ReflexErrorCode::RateLimited.as_str(), "RATE_LIMITED");
        assert_eq!(
            ReflexErrorCode::ExternalProvider.as_str(),
            "EXTERNAL_PROVIDER"
        );
        assert_eq!(ReflexErrorCode::Verification.as_str(), "VERIFICATION");
        assert_eq!(ReflexErrorCode::Internal.as_str(), "INTERNAL");
    }

    #[test]
    fn ep014_unit_error_preserves_context() {
        let err = ReflexError::validation("bad input", Some("reflex".into())).with_context(
            Some("corr-1".into()),
            Some("actor-1".into()),
            Some("t-1".into()),
        );
        assert_eq!(err.code, ReflexErrorCode::Validation);
        assert_eq!(err.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(err.actor.as_deref(), Some("actor-1"));
        assert_eq!(err.tenant_id.as_deref(), Some("t-1"));
        assert_eq!(err.resource.as_deref(), Some("reflex"));
    }

    #[test]
    fn ep014_unit_error_serializes_without_secrets() {
        let err = ReflexError::external("provider unreachable", None);
        let v = serde_json::to_value(&err).unwrap();
        // Message must not leak provider internals beyond the redacted string.
        assert_eq!(v["code"], "EXTERNAL_PROVIDER");
        assert_eq!(v["message"], "provider unreachable");
        assert!(v.get("correlation_id").unwrap().is_null());
    }
}
