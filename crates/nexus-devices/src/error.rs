//! EP-024 device typed errors (SPEC-006 codes).
//!
//! All failures preserve correlation and redact sensitive content.
//! Free-form provider payloads are normalized at the infrastructure
//! boundary and never become domain contracts.

use std::fmt;

/// Canonical SPEC-006 error codes for the device surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevicesErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication/authorization rejected the request.
    Authorization,
    /// Policy rejected the request (privacy, approval class).
    Policy,
    /// The referenced device/zone/robot does not exist.
    NotFound,
    /// A state conflict (idempotency, version, lifecycle, digest).
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
    /// Internal invariant failure.
    Internal,
}

impl DevicesErrorCode {
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
            Self::Internal => "INTERNAL",
        }
    }
}

/// Typed device error preserving correlation and resource context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevicesError {
    pub code: DevicesErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl DevicesError {
    pub fn new(
        code: DevicesErrorCode,
        message: impl Into<String>,
        correlation_id: Option<Box<str>>,
        resource: Option<Box<str>>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id,
            resource,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(DevicesErrorCode::Unavailable, message, None, None)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(DevicesErrorCode::Validation, message, None, None)
    }

    pub fn vocabulary(message: impl Into<String>) -> Self {
        Self::new(DevicesErrorCode::Vocabulary, message, None, None)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(DevicesErrorCode::Policy, message, None, None)
    }
}

impl fmt::Display for DevicesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.code.as_str(), self.message)
    }
}

impl std::error::Error for DevicesError {}
