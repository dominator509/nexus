//! A2A typed errors (SPEC-006 codes).

use std::fmt;

/// Canonical SPEC-006 error codes for the A2A gateway.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A2AErrorCode {
    Validation,
    NotFound,
    Authorization,
    Unavailable,
    Timeout,
    Conflict,
    MalformedProviderResponse,
    Internal,
}

impl A2AErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::NotFound => "NOT_FOUND",
            Self::Authorization => "AUTHORIZATION",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::Conflict => "CONFLICT",
            Self::MalformedProviderResponse => "MALFORMED_PROVIDER_RESPONSE",
            Self::Internal => "INTERNAL",
        }
    }
}

/// Typed A2A error preserving request context (redacted messages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct A2AError {
    pub code: A2AErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl A2AError {
    pub fn new(
        code: A2AErrorCode,
        message: impl Into<String>,
        correlation_id: Option<String>,
        actor: Option<String>,
        tenant_id: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id: correlation_id.map(String::into_boxed_str),
            actor: actor.map(String::into_boxed_str),
            tenant_id: tenant_id.map(String::into_boxed_str),
            resource: resource.map(String::into_boxed_str),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(A2AErrorCode::Validation, message, None, None, None, None)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(A2AErrorCode::NotFound, message, None, None, None, None)
    }

    pub fn authorization(message: impl Into<String>) -> Self {
        Self::new(A2AErrorCode::Authorization, message, None, None, None, None)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(A2AErrorCode::Conflict, message, None, None, None, None)
    }

    pub fn with_context(
        mut self,
        correlation_id: impl Into<String>,
        actor: impl Into<String>,
        tenant_id: impl Into<String>,
        resource: impl Into<String>,
    ) -> Self {
        self.correlation_id = Some(correlation_id.into().into_boxed_str());
        self.actor = Some(actor.into().into_boxed_str());
        self.tenant_id = Some(tenant_id.into().into_boxed_str());
        self.resource = Some(resource.into().into_boxed_str());
        self
    }
}

impl fmt::Display for A2AError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for A2AError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_a2a_error_wire_codes_are_canonical() {
        assert_eq!(A2AErrorCode::Validation.as_str(), "VALIDATION");
        assert_eq!(A2AErrorCode::NotFound.as_str(), "NOT_FOUND");
        assert_eq!(A2AErrorCode::Authorization.as_str(), "AUTHORIZATION");
        assert_eq!(A2AErrorCode::Conflict.as_str(), "CONFLICT");
        assert_eq!(A2AErrorCode::Timeout.as_str(), "TIMEOUT");
    }

    #[test]
    fn ep012_unit_a2a_error_preserves_context() {
        let err = A2AError::validation("bad task").with_context(
            "corr-1",
            "agent:alice",
            "tenant-1",
            "tasks",
        );
        assert_eq!(err.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(err.actor.as_deref(), Some("agent:alice"));
        assert_eq!(err.tenant_id.as_deref(), Some("tenant-1"));
    }
}
