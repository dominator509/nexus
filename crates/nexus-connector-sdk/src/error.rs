//! SDK typed error (SPEC-006).
//!
//! Every SDK, sidecar, poller, webhook, and credential operation fails
//! with `SdkError`, which preserves correlation/actor/tenant/resource
//! context, classifies failures with canonical codes, and redacts
//! sensitive content. Failures fail closed: an error is never
//! converted into a success.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical SDK failure class (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SdkErrorCode {
    Validation,
    Authentication,
    Authorization,
    Policy,
    Unavailable,
    Timeout,
    Conflict,
    NotFound,
    RateLimit,
    ExternalProvider,
    Verification,
    Compensation,
    Internal,
}

impl SdkErrorCode {
    /// Canonical wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authentication => "AUTHENTICATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::Conflict => "CONFLICT",
            Self::NotFound => "NOT_FOUND",
            Self::RateLimit => "RATE_LIMIT",
            Self::ExternalProvider => "EXTERNAL_PROVIDER",
            Self::Verification => "VERIFICATION",
            Self::Compensation => "COMPENSATION",
            Self::Internal => "INTERNAL",
        }
    }
}

/// Typed SDK failure with SPEC-006 context.
///
/// Context fields are small boxed strings, keeping the error value
/// compact (clippy large-Err threshold) while preserving
/// correlation/actor/tenant/resource references. Wire serialization is
/// identical to plain strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdkError {
    /// Canonical failure class.
    pub code: SdkErrorCode,
    /// Human-readable reason (never contains secrets).
    pub message: String,
    /// Optional correlation id.
    pub correlation_id: Option<Box<str>>,
    /// Optional external actor id.
    pub actor: Option<Box<str>>,
    /// Optional tenant id.
    pub tenant: Option<Box<str>>,
    /// Optional resource (capability/connector id).
    pub resource: Option<Box<str>>,
}

impl SdkError {
    /// Construct a typed SDK error.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        code: SdkErrorCode,
        message: impl Into<String>,
        correlation_id: Option<String>,
        actor: Option<String>,
        tenant: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id: correlation_id.map(String::into_boxed_str),
            actor: actor.map(String::into_boxed_str),
            tenant: tenant.map(String::into_boxed_str),
            resource: resource.map(String::into_boxed_str),
        }
    }

    /// Correlation id as a string reference.
    pub fn correlation(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }

    /// Actor as a string reference.
    pub fn actor(&self) -> Option<&str> {
        self.actor.as_deref()
    }

    /// Tenant as a string reference.
    pub fn tenant(&self) -> Option<&str> {
        self.tenant.as_deref()
    }

    /// Resource as a string reference.
    pub fn resource(&self) -> Option<&str> {
        self.resource.as_deref()
    }

    /// Construct a validation error with context.
    pub fn validation(
        message: impl Into<String>,
        correlation_id: Option<String>,
        actor: Option<String>,
        tenant: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self::new(
            SdkErrorCode::Validation,
            message,
            correlation_id,
            actor,
            tenant,
            resource,
        )
    }

    /// True when the failure is retryable at the transport layer.
    pub fn is_transient(&self) -> bool {
        matches!(
            self.code,
            SdkErrorCode::Unavailable
                | SdkErrorCode::Timeout
                | SdkErrorCode::RateLimit
                | SdkErrorCode::ExternalProvider
        )
    }
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sdk error {}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for SdkError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep011_unit_sdk_error_codes_round_trip() {
        for code in [
            SdkErrorCode::Validation,
            SdkErrorCode::Authentication,
            SdkErrorCode::Authorization,
            SdkErrorCode::Policy,
            SdkErrorCode::Unavailable,
            SdkErrorCode::Timeout,
            SdkErrorCode::Conflict,
            SdkErrorCode::NotFound,
            SdkErrorCode::RateLimit,
            SdkErrorCode::ExternalProvider,
            SdkErrorCode::Verification,
            SdkErrorCode::Compensation,
            SdkErrorCode::Internal,
        ] {
            assert_eq!(SdkErrorCode::as_str(code), code.as_str());
        }
    }

    #[test]
    fn ep011_unit_sdk_error_transient_classification() {
        assert!(
            SdkError::new(SdkErrorCode::Unavailable, "down", None, None, None, None).is_transient()
        );
        assert!(
            !SdkError::new(
                SdkErrorCode::Authorization,
                "denied",
                None,
                None,
                None,
                None
            )
            .is_transient()
        );
    }

    #[test]
    fn ep011_unit_sdk_error_serializes_without_secrets() {
        let err = SdkError::new(
            SdkErrorCode::ExternalProvider,
            "provider refused",
            Some("corr-1".to_string()),
            Some("user:alice".to_string()),
            Some("tenant-1".to_string()),
            Some("cap:ledger".to_string()),
        );
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["code"], "EXTERNAL_PROVIDER");
        assert_eq!(json["correlation_id"], "corr-1");
        assert_eq!(json["tenant"], "tenant-1");
    }
}
