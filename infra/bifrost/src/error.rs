//! Bifrost adapter typed errors (SPEC-006; EP-013 M2).
//!
//! The adapter maps provider failures to typed SPEC-006 codes. The
//! message is REDACTED: it never contains prompt text, model output,
//! provider credentials, or private content.

use nexus_model_gateway::{ModelGatewayError, ModelGatewayErrorCode};
use serde::{Deserialize, Serialize};

/// Bifrost adapter error (fail closed, typed, redacted).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BifrostError {
    pub code: ModelGatewayErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
    /// The provider id that produced the failure (when known).
    pub provider_id: Option<Box<str>>,
}

impl BifrostError {
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

    pub fn validation(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            ModelGatewayErrorCode::Validation,
            message,
            None,
            None,
            None,
            resource,
            None,
        )
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

    pub fn rate_limited(
        message: impl Into<String>,
        resource: Option<String>,
        provider_id: Option<String>,
    ) -> Self {
        Self::new(
            ModelGatewayErrorCode::RateLimited,
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

impl From<BifrostError> for ModelGatewayError {
    fn from(e: BifrostError) -> Self {
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

impl From<ModelGatewayError> for BifrostError {
    fn from(e: ModelGatewayError) -> Self {
        Self::new(
            e.code,
            e.message,
            e.correlation_id.map(|s| s.to_string()),
            e.actor.map(|s| s.to_string()),
            e.tenant_id.map(|s| s.to_string()),
            e.resource.map(|s| s.to_string()),
            None,
        )
    }
}

impl std::fmt::Display for BifrostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for BifrostError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep013_unit_bifrost_error_typed_and_redacted() {
        let e = BifrostError::external(
            "provider returned an error",
            Some("model/bifrost".into()),
            Some("bifrost".into()),
        );
        assert_eq!(e.code, ModelGatewayErrorCode::ExternalProvider);
        assert_eq!(e.provider_id.as_deref(), Some("bifrost"));
        let v = serde_json::to_value(&e).unwrap();
        // The error enum serializes with its declared variant names;
        // the canonical string form is available through `as_str`.
        assert_eq!(v["code"], "ExternalProvider");
        assert_eq!(e.code.as_str(), "EXTERNAL_PROVIDER");
        assert_eq!(v["provider_id"], "bifrost");
    }

    #[test]
    fn ep013_unit_bifrost_error_converts_to_gateway_error() {
        let e = BifrostError::rate_limited("rate limit exceeded", None, Some("bifrost".into()));
        let gw: ModelGatewayError = e.into();
        assert_eq!(gw.code, ModelGatewayErrorCode::RateLimited);
    }

    #[test]
    fn ep013_unit_bifrost_error_from_gateway_error() {
        let gw = ModelGatewayError::validation("bad request", Some("model".into()));
        let b: BifrostError = gw.into();
        assert_eq!(b.code, ModelGatewayErrorCode::Validation);
    }
}
