//! EP-027 fax error surface (SPEC-006 codes; SPEC-014 error states).
//! Every error preserves correlation and resource references, and
//! redacts sensitive content (never raw fax document bodies,
//! credentials, or artifact content in telemetry).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical fax error codes (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FaxErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication/authorization rejected the request (SPEC-005).
    Authorization,
    /// Policy rejected the request (scope, approval class, carrier,
    /// document scan).
    Policy,
    /// The referenced fax job/document/route does not exist.
    NotFound,
    /// A state conflict (idempotency, lifecycle, digest).
    Conflict,
    /// The provider or transport is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// Verification failed (expected target state not observed).
    Verification,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// The external provider returned a malformed or unexpected
    /// response.
    External,
    /// Rate limit exceeded.
    RateLimit,
    /// Internal invariant failure.
    Internal,
}

impl FaxErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::Verification => "VERIFICATION",
            Self::Vocabulary => "VOCABULARY",
            Self::External => "EXTERNAL",
            Self::RateLimit => "RATE_LIMIT",
            Self::Internal => "INTERNAL",
        }
    }
}

/// Canonical fax error with correlation and resource context.
///
/// The `context` field is a redacted human-readable message: it must
/// never contain fax document content, credentials, or carrier
/// artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaxError {
    pub code: FaxErrorCode,
    pub message: String,
    pub correlation: Option<String>,
    pub resource: Option<String>,
}

impl FaxError {
    pub fn new(
        code: FaxErrorCode,
        message: impl Into<String>,
        correlation: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation,
            resource,
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::Validation, message, None, None)
    }

    pub fn authorization(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::Authorization, message, None, None)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::Policy, message, None, None)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::NotFound, message, None, None)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::Conflict, message, None, None)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::Unavailable, message, None, None)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::Timeout, message, None, None)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::Verification, message, None, None)
    }

    pub fn vocabulary(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::Vocabulary, message, None, None)
    }

    pub fn external(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::External, message, None, None)
    }

    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::RateLimit, message, None, None)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(FaxErrorCode::Internal, message, None, None)
    }

    /// Attach correlation context (canonical `fax-<nanos>-<seq>`).
    pub fn with_correlation(mut self, correlation: impl Into<String>) -> Self {
        self.correlation = Some(correlation.into());
        self
    }

    /// Attach the failing resource reference.
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }
}

impl fmt::Display for FaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for FaxError {}
