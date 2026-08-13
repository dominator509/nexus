//! Canonical policy errors (SPEC-006).
//!
//! Every boundary returns a stable machine code and safe human
//! explanation. All failures distinguish validation, authentication,
//! authorization, policy, unavailable, timeout, conflict, rate limit,
//! external provider, verification, compensation, and internal
//! invariant failures, and preserve correlation.

use std::fmt;

use nexus_domain::CorrelationId;

/// Stable machine codes for policy boundary failures (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PolicyErrorCode {
    Validation,
    Authentication,
    Authorization,
    Policy,
    Unavailable,
    Timeout,
    Conflict,
    RateLimit,
    ExternalProvider,
    Verification,
    Compensation,
    InternalInvariant,
}

impl PolicyErrorCode {
    /// Canonical wire code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "validation",
            Self::Authentication => "authentication",
            Self::Authorization => "authorization",
            Self::Policy => "policy",
            Self::Unavailable => "unavailable",
            Self::Timeout => "timeout",
            Self::Conflict => "conflict",
            Self::RateLimit => "rate_limit",
            Self::ExternalProvider => "external_provider",
            Self::Verification => "verification",
            Self::Compensation => "compensation",
            Self::InternalInvariant => "internal_invariant",
        }
    }
}

impl fmt::Display for PolicyErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed policy boundary failure (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyError {
    /// Stable machine code.
    pub code: PolicyErrorCode,
    /// Safe human explanation (redacted; never secrets or prompts).
    pub message: String,
    /// Correlation of the failing request, when known.
    pub correlation: Option<CorrelationId>,
}

impl PolicyError {
    /// Construct a policy error.
    pub fn new(
        code: PolicyErrorCode,
        message: impl Into<String>,
        correlation: Option<CorrelationId>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation,
        }
    }

    /// A validation failure.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(PolicyErrorCode::Validation, message, None)
    }

    /// An internal invariant failure (never leaks internals).
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(PolicyErrorCode::InternalInvariant, message, None)
    }
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PolicyError {}
