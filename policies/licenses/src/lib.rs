//! nexus-supply-chain-policy-io: real dependency and transport
//! integration for EP-039 M3 (SPEC-019; LICENSE_POLICY.md).
//!
//! This crate is the transport boundary that connects the Nexus
//! supply-chain contract (M1) and the deterministic policy engine (M2)
//! to REAL dependency data:
//!
//! - loads the checked-in policies/licenses/ files (allowlist,
//!   classes, sidecar obligations, waivers) with deny-unknown
//!   semantics
//! - parses the REAL workspace Cargo.lock (every locked package,
//!   including transitives - TRANSITIVE DEPENDENCY != OUT OF SCOPE)
//! - resolves each package's REAL license declaration from the real
//!   cargo registry cache and workspace manifests
//! - classifies SPDX expressions at the boundary (OR/AND/WITH/parens/
//!   slash; unknown branches fail closed; expressions containing
//!   copyleft are never auto-approved merely because MIT appears)
//! - evaluates the real inventory through the M2 engine
//! - emits redacted deterministic evidence
//!
//! Certification boundary (honest): this milestone certifies real
//! dependency + license transport integration for the exact exercised
//! local surface (real Cargo.lock, real registry cache, checked-in
//! policy files). It does NOT certify: actual third-party legal
//! clearance, production artifact SBOM completeness, container image
//! provenance, SLSA/in-toto signing, external advisory feed monitoring,
//! GitHub dependency submission, or remote synchronization.

pub mod evidence;
pub mod inventory;
pub mod lockfile;
pub mod policy_files;
pub mod resolve;
pub mod spdx;

pub use evidence::{assert_redacted, inventory_evidence, redact};
pub use inventory::{evaluate_inventory, evaluate_package, InventoryReport, PackageEvaluation};
pub use lockfile::{read_lockfile, LockedPackage, Lockfile};
pub use policy_files::{load_policy_files, PolicyFiles};
pub use resolve::{resolve_license, ResolvedLicense};
pub use spdx::{classify_spdx, SpdxClassification};
