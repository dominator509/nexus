//! EP-024 media typed errors (SPEC-006 codes).

use std::fmt;

/// Canonical SPEC-006 error codes for the media surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaErrorCode {
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

impl MediaErrorCode {
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

/// Typed media error preserving correlation and resource context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaError {
    pub code: MediaErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl MediaError {
    pub fn new(
        code: MediaErrorCode,
        message: impl Into<String>,
        correlation_id: Option<Box<str>>,
        resource: Option<Box<str>>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id,
            resource,
        }
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(MediaErrorCode::Unavailable, message, None, None)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(MediaErrorCode::Verification, message, None, None)
    }
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.code.as_str(), self.message)
    }
}

impl std::error::Error for MediaError {}

impl From<MediaError> for nexus_devices::DevicesError {
    fn from(error: MediaError) -> Self {
        let code = match error.code {
            MediaErrorCode::Validation => nexus_devices::DevicesErrorCode::Validation,
            MediaErrorCode::Authorization => nexus_devices::DevicesErrorCode::Authorization,
            MediaErrorCode::Policy => nexus_devices::DevicesErrorCode::Policy,
            MediaErrorCode::NotFound => nexus_devices::DevicesErrorCode::NotFound,
            MediaErrorCode::Conflict => nexus_devices::DevicesErrorCode::Conflict,
            MediaErrorCode::Unavailable => nexus_devices::DevicesErrorCode::Unavailable,
            MediaErrorCode::Timeout => nexus_devices::DevicesErrorCode::Timeout,
            MediaErrorCode::Verification => nexus_devices::DevicesErrorCode::Verification,
            MediaErrorCode::Vocabulary => nexus_devices::DevicesErrorCode::Vocabulary,
            MediaErrorCode::External => nexus_devices::DevicesErrorCode::External,
            MediaErrorCode::Internal => nexus_devices::DevicesErrorCode::Internal,
        };
        nexus_devices::DevicesError::new(code, error.message, error.correlation_id, error.resource)
    }
}
