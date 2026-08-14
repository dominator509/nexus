//! EP-012 fabric typed errors (SPEC-006 codes).

use std::fmt;

/// Canonical SPEC-006 error codes for the fabric surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FabricErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// The requested resource or endpoint does not exist.
    NotFound,
    /// Authentication/authorization rejected the request.
    Authorization,
    /// The provider or transport is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// A state conflict (idempotency, version, lifecycle).
    Conflict,
    /// The remote/provider returned a malformed or invalid payload.
    MalformedProviderResponse,
    /// Internal invariant failure.
    Internal,
}

impl FabricErrorCode {
    /// Canonical wire value.
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

/// Typed fabric error preserving request context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricError {
    pub code: FabricErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl FabricError {
    pub fn new(
        code: FabricErrorCode,
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

    pub fn validation(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            FabricErrorCode::Validation,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn not_found(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            FabricErrorCode::NotFound,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    /// Redacted message policy: secrets and private content never enter
    /// error messages (SPEC-003 security).
    pub fn redacted_message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for FabricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for FabricError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_error_wire_codes_are_canonical() {
        for (code, wire) in [
            (FabricErrorCode::Validation, "VALIDATION"),
            (FabricErrorCode::NotFound, "NOT_FOUND"),
            (FabricErrorCode::Authorization, "AUTHORIZATION"),
            (FabricErrorCode::Unavailable, "UNAVAILABLE"),
            (FabricErrorCode::Timeout, "TIMEOUT"),
            (FabricErrorCode::Conflict, "CONFLICT"),
            (
                FabricErrorCode::MalformedProviderResponse,
                "MALFORMED_PROVIDER_RESPONSE",
            ),
            (FabricErrorCode::Internal, "INTERNAL"),
        ] {
            assert_eq!(code.as_str(), wire);
        }
    }

    #[test]
    fn ep012_unit_error_preserves_context() {
        let err = FabricError::new(
            FabricErrorCode::Authorization,
            "denied",
            Some("corr-1".into()),
            Some("user:alice".into()),
            Some("tenant-1".into()),
            Some("res-1".into()),
        );
        assert_eq!(err.code, FabricErrorCode::Authorization);
        assert_eq!(err.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(err.actor.as_deref(), Some("user:alice"));
        assert_eq!(err.tenant_id.as_deref(), Some("tenant-1"));
        assert_eq!(err.resource.as_deref(), Some("res-1"));
        assert!(err.to_string().contains("AUTHORIZATION"));
    }
}
