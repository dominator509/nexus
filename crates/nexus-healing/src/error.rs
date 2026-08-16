//! EP-019 healing typed errors (SPEC-006 codes).
//!
//! All failures preserve correlation, redact sensitive content, and
//! distinguish validation, authorization, policy, unavailable, timeout,
//! conflict, verification, vocabulary, and internal invariant failures.
//! A model/agent can never return `Remediated` through this type; only
//! real verification produces the terminal state.

use std::fmt;

/// Canonical SPEC-006 error codes for the healing surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealingErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication/authorization rejected the request.
    Authorization,
    /// Policy rejected the request (approval class, risk ceiling).
    Policy,
    /// The referenced incident/diagnosis/patch/approval does not exist.
    NotFound,
    /// A state conflict (idempotency, version, lifecycle, digest).
    Conflict,
    /// The provider or transport is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// Verification failed (reproduction, validation, security, post-deploy).
    Verification,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// Internal invariant failure.
    Internal,
}

impl HealingErrorCode {
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
            Self::Internal => "INTERNAL",
        }
    }
}

/// Typed healing error preserving correlation and resource context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealingError {
    pub code: HealingErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl HealingError {
    pub fn new(
        code: HealingErrorCode,
        message: impl Into<String>,
        correlation_id: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id: correlation_id.map(Into::into),
            resource: resource.map(Into::into),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(HealingErrorCode::Validation, message, None, None)
    }

    pub fn authorization(message: impl Into<String>) -> Self {
        Self::new(HealingErrorCode::Authorization, message, None, None)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(HealingErrorCode::Policy, message, None, None)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(HealingErrorCode::NotFound, message, None, None)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(HealingErrorCode::Conflict, message, None, None)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(HealingErrorCode::Unavailable, message, None, None)
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(HealingErrorCode::Timeout, message, None, None)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(HealingErrorCode::Verification, message, None, None)
    }

    pub fn vocabulary(enum_name: &str, value: &str) -> Self {
        Self::new(
            HealingErrorCode::Vocabulary,
            format!("unknown {enum_name} value: {value}"),
            None,
            None,
        )
    }

    pub fn with_correlation(mut self, correlation_id: String) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }
}

impl fmt::Display for HealingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for HealingError {}
