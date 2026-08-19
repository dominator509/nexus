//! EP-029 social command center error surface (SPEC-006 codes;
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

/// Canonical social error code (SPEC-006; SPEC-015 error states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SocialErrorCode {
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

impl SocialErrorCode {
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

impl fmt::Display for SocialErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical social error (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialError {
    pub code: SocialErrorCode,
    pub message: String,
    pub correlation: Option<Box<str>>,
    pub actor: Option<Box<str>>,
    pub tenant: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl SocialError {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        code: SocialErrorCode,
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
        Self::new(SocialErrorCode::Validation, message, None, None, None, None)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(SocialErrorCode::Policy, message, None, None, None, None)
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(
            SocialErrorCode::Unavailable,
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

impl fmt::Display for SocialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for SocialError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep029_unit_error_codes_wire_spelling_locked() {
        assert_eq!(SocialErrorCode::Validation.as_str(), "VALIDATION");
        assert_eq!(SocialErrorCode::Policy.as_str(), "POLICY");
        assert_eq!(
            SocialErrorCode::ExternalProvider.as_str(),
            "EXTERNAL_PROVIDER"
        );
        let json = serde_json::to_string(&SocialErrorCode::Compensation).unwrap();
        assert_eq!(json, "\"COMPENSATION\"");
    }

    #[test]
    fn ep029_unit_error_roundtrips_serde_and_carries_context() {
        let err = SocialError::new(
            SocialErrorCode::Authorization,
            "denied",
            Some("corr-1".into()),
            Some("principal-1".into()),
            Some("tenant-1".into()),
            Some("res-1".into()),
        );
        let json = serde_json::to_string(&err).unwrap();
        let back: SocialError = serde_json::from_str(&json).unwrap();
        assert_eq!(back, err);
        assert_eq!(back.code, SocialErrorCode::Authorization);
    }
}
