//! Typed event-layer errors (SPEC-006).
//!
//! Mirrors the SPEC-006 code ladder used by `nexus-data::DataError` so a
//! consumer can map every boundary failure to the same stable machine
//! codes, preserving correlation for incident tracing.

use std::fmt;

/// Stable machine code for an event-layer failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventErrorCode {
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

impl EventErrorCode {
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

impl fmt::Display for EventErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed event-layer error with a stable code and optional correlation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventError {
    code: EventErrorCode,
    message: String,
    correlation_id: Option<String>,
}

impl EventError {
    /// Create a new error with the given code and message.
    pub fn new(code: EventErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id: None,
        }
    }

    /// Attach a correlation reference for incident tracing.
    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// The stable machine code.
    pub const fn code(&self) -> EventErrorCode {
        self.code
    }

    /// The human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The correlation reference, when attached.
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }
}

impl fmt::Display for EventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for EventError {}

impl From<serde_json::Error> for EventError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(EventErrorCode::Validation, format!("json: {err}"))
    }
}
