//! MCP typed errors (SPEC-006 codes).

use std::fmt;

/// Canonical SPEC-006 error codes for the MCP engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpErrorCode {
    Validation,
    NotFound,
    Authorization,
    Unavailable,
    Timeout,
    Conflict,
    MalformedProviderResponse,
    Internal,
}

impl McpErrorCode {
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

/// Typed MCP error preserving request context (redacted messages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpError {
    pub code: McpErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl McpError {
    pub fn new(
        code: McpErrorCode,
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
        Self::new(McpErrorCode::Validation, message, None, None, None, None)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(McpErrorCode::NotFound, message, None, None, None, None)
    }

    pub fn authorization(message: impl Into<String>) -> Self {
        Self::new(McpErrorCode::Authorization, message, None, None, None, None)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(McpErrorCode::Conflict, message, None, None, None, None)
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

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for McpError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_mcp_error_wire_codes_are_canonical() {
        assert_eq!(McpErrorCode::Validation.as_str(), "VALIDATION");
        assert_eq!(McpErrorCode::NotFound.as_str(), "NOT_FOUND");
        assert_eq!(McpErrorCode::Authorization.as_str(), "AUTHORIZATION");
        assert_eq!(McpErrorCode::Unavailable.as_str(), "UNAVAILABLE");
        assert_eq!(McpErrorCode::Timeout.as_str(), "TIMEOUT");
        assert_eq!(McpErrorCode::Conflict.as_str(), "CONFLICT");
    }

    #[test]
    fn ep012_unit_mcp_error_preserves_redacted_context() {
        let err = McpError::validation("bad request").with_context(
            "corr-1",
            "user:alice",
            "tenant-1",
            "tools/list",
        );
        assert_eq!(err.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(err.actor.as_deref(), Some("user:alice"));
        assert_eq!(err.tenant_id.as_deref(), Some("tenant-1"));
        assert_eq!(err.resource.as_deref(), Some("tools/list"));
        assert!(err.to_string().contains("VALIDATION"));
    }
}
