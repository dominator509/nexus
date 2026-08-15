//! EP-017 agent error type (SPEC-006; ADR-024).
//!
//! All failures use SPEC-006 codes, preserve correlation, redact
//! sensitive content, and distinguish validation, authentication,
//! authorization, policy, unavailable, timeout, conflict, rate limit,
//! external provider, verification, compensation, and internal
//! invariant failures.

use serde::{Deserialize, Serialize};

/// SPEC-006 error codes used by the agent orchestrator plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentsErrorCode {
    Validation,
    Authentication,
    Authorization,
    Policy,
    Unavailable,
    Timeout,
    NotFound,
    Conflict,
    RateLimited,
    ExternalProvider,
    Verification,
    Compensation,
    Internal,
}

impl AgentsErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authentication => "AUTHENTICATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::RateLimited => "RATE_LIMITED",
            Self::ExternalProvider => "EXTERNAL_PROVIDER",
            Self::Verification => "VERIFICATION",
            Self::Compensation => "COMPENSATION",
            Self::Internal => "INTERNAL",
        }
    }
}

/// Structured SPEC-006 error. The message is redacted by construction:
/// never memory content, credentials, prompts, or task payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentsError {
    pub code: AgentsErrorCode,
    pub message: String,
    pub correlation_id: Option<String>,
    pub actor: Option<String>,
    pub tenant_id: Option<String>,
    pub resource: Option<String>,
}

impl AgentsError {
    pub fn new(
        code: AgentsErrorCode,
        message: impl Into<String>,
        correlation_id: Option<String>,
        actor: Option<String>,
        tenant_id: Option<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id,
            actor,
            tenant_id,
            resource,
        }
    }

    pub fn validation(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            AgentsErrorCode::Validation,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn authorization(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            AgentsErrorCode::Authorization,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn policy(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(AgentsErrorCode::Policy, message, None, None, None, resource)
    }

    pub fn not_found(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            AgentsErrorCode::NotFound,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    pub fn unavailable(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(
            AgentsErrorCode::Unavailable,
            message,
            None,
            None,
            None,
            resource,
        )
    }

    /// Redacted message accessor for telemetry and audit (never raw
    /// task or payload content).
    pub fn redacted_message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for AgentsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AgentsError {}

/// Vocabulary parse/rejection error (fail closed on unknown values).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentsVocabularyError {
    pub enum_name: &'static str,
    pub value: String,
}

impl AgentsVocabularyError {
    pub fn unknown(enum_name: &'static str, value: &str) -> Self {
        Self {
            enum_name,
            value: value.to_string(),
        }
    }
}

impl std::fmt::Display for AgentsVocabularyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown {} value: {:?}", self.enum_name, self.value)
    }
}

impl std::error::Error for AgentsVocabularyError {}
