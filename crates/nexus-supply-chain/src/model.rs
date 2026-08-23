//! EP-039 supply-chain model (SPEC-019 canonical terms).
//!
//! The model encodes the permanent truthfulness invariants:
//! - DEPENDENCY EXISTS != LICENSE APPROVED (approval is a separate state)
//! - LICENSE STRING PRESENT != LICENSE VERIFIED (verification is separate)
//! - SBOM GENERATED != SBOM VERIFIED (verification requires completeness)
//! - PACKAGE NAME MATCH != SAME ARTIFACT (digest is the identity)
//! - IMAGE TAG != IMAGE DIGEST (tags are mutable; digests are not)
//!
//! No component becomes trusted merely because it is present in a
//! lockfile. Approval, verification, and provenance are explicit fields
//! with fail-closed defaults.

use nexus_domain::ArtifactId;
use serde::{Deserialize, Serialize};

use crate::vocabulary::{
    ApprovalState, IntegrationMode, LicenseClass, LicenseReview, RiskClass, VerificationResult,
    WaiverState,
};

/// Content-address identity for a component or artifact (SPEC-019
/// ArtifactDigest). Tags are mutable; this digest is not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactDigest {
    /// Algorithm name, canonical lowercase (sha256, sha512).
    pub algorithm: String,
    /// Hex-encoded digest value (lowercase).
    pub hex: String,
}

impl ArtifactDigest {
    /// Parse a canonical `alg:hex` digest string.
    pub fn parse(s: &str) -> Result<Self, String> {
        let (alg, hex) = s
            .split_once(':')
            .ok_or_else(|| "digest must be alg:hex".to_string())?;
        if alg.is_empty()
            || !alg
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        {
            return Err("digest algorithm must be lowercase alnum".to_string());
        }
        if hex.len() < 32 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("digest hex must be lowercase hex of at least 32 chars".to_string());
        }
        if hex.chars().any(|c| c.is_ascii_uppercase()) {
            return Err("digest hex must be lowercase".to_string());
        }
        Ok(Self {
            algorithm: alg.to_string(),
            hex: hex.to_string(),
        })
    }

    pub fn as_str(&self) -> String {
        format!("{}:{}", self.algorithm, self.hex)
    }
}

/// A versioned component identity: name + version + source + registry +
/// lockfile + digest. Identity is NOT approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentIdentity {
    /// Package name (crate/npm/pip/oci reference).
    pub name: String,
    /// Exact version string.
    pub version: String,
    /// Source/origin URL or reference.
    pub source: String,
    /// Registry (crates.io, npmjs, pypi, ghcr.io, ...).
    pub registry: String,
    /// Lockfile identity that pinned this component.
    pub lockfile: String,
    /// Content digest when available.
    pub digest: Option<ArtifactDigest>,
}

impl ComponentIdentity {
    /// Package name match alone is never "same artifact" - digest is the
    /// identity.
    pub fn same_artifact(&self, other: &Self) -> bool {
        match (&self.digest, &other.digest) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

/// A fully reviewed component record. Presence here does NOT imply
/// approval - `approval` is explicit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Component {
    pub identity: ComponentIdentity,
    /// License SPDX string as declared by upstream.
    pub license_spdx: Option<String>,
    /// Canonical license class derived from policy.
    pub license_class: Option<LicenseClass>,
    /// Explicit review outcome for this exact component+version.
    pub review: LicenseReview,
    /// Approval state - distinct from existence.
    pub approval: ApprovalState,
    /// How the component integrates.
    pub integration_mode: IntegrationMode,
    /// Risk classification.
    pub risk: RiskClass,
    /// Component owner (node/milestone that admitted it).
    pub owner: String,
    /// Verification result - distinct from presence.
    pub verification: VerificationResult,
    /// Evidence timestamp (unix seconds) and run id.
    pub evidence_ts: u64,
    pub run_id: String,
}

impl Component {
    /// A component is admissible for release only when every gate is
    /// explicitly green: approved + verified + license reviewed.
    pub fn is_releasable(&self) -> bool {
        self.approval == ApprovalState::Approved
            && self.verification == VerificationResult::Verified
            && self.review == LicenseReview::Approved
    }

    /// Fail-closed: unknown/missing license is never safe.
    pub fn license_is_safe(&self) -> bool {
        matches!(self.review, LicenseReview::Approved)
    }
}

/// Sidecar boundary declaration (SPEC-019 behavior 2). Copyleft components
/// run process-separated; the boundary records the isolation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentBoundary {
    /// Component id this boundary applies to.
    pub component: String,
    /// The process/appliance the component runs in.
    pub sidecar_process: String,
    /// Documented API contract between Nexus and the sidecar.
    pub api_contract: String,
    /// License class requiring the boundary.
    pub license_class: LicenseClass,
    /// Source-offer duty for the sidecar component.
    pub source_offer: SourceOffer,
}

