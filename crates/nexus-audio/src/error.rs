//! EP-022 audio typed errors (SPEC-006 codes).
//!
//! All failures preserve correlation and redact sensitive content.
//! Raw audio is never placed in error payloads or messages.

use std::fmt;

/// Canonical SPEC-006 error codes for the audio surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication/authorization rejected the request.
    Authorization,
    /// Policy rejected the request (room privacy, approval class).
    Policy,
    /// The referenced endpoint/satellite/device does not exist.
    NotFound,
    /// A state conflict (idempotency, version, lifecycle, digest).
    Conflict,
    /// The provider or transport is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// Verification failed (expected target state not observed).
    Verification,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// The external provider returned a malformed or unexpected
    /// response.
    External,
    /// Internal invariant failure.
    Internal,
}

impl AudioErrorCode {
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

/// Typed audio error preserving correlation and resource context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioError {
    pub code: AudioErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl AudioError {
    pub fn new(
        code: AudioErrorCode,
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
        Self::new(AudioErrorCode::Unavailable, message, None, None)
    }

    /// Structured redacted surface. Raw audio is never included.
    pub fn as_dict(&self) -> serde_json::Value {
        let mut value = serde_json::json!({
            "code": self.code.as_str(),
            "message": self.message,
        });
        if let Some(correlation_id) = &self.correlation_id {
            value["correlation_id"] = serde_json::Value::String(correlation_id.to_string());
        }
        if let Some(resource) = &self.resource {
            value["resource"] = serde_json::Value::String(resource.to_string());
        }
        value
    }
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AudioError {}

impl From<VocabularyError> for AudioError {
    fn from(value: VocabularyError) -> Self {
        Self::new(AudioErrorCode::Vocabulary, value.0, None, None)
    }
}

/// Vocabulary rejection error (unknown canonical value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyError(pub String);
