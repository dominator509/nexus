//! EP-028 Hydra business-control error surface (SPEC-006 codes;
//! SPEC-015 error states).
//!
//! Every failure distinguishes validation, authentication,
//! authorization, policy, unavailable, timeout, conflict, rate limit,
//! external provider, verification, compensation, and internal
//! invariant failures, and preserves correlation, actor, tenant, and
//! resource references where available. Messages never contain
//! secrets, prompts, or private payloads (least privilege, SPEC-005).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical Hydra error code (SPEC-006; SPEC-015 error states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HydraErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication failed or is missing.
    Authentication,
    /// The principal is not authorized for the capability.
    Authorization,
    /// A contextual policy denied the request (scope, approval class).
    Policy,
    /// The provider, capability, or resource is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// A conflicting state prevented the operation (idempotency,
    /// lifecycle).
    Conflict,
    /// The referenced Hydra object does not exist.
    NotFound,
    /// The caller exceeded a declared rate limit.
    RateLimit,
    /// An external provider returned a failure.
    ExternalProvider,
    /// Verification of a side effect failed (exact-target mismatch).
    Verification,
    /// A compensating action was required and did not complete.
    Compensation,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// An internal invariant was violated.
    Internal,
}

impl HydraErrorCode {
    /// Canonical wire string for this class.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authentication => "AUTHENTICATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::Conflict => "CONFLICT",
            Self::NotFound => "NOT_FOUND",
            Self::RateLimit => "RATE_LIMIT",
            Self::ExternalProvider => "EXTERNAL_PROVIDER",
            Self::Verification => "VERIFICATION",
            Self::Compensation => "COMPENSATION",
            Self::Vocabulary => "VOCABULARY",
            Self::Internal => "INTERNAL",
        }
    }
}

impl fmt::Display for HydraErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical Hydra error (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydraError {
    /// Canonical error code.
    pub code: HydraErrorCode,
    /// Human-readable, redacted description. Never contains secrets,
    /// prompts, or private payloads.
    pub message: String,
    /// Request correlation identifier when available.
    pub correlation: Option<Box<str>>,
    /// Actor principal identifier when available.
    pub actor: Option<Box<str>>,
    /// Tenant identifier when available.
    pub tenant: Option<Box<str>>,
    /// Resource identifier when available.
    pub resource: Option<Box<str>>,
}

impl HydraError {
    /// Construct a typed error preserving optional context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        code: HydraErrorCode,
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

    /// Construct a validation error.
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(HydraErrorCode::Validation, message, None, None, None, None)
    }

    /// Construct a policy error.
    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(HydraErrorCode::Policy, message, None, None, None, None)
    }

    /// True when the failure class is fail-closed (every error class is
    /// fail-closed by construction; a successful outcome is never an
    /// error).
    pub fn is_fail_closed(&self) -> bool {
        true
    }
}

impl fmt::Display for HydraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for HydraError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep028_unit_error_roundtrips_serde() {
        let err = HydraError::new(
            HydraErrorCode::Policy,
            "scope denied",
            Some("corr-1".into()),
            Some("actor-1".into()),
            Some("tenant-1".into()),
            Some("res-1".into()),
        );
        let json = serde_json::to_string(&err).unwrap();
        let back: HydraError = serde_json::from_str(&json).unwrap();
        assert_eq!(back, err);
        assert_eq!(back.code, HydraErrorCode::Policy);
    }

    #[test]
    fn ep028_unit_error_code_wire_spelling_locked() {
        assert_eq!(HydraErrorCode::Validation.as_str(), "VALIDATION");
        assert_eq!(HydraErrorCode::Compensation.as_str(), "COMPENSATION");
        assert_eq!(
            HydraErrorCode::ExternalProvider.as_str(),
            "EXTERNAL_PROVIDER"
        );
        let json = serde_json::to_string(&HydraErrorCode::Compensation).unwrap();
        assert_eq!(json, "\"COMPENSATION\"");
        // Unknown code must fail closed at the wire.
        let bad: Result<HydraErrorCode, _> = serde_json::from_str("\"FABRICATED\"");
        assert!(bad.is_err());
    }
}
