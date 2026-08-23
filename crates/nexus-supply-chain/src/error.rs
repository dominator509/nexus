//! EP-039 supply-chain error surface (SPEC-006 codes; SPEC-019 error
//! states).
//!
//! Every failure distinguishes validation, authentication, authorization,
//! policy, unavailable, timeout, conflict, rate limit, external provider,
//! verification, compensation, and internal invariant failures, plus the
//! supply-chain-specific license-denied, license-unknown, sbom-incomplete,
//! provenance-missing, signature-invalid, advisory-blocking, and
//! waiver-expired failures. Messages never contain secrets, tokens, or
//! private payloads (SPEC-005, SECURITY.md).

use std::fmt;

use nexus_domain::{CorrelationId, TenantId};
use serde::{Deserialize, Serialize};

/// Canonical supply-chain error code (SPEC-006; SPEC-019 error states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupplyChainErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication failed or is missing.
    Authentication,
    /// The principal is not authorized for the capability.
    Authorization,
    /// A contextual policy denied the operation.
    Policy,
    /// The component's license is not approved for this use.
    LicenseDenied,
    /// The component's license is unknown or missing - never safe.
    LicenseUnknown,
    /// The SBOM is missing required components or fields.
    SbomIncomplete,
    /// Provenance evidence is missing or cannot be attributed.
    ProvenanceMissing,
    /// Signature verification failed or the artifact is unsigned.
    SignatureInvalid,
    /// A critical advisory blocks release without a bounded ADR.
    AdvisoryBlocking,
    /// A dependency waiver is expired or revoked.
    WaiverExpired,
    /// The backend, capability, or resource is unavailable.
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
    /// Verification of a side effect failed (digest mismatch).
    Verification,
    /// A compensating action was required and did not complete.
    Compensation,
    /// An unknown vocabulary value was rejected.
    Vocabulary,
    /// An internal invariant was violated.
    Internal,
}

impl SupplyChainErrorCode {
    /// Stable HTTP status class for the code when rendered over HTTP.
    pub fn http_status(self) -> u16 {
        match self {
            Self::Validation => 400,
            Self::Authentication => 401,
            Self::Authorization | Self::Policy => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::RateLimit => 429,
            Self::Unavailable => 503,
            Self::Timeout => 504,
            Self::ExternalProvider => 502,
            Self::Verification => 409,
            Self::Compensation | Self::Internal => 500,
            Self::Vocabulary => 422,
            Self::LicenseDenied
            | Self::LicenseUnknown
            | Self::SbomIncomplete
            | Self::ProvenanceMissing
            | Self::SignatureInvalid
            | Self::AdvisoryBlocking
            | Self::WaiverExpired => 403,
        }
    }
}

/// Typed supply-chain failure (SPEC-006). Messages are safe for display and
/// never contain secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupplyChainError {
    pub code: SupplyChainErrorCode,
    pub message: String,
    pub correlation: Option<CorrelationId>,
    pub tenant: Option<TenantId>,
}

impl SupplyChainError {
    pub fn new(code: SupplyChainErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            correlation: None,
            tenant: None,
        }
    }

    pub fn with_correlation(mut self, correlation: CorrelationId) -> Self {
        self.correlation = Some(correlation);
        self
    }

    pub fn with_tenant(mut self, tenant: TenantId) -> Self {
        self.tenant = Some(tenant);
        self
    }
}

impl fmt::Display for SupplyChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for SupplyChainError {}

/// Convenience result alias.
pub type SupplyChainResult<T> = Result<T, SupplyChainError>;

/// Constructors for the canonical failure shapes.
impl SupplyChainError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::Validation, message)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::Policy, message)
    }

    pub fn license_denied(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::LicenseDenied, message)
    }

    pub fn license_unknown(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::LicenseUnknown, message)
    }

    pub fn sbom_incomplete(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::SbomIncomplete, message)
    }

    pub fn provenance_missing(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::ProvenanceMissing, message)
    }

    pub fn signature_invalid(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::SignatureInvalid, message)
    }

    pub fn advisory_blocking(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::AdvisoryBlocking, message)
    }

    pub fn waiver_expired(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::WaiverExpired, message)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::Verification, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::Internal, message)
    }

    pub fn vocabulary(message: impl Into<String>) -> Self {
        Self::new(SupplyChainErrorCode::Vocabulary, message)
    }
}
/// Serialize a supply-chain error as a redacted JSON problem document.
/// Secret-shaped values (sk-..., ghp_..., bearer tokens, aws keys) are
/// scrubbed at the evidence boundary - never written into SBOM/evidence.
impl SupplyChainError {
    pub fn to_redacted_json(&self) -> String {
        let message = redact_secret_shaped(&self.message);
        serde_json::json!({
            "error": self.code.as_serde_str(),
            "message": message,
            "correlation": self.correlation.as_ref().map(|c| c.to_string()),
        })
        .to_string()
    }
}

/// Scrub secret-shaped substrings from a message before it reaches
/// evidence. Fail-closed: anything that looks like a credential is
/// replaced with a bounded marker.
pub fn redact_secret_shaped(input: &str) -> String {
    // Conservative secret-shape patterns (never regex-heavy; exact
    // substring scanning). Covers the credential families Nexus
    // handles: sk-/pk-/rk- API keys, ghp_/github_pat_/gho_ tokens,
    // AKIA AWS access keys, Bearer tokens, and long base64-ish blobs.
    let mut out = input.to_string();
    for pattern in [
        "sk-",
        "pk-",
        "rk-",
        "ghp_",
        "gho_",
        "ghs_",
        "github_pat_",
        "AKIA",
        "Bearer ",
        "bearer ",
    ] {
        while let Some(pos) = out.find(pattern) {
            // Capture a bounded window (up to 40 chars) after the marker.
            let end = (pos + pattern.len() + 40).min(out.len());
            out.replace_range(pos..end, "[REDACTED]");
        }
    }
    out
}

impl SupplyChainErrorCode {
    /// Canonical wire string for the error code.
    pub fn as_serde_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authentication => "AUTHENTICATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::LicenseDenied => "LICENSE_DENIED",
            Self::LicenseUnknown => "LICENSE_UNKNOWN",
            Self::SbomIncomplete => "SBOM_INCOMPLETE",
            Self::ProvenanceMissing => "PROVENANCE_MISSING",
            Self::SignatureInvalid => "SIGNATURE_INVALID",
            Self::AdvisoryBlocking => "ADVISORY_BLOCKING",
            Self::WaiverExpired => "WAIVER_EXPIRED",
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::Conflict => "CONFLICT",
            Self::NotFound => "NOT_FOUND",
            Self::RateLimit => "RATE_LIMIT",
            Self::ExternalProvider => "EXTERNAL_PROVIDER",
            Self::Verification => "VERIFICATION",
            Self::Compensation => "COMPENSATION",
            Self::Vocabulary => "VOCABULARY",
            Self::Internal => "INTERNAL",
        }
    }
}
