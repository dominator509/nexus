//! Typed data-layer errors (SPEC-006).
//!
//! Every boundary error carries a stable machine code and preserves the
//! request correlation. Codes distinguish validation, authentication,
//! authorization, policy, unavailable, timeout, conflict, rate limit,
//! external provider, verification, compensation, and internal invariant
//! failures (SPEC-006 required behavior).

use std::fmt;

/// Stable machine code for a data-layer failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataErrorCode {
    /// Input failed canonical validation.
    Validation,
    /// Caller failed authentication.
    Authentication,
    /// Caller lacks authorization.
    Authorization,
    /// A policy decision denied the operation.
    Policy,
    /// A dependency or component is unavailable.
    Unavailable,
    /// The operation timed out.
    Timeout,
    /// A state conflict (idempotency or optimistic concurrency).
    Conflict,
    /// Rate limit exceeded.
    RateLimit,
    /// An external provider failed.
    ExternalProvider,
    /// Verification of an external effect failed.
    Verification,
    /// Compensation for a partial effect failed.
    Compensation,
    /// An internal invariant was violated.
    Invariant,
}

impl DataErrorCode {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authentication => "AUTHENTICATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::Conflict => "CONFLICT",
            Self::RateLimit => "RATE_LIMIT",
            Self::ExternalProvider => "EXTERNAL_PROVIDER",
            Self::Verification => "VERIFICATION",
            Self::Compensation => "COMPENSATION",
            Self::Invariant => "INVARIANT",
        }
    }
}

impl fmt::Display for DataErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed data-layer error (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataError {
    code: DataErrorCode,
    message: String,
    /// Optional request correlation preserved across the boundary.
    correlation_id: Option<String>,
}

impl DataError {
    /// Create a new error with a stable code and safe message.
    pub fn new(code: DataErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id: None,
        }
    }

    /// Attach the request correlation for incident correlation.
    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Stable machine code.
    pub const fn code(&self) -> DataErrorCode {
        self.code
    }

    /// Safe human explanation; never contains secrets or private content.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Preserved correlation, when available.
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }
}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for DataError {}

impl From<serde_json::Error> for DataError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(DataErrorCode::Validation, format!("json: {err}"))
    }
}
