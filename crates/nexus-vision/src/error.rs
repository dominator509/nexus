//! EP-023 vision typed errors (SPEC-006 codes).
//!
//! All failures preserve correlation and redact sensitive content.
//! Raw video or image data never enters error payloads or messages.

use std::fmt;

/// Canonical SPEC-006 error codes for the vision surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisionErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication/authorization rejected the request.
    Authorization,
    /// Policy rejected the request (privacy, approval class).
    Policy,
    /// The referenced camera/stream/device does not exist.
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

impl VisionErrorCode {
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

/// Typed vision error preserving correlation and resource context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisionError {
    pub code: VisionErrorCode,
    pub message: String,
    pub correlation_id: Option<Box<str>>,
    pub resource: Option<Box<str>>,
}

impl VisionError {
    pub fn new(
        code: VisionErrorCode,
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
        Self::new(VisionErrorCode::Unavailable, message, None, None)
    }

    /// Structured redacted surface. Raw video is never included.
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

impl fmt::Display for VisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for VisionError {}

/// Vocabulary rejection error (unknown canonical value).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VocabularyError(pub String);
