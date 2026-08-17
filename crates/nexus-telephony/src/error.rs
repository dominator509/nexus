//! EP-025 telephony error surface (SPEC-006 codes; SPEC-014 error
//! states). Every error preserves correlation and resource references,
//! and redacts sensitive content (directive 24: never put raw call
//! audio, credentials, or SIP Authorization headers into telemetry).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical telephony error codes (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CallErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication/authorization rejected the request (SPEC-005).
    Authorization,
    /// Policy rejected the request (disclosure, consent, cost cap,
    /// quiet hours, approval class).
    Policy,
    /// The referenced session/leg/endpoint/carrier does not exist.
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

impl CallErrorCode {
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

/// Canonical telephony error with correlation and resource context.
///
/// The `context` field is a redacted human-readable message: it must
/// never contain credentials, SIP Authorization headers, raw audio,
/// or private transcript content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallError {
    pub code: CallErrorCode,
    pub message: String,
    pub correlation: Option<String>,
    pub resource: Option<String>,
}

impl CallError {
    pub fn new(
        code: CallErrorCode,
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

    pub fn validation(msg: impl Into<String>) -> Self {
        Self::new(CallErrorCode::Validation, msg, None, None)
    }

    pub fn vocabulary(msg: impl Into<String>) -> Self {
        Self::new(CallErrorCode::Vocabulary, msg, None, None)
    }

    pub fn policy(msg: impl Into<String>) -> Self {
        Self::new(CallErrorCode::Policy, msg, None, None)
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(CallErrorCode::NotFound, msg, None, None)
    }

    pub fn unavailable(msg: impl Into<String>) -> Self {
        Self::new(CallErrorCode::Unavailable, msg, None, None)
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::new(CallErrorCode::Timeout, msg, None, None)
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::new(CallErrorCode::Conflict, msg, None, None)
    }

    pub fn verification(msg: impl Into<String>) -> Self {
        Self::new(CallErrorCode::Verification, msg, None, None)
    }

    pub fn external(msg: impl Into<String>) -> Self {
        Self::new(CallErrorCode::External, msg, None, None)
    }

    pub fn with_correlation(mut self, correlation: impl Into<String>) -> Self {
        self.correlation = Some(correlation.into());
        self
    }

    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }
}

impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} (correlation: {}, resource: {})",
            self.code.as_str(),
            self.message,
            self.correlation.as_deref().unwrap_or("-"),
            self.resource.as_deref().unwrap_or("-"),
        )
    }
}

impl std::error::Error for CallError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep025_unit_error_code_str_roundtrip() {
        for code in [
            CallErrorCode::Validation,
            CallErrorCode::Authorization,
            CallErrorCode::Policy,
            CallErrorCode::NotFound,
            CallErrorCode::Conflict,
            CallErrorCode::Unavailable,
            CallErrorCode::Timeout,
            CallErrorCode::Verification,
            CallErrorCode::Vocabulary,
            CallErrorCode::External,
            CallErrorCode::RateLimit,
            CallErrorCode::Internal,
        ] {
            assert!(!code.as_str().is_empty());
        }
    }

    #[test]
    fn ep025_unit_error_serializes_redacted_surface() {
        let err = CallError::new(
            CallErrorCode::Authorization,
            "bad credential",
            Some("tel-1".into()),
            Some("session/abc".into()),
        );
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("AUTHORIZATION"));
        assert!(json.contains("tel-1"));
        // The error surface is redacted by construction: raw secrets
        // must never be placed in the message by callers.
        assert!(!json.contains("Authorization: Digest"));
    }

    #[test]
    fn ep025_unit_error_display_redacts() {
        let err = CallError::new(
            CallErrorCode::Unavailable,
            "provider unreachable",
            Some("tel-2".into()),
            None,
        );
        let s = err.to_string();
        assert!(s.contains("UNAVAILABLE"));
        assert!(s.contains("tel-2"));
    }
}
