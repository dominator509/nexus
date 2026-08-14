//! Canonical typed errors for the capability and connector domain
//! (SPEC-006).
//!
//! Every failure distinguishes validation, authentication,
//! authorization, policy, unavailable, timeout, conflict, rate limit,
//! external provider, verification, compensation, and internal
//! invariant failures, and preserves request, correlation, actor,
//! tenant, and resource references where available. Messages never
//! contain secrets, prompts, or private payloads.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical error class (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityErrorCode {
    /// Input failed validation against the canonical contract.
    Validation,
    /// Authentication failed or is missing.
    Authentication,
    /// The principal is not authorized for the capability.
    Authorization,
    /// A contextual policy denied the invocation.
    Policy,
    /// The capability or connector is unavailable.
    Unavailable,
    /// The request timed out.
    Timeout,
    /// A conflicting state prevented the operation.
    Conflict,
    /// The referenced capability or resource was not found.
    NotFound,
    /// The caller exceeded a declared rate limit.
    RateLimit,
    /// An external provider returned a failure.
    ExternalProvider,
    /// Verification of a side effect failed.
    Verification,
    /// A compensating action was required and did not complete.
    Compensation,
    /// An internal invariant was violated.
    Internal,
}

impl CapabilityErrorCode {
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
            Self::Internal => "INTERNAL",
        }
    }
}

impl fmt::Display for CapabilityErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical capability/connector error (SPEC-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityError {
    /// Canonical error class.
    pub code: CapabilityErrorCode,
    /// Human-readable, redacted description. Never contains secrets,
    /// prompts, or private payloads.
    pub message: String,
    /// Request correlation identifier when available.
    pub correlation: Option<Box<str>>,
    /// Actor principal identifier when available.
    pub actor: Option<Box<str>>,
    /// Tenant identifier when available.
    pub tenant: Option<Box<str>>,
    /// Resource/capability identifier when available.
    pub resource: Option<Box<str>>,
}

impl CapabilityError {
    /// Construct a typed error preserving optional context.
    pub fn new(
        code: CapabilityErrorCode,
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
        Self::new(
            CapabilityErrorCode::Validation,
            message,
            None,
            None,
            None,
            None,
        )
    }

    /// True when the failure class is fail-closed (every class except a
    /// successful outcome is fail-closed by construction).
    pub fn is_fail_closed(&self) -> bool {
        true
    }
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CapabilityError {}
