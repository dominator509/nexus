//! EP-040 testing/hardening/chaos error surface (SPEC-006 codes; SPEC-008
//! error states; TESTING.md).
//!
//! Every failure distinguishes validation, authentication, authorization,
//! policy, unavailable, timeout, conflict, rate limit, external provider,
//! verification, compensation, and internal invariant failures, plus the
//! testing-specific zero-test-collection, required-skip, required-ignore,
//! vacuous-gate, resource-residue, blast-radius-exceeded,
//! rollback-unavailable, flake-unresolved, mock-only-certification, and
//! missing-evidence failures. Messages never contain secrets, tokens, or
//! private payloads (SPEC-005, SECURITY.md).

use std::fmt;

use nexus_domain::{CorrelationId, TenantId};
use serde::{Deserialize, Serialize};

/// Canonical testing/hardening/chaos error code (SPEC-006; SPEC-008 error
/// states).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TestingErrorCode {
    /// Request or contract validation failed.
    Validation,
    /// Authentication failed or is missing.
    Authentication,
    /// The principal is not authorized for the capability.
    Authorization,
    /// A contextual policy denied the operation.
    Policy,
    /// A required test collection is empty - zero tests is never green.
    ZeroTestCollection,
    /// A required test was skipped; SKIPPED TEST != PASSED TEST.
    RequiredTestSkipped,
    /// A required test was ignored; IGNORED TEST != PASSED TEST.
    RequiredTestIgnored,
    /// The gate proof is vacuous (artifact-only, no-op branch, phantom path).
    VacuousGate,
    /// Resource cleanup was attempted but residue remains verified.
    ResourceResidue,
    /// A chaos scenario exceeded its declared blast radius.
    BlastRadiusExceeded,
    /// A chaos scenario lacks a rollback path or safety precondition.
    RollbackUnavailable,
    /// A flake was retried green without a root cause fix.
    FlakeUnresolved,
    /// Mock/fixture-only evidence cannot certify a production path.
    MockOnlyCertification,
    /// Required certification evidence is missing.
    MissingEvidence,
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

impl TestingErrorCode {
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
            Self::ZeroTestCollection
            | Self::RequiredTestSkipped
            | Self::RequiredTestIgnored
            | Self::VacuousGate
            | Self::ResourceResidue
            | Self::BlastRadiusExceeded
            | Self::RollbackUnavailable
            | Self::FlakeUnresolved
            | Self::MockOnlyCertification
            | Self::MissingEvidence => 422,
        }
    }
}

/// Typed testing/hardening/chaos failure (SPEC-006). Messages are safe for
/// display and never contain secrets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestingError {
    pub code: TestingErrorCode,
    pub message: String,
    pub correlation: Option<CorrelationId>,
    pub tenant: Option<TenantId>,
}

impl TestingError {
    pub fn new(code: TestingErrorCode, message: impl Into<String>) -> Self {
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

impl fmt::Display for TestingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for TestingError {}

/// Convenience result alias.
pub type TestingResult<T> = Result<T, TestingError>;

/// Constructors for the canonical failure shapes.
impl TestingError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::Validation, message)
    }

    pub fn policy(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::Policy, message)
    }

    pub fn zero_test_collection(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::ZeroTestCollection, message)
    }

    pub fn required_skip(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::RequiredTestSkipped, message)
    }

    pub fn required_ignore(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::RequiredTestIgnored, message)
    }

    pub fn vacuous_gate(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::VacuousGate, message)
    }

    pub fn resource_residue(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::ResourceResidue, message)
    }

    pub fn blast_radius_exceeded(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::BlastRadiusExceeded, message)
    }

    pub fn rollback_unavailable(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::RollbackUnavailable, message)
    }

    pub fn flake_unresolved(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::FlakeUnresolved, message)
    }

    pub fn mock_only(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::MockOnlyCertification, message)
    }

    pub fn missing_evidence(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::MissingEvidence, message)
    }

    pub fn verification(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::Verification, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::Internal, message)
    }

    pub fn vocabulary(message: impl Into<String>) -> Self {
        Self::new(TestingErrorCode::Vocabulary, message)
    }
}

/// Serialize a testing error as a redacted JSON problem document.
/// Secret-shaped values (sk-..., ghp_..., bearer tokens, aws keys) are
/// scrubbed at the evidence boundary - never written into test evidence.
impl TestingError {
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

impl TestingErrorCode {
    /// Canonical wire string for the error code.
    pub fn as_serde_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authentication => "AUTHENTICATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::ZeroTestCollection => "ZERO_TEST_COLLECTION",
            Self::RequiredTestSkipped => "REQUIRED_TEST_SKIPPED",
            Self::RequiredTestIgnored => "REQUIRED_TEST_IGNORED",
            Self::VacuousGate => "VACUOUS_GATE",
            Self::ResourceResidue => "RESOURCE_RESIDUE",
            Self::BlastRadiusExceeded => "BLAST_RADIUS_EXCEEDED",
            Self::RollbackUnavailable => "ROLLBACK_UNAVAILABLE",
            Self::FlakeUnresolved => "FLAKE_UNRESOLVED",
            Self::MockOnlyCertification => "MOCK_ONLY_CERTIFICATION",
            Self::MissingEvidence => "MISSING_EVIDENCE",
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
