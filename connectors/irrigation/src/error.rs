//! EP-024 irrigation connector error type (SPEC-011; M4).
//!
//! Canonical SPEC-006 error codes, mirroring the appliance/media
//! connectors. Conversions from the EP-020 `HomeError` preserve the
//! canonical code when the provider transport already classifies it;
//! provider transport errors that the EP-020 boundary reports as
//! External are mapped honestly (never relabeled as a benign zone
//! state).

use nexus_devices::{DevicesError, DevicesErrorCode};

/// Canonical irrigation connector error code (SPEC-006).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrrigationErrorCode {
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

impl IrrigationErrorCode {
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

/// Irrigation connector error with a canonical code and optional
/// correlation + resource (the canonical zone identity).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrrigationError {
    pub code: IrrigationErrorCode,
    pub message: String,
    pub correlation: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl IrrigationError {
    pub fn new(
        code: IrrigationErrorCode,
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
        Self::new(IrrigationErrorCode::Unavailable, message, None, None)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(IrrigationErrorCode::Verification, message, None, None)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(IrrigationErrorCode::NotFound, message, None, None)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(IrrigationErrorCode::Policy, message, None, None)
    }
}

impl std::fmt::Display for IrrigationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for IrrigationError {}

/// Convert the EP-020 provider error into the irrigation canonical
/// code.
impl From<nexus_home::HomeError> for IrrigationError {
    fn from(error: nexus_home::HomeError) -> Self {
        use nexus_home::HomeErrorCode;
        let code = match error.code {
            HomeErrorCode::Validation => IrrigationErrorCode::Validation,
            HomeErrorCode::Authorization => IrrigationErrorCode::Authorization,
            HomeErrorCode::Policy => IrrigationErrorCode::Policy,
            HomeErrorCode::NotFound => IrrigationErrorCode::NotFound,
            HomeErrorCode::Conflict => IrrigationErrorCode::Conflict,
            HomeErrorCode::Unavailable => IrrigationErrorCode::Unavailable,
            HomeErrorCode::Timeout => IrrigationErrorCode::Timeout,
            HomeErrorCode::Verification => IrrigationErrorCode::Verification,
            HomeErrorCode::Vocabulary => IrrigationErrorCode::Vocabulary,
            HomeErrorCode::External => IrrigationErrorCode::External,
            HomeErrorCode::Internal => IrrigationErrorCode::Internal,
        };
        Self::new(code, error.message, None, error.resource)
    }
}

/// Convert into the canonical devices error for the `IrrigationProvider`
/// port (nexus-devices).
impl From<IrrigationError> for DevicesError {
    fn from(error: IrrigationError) -> Self {
        let code = match error.code {
            IrrigationErrorCode::Validation => DevicesErrorCode::Validation,
            IrrigationErrorCode::Authorization => DevicesErrorCode::Authorization,
            IrrigationErrorCode::Policy => DevicesErrorCode::Policy,
            IrrigationErrorCode::NotFound => DevicesErrorCode::NotFound,
            IrrigationErrorCode::Conflict => DevicesErrorCode::Conflict,
            IrrigationErrorCode::Unavailable => DevicesErrorCode::Unavailable,
            IrrigationErrorCode::Timeout => DevicesErrorCode::Timeout,
            IrrigationErrorCode::Verification => DevicesErrorCode::Verification,
            IrrigationErrorCode::Vocabulary => DevicesErrorCode::Vocabulary,
            IrrigationErrorCode::External => DevicesErrorCode::External,
            IrrigationErrorCode::Internal => DevicesErrorCode::Internal,
        };
        DevicesError::new(code, error.message, error.correlation, error.resource)
    }
}
