//! Transport typed errors (SPEC-006; EP-013 M3).
//!
//! Real HTTP failures are classified into typed codes. Messages are
//! REDACTED: no credential, prompt text, model output, or private
//! content.

use nexus_model_gateway::{ModelGatewayError, ModelGatewayErrorCode};
use serde::{Deserialize, Serialize};

/// Transport error (fail closed, typed, redacted).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportError {
    pub code: ModelGatewayErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
    pub provider_id: Option<Box<str>>,
}

impl TransportError {
    pub fn new(
        code: ModelGatewayErrorCode,
        message: impl Into<String>,
        correlation_id: Option<String>,
        actor: Option<String>,
        tenant_id: Option<String>,
        resource: Option<String>,
        provider_id: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id: correlation_id.map(Into::into),
            actor: actor.map(Into::into),
            tenant_id: tenant_id.map(Into::into),
            resource: resource.map(Into::into),
            provider_id: provider_id.map(Into::into),
        }
    }

    pub fn unavailable(
        message: impl Into<String>,
        resource: Option<String>,
        provider_id: Option<String>,
    ) -> Self {
        Self::new(
            ModelGatewayErrorCode::Unavailable,
            message,
            None,
            None,
            None,
            resource,
            provider_id,
        )
    }

    pub fn timeout(
        message: impl Into<String>,
        resource: Option<String>,
        provider_id: Option<String>,
    ) -> Self {
        Self::new(
            ModelGatewayErrorCode::Timeout,
            message,
            None,
            None,
            None,
            resource,
            provider_id,
        )
    }

    pub fn external(
        message: impl Into<String>,
        resource: Option<String>,
        provider_id: Option<String>,
    ) -> Self {
        Self::new(
            ModelGatewayErrorCode::ExternalProvider,
            message,
            None,
            None,
            None,
            resource,
            provider_id,
        )
    }

    pub fn validation(
        message: impl Into<String>,
        resource: Option<String>,
        provider_id: Option<String>,
    ) -> Self {
        Self::new(
            ModelGatewayErrorCode::Validation,
            message,
            None,
            None,
            None,
            resource,
            provider_id,
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

impl From<TransportError> for ModelGatewayError {
    fn from(e: TransportError) -> Self {
        ModelGatewayError::new(
            e.code,
            e.message,
            e.correlation_id.map(|s| s.to_string()),
            e.actor.map(|s| s.to_string()),
            e.tenant_id.map(|s| s.to_string()),
            e.resource.map(|s| s.to_string()),
        )
    }
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for TransportError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep013_unit_transport_error_typed() {
        let e = TransportError::timeout("provider timed out", None, Some("bifrost".into()));
        assert_eq!(e.code, ModelGatewayErrorCode::Timeout);
        assert_eq!(e.provider_id.as_deref(), Some("bifrost"));
        assert_eq!(e.code.as_str(), "TIMEOUT");
    }

    #[test]
    fn ep013_unit_transport_error_converts_to_gateway_error() {
        let e = TransportError::unavailable("down", None, None);
        let gw: ModelGatewayError = e.into();
        assert_eq!(gw.code, ModelGatewayErrorCode::Unavailable);
    }
}
