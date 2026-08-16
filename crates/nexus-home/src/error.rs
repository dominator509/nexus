//! EP-020 home typed errors (SPEC-006 codes).
//!
//! All failures preserve correlation, redact sensitive content, and
//! distinguish validation, authentication, authorization, policy,
//! unavailable, timeout, conflict, verification, vocabulary, external
//! provider, and internal invariant failures. Home Assistant credentials
//! are never logged, stored in manifests, or exposed through this type.

use std::fmt;

/// Canonical SPEC-006 error codes for the home surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication/authorization rejected the request.
    Authorization,
    /// Policy rejected the request (approval class, risk ceiling).
    Policy,
    /// The referenced device/entity/capability does not exist.
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

impl HomeErrorCode {
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

/// Typed home error preserving correlation and resource context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeError {
    pub code: HomeErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl HomeError {
    pub fn new(
        code: HomeErrorCode,
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

    /// Redacted summary: error code + resource + correlation, never
    /// message bodies that could carry provider payloads or secrets.
    pub fn redacted(&self) -> String {
        match (&self.correlation_id, &self.resource) {
            (Some(c), Some(r)) => {
                format!("{} resource={} correlation={}", self.code.as_str(), r, c)
            }
            (Some(c), None) => format!("{} correlation={}", self.code.as_str(), c),
            (None, Some(r)) => format!("{} resource={}", self.code.as_str(), r),
            (None, None) => self.code.as_str().to_string(),
        }
    }
}

impl fmt::Display for HomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.redacted())
    }
}

impl std::error::Error for HomeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep020_unit_error_codes_are_typed() {
        assert_eq!(HomeErrorCode::Verification.as_str(), "VERIFICATION");
        assert_eq!(HomeErrorCode::External.as_str(), "EXTERNAL");
        assert_eq!(HomeErrorCode::Timeout.as_str(), "TIMEOUT");
    }

    #[test]
    fn ep020_unit_error_redaction_hides_message() {
        let err = HomeError::new(
            HomeErrorCode::External,
            "provider payload with token=secret",
            Some(Box::from("corr-1")),
            Some(Box::from("light.kitchen")),
        );
        let red = err.redacted();
        assert!(!red.contains("secret"));
        assert!(red.contains("EXTERNAL"));
        assert!(red.contains("light.kitchen"));
        assert!(red.contains("corr-1"));
    }
}
