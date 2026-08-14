//! Canonical trust error surface (SPEC-006).
//!
//! Every trust operation fails closed with a typed code and a redacted
//! message. Provider adapters preserve their typed cause in the message
//! (audit/observability) without leaking secrets.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical trust error codes (SPEC-006 error classes at the trust
/// boundary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrustErrorCode {
    /// Provider unavailable (connection, DNS, transport).
    Unavailable,
    /// Provider timed out.
    Timeout,
    /// Malformed provider response.
    MalformedProviderResponse,
    /// Invalid reference or input.
    InvalidReference,
    /// Secret/certificate/token does not exist or is revoked/expired.
    NotFound,
    /// Authentication/authorization failure at the provider boundary.
    ProviderAuthorization,
    /// Operation rejected because the object is in the wrong state.
    StateConflict,
    /// Internal invariant violation.
    Internal,
}

impl TrustErrorCode {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::MalformedProviderResponse => "MALFORMED_PROVIDER_RESPONSE",
            Self::InvalidReference => "INVALID_REFERENCE",
            Self::NotFound => "NOT_FOUND",
            Self::ProviderAuthorization => "PROVIDER_AUTHORIZATION",
            Self::StateConflict => "STATE_CONFLICT",
            Self::Internal => "INTERNAL",
        }
    }
}

impl fmt::Display for TrustErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A trust error: typed code + redacted message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustError {
    /// Typed error code.
    pub code: TrustErrorCode,
    /// Redacted human-safe message (never secrets).
    pub message: String,
}

impl TrustError {
    /// Construct a trust error.
    pub fn new(code: TrustErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// A validation/reference error.
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(TrustErrorCode::InvalidReference, message)
    }

    /// A not-found/revoked/expired error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(TrustErrorCode::NotFound, message)
    }

    /// An internal invariant error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(TrustErrorCode::Internal, message)
    }
}

impl fmt::Display for TrustError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for TrustError {}
