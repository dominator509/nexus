//! Typed OpenBao error surface (EP-009 M2 directive F).
//!
//! Every provider failure maps to a typed code that stays
//! distinguishable for audit (auth vs missing vs destroyed) while the
//! message never contains secrets, tokens, or payloads.

use std::fmt;

use nexus_trust::{TrustError, TrustErrorCode};

/// Typed OpenBao adapter failure classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpenBaoErrorCode {
    /// Provider unreachable (connection refused, DNS, transport).
    Unavailable,
    /// Request exceeded the configured timeout.
    Timeout,
    /// Provider returned a malformed/unparseable response.
    MalformedProviderResponse,
    /// Authentication failed (bad AppRole credential, revoked token).
    AuthenticationFailed,
    /// Policy denies the requested path/operation.
    PermissionDenied,
    /// Secret or version does not exist.
    NotFound,
    /// Secret version was permanently destroyed.
    Destroyed,
    /// Version mismatch (CAS conflict / stale version read).
    VersionMismatch,
    /// Operation violates provider policy in an uncategorized way.
    PolicyViolation,
    /// Internal adapter invariant violated.
    Internal,
}

impl OpenBaoErrorCode {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "OPENBAO_UNAVAILABLE",
            Self::Timeout => "OPENBAO_TIMEOUT",
            Self::MalformedProviderResponse => "OPENBAO_MALFORMED_RESPONSE",
            Self::AuthenticationFailed => "OPENBAO_AUTHENTICATION_FAILED",
            Self::PermissionDenied => "OPENBAO_PERMISSION_DENIED",
            Self::NotFound => "OPENBAO_NOT_FOUND",
            Self::Destroyed => "OPENBAO_DESTROYED",
            Self::VersionMismatch => "OPENBAO_VERSION_MISMATCH",
            Self::PolicyViolation => "OPENBAO_POLICY_VIOLATION",
            Self::Internal => "OPENBAO_INTERNAL",
        }
    }

    /// Map to the canonical nexus-trust code (SPEC-006).
    pub const fn trust_code(self) -> TrustErrorCode {
        match self {
            Self::Unavailable => TrustErrorCode::Unavailable,
            Self::Timeout => TrustErrorCode::Timeout,
            Self::MalformedProviderResponse => TrustErrorCode::MalformedProviderResponse,
            Self::AuthenticationFailed | Self::PermissionDenied | Self::PolicyViolation => {
                TrustErrorCode::ProviderAuthorization
            }
            Self::NotFound => TrustErrorCode::NotFound,
            Self::Destroyed => TrustErrorCode::NotFound,
            Self::VersionMismatch => TrustErrorCode::StateConflict,
            Self::Internal => TrustErrorCode::Internal,
        }
    }
}

/// A typed OpenBao adapter error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenBaoError {
    /// Typed failure class.
    pub code: OpenBaoErrorCode,
    /// Redacted human-safe message (never secrets/tokens/payloads).
    pub message: String,
    /// HTTP status observed, when available.
    pub http_status: Option<u16>,
}

impl OpenBaoError {
    /// Construct a typed error.
    pub fn new(code: OpenBaoErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            http_status: None,
        }
    }

    /// Construct with an observed HTTP status.
    pub fn with_status(code: OpenBaoErrorCode, message: impl Into<String>, status: u16) -> Self {
        Self {
            code,
            message: message.into(),
            http_status: Some(status),
        }
    }

    /// Map to the canonical trust error.
    pub fn into_trust(self) -> TrustError {
        TrustError::new(self.code.trust_code(), self.message)
    }
}

impl fmt::Display for OpenBaoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.http_status {
            Some(s) => write!(f, "{} (http {})", self.code.as_str(), s),
            None => write!(f, "{}", self.code.as_str()),
        }
    }
}

impl std::error::Error for OpenBaoError {}

impl From<OpenBaoError> for TrustError {
    fn from(e: OpenBaoError) -> Self {
        e.into_trust()
    }
}
