//! EP-035 setup wizard error surface (SPEC-006 codes; SPEC-004/SPEC-016
//! error states).
//!
//! Every failure distinguishes validation, authentication, authorization,
//! policy, unavailable, timeout, conflict, rate limit, external provider,
//! verification, compensation, and internal invariant failures, and
//! preserves correlation, actor, tenant, and resource references where
//! available. Messages never contain secrets, prompts, or private payloads
//! (least privilege, SPEC-005).

use std::fmt;

use nexus_domain::CorrelationId;
use serde::{Deserialize, Serialize};

/// Canonical setup error code (SPEC-006; SPEC-004/016 error states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SetupErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication failed or is missing.
    Authentication,
    /// The principal is not authorized for the capability.
    Authorization,
    /// A contextual policy denied the request (state transition,
    /// approval class).
    Policy,
    /// The provider, capability, or resource is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// A conflicting state prevented the operation (idempotency,
    /// lifecycle).
    Conflict,
    /// The referenced object does not exist.
    NotFound,
    /// The caller exceeded a declared rate limit.
    RateLimit,
    /// An external provider returned a failure.
    ExternalProvider,
    /// Verification of a side effect failed (exact-target mismatch).
    Verification,
    /// A compensating action was required and did not complete.
    Compensation,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// An internal invariant was violated.
    Internal,
}

impl SetupErrorCode {
    /// Stable HTTP status class for the code when rendered over HTTP.
    pub fn http_status(self) -> u16 {
        match self {
            Self::Validation => 400,
            Self::Authentication => 401,
            Self::Authorization | Self::Policy => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::RateLimit => 429,
            Self::Unavailable => 503,
            Self::Timeout => 504,
            Self::ExternalProvider => 502,
            Self::Verification => 409,
            Self::Compensation | Self::Internal => 500,
            Self::Vocabulary => 422,
        }
    }
}

/// Typed setup failure (SPEC-006). Messages are safe for display and
/// never contain secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupError {
    pub code: SetupErrorCode,
    pub message: String,
    pub correlation: Option<CorrelationId>,
    pub actor: Option<Box<str>>,
    pub tenant: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl SetupError {
    pub fn new(
        code: SetupErrorCode,
        message: impl Into<String>,
        correlation: Option<CorrelationId>,
        actor: Option<Box<str>>,
        tenant: Option<Box<str>>,
        resource: Option<Box<str>>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation,
            actor,
            tenant,
            resource,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(SetupErrorCode::Validation, message, None, None, None, None)
    }

    pub fn vocabulary(message: impl Into<String>) -> Self {
        Self::new(SetupErrorCode::Vocabulary, message, None, None, None, None)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(SetupErrorCode::Policy, message, None, None, None, None)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(
            SetupErrorCode::Verification,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(SetupErrorCode::Conflict, message, None, None, None, None)
    }

    pub fn with_correlation(mut self, correlation: CorrelationId) -> Self {
        self.correlation = Some(correlation);
        self
    }

    pub fn with_actor(mut self, actor: impl Into<Box<str>>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    pub fn with_resource(mut self, resource: impl Into<Box<str>>) -> Self {
        self.resource = Some(resource.into());
        self
    }
}

impl fmt::Display for SetupError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for SetupError {}

impl SetupErrorCode {
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
            Self::ExternalProvider => "EXTERNAL",
            Self::Verification => "VERIFICATION",
            Self::Compensation => "COMPENSATION",
            Self::Vocabulary => "VOCABULARY",
            Self::Internal => "INTERNAL",
        }
    }
}

/// Convenience result alias for setup operations.
pub type SetupResult<T> = Result<T, SetupError>;
