//! EP-039 supply-chain port surface (SPEC-019).
//!
//! Every interface is provider-neutral and versioned. Provider
//! implementations (M2+) add internal types but cannot alter these
//! contracts. No OCI registry, package manager, signature provider,
//! scanner, or advisory feed is asserted in M1.

use crate::error::SupplyChainResult;
use crate::model::{Component, ComponentBoundary, SbomDocument};
use crate::vocabulary::{LicenseClass, LicenseReview};

/// Classify a license string into a canonical class.
pub trait LicenseClassifier {
    /// Classify an SPDX license string.
    fn classify(&self, spdx: &str) -> SupplyChainResult<LicenseClass>;

    /// Review a component's license for the exact component+version.
    /// Unknown/missing licenses DENY - never safe.
    fn review(&self, component: &Component) -> SupplyChainResult<LicenseReview>;
}

/// Default canonical classifier implementing LICENSE_POLICY.md classes.
#[derive(Debug, Default)]
pub struct LicenseClassifierPort;

impl LicenseClassifierPort {
    pub fn new() -> Self {
        Self
    }
}

impl LicenseClassifier for LicenseClassifierPort {
    fn classify(&self, spdx: &str) -> SupplyChainResult<LicenseClass> {
        let norm = spdx.trim().to_ascii_uppercase();
        match norm.as_str() {
            "MIT"
            | "APACHE-2.0"
            | "APACHE-2.0 OR MIT"
            | "BSD-2-CLAUSE"
            | "BSD-3-CLAUSE"
            | "ISC"
            | "POSTGRESQL"
            | "PSF-2.0"
            | "CDLA-PERMISSIVE-2.0"
            | "UNICODE-3.0"
            | "UNLICENSE"
            | "0BSD" => Ok(LicenseClass::Green),
            "MPL-2.0" | "LGPL-2.1" | "LGPL-2.1-ONLY" | "LGPL-3.0" | "LGPL-3.0-ONLY" => {
                Ok(LicenseClass::Review)
            }
            "GPL-2.0" | "GPL-2.0-ONLY" | "GPL-2.0-OR-LATER" | "GPL-3.0" | "GPL-3.0-ONLY"
            | "GPL-3.0-OR-LATER" | "AGPL-3.0" | "AGPL-3.0-ONLY" | "AGPL-3.0-OR-LATER" => {
                Ok(LicenseClass::Sidecar)
            }
            "COMMERCIAL" | "PROPRIETARY" | "COMMERCIAL API TERMS" => Ok(LicenseClass::External),
            _ => Err(crate::error::SupplyChainError::license_unknown(format!(
                "unknown or unclassified license: {spdx}"
            ))),
        }
    }

    fn review(&self, component: &Component) -> SupplyChainResult<LicenseReview> {
        let spdx = match &component.license_spdx {
            Some(s) if !s.trim().is_empty() => s,
            _ => {
                return Err(crate::error::SupplyChainError::license_unknown(
                    "component has no license string",
                ))
            }
        };
        match self.classify(spdx)? {
            LicenseClass::Green => Ok(LicenseReview::Approved),
            LicenseClass::Review => Ok(LicenseReview::NeedsReview),
            LicenseClass::Sidecar => Ok(LicenseReview::NeedsReview),
            LicenseClass::External => Ok(LicenseReview::NeedsReview),
            LicenseClass::Prohibited => Err(crate::error::SupplyChainError::license_denied(
                format!("prohibited license class for {spdx}"),
            )),
        }
    }
}

/// Component boundary port: declare and validate sidecar isolation.
pub trait ComponentBoundaryPort {
    /// Validate a declared boundary satisfies copyleft isolation.
    fn validate(&self, boundary: &ComponentBoundary) -> SupplyChainResult<()>;
}

/// SBOM generation contract. GENERATED != VERIFIED.
pub trait SbomGeneratorPort {
    /// Generate an SBOM document from the current dependency inventory.
    fn generate(&self, run_id: &str) -> SupplyChainResult<SbomDocument>;

    /// Verify an SBOM: completeness, currency, and required packages.
    fn verify(&self, sbom: &SbomDocument) -> SupplyChainResult<()>;
}

/// Artifact signing contract. UNSIGNED != TRUSTED.
pub trait ArtifactSigner {
    /// Sign an artifact digest.
    fn sign(&self, digest: &crate::model::ArtifactDigest) -> SupplyChainResult<Vec<u8>>;

    /// Verify a signature over an artifact digest.
    fn verify(
        &self,
        digest: &crate::model::ArtifactDigest,
        signature: &[u8],
    ) -> SupplyChainResult<()>;
}

/// Advisory monitoring contract. CRITICAL ADVISORY WITHOUT MITIGATION !=
/// RELEASABLE.
pub trait AdvisoryMonitor {
    /// Check whether any critical advisory blocks the release.
    fn release_blocked(&self) -> SupplyChainResult<bool>;
}

/// Dependency waiver contract. WAIVER PRESENT != WAIVER ACTIVE.
pub trait DependencyWaiverPort {
    /// Validate a waiver is active for the package+version.
    fn validate(&self, package: &str, version: &str, now_ts: u64) -> SupplyChainResult<()>;
}

// Re-export names used by the interface map for a stable surface.
#[allow(unused_imports)]
use crate::model::ArtifactDigest as _ArtifactDigest;
