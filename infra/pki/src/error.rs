//! Typed PKI adapter error surface (EP-009 M4 directive R).
//!
//! Every provider failure maps to a typed code that stays
//! distinguishable for audit (unavailable vs permission vs role
//! violation vs malformed) while the message never contains secrets,
//! tokens, private keys, or raw provider payloads.

use std::fmt;

use nexus_trust::{TrustError, TrustErrorCode};

/// Typed PKI adapter failure classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PkiErrorCode {
    /// Provider unreachable (connection refused, DNS, transport).
    Unavailable,
    /// Request exceeded the configured timeout.
    Timeout,
    /// Provider returned a malformed/unparseable response.
    MalformedProviderResponse,
    /// Authentication failed (bad token, revoked credential).
    AuthenticationFailed,
    /// Policy denies the requested path/operation.
    PermissionDenied,
    /// Certificate/issuer/CRL does not exist.
    NotFound,
    /// CSR is malformed or unparseable.
    CsrRejected,
    /// Requested identity is outside the issuance role constraints.
    RoleViolation,
    /// Requested TTL is outside the role policy.
    TtlViolation,
    /// Certificate fails cryptographic validation (chain, signature).
    CertificateInvalid,
    /// Certificate identity does not bind to the expected ServiceIdentity.
    IdentityMismatch,
    /// Certificate is revoked.
    Revoked,
    /// Certificate is outside its validity window.
    ValidityWindow,
    /// Internal adapter invariant violated.
    Internal,
}

impl PkiErrorCode {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "PKI_UNAVAILABLE",
            Self::Timeout => "PKI_TIMEOUT",
            Self::MalformedProviderResponse => "PKI_MALFORMED_RESPONSE",
            Self::AuthenticationFailed => "PKI_AUTHENTICATION_FAILED",
            Self::PermissionDenied => "PKI_PERMISSION_DENIED",
            Self::NotFound => "PKI_NOT_FOUND",
            Self::CsrRejected => "PKI_CSR_REJECTED",
            Self::RoleViolation => "PKI_ROLE_VIOLATION",
            Self::TtlViolation => "PKI_TTL_VIOLATION",
            Self::CertificateInvalid => "PKI_CERTIFICATE_INVALID",
            Self::IdentityMismatch => "PKI_IDENTITY_MISMATCH",
            Self::Revoked => "PKI_REVOKED",
            Self::ValidityWindow => "PKI_VALIDITY_WINDOW",
            Self::Internal => "PKI_INTERNAL",
        }
    }

    /// Map to the canonical nexus-trust code (SPEC-006).
    pub const fn trust_code(self) -> TrustErrorCode {
        match self {
            Self::Unavailable => TrustErrorCode::Unavailable,
            Self::Timeout => TrustErrorCode::Timeout,
            Self::MalformedProviderResponse => TrustErrorCode::MalformedProviderResponse,
            Self::AuthenticationFailed | Self::PermissionDenied => {
                TrustErrorCode::ProviderAuthorization
            }
            Self::NotFound => TrustErrorCode::NotFound,
            Self::CsrRejected | Self::RoleViolation | Self::TtlViolation => {
                TrustErrorCode::InvalidReference
            }
            Self::CertificateInvalid
            | Self::IdentityMismatch
            | Self::Revoked
            | Self::ValidityWindow => TrustErrorCode::StateConflict,
            Self::Internal => TrustErrorCode::Internal,
        }
    }
}

/// A typed PKI adapter error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PkiError {
    /// Typed failure class.
    pub code: PkiErrorCode,
    /// Redacted human-safe message (never secrets/tokens/keys/payloads).
    pub message: String,
    /// HTTP status observed, when available.
    pub http_status: Option<u16>,
}

impl PkiError {
    /// Construct a typed error.
    pub fn new(code: PkiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            http_status: None,
        }
    }

    /// Construct with an observed HTTP status.
    pub fn with_status(code: PkiErrorCode, message: impl Into<String>, status: u16) -> Self {
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

impl fmt::Display for PkiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.http_status {
            Some(s) => write!(f, "{} (http {})", self.code.as_str(), s),
            None => write!(f, "{}", self.code.as_str()),
        }
    }
}

impl std::error::Error for PkiError {}

impl From<PkiError> for TrustError {
    fn from(e: PkiError) -> Self {
        e.into_trust()
    }
}
