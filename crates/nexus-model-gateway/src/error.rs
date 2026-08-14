//! EP-013 model gateway errors (SPEC-006 typed errors).

use serde::{Deserialize, Serialize};

/// Canonical model gateway error codes (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelGatewayErrorCode {
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

impl ModelGatewayErrorCode {
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

/// Typed model gateway error.
///
/// The message is REDACTED by default: it never contains prompt text,
/// model output, provider credentials, or private content. Adapters
/// construct errors with typed codes and resource references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelGatewayError {
    pub code: ModelGatewayErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl ModelGatewayError {
    pub fn new(
        code: ModelGatewayErrorCode,
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
            ModelGatewayErrorCode::Validation,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn not_found(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ModelGatewayErrorCode::NotFound,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn authorization(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ModelGatewayErrorCode::Authorization,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn unavailable(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ModelGatewayErrorCode::Unavailable,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn conflict(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ModelGatewayErrorCode::Conflict,
            message,
            None,
            None,
            None,
            resource,
        )
    }

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

impl std::fmt::Display for ModelGatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ModelGatewayError {}
