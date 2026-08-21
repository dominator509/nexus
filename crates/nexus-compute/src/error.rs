//! EP-036 compute fabric error surface (SPEC-006 codes; SPEC-016 error
//! states).
//!
//! Every failure distinguishes validation, authentication, authorization,
//! policy, unavailable, timeout, conflict, rate limit, external provider,
//! verification, compensation, and internal invariant failures, and
//! preserves correlation, actor, tenant, and resource references where
//! available. Messages never contain secrets, tokens, or private payloads
//! (least privilege, SPEC-005).

use std::fmt;

use nexus_domain::CorrelationId;
use serde::{Deserialize, Serialize};

/// Canonical compute fabric error code (SPEC-006; SPEC-016 error states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ComputeErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication failed or is missing.
    Authentication,
    /// The principal is not authorized for the capability.
    Authorization,
    /// A contextual policy denied the request (state transition,
    /// placement constraint, budget ceiling).
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

impl ComputeErrorCode {
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

/// Typed compute fabric failure (SPEC-006). Messages are safe for display
/// and never contain secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeError {
    pub code: ComputeErrorCode,
    pub message: String,
    pub correlation: Option<CorrelationId>,
    pub actor: Option<Box<str>>,
    pub tenant: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl ComputeError {
    pub fn new(
        code: ComputeErrorCode,
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
        Self::new(
            ComputeErrorCode::Validation,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(ComputeErrorCode::Policy, message, None, None, None, None)
    }

    pub fn vocabulary(message: impl Into<String>) -> Self {
        Self::new(
            ComputeErrorCode::Vocabulary,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(
            ComputeErrorCode::Verification,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(
            ComputeErrorCode::Unavailable,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn with_context(
        mut self,
        correlation: impl Into<CorrelationId>,
        actor: impl Into<String>,
        tenant: impl Into<String>,
        resource: impl Into<String>,
    ) -> Self {
        self.correlation = Some(correlation.into());
        self.actor = Some(actor.into().into_boxed_str());
        self.tenant = Some(tenant.into().into_boxed_str());
        self.resource = Some(resource.into().into_boxed_str());
        self
    }
}

impl fmt::Display for ComputeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_http_str(), self.message)
    }
}

impl std::error::Error for ComputeError {}

impl ComputeErrorCode {
    fn as_http_str(self) -> &'static str {
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
            Self::Vocabulary => "VOCABULARY",
            Self::Internal => "INTERNAL",
        }
    }
}

/// Convenience result alias.
pub type ComputeResult<T> = Result<T, ComputeError>;
