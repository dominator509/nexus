//! EP-024 M5 vacuum connector error type (SPEC-011; M5).
//!
//! Canonical SPEC-006 error codes, mirroring the appliance/media/
//! irrigation connectors. Conversions from the EP-020 `HomeError`
//! preserve the canonical code when the provider transport already
//! classifies it; provider transport errors that the EP-020 boundary
//! reports as External are mapped honestly (never relabeled as a
//! benign vacuum state).

use nexus_devices::{DevicesError, DevicesErrorCode};

/// Canonical vacuum connector error code (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VacuumErrorCode {
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

impl VacuumErrorCode {
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

/// Vacuum connector error with a canonical code and optional
/// correlation + resource (the canonical vacuum identity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VacuumError {
    pub code: VacuumErrorCode,
    pub message: String,
    pub correlation: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl VacuumError {
    pub fn new(
        code: VacuumErrorCode,
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
        Self::new(VacuumErrorCode::Unavailable, message, None, None)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(VacuumErrorCode::Verification, message, None, None)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(VacuumErrorCode::NotFound, message, None, None)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(VacuumErrorCode::Policy, message, None, None)
    }
}

impl std::fmt::Display for VacuumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for VacuumError {}

/// Convert the EP-020 provider error into the vacuum canonical code.
impl From<nexus_home::HomeError> for VacuumError {
    fn from(error: nexus_home::HomeError) -> Self {
        use nexus_home::HomeErrorCode;
        let code = match error.code {
            HomeErrorCode::Validation => VacuumErrorCode::Validation,
            HomeErrorCode::Authorization => VacuumErrorCode::Authorization,
            HomeErrorCode::Policy => VacuumErrorCode::Policy,
            HomeErrorCode::NotFound => VacuumErrorCode::NotFound,
            HomeErrorCode::Conflict => VacuumErrorCode::Conflict,
            HomeErrorCode::Unavailable => VacuumErrorCode::Unavailable,
            HomeErrorCode::Timeout => VacuumErrorCode::Timeout,
            HomeErrorCode::Verification => VacuumErrorCode::Verification,
            HomeErrorCode::Vocabulary => VacuumErrorCode::Vocabulary,
            HomeErrorCode::External => VacuumErrorCode::External,
            HomeErrorCode::Internal => VacuumErrorCode::Internal,
        };
        Self::new(code, error.message, None, error.resource)
    }
}

/// Convert into the canonical devices error for the `VacuumProvider`
/// port (nexus-devices).
impl From<VacuumError> for DevicesError {
    fn from(error: VacuumError) -> Self {
        let code = match error.code {
            VacuumErrorCode::Validation => DevicesErrorCode::Validation,
            VacuumErrorCode::Authorization => DevicesErrorCode::Authorization,
            VacuumErrorCode::Policy => DevicesErrorCode::Policy,
            VacuumErrorCode::NotFound => DevicesErrorCode::NotFound,
            VacuumErrorCode::Conflict => DevicesErrorCode::Conflict,
            VacuumErrorCode::Unavailable => DevicesErrorCode::Unavailable,
            VacuumErrorCode::Timeout => DevicesErrorCode::Timeout,
            VacuumErrorCode::Verification => DevicesErrorCode::Verification,
            VacuumErrorCode::Vocabulary => DevicesErrorCode::Vocabulary,
            VacuumErrorCode::External => DevicesErrorCode::External,
            VacuumErrorCode::Internal => DevicesErrorCode::Internal,
        };
        DevicesError::new(code, error.message, error.correlation, error.resource)
    }
}
