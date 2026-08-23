//! EP-038 observability error surface (SPEC-006 codes; SPEC-007 error
//! states).
//!
//! Every failure distinguishes validation, authentication, authorization,
//! policy, unavailable, timeout, conflict, rate limit, external provider,
//! verification, compensation, internal invariant, and the
//! observability-specific redaction-denied, unsupported-signal,
//! stale-evidence, and insufficient-data failures. Messages never contain
//! secrets, tokens, prompts, or private payloads (SPEC-007 behavior 2).

use std::fmt;

use nexus_domain::{CorrelationId, TenantId};
use serde::{Deserialize, Serialize};

/// Canonical observability error code (SPEC-006; SPEC-007 error states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ObservabilityErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication failed or is missing.
    Authentication,
    /// The principal is not authorized for the capability.
    Authorization,
    /// A contextual policy denied the operation.
    Policy,
    /// Redaction policy denied egress of a value (fail-closed).
    RedactionDenied,
    /// The telemetry sink, backend, or capability is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// A conflicting state prevented the operation.
    Conflict,
    /// The referenced object does not exist.
    NotFound,
    /// The caller exceeded a declared rate limit.
    RateLimit,
    /// An external provider returned a failure.
    ExternalProvider,
    /// Verification of a side effect failed.
    Verification,
    /// A compensating action was required and did not complete.
    Compensation,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// The telemetry signal type is not supported by this surface.
    UnsupportedSignal,
    /// Health evidence is older than the freshness window.
    StaleEvidence,
    /// There is not enough data to evaluate (SLO with no events).
    InsufficientData,
    /// An internal invariant was violated.
    Internal,
}

impl ObservabilityErrorCode {
    /// Stable HTTP status class for the code when rendered over HTTP.
    pub fn http_status(self) -> u16 {
        match self {
            Self::Validation => 400,
            Self::Authentication => 401,
            Self::Authorization | Self::Policy | Self::RedactionDenied => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::RateLimit => 429,
            Self::Unavailable | Self::ExternalProvider => 503,
            Self::Timeout => 504,
            Self::Vocabulary
            | Self::UnsupportedSignal
            | Self::StaleEvidence
            | Self::InsufficientData
            | Self::Verification
            | Self::Compensation
            | Self::Internal => 500,
        }
    }
}

/// Canonical observability error carrying SPEC-006 context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservabilityError {
    pub code: ObservabilityErrorCode,
    pub message: String,
    pub correlation: Option<CorrelationId>,
    pub actor: Option<Box<str>>,
    pub tenant: Option<TenantId>,
    pub resource: Option<Box<str>>,
}

impl ObservabilityError {
    fn new(
        code: ObservabilityErrorCode,
        message: impl Into<String>,
        correlation: Option<CorrelationId>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Attach the acting principal (redacted display only).
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into().into_boxed_str());
        self
    }

    /// Attach the tenant boundary.
    pub fn with_tenant(mut self, tenant: TenantId) -> Self {
        self.tenant = Some(tenant);
        self
    }

    /// Attach the affected resource identifier.
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into().into_boxed_str());
        self
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::Validation, message, None)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::Policy, message, None)
    }

    pub fn redaction_denied(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::RedactionDenied, message, None)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::Conflict, message, None)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::NotFound, message, None)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::Unavailable, message, None)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::Timeout, message, None)
    }

    pub fn external_provider(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::ExternalProvider, message, None)
    }

    pub fn vocabulary(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::Vocabulary, message, None)
    }

    pub fn unsupported_signal(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::UnsupportedSignal, message, None)
    }

    pub fn stale_evidence(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::StaleEvidence, message, None)
    }

    pub fn insufficient_data(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::InsufficientData, message, None)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ObservabilityErrorCode::Internal, message, None)
    }

    pub fn with_correlation(mut self, correlation: CorrelationId) -> Self {
        self.correlation = Some(correlation);
        self
    }
}

impl fmt::Display for ObservabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "observability {:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ObservabilityError {}

/// Canonical result alias for observability operations.
///
/// `ObservabilityError` intentionally carries SPEC-006 context fields
/// (correlation, actor, tenant, resource) for tracing; telemetry error
/// paths are not hot loops, so the slightly oversized Err variant is
/// acceptable and boxed errors would obscure the public contract.
#[allow(clippy::result_large_err)]
pub type ObservabilityResult<T> = Result<T, ObservabilityError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep038_unit_error_codes_are_canonical() {
        let e = ObservabilityError::redaction_denied("sensitive egress blocked");
        assert_eq!(e.code, ObservabilityErrorCode::RedactionDenied);
        assert_eq!(e.code.http_status(), 403);
        let e2 = ObservabilityError::insufficient_data("zero denominator");
        assert_eq!(e2.code, ObservabilityErrorCode::InsufficientData);
        let e3 = ObservabilityError::stale_evidence("last seen 5m ago");
        assert_eq!(e3.code, ObservabilityErrorCode::StaleEvidence);
        let e4 = ObservabilityError::unsupported_signal("unknown signal");
        assert_eq!(e4.code, ObservabilityErrorCode::UnsupportedSignal);
    }

    #[test]
    fn ep038_unit_error_serializes_without_secrets() {
        let e = ObservabilityError::policy("denied")
            .with_correlation("01970000-0000-7000-8000-000000000011".parse().unwrap());
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("POLICY"));
        assert!(json.contains("01970000-0000-7000-8000-000000000011"));
        assert!(!json.contains("secret"));
    }
}
