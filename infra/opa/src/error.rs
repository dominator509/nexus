//! Typed OPA provider failures (EP-008 M4 directive A).
//!
//! Every provider failure is classified into a stable machine code and
//! mapped onto the canonical `nexus-policy` error surface while
//! PRESERVING the typed cause for audit/observability (directive A:
//! do not turn provider errors into an implicit DENY without the typed
//! cause). Fail closed: no failure becomes an allow.

use std::fmt;

use nexus_policy::error::{PolicyError, PolicyErrorCode};

/// Stable machine codes for OPA adapter failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpaErrorCode {
    /// Provider unreachable (connection refused, DNS, no route).
    Unavailable,
    /// Request exceeded the configured timeout or evaluation deadline.
    Timeout,
    /// Response was not parseable / had an unexpected shape.
    MalformedProviderResponse,
    /// Expected policy bundle version/digest differs from loaded bundle.
    PolicyBundleVersionMismatch,
    /// The canonical policy input was invalid (missing/invalid fields).
    InvalidPolicyInput,
    /// The policy query path returned no defined result (undefined).
    UndefinedDecision,
    /// The provider failed while evaluating the policy.
    ProviderEvaluationFailure,
}

impl OpaErrorCode {
    /// Canonical wire code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::Timeout => "timeout",
            Self::MalformedProviderResponse => "malformed_provider_response",
            Self::PolicyBundleVersionMismatch => "policy_bundle_version_mismatch",
            Self::InvalidPolicyInput => "invalid_policy_input",
            Self::UndefinedDecision => "undefined_decision",
            Self::ProviderEvaluationFailure => "provider_evaluation_failure",
        }
    }

    /// Map onto the canonical policy error code (SPEC-006).
    pub const fn policy_code(self) -> PolicyErrorCode {
        match self {
            Self::Unavailable => PolicyErrorCode::Unavailable,
            Self::Timeout => PolicyErrorCode::Timeout,
            Self::MalformedProviderResponse
            | Self::PolicyBundleVersionMismatch
            | Self::UndefinedDecision
            | Self::ProviderEvaluationFailure => PolicyErrorCode::ExternalProvider,
            Self::InvalidPolicyInput => PolicyErrorCode::Validation,
        }
    }
}

impl fmt::Display for OpaErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed OPA provider failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpaError {
    /// Stable machine code.
    pub code: OpaErrorCode,
    /// Redacted explanation (never secrets, tokens, or full payloads).
    pub message: String,
}

impl OpaError {
    /// Construct a typed provider failure.
    pub fn new(code: OpaErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// Unavailable provider.
    pub fn unavailable(detail: impl Into<String>) -> Self {
        Self::new(OpaErrorCode::Unavailable, detail)
    }

    /// Timeout.
    pub fn timeout(detail: impl Into<String>) -> Self {
        Self::new(OpaErrorCode::Timeout, detail)
    }

    /// Malformed provider response.
    pub fn malformed(detail: impl Into<String>) -> Self {
        Self::new(OpaErrorCode::MalformedProviderResponse, detail)
    }

    /// Policy bundle version mismatch.
    pub fn version_mismatch(detail: impl Into<String>) -> Self {
        Self::new(OpaErrorCode::PolicyBundleVersionMismatch, detail)
    }

    /// Invalid policy input.
    pub fn invalid_input(detail: impl Into<String>) -> Self {
        Self::new(OpaErrorCode::InvalidPolicyInput, detail)
    }

    /// Undefined decision.
    pub fn undefined(detail: impl Into<String>) -> Self {
        Self::new(OpaErrorCode::UndefinedDecision, detail)
    }

    /// Provider evaluation failure.
    pub fn evaluation(detail: impl Into<String>) -> Self {
        Self::new(OpaErrorCode::ProviderEvaluationFailure, detail)
    }

    /// Map onto the canonical `nexus-policy` error surface while
    /// preserving the typed cause in the message. Fail closed: any
    /// provider failure becomes a policy error, never an allow.
    pub fn into_policy(self) -> PolicyError {
        PolicyError::new(
            self.code.policy_code(),
            format!("opa {}: {}", self.code, self.message),
            None,
        )
    }
}

impl fmt::Display for OpaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "opa {}: {}", self.code, self.message)
    }
}

impl std::error::Error for OpaError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep008_unit_opa_error_codes_are_stable() {
        assert_eq!(OpaErrorCode::Unavailable.as_str(), "unavailable");
        assert_eq!(OpaErrorCode::Timeout.as_str(), "timeout");
        assert_eq!(
            OpaErrorCode::MalformedProviderResponse.as_str(),
            "malformed_provider_response"
        );
        assert_eq!(
            OpaErrorCode::PolicyBundleVersionMismatch.as_str(),
            "policy_bundle_version_mismatch"
        );
        assert_eq!(
            OpaErrorCode::InvalidPolicyInput.as_str(),
            "invalid_policy_input"
        );
        assert_eq!(
            OpaErrorCode::UndefinedDecision.as_str(),
            "undefined_decision"
        );
        assert_eq!(
            OpaErrorCode::ProviderEvaluationFailure.as_str(),
            "provider_evaluation_failure"
        );
    }

    #[test]
    fn ep008_unit_opa_error_maps_fail_closed_with_typed_cause() {
        for code in [
            OpaErrorCode::Unavailable,
            OpaErrorCode::Timeout,
            OpaErrorCode::MalformedProviderResponse,
            OpaErrorCode::PolicyBundleVersionMismatch,
            OpaErrorCode::InvalidPolicyInput,
            OpaErrorCode::UndefinedDecision,
            OpaErrorCode::ProviderEvaluationFailure,
        ] {
            let err = OpaError::new(code, "detail");
            let policy = err.clone().into_policy();
            assert_eq!(policy.code, code.policy_code());
            // The typed cause is preserved for audit/observability.
            assert!(policy.message.contains(code.as_str()), "{}", policy.message);
            assert!(policy.message.contains("detail"));
        }
    }
}
