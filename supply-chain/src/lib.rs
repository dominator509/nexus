//! nexus-supply-chain-policy: deterministic supply-chain policy engine
//! (SPEC-019; LICENSE_POLICY.md; EP-039 M2).
//!
//! This crate implements the production behavior and deterministic
//! invariants owned by EP-039, behind the M1 contract ports:
//!
//! - license classification behavior (GREEN only under exact policy,
//!   REVIEW requires review state, SIDECAR requires sidecar terms,
//!   EXTERNAL is never auto-approved, PROHIBITED/UNKNOWN/MISSING fail
//!   closed, fuzzy strings never bypass policy)
//! - component boundary evaluation (copyleft sidecar isolation, source
//!   offer duty, transitive dependencies always in scope, test fixtures
//!   not safe by default)
//! - SBOM verification behavior (empty/stale/missing/duplicate/lockfile
//!   mismatch/name collision fail; generated != verified)
//! - deterministic provenance evidence (source, version, registry,
//!   lockfile, digest, license, owner, policy result, run_id bound)
//! - waiver validation (absent/expired/wrong package/wrong version/wrong
//!   scope/wildcard denied; valid waiver permits only the exact bounded
//!   decision)
//! - advisory evaluation (known advisory -> risk state; unreviewed not
//!   safe; ignored requires exact justification; fixed version safe only
//!   when dependency resolves to the fixed version; unknown status not
//!   certified)
//! - redacted evidence serialization (to_redacted_json never leaks
//!   sk-/ghp_/AKIA/Bearer/credentials/private URLs)
//!
//! The engine is pure: no I/O, no provider SDK, no network. It consumes
//! M1 model types through the M1 ports and returns typed SPEC-006 errors.
//!
//! Certification boundary (honest): this milestone certifies the
//! deterministic policy behavior for the exact exercised policy surface.
//! It does NOT certify: actual third-party legal clearance, production
//! artifact SBOM completeness, container image provenance, SLSA/in-toto
//! signing, external advisory feed monitoring, GitHub dependency
//! submission, or remote synchronization.

pub mod advisory;
pub mod boundary;
pub mod evidence;
pub mod license;
pub mod provenance;
pub mod sbom;
pub mod waiver;

pub use advisory::{AdvisoryEvaluation, AdvisoryPolicy, AdvisoryPolicyConfig};
pub use boundary::{BoundaryEvaluation, BoundaryPolicy, BoundaryPolicyConfig};
pub use evidence::{evidence_boundary, redact_secret_shaped, EvidenceDocument, EvidenceRedaction};
pub use license::{LicenseEvaluation, LicensePolicy, LicensePolicyConfig};
pub use provenance::{ProvenanceEvaluation, ProvenancePolicy, ProvenancePolicyConfig};
pub use sbom::{SbomEvaluation, SbomPolicy, SbomPolicyConfig};
pub use waiver::{WaiverEvaluation, WaiverPolicy, WaiverPolicyConfig};
