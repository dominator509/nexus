//! EP-032 notification error surface (SPEC-006 codes; SPEC-014 error
//! states). Every error preserves correlation and resource references,
//! and redacts sensitive content (never notification bodies, private
//! content, or provider credentials in telemetry).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical notification error codes (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication/authorization rejected the request (SPEC-005).
    Authorization,
    /// Policy rejected the request (scope, privacy class, approval).
    Policy,
    /// The referenced notification/recipient/channel does not exist.
    NotFound,
    /// A state conflict (idempotency, lifecycle, duplicate).
    Conflict,
    /// The provider or transport is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// Verification of a side effect failed (exact-target mismatch).
    Verification,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// The external provider returned a malformed or unexpected
    /// response.
    External,
    /// Rate limit exceeded.
    RateLimit,
    /// A compensating action was required and did not complete.
    Compensation,
    /// Internal invariant failure.
    Internal,
}

impl NotificationErrorCode {
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
            Self::Compensation => "COMPENSATION",
            Self::Internal => "INTERNAL",
        }
    }
}

impl fmt::Display for NotificationErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical notification error (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationError {
    pub code: NotificationErrorCode,
    pub message: String,
    pub correlation: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl NotificationError {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        code: NotificationErrorCode,
        message: impl Into<String>,
        correlation: Option<String>,
        actor: Option<String>,
        tenant: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation: correlation.map(String::into_boxed_str),
            actor: actor.map(String::into_boxed_str),
            tenant: tenant.map(String::into_boxed_str),
            resource: resource.map(String::into_boxed_str),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(
            NotificationErrorCode::Validation,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(
            NotificationErrorCode::Policy,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(
            NotificationErrorCode::Unavailable,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn external(message: impl Into<String>) -> Self {
        Self::new(
            NotificationErrorCode::External,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(
            NotificationErrorCode::Internal,
            message,
            None,
            None,
            None,
            None,
        )
    }
}

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for NotificationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep032_unit_error_uses_spec006_codes() {
        // EP-032 failures use the canonical SPEC-006 codes; they are
        // never redefined or widened.
        let err = NotificationError::unavailable("no channel bound");
        assert_eq!(err.code, NotificationErrorCode::Unavailable);
        let err = NotificationError::validation("bad envelope");
        assert_eq!(err.code, NotificationErrorCode::Validation);
        let err = NotificationError::policy("privacy denied");
        assert_eq!(err.code, NotificationErrorCode::Policy);
        let json = serde_json::to_string(&NotificationErrorCode::Verification).unwrap();
        assert_eq!(json, "\"VERIFICATION\"");
    }

    #[test]
    fn ep032_unit_error_serde_rejects_unknown_code() {
        let res: Result<NotificationErrorCode, _> = serde_json::from_str("\"PURPLE\"");
        assert!(res.is_err());
    }
}
