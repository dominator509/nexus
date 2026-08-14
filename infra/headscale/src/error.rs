//! Headscale adapter typed errors (SPEC-006).
//!
//! Every adapter operation fails closed with a canonical trust code and
//! a redacted message. Provider (CLI/gRPC) failure causes are preserved
//! as typed error codes, never collapsed into a generic failure.

use std::fmt;

use nexus_trust::{TrustError, TrustErrorCode};

/// Headscale adapter error classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadscaleErrorCode {
    /// The headscale CLI binary is missing or cannot be executed.
    BinaryUnavailable,
    /// Remote gRPC connection failed (address unreachable, timeout).
    Unavailable,
    /// API key rejected / permission denied at the provider boundary.
    ProviderAuthorization,
    /// Provider returned malformed or unexpected output.
    MalformedProviderResponse,
    /// Requested node/user does not exist.
    NotFound,
    /// Operation rejected because the object is in the wrong state
    /// (e.g. registering an already-registered node).
    StateConflict,
    /// Internal invariant violation.
    Internal,
}

impl HeadscaleErrorCode {
    /// Map to the canonical nexus-trust typed code.
    pub const fn trust_code(self) -> TrustErrorCode {
        match self {
            Self::BinaryUnavailable => TrustErrorCode::Unavailable,
            Self::Unavailable => TrustErrorCode::Unavailable,
            Self::ProviderAuthorization => TrustErrorCode::ProviderAuthorization,
            Self::MalformedProviderResponse => TrustErrorCode::MalformedProviderResponse,
            Self::NotFound => TrustErrorCode::NotFound,
            Self::StateConflict => TrustErrorCode::StateConflict,
            Self::Internal => TrustErrorCode::Internal,
        }
    }

    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BinaryUnavailable => "HEADSCALE_BINARY_UNAVAILABLE",
            Self::Unavailable => "HEADSCALE_UNAVAILABLE",
            Self::ProviderAuthorization => "HEADSCALE_PROVIDER_AUTHORIZATION",
            Self::MalformedProviderResponse => "HEADSCALE_MALFORMED_PROVIDER_RESPONSE",
            Self::NotFound => "HEADSCALE_NOT_FOUND",
            Self::StateConflict => "HEADSCALE_STATE_CONFLICT",
            Self::Internal => "HEADSCALE_INTERNAL",
        }
    }
}

impl fmt::Display for HeadscaleErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A headscale adapter error: typed code + redacted message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadscaleError {
    /// Typed error code.
    pub code: HeadscaleErrorCode,
    /// Redacted human-safe message (never secrets).
    pub message: String,
}

impl HeadscaleError {
    /// Construct a headscale adapter error.
    pub fn new(code: HeadscaleErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Convert into a canonical nexus-trust error.
    pub fn into_trust(self) -> TrustError {
        TrustError::new(self.code.trust_code(), self.message)
    }
}

impl fmt::Display for HeadscaleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for HeadscaleError {}

impl From<HeadscaleError> for TrustError {
    fn from(e: HeadscaleError) -> Self {
        e.into_trust()
    }
}
