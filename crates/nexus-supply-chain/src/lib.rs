//! nexus-supply-chain: provider-neutral supply-chain contracts (SPEC-019;
//! EP-039 M1).
//!
//! This crate owns the canonical license, SBOM, provenance, signature,
//! advisory, and dependency-waiver model: license classification,
//! component boundary, SBOM generation contract, artifact signing,
//! provenance attestation, advisory monitoring, and dependency waiver
//! policy.
//!
//! M1 is the contract layer only. No OCI registry, package manager,
//! signature provider, scanner, or advisory feed is asserted in M1;
//! image scanning, signing backends, SBOM producers, and advisory
//! ingestion are NOT certified until later milestones.
//!
//! Permanent invariants encoded here and proven by tests:
//! - DEPENDENCY EXISTS != LICENSE APPROVED
//! - LICENSE STRING PRESENT != LICENSE VERIFIED
//! - ALLOWLIST ENTRY != LEGAL APPROVAL FOR ALL USES
//! - UNKNOWN LICENSE != SAFE
//! - MISSING LICENSE != SAFE
//! - TRANSITIVE DEPENDENCY != OUT OF SCOPE
//! - BUILD PASSED != SBOM COMPLETE
//! - LOCKFILE EXISTS != ALL ARTIFACTS ACCOUNTED FOR
//! - SBOM GENERATED != SBOM VERIFIED
//! - PACKAGE NAME MATCH != SAME ARTIFACT
//! - IMAGE TAG != IMAGE DIGEST

pub mod error;
pub mod model;
pub mod port;
pub mod signer;
pub mod vocabulary;

pub use error::{SupplyChainError, SupplyChainErrorCode, SupplyChainResult};
pub use model::{
    Advisory, AdvisoryAffected, ArtifactDigest, Component, ComponentBoundary, ComponentIdentity,
    DependencyWaiver, ProvenanceAttestation, SbomDocument, SbomPackage, SbomVerification,
    SourceOffer,
};
pub use port::{
    AdvisoryMonitor, ArtifactSigner, ComponentBoundaryPort, DependencyWaiverPort,
    LicenseClassifier, LicenseClassifierPort, SbomGeneratorPort,
};
pub use vocabulary::{
    AdvisorySeverity, ApprovalState, IntegrationMode, LicenseClass, LicenseReview, RiskClass,
    VerificationResult, WaiverState,
};
