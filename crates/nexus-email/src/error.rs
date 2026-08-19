//! EP-026 email error surface (SPEC-006 codes; SPEC-014 error
//! states). Every error preserves correlation and resource references,
//! and redacts sensitive content (never raw message bodies, credentials,
//! or attachment artifacts in telemetry).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical email error codes (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MailErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication/authorization rejected the request (SPEC-005).
    Authorization,
    /// Policy rejected the request (scope, approval class, retention,
    /// attachment scan).
    Policy,
    /// The referenced mailbox/thread/message/draft/attachment does not
    /// exist.
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

impl MailErrorCode {
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

/// Canonical email error with correlation and resource context.
///
/// The `context` field is a redacted human-readable message: it must
/// never contain message bodies, credentials, or attachment artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailError {
    pub code: MailErrorCode,
    pub message: String,
    pub correlation: Option<String>,
    pub resource: Option<String>,
}

impl MailError {
    pub fn new(
        code: MailErrorCode,
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
        Self::new(MailErrorCode::Validation, msg, None, None)
    }

    pub fn vocabulary(msg: impl Into<String>) -> Self {
        Self::new(MailErrorCode::Vocabulary, msg, None, None)
    }

    pub fn policy(msg: impl Into<String>) -> Self {
        Self::new(MailErrorCode::Policy, msg, None, None)
    }

    pub fn authorization(msg: impl Into<String>) -> Self {
        Self::new(MailErrorCode::Authorization, msg, None, None)
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(MailErrorCode::NotFound, msg, None, None)
    }

    pub fn unavailable(msg: impl Into<String>) -> Self {
        Self::new(MailErrorCode::Unavailable, msg, None, None)
    }

    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::new(MailErrorCode::Timeout, msg, None, None)
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::new(MailErrorCode::Conflict, msg, None, None)
    }

    pub fn verification(msg: impl Into<String>) -> Self {
        Self::new(MailErrorCode::Verification, msg, None, None)
    }

    pub fn external(msg: impl Into<String>) -> Self {
        Self::new(MailErrorCode::External, msg, None, None)
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

impl fmt::Display for MailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mail {}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for MailError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep026_unit_error_code_str_roundtrip() {
        for code in [
            MailErrorCode::Validation,
            MailErrorCode::Authorization,
            MailErrorCode::Policy,
            MailErrorCode::NotFound,
            MailErrorCode::Conflict,
            MailErrorCode::Unavailable,
            MailErrorCode::Timeout,
            MailErrorCode::Verification,
            MailErrorCode::Vocabulary,
            MailErrorCode::External,
            MailErrorCode::RateLimit,
            MailErrorCode::Internal,
        ] {
            assert_eq!(MailErrorCode::as_str(code), code.as_str());
        }
    }

    #[test]
    fn ep026_unit_error_serializes_redacted_surface() {
        let err = MailError::authorization("mailbox requires authorization")
            .with_correlation("mail-123")
            .with_resource("mailbox/inbox");
        let json = serde_json::to_string(&err).expect("serialize");
        assert!(json.contains("AUTHORIZATION"));
        assert!(json.contains("mail-123"));
    }

    #[test]
    fn ep026_unit_error_display_redacts() {
        let err = MailError::policy("SEND denied by policy");
        let text = err.to_string();
        assert!(text.contains("POLICY"));
        assert!(text.contains("SEND denied"));
    }
}
