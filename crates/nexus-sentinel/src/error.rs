//! EP-030 sentinel core error surface (SPEC-006 codes; SPEC-013 error
//! states).
//!
//! Every failure distinguishes validation, authentication,
//! authorization, policy, unavailable, timeout, conflict, rate limit,
//! external provider, verification, compensation, and internal
//! invariant failures, and preserves correlation, actor, tenant, and
//! resource references where available. Messages never contain
//! secrets, prompts, or private payloads (least privilege, SPEC-005).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical sentinel error code (SPEC-006; SPEC-013 error states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SentinelErrorCode {
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
    /// The referenced object does not exist.
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

impl SentinelErrorCode {
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

impl fmt::Display for SentinelErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical sentinel error (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentinelError {
    pub code: SentinelErrorCode,
    pub message: String,
    pub correlation: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl SentinelError {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        code: SentinelErrorCode,
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
            SentinelErrorCode::Validation,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(SentinelErrorCode::Policy, message, None, None, None, None)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(
            SentinelErrorCode::Unavailable,
            message,
            None,
            None,
            None,
            None,
        )
    }

    pub fn with_correlation(mut self, correlation: impl Into<String>) -> Self {
        self.correlation = Some(correlation.into().into_boxed_str());
        self
    }

    pub fn with_actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into().into_boxed_str());
        self
    }

    pub fn with_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(tenant.into().into_boxed_str());
        self
    }

    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into().into_boxed_str());
        self
    }
}

impl fmt::Display for SentinelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SentinelError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep030_unit_error_codes_wire_spelling_locked() {
        assert_eq!(SentinelErrorCode::Validation.as_str(), "VALIDATION");
        assert_eq!(SentinelErrorCode::Policy.as_str(), "POLICY");
        assert_eq!(
            SentinelErrorCode::ExternalProvider.as_str(),
            "EXTERNAL_PROVIDER"
        );
        let json = serde_json::to_string(&SentinelErrorCode::Verification).unwrap();
        assert_eq!(json, "\"VERIFICATION\"");
    }

    #[test]
    fn ep030_unit_error_roundtrips_serde_and_carries_context() {
        let err = SentinelError::new(
            SentinelErrorCode::Authorization,
            "denied",
            Some("corr-1".into()),
            Some("principal-1".into()),
            Some("tenant-1".into()),
            Some("res-1".into()),
        );
        let json = serde_json::to_string(&err).unwrap();
        let back: SentinelError = serde_json::from_str(&json).unwrap();
        assert_eq!(back, err);
        assert_eq!(back.code, SentinelErrorCode::Authorization);
    }
}
