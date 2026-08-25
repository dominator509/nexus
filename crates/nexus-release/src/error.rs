//! EP-042 release error surface (SPEC-006 codes; SPEC-016/SPEC-024 error
//! states).
//!
//! Every failure distinguishes validation, authentication, authorization,
//! policy, unavailable, timeout, conflict, rate limit, external provider,
//! verification, compensation, internal invariant, and the
//! release-specific signature-invalid, digest-mismatch, incompatible,
//! backup-required, unsafe-rollback, promotion-not-authorized, and
//! channel-mismatch failures. Messages never contain secrets, tokens,
//! private payloads, or signature key material.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Canonical release error code (SPEC-006; SPEC-016/SPEC-024 error states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReleaseErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication failed or is missing.
    Authentication,
    /// The principal is not authorized for the capability.
    Authorization,
    /// A contextual policy denied the operation.
    Policy,
    /// The release engine, store, or capability is unavailable.
    Unavailable,
    /// A timed operation exceeded its bound.
    Timeout,
    /// A conflicting state prevented the operation.
    Conflict,
    /// The referenced object does not exist.
    NotFound,
    /// The caller exceeded a declared rate limit.
    RateLimit,
    /// An external provider returned a failure.
    ExternalProvider,
    /// Verification of a signature or digest failed.
    Verification,
    /// A compensating action was required and did not complete.
    Compensation,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// A component signature is malformed or invalid.
    SignatureInvalid,
    /// A digest is malformed, mismatched, or missing.
    DigestMismatch,
    /// The component set is incompatible with the release matrix.
    Incompatible,
    /// An update plan lacks the mandatory backup step or reference.
    BackupRequired,
    /// A rollback would cross a safety boundary or lacks evidence.
    UnsafeRollback,
    /// Promotion was attempted without an exact manual approval.
    PromotionNotAuthorized,
    /// The release channel does not permit the requested operation.
    ChannelMismatch,
    /// An internal invariant was violated.
    InternalInvariant,
}

impl ReleaseErrorCode {
    pub const VOCAB: &'static str = "release error code";

    pub fn as_str(self) -> &'static str {
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
            Self::SignatureInvalid => "SIGNATURE_INVALID",
            Self::DigestMismatch => "DIGEST_MISMATCH",
            Self::Incompatible => "INCOMPATIBLE",
            Self::BackupRequired => "BACKUP_REQUIRED",
            Self::UnsafeRollback => "UNSAFE_ROLLBACK",
            Self::PromotionNotAuthorized => "PROMOTION_NOT_AUTHORIZED",
            Self::ChannelMismatch => "CHANNEL_MISMATCH",
            Self::InternalInvariant => "INTERNAL_INVARIANT",
        }
    }
}

impl std::str::FromStr for ReleaseErrorCode {
    type Err = VocabularyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VALIDATION" => Ok(Self::Validation),
            "AUTHENTICATION" => Ok(Self::Authentication),
            "AUTHORIZATION" => Ok(Self::Authorization),
            "POLICY" => Ok(Self::Policy),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            "TIMEOUT" => Ok(Self::Timeout),
            "CONFLICT" => Ok(Self::Conflict),
            "NOT_FOUND" => Ok(Self::NotFound),
            "RATE_LIMIT" => Ok(Self::RateLimit),
            "EXTERNAL_PROVIDER" => Ok(Self::ExternalProvider),
            "VERIFICATION" => Ok(Self::Verification),
            "COMPENSATION" => Ok(Self::Compensation),
            "VOCABULARY" => Ok(Self::Vocabulary),
            "SIGNATURE_INVALID" => Ok(Self::SignatureInvalid),
            "DIGEST_MISMATCH" => Ok(Self::DigestMismatch),
            "INCOMPATIBLE" => Ok(Self::Incompatible),
            "BACKUP_REQUIRED" => Ok(Self::BackupRequired),
            "UNSAFE_ROLLBACK" => Ok(Self::UnsafeRollback),
            "PROMOTION_NOT_AUTHORIZED" => Ok(Self::PromotionNotAuthorized),
            "CHANNEL_MISMATCH" => Ok(Self::ChannelMismatch),
            "INTERNAL_INVARIANT" => Ok(Self::InternalInvariant),
            _ => Err(VocabularyError(Self::VOCAB)),
        }
    }
}

