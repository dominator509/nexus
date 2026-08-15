//! EP-044 runtime typed errors (SPEC-006).

/// Typed runtime error codes. Redacted message policy: errors never
/// carry credentials, bearer tokens, or private data (SPEC-005,
/// SECURITY.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeErrorCode {
    InvalidConfiguration,
    Unavailable,
    Timeout,
    MalformedResponse,
    NotFound,
    Conflict,
    Internal,
}

/// Runtime error carrying a typed code, a redacted message, and an
/// optional correlation reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub code: RuntimeErrorCode,
    pub message: String,
    pub correlation_id: Option<String>,
}

impl RuntimeError {
    pub fn new(
        code: RuntimeErrorCode,
        message: impl Into<String>,
        correlation_id: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id,
        }
    }

    pub fn code(&self) -> RuntimeErrorCode {
        self.code
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for RuntimeError {}
