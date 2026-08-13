//! Typed OpenFGA provider failures (EP-008 M3 directive A).
//!
//! Every provider failure is classified into a stable machine code and
//! mapped onto the canonical `nexus-policy` error surface. The adapter
//! never leaks raw provider errors upward, and every failure fails
//! closed (a denial/error, never a grant).

use std::fmt;

use nexus_policy::error::{PolicyError, PolicyErrorCode};

/// Stable machine codes for OpenFGA adapter failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpenFgaErrorCode {
    /// Provider unreachable (connection refused, DNS, no route).
    Unavailable,
    /// Request exceeded the configured timeout.
    Timeout,
    /// Response was not parseable / had an unexpected shape.
    MalformedProviderResponse,
    /// Store or authorization model does not match the configured ids.
    ModelStoreMismatch,
    /// The relationship request itself is invalid (bad user/object/
    /// relation shape).
    InvalidRelationshipRequest,
    /// The provider rejected the caller's credentials/authorization.
    ProviderAuthorizationFailure,
}

impl OpenFgaErrorCode {
    /// Canonical wire code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Timeout => "timeout",
            Self::MalformedProviderResponse => "malformed_provider_response",
            Self::ModelStoreMismatch => "model_store_mismatch",
            Self::InvalidRelationshipRequest => "invalid_relationship_request",
            Self::ProviderAuthorizationFailure => "provider_authorization_failure",
        }
    }

    /// Map onto the canonical policy error code (SPEC-006).
    pub const fn policy_code(self) -> PolicyErrorCode {
        match self {
            Self::Unavailable => PolicyErrorCode::Unavailable,
            Self::Timeout => PolicyErrorCode::Timeout,
            Self::MalformedProviderResponse
            | Self::ModelStoreMismatch
            | Self::ProviderAuthorizationFailure => PolicyErrorCode::ExternalProvider,
            Self::InvalidRelationshipRequest => PolicyErrorCode::Validation,
        }
    }
}

impl fmt::Display for OpenFgaErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed OpenFGA provider failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenFgaError {
    /// Stable machine code.
    pub code: OpenFgaErrorCode,
    /// Redacted explanation (never secrets, tokens, or full payloads).
    pub message: String,
}

impl OpenFgaError {
    /// Construct a typed provider failure.
    pub fn new(code: OpenFgaErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Unavailable provider.
    pub fn unavailable(detail: impl Into<String>) -> Self {
        Self::new(OpenFgaErrorCode::Unavailable, detail)
    }

    /// Timeout.
    pub fn timeout(detail: impl Into<String>) -> Self {
        Self::new(OpenFgaErrorCode::Timeout, detail)
    }

    /// Malformed provider response.
    pub fn malformed(detail: impl Into<String>) -> Self {
        Self::new(OpenFgaErrorCode::MalformedProviderResponse, detail)
    }

    /// Store/model mismatch.
    pub fn mismatch(detail: impl Into<String>) -> Self {
        Self::new(OpenFgaErrorCode::ModelStoreMismatch, detail)
    }

    /// Invalid relationship request.
    pub fn invalid_request(detail: impl Into<String>) -> Self {
        Self::new(OpenFgaErrorCode::InvalidRelationshipRequest, detail)
    }

    /// Provider authorization failure.
    pub fn authorization(detail: impl Into<String>) -> Self {
        Self::new(OpenFgaErrorCode::ProviderAuthorizationFailure, detail)
    }

    /// Map onto the canonical `nexus-policy` error surface. Fail
    /// closed: any provider failure becomes a policy error, never an
    /// allow.
    pub fn into_policy(self) -> PolicyError {
        PolicyError::new(self.code.policy_code(), self.message, None)
    }
}

impl fmt::Display for OpenFgaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "openfga {}: {}", self.code, self.message)
    }
}

impl std::error::Error for OpenFgaError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep008_unit_error_codes_are_stable() {
        assert_eq!(OpenFgaErrorCode::Unavailable.as_str(), "unavailable");
        assert_eq!(OpenFgaErrorCode::Timeout.as_str(), "timeout");
        assert_eq!(
            OpenFgaErrorCode::MalformedProviderResponse.as_str(),
            "malformed_provider_response"
        );
        assert_eq!(
            OpenFgaErrorCode::ModelStoreMismatch.as_str(),
            "model_store_mismatch"
        );
        assert_eq!(
            OpenFgaErrorCode::InvalidRelationshipRequest.as_str(),
            "invalid_relationship_request"
        );
        assert_eq!(
            OpenFgaErrorCode::ProviderAuthorizationFailure.as_str(),
            "provider_authorization_failure"
        );
    }

    #[test]
    fn ep008_unit_error_maps_fail_closed() {
        for code in [
            OpenFgaErrorCode::Unavailable,
            OpenFgaErrorCode::Timeout,
            OpenFgaErrorCode::MalformedProviderResponse,
            OpenFgaErrorCode::ModelStoreMismatch,
            OpenFgaErrorCode::InvalidRelationshipRequest,
            OpenFgaErrorCode::ProviderAuthorizationFailure,
        ] {
            let err = OpenFgaError::new(code, "detail");
            let policy = err.clone().into_policy();
            assert_eq!(policy.code, code.policy_code());
            assert!(policy.message.contains("detail"));
        }
        // Unavailable/timeout map to distinct codes.
        assert_eq!(
            OpenFgaErrorCode::Unavailable.policy_code(),
            PolicyErrorCode::Unavailable
        );
        assert_eq!(
            OpenFgaErrorCode::Timeout.policy_code(),
            PolicyErrorCode::Timeout
        );
    }
}
