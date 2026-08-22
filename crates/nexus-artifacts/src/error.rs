//! EP-037 artifact storage error surface (SPEC-006 codes; SPEC-024 error
//! states).
//!
//! Every failure distinguishes validation, authentication, authorization,
//! policy, unavailable, timeout, conflict, rate limit, external provider,
//! verification, compensation, and internal invariant failures, and
//! preserves correlation, actor, tenant, and resource references where
//! available. Messages never contain secrets, tokens, or private payloads
//! (least privilege, SPEC-005, SECURITY.md).

use std::fmt;

use nexus_domain::{CorrelationId, TenantId};
use serde::{Deserialize, Serialize};

/// Canonical artifact storage error code (SPEC-006; SPEC-024 error states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication failed or is missing.
    Authentication,
    /// The principal is not authorized for the capability.
    Authorization,
    /// A contextual policy denied the request (state transition,
    /// retention boundary, delete-before-verify).
    Policy,
    /// The backend, capability, or resource is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// A conflicting state prevented the operation (idempotency,
    /// lifecycle, version conflict).
    Conflict,
    /// The referenced object does not exist.
    NotFound,
    /// The caller exceeded a declared rate limit.
    RateLimit,
    /// An external provider returned a failure.
    ExternalProvider,
    /// Verification of a side effect failed (hash mismatch,
    /// exact-target readback).
    Verification,
    /// A compensating action was required and did not complete.
    Compensation,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// An internal invariant was violated.
    Internal,
}

impl ArtifactErrorCode {
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

/// Typed artifact storage failure (SPEC-006). Messages are safe for display
/// and never contain secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactError {
    pub code: ArtifactErrorCode,
    pub message: String,
    pub correlation: Option<CorrelationId>,
    pub actor: Option<Box<str>>,
    pub tenant: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl ArtifactError {
    /// Construct a validation failure.
    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::Validation,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct a policy denial.
    pub fn policy(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::Policy,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct a verification failure.
    pub fn verification(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::Verification,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct a not-found failure.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::NotFound,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct a conflict failure.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::Conflict,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct a vocabulary rejection.
    pub fn vocabulary(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::Vocabulary,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct a timeout failure.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::Timeout,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct an unavailable failure.
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::Unavailable,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct an authorization failure.
    pub fn authorization(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::Authorization,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct an external provider failure.
    pub fn external(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::ExternalProvider,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Construct an internal invariant failure.
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: ArtifactErrorCode::Internal,
            message: message.into(),
            correlation: None,
            actor: None,
            tenant: None,
            resource: None,
        }
    }

    /// Attach correlation context to the failure.
    pub fn with_correlation(mut self, correlation: CorrelationId) -> Self {
        self.correlation = Some(correlation);
        self
    }

    /// Attach tenant context to the failure.
    pub fn with_tenant(mut self, tenant: TenantId) -> Self {
        self.tenant = Some(Box::from(tenant.as_str()));
        self
    }

    /// Attach actor context to the failure.
    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(Box::from(actor.into()));
        self
    }

    /// Attach a resource reference to the failure.
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(Box::from(resource.into()));
        self
    }
}

impl fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ArtifactError {}

/// Convenience result alias for artifact operations.
pub type ArtifactResult<T> = Result<T, ArtifactError>;