/// Source-offer duty (SPEC-019 behavior 2 / license obligations).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceOffer {
    /// Where the source can be obtained.
    pub url: String,
    /// Version the offer covers.
    pub version: String,
    /// Written offer valid through (unix seconds), when applicable.
    pub valid_through: Option<u64>,
}

/// An SBOM package entry. A lockfile row is NOT an SBOM entry - SBOM
/// requires version, source, and license accounting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SbomPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    pub license_spdx: Option<String>,
    pub digest: Option<ArtifactDigest>,
    pub is_transitive: bool,
}

/// SBOM document. GENERATED != VERIFIED: verification requires every
/// required field to be present and complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SbomDocument {
    pub format: String,
    pub spec_version: String,
    pub packages: Vec<SbomPackage>,
    pub generated_at_ts: u64,
    pub run_id: String,
    /// Verification outcome - distinct from generation.
    pub verification: SbomVerification,
}

impl SbomDocument {
    /// An SBOM is complete only when every package has version, source,
    /// license, and digest where available. BUILD PASSED != SBOM COMPLETE.
    pub fn is_complete(&self) -> bool {
        if self.packages.is_empty() {
            return false;
        }
        self.packages.iter().all(|p| {
            !p.name.is_empty()
                && !p.version.is_empty()
                && !p.source.is_empty()
                && p.license_spdx.is_some()
        })
    }

    /// Transitive dependencies are ALWAYS in scope - never excluded.
    pub fn has_all_required(&self, required: &[&str]) -> bool {
        let names: std::collections::HashSet<&str> =
            self.packages.iter().map(|p| p.name.as_str()).collect();
        required.iter().all(|r| names.contains(r))
    }

    /// Reject stale SBOMs: generation must be within a window and bound to
    /// the current run id.
    pub fn is_current(&self, now_ts: u64, max_age_secs: u64, run_id: &str) -> bool {
        self.run_id == run_id && now_ts.saturating_sub(self.generated_at_ts) <= max_age_secs
    }
}

/// SBOM verification outcome - distinct from SBOM generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SbomVerification {
    /// Complete and bound to the current run.
    Verified,
    /// Generated but not verified (or verification failed).
    NotVerified,
}

/// Provenance attestation: who built what, from what source, with what
/// digest, when. PROVENANCE EXISTS != PROVENANCE VERIFIED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceAttestation {
    pub artifact: ArtifactId,
    pub builder: String,
    pub source: String,
    pub digest: ArtifactDigest,
    pub generated_at_ts: u64,
    pub run_id: String,
    /// Signature verification outcome.
    pub signature: VerificationResult,
}

impl ProvenanceAttestation {
    /// An attestation is only trustworthy when the signature is verified.
    pub fn is_trusted(&self) -> bool {
        self.signature == VerificationResult::Verified
    }
}

/// Dependency waiver (SPEC-019 behavior 8): owner, exact version, reason,
/// controls, expiry, replacement plan. Never permanent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyWaiver {
    pub package: String,
    pub version: String,
    pub owner: String,
    pub reason: String,
    pub controls: Vec<String>,
    pub expires_at_ts: u64,
    pub replacement_plan: String,
    pub state: WaiverState,
}

impl DependencyWaiver {
    /// A waiver is usable only while active and unexpired.
    pub fn is_active(&self, now_ts: u64) -> bool {
        self.state == WaiverState::Active && now_ts <= self.expires_at_ts
    }
}

/// Advisory (SPEC-019 behavior 7). Critical advisories block release
/// unless a time-bounded ADR documents mitigation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Advisory {
    pub id: String,
    pub package: String,
    pub affected_versions: Vec<String>,
    pub severity: crate::vocabulary::AdvisorySeverity,
    pub summary: String,
    /// ADR reference that documents mitigation, when present.
    pub mitigation_adr: Option<String>,
    /// Mitigation ADR expiry (unix seconds), when bounded.
    pub mitigation_expires_ts: Option<u64>,
}

/// Advisory affected component record (package + version + advisory id).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryAffected {
    pub advisory_id: String,
    pub package: String,
    pub version: String,
}

/// Build a minimal component record with fail-closed defaults.
pub fn component(
    name: &str,
    version: &str,
    license_spdx: Option<&str>,
    review: LicenseReview,
    approval: ApprovalState,
) -> Component {
    Component {
        identity: ComponentIdentity {
            name: name.to_string(),
            version: version.to_string(),
            source: format!("https://example.invalid/{name}"),
            registry: "test".to_string(),
            lockfile: "Cargo.lock".to_string(),
            digest: None,
        },
        license_spdx: license_spdx.map(str::to_string),
        license_class: None,
        review,
        approval,
        integration_mode: IntegrationMode::Embedded,
        risk: RiskClass::Low,
        owner: "ep039-test".to_string(),
        verification: VerificationResult::Unverified,
        evidence_ts: 1_700_000_000,
        run_id: "test-run".to_string(),
    }
}
