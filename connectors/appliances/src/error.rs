//! EP-024 appliance connector error type (SPEC-011; M3).
//!
//! Canonical SPEC-006 error codes, mirroring the media connector
//! (connectors/media/src/error.rs). Conversions from the EP-020
//! `HomeError` preserve the canonical code when the provider transport
//! already classifies it; provider transport errors that the EP-020
//! boundary reports as External are mapped honestly (never relabeled
//! as a benign appliance state).

use nexus_devices::{DevicesError, DevicesErrorCode};

/// Canonical appliance connector error code (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplianceErrorCode {
    Validation,
    Authorization,
    Policy,
    NotFound,
    Conflict,
    Unavailable,
    Timeout,
    Verification,
    Vocabulary,
    External,
    Internal,
}

impl ApplianceErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::Verification => "VERIFICATION",
            Self::Vocabulary => "VOCABULARY",
            Self::External => "EXTERNAL",
            Self::Internal => "INTERNAL",
        }
    }
}

/// Appliance connector error with a canonical code and optional
/// correlation + resource (the canonical target identity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplianceError {
    pub code: ApplianceErrorCode,
    pub message: String,
    pub correlation: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl ApplianceError {
    pub fn new(
        code: ApplianceErrorCode,
        message: impl Into<String>,
        correlation: Option<Box<str>>,
        resource: Option<Box<str>>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation,
            resource,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(ApplianceErrorCode::Unavailable, message, None, None)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(ApplianceErrorCode::Verification, message, None, None)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ApplianceErrorCode::NotFound, message, None, None)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(ApplianceErrorCode::Policy, message, None, None)
    }
}

impl std::fmt::Display for ApplianceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ApplianceError {}

/// Convert the EP-020 provider error into the appliance canonical
/// code. Codes the EP-020 boundary already classifies are preserved;
/// the rest map to External (never a benign appliance state).
impl From<nexus_home::HomeError> for ApplianceError {
    fn from(error: nexus_home::HomeError) -> Self {
        use nexus_home::HomeErrorCode;
        let code = match error.code {
            HomeErrorCode::Validation => ApplianceErrorCode::Validation,
            HomeErrorCode::Authorization => ApplianceErrorCode::Authorization,
            HomeErrorCode::Policy => ApplianceErrorCode::Policy,
            HomeErrorCode::NotFound => ApplianceErrorCode::NotFound,
            HomeErrorCode::Conflict => ApplianceErrorCode::Conflict,
            HomeErrorCode::Unavailable => ApplianceErrorCode::Unavailable,
            HomeErrorCode::Timeout => ApplianceErrorCode::Timeout,
            HomeErrorCode::Verification => ApplianceErrorCode::Verification,
            HomeErrorCode::Vocabulary => ApplianceErrorCode::Vocabulary,
            HomeErrorCode::External => ApplianceErrorCode::External,
            HomeErrorCode::Internal => ApplianceErrorCode::Internal,
        };
        Self::new(code, error.message, None, error.resource)
    }
}

/// Convert into the canonical devices error for the `ApplianceProvider`
/// port (nexus-devices). Codes map one-to-one; the message is
/// preserved so downstream policy/audit can cite the exact reason.
impl From<ApplianceError> for DevicesError {
    fn from(error: ApplianceError) -> Self {
        let code = match error.code {
            ApplianceErrorCode::Validation => DevicesErrorCode::Validation,
            ApplianceErrorCode::Authorization => DevicesErrorCode::Authorization,
            ApplianceErrorCode::Policy => DevicesErrorCode::Policy,
            ApplianceErrorCode::NotFound => DevicesErrorCode::NotFound,
            ApplianceErrorCode::Conflict => DevicesErrorCode::Conflict,
            ApplianceErrorCode::Unavailable => DevicesErrorCode::Unavailable,
            ApplianceErrorCode::Timeout => DevicesErrorCode::Timeout,
            ApplianceErrorCode::Verification => DevicesErrorCode::Verification,
            ApplianceErrorCode::Vocabulary => DevicesErrorCode::Vocabulary,
            ApplianceErrorCode::External => DevicesErrorCode::External,
            ApplianceErrorCode::Internal => DevicesErrorCode::Internal,
        };
        DevicesError::new(code, error.message, error.correlation, error.resource)
    }
}