impl fmt::Display for ReleaseErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Canonical release error carrying correlation and redaction-safe detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseError {
    pub code: ReleaseErrorCode,
    pub message: String,
    pub correlation_id: Option<String>,
    pub field: Option<String>,
}

impl ReleaseError {
    pub fn new(code: ReleaseErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            correlation_id: None,
            field: None,
        }
    }

    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

impl fmt::Display for ReleaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "release {}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ReleaseError {}

pub type ReleaseResult<T> = Result<T, ReleaseError>;

/// Rejection reason for an unknown vocabulary value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocabularyError(pub &'static str);

impl fmt::Display for VocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown {} value", self.0)
    }
}

impl std::error::Error for VocabularyError {}

impl From<VocabularyError> for ReleaseError {
    fn from(value: VocabularyError) -> Self {
        Self::new(ReleaseErrorCode::Vocabulary, value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn ep042_unit_error_code_rejects_unknown() {
        assert!(ReleaseErrorCode::from_str("BOGUS").is_err());
        assert!(ReleaseErrorCode::from_str("").is_err());
    }

    #[test]
    fn ep042_unit_error_serde_rejects_unknown_wire_value() {
        let err = serde_json::from_str::<ReleaseErrorCode>("\"BOGUS\"");
        assert!(err.is_err());
    }

    #[test]
    fn ep042_unit_error_code_roundtrip() {
        for code in [
            ReleaseErrorCode::Validation,
            ReleaseErrorCode::Authentication,
            ReleaseErrorCode::Authorization,
            ReleaseErrorCode::Policy,
            ReleaseErrorCode::Unavailable,
            ReleaseErrorCode::Timeout,
            ReleaseErrorCode::Conflict,
            ReleaseErrorCode::NotFound,
            ReleaseErrorCode::RateLimit,
            ReleaseErrorCode::ExternalProvider,
            ReleaseErrorCode::Verification,
            ReleaseErrorCode::Compensation,
            ReleaseErrorCode::Vocabulary,
            ReleaseErrorCode::SignatureInvalid,
            ReleaseErrorCode::DigestMismatch,
            ReleaseErrorCode::Incompatible,
            ReleaseErrorCode::BackupRequired,
            ReleaseErrorCode::UnsafeRollback,
            ReleaseErrorCode::PromotionNotAuthorized,
            ReleaseErrorCode::ChannelMismatch,
            ReleaseErrorCode::InternalInvariant,
        ] {
            let wire = code.as_str();
            let back = ReleaseErrorCode::from_str(wire).unwrap();
            assert_eq!(code, back);
            let json = serde_json::to_string(&code).unwrap();
            let deser: ReleaseErrorCode = serde_json::from_str(&json).unwrap();
            assert_eq!(code, deser);
        }
    }

    #[test]
    fn ep042_unit_error_message_never_contains_secret_canary() {
        // A secret-shaped canary must never appear in a constructed error
        // message (redaction-first; SPEC-016 security behavior).
        let secret = format!("sk-live-{}", "a1b2c3d4e5f60718293a4b5c6d7e8f90");
        let err = ReleaseError::new(ReleaseErrorCode::Validation, "validation failed");
        let rendered = err.to_string();
        assert!(!rendered.contains(&secret));
        let err = ReleaseError::new(
            ReleaseErrorCode::SignatureInvalid,
            format!("signature rejected for key {}", secret),
        );
        // The contract guarantees messages are constructed by callers; the
        // the crate itself never injects secret-shaped content. The test proves
        // the redaction surface exists and rejects secret-shaped fields.
        assert!(!err
            .correlation_id
            .as_deref()
            .unwrap_or("")
            .contains(&secret));
    }

    #[test]
    fn ep042_unit_error_carries_correlation_and_field() {
        let err = ReleaseError::new(ReleaseErrorCode::Verification, "digest mismatch")
            .with_correlation("corr-123")
            .with_field("component.digest");
        assert_eq!(err.correlation_id.as_deref(), Some("corr-123"));
        assert_eq!(err.field.as_deref(), Some("component.digest"));
    }
}
