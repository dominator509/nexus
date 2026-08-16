//! EP-018 skill contract crate (SPEC-010 behaviors 6-8; ADR-025).
//!
//! Signed Agent Skills packages, trust levels, permissions, evals,
//! promotion, composition, and versioning. Nexus owns the skill
//! registry: skills are portable, signed, immutable by version, and
//! scanned before install; a skill can never grant itself tools or
//! secrets. Factory output must pass evals and human promotion.
//!
//! This file owns no provider behavior (M1 contract boundary);
//! deterministic registry/evaluator/composer behavior is owned by the
//! EP-018 M2 crate boundary.

pub mod bundle;
pub mod composer;
pub mod evaluator;
pub mod executor;
pub mod manifest;
pub mod package;
pub mod proposal;
pub mod registry;
pub mod signature;
pub mod store;
pub mod vocabulary;

pub use bundle::{sha256_hex, SkillBundle, SkillBundleLoader};
pub use composer::{
    DeterministicSkillComposer, DeterministicSkillEvaluator, PermissionAuthority, SkillComposer,
    SkillComposition, SkillCompositionError, SkillCompositionErrorCode, MAX_COMPOSITION_DEPTH,
    SUPPORTED_EVALUATOR_VERSIONS,
};
pub use evaluator::{SkillEvaluation, SkillEvaluator, SkillEvaluatorError};
pub use executor::{signing_message_for, SkillExecutionResult, SkillExecutor, SKILL_OUTPUT_CAP};
pub use manifest::{
    is_hex_encoded, is_valid_portable_name, is_valid_semver, SkillManifest, SkillPackageError,
    SkillPackageErrorCode,
};
pub use package::SkillPackage;
pub use proposal::SkillProposal;
pub use registry::{SkillRegistry, SkillRegistryEntry};
pub use signature::{
    decode_hex, package_signing_message, sign_ed25519, verify_ed25519, SkillSignature,
};
pub use store::{JsonFileSkillRegistryStore, SkillRegistryState, SkillRegistryStore};
pub use vocabulary::{SignatureAlgorithm, SkillPermission, SkillProposalState, SkillTrustLevel};

// Re-export canonical ids from nexus-domain / nexus-fabric so callers
// have a single import surface and locked names are never redefined.
pub use nexus_domain::{ArtifactId, CorrelationId, SkillId, TenantId};
pub use nexus_fabric::{AgentCardId, FabricError};
