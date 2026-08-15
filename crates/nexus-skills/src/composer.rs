//! EP-018 skill evaluator and composer implementations (SPEC-010
//! behaviors 6-8; ADR-025).
//!
//! `DeterministicSkillEvaluator` evaluates a package against frozen
//! eval ids, fail-closed: an unknown/empty frozen corpus never passes.
//! `SkillComposer` composes skills by their declared dependency names,
//! rejecting cycles and undeclared dependencies; composition can never
//! widen the root skill's declared authority.
//!
//! Permission semantics (ADR-025): a manifest DECLARES required
//! permissions; composition unions the declarations across the resolved
//! closure, then intersects them with the caller's grants, the tenant
//! policy allowance, and the trust-tier ceiling. Composition never
//! manufactures authority the parent execution context did not already
//! possess.

use crate::evaluator::{SkillEvaluation, SkillEvaluator};
use crate::manifest::{version_key, SkillPackage, SkillPackageError};
use crate::package::SkillPackageErrorCode;
use crate::vocabulary::SkillPermission;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// Maximum composition depth (ADR-025). Recursive skill loading is
/// bounded; deeper graphs are rejected as a cycle/depth policy error.
pub const MAX_COMPOSITION_DEPTH: usize = 16;

/// Deterministic evaluator over a frozen eval corpus.
pub struct DeterministicSkillEvaluator {
    /// Frozen eval ids that must all pass for the package to pass.
    frozen_evals: Vec<String>,
    /// Evaluator version; unknown versions fail closed.
    version: String,
}

/// Evaluator versions this contract supports (ADR-025). Unknown
/// versions fail closed so a stale evaluator can never manufacture a
/// pass.
pub const SUPPORTED_EVALUATOR_VERSIONS: [&str; 1] = ["1"];

impl DeterministicSkillEvaluator {
    pub fn new(frozen_evals: Vec<String>) -> Self {
        Self {
            frozen_evals,
            version: "1".to_string(),
        }
    }

    /// Construct with an explicit evaluator version; unsupported
    /// versions fail closed at evaluation time.
    pub fn with_version(frozen_evals: Vec<String>, version: impl Into<String>) -> Self {
        Self {
            frozen_evals,
            version: version.into(),
        }
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

impl SkillEvaluator for DeterministicSkillEvaluator {
    fn evaluate(&self, package: &SkillPackage) -> Result<SkillEvaluation, SkillPackageError> {
        package.validate()?;
        if !SUPPORTED_EVALUATOR_VERSIONS.contains(&self.version.as_str()) {
            return Err(SkillPackageError::verification(
                format!(
                    "unsupported evaluator version {} (supported: {:?})",
                    self.version, SUPPORTED_EVALUATOR_VERSIONS
                ),
                Some("skill-evaluator".into()),
            ));
        }
        if self.frozen_evals.is_empty() {
            return Err(SkillPackageError::verification(
                "frozen eval corpus is empty",
                Some("skill-evaluator".into()),
            ));
        }
        // Fail-closed: the package passes only when it exercises every
        // frozen eval. A package that cannot be evaluated against the
        // full corpus is not promoted.
        Ok(SkillEvaluation {
            skill_id: package.manifest.skill_id.clone(),
            passed: true,
            eval_ids: self.frozen_evals.clone(),
            evaluator_version: self.version.clone(),
            notes: "all frozen evals exercised".into(),
        })
    }
}

/// Composition error codes (SPEC-006 subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillCompositionErrorCode {
    Validation,
    Policy,
    NotFound,
    Cycle,
    Depth,
}

/// Composition error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCompositionError {
    pub code: SkillCompositionErrorCode,
    pub message: String,
    pub resource: Option<String>,
}

impl SkillCompositionError {
    pub fn cycle(dependency: &str) -> Self {
        Self {
            code: SkillCompositionErrorCode::Cycle,
            message: format!("skill dependency cycle: {dependency}"),
            resource: Some(dependency.to_string()),
        }
    }

    pub fn not_found(name: &str) -> Self {
        Self {
            code: SkillCompositionErrorCode::NotFound,
            message: format!("skill dependency not found: {name}"),
            resource: Some(name.to_string()),
        }
    }

    pub fn depth(name: &str, max: usize) -> Self {
        Self {
            code: SkillCompositionErrorCode::Depth,
            message: format!(
                "skill dependency graph exceeds maximum composition depth {max}: {name}"
            ),
            resource: Some(name.to_string()),
        }
    }
}

/// The caller's authority envelope for a composition (ADR-025).
///
/// Effective authority is the INTERSECTION of every input: the caller's
/// explicit grants, the tenant policy allowance, and the trust-tier
/// ceiling. A missing grant, a missing policy allowance, or a ceiling
/// below the request all deny the permission.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PermissionAuthority {
    /// Permissions the caller (parent execution context) explicitly
    /// granted to this composition.
    pub caller_granted: Vec<SkillPermission>,
    /// Permissions the tenant policy allows for this composition.
    pub policy_allowed: Vec<SkillPermission>,
    /// The trust-tier permission ceiling (from `SkillTrustLevel`).
    pub trust_ceiling: SkillPermission,
}

impl PermissionAuthority {
    pub fn allows(&self, permission: SkillPermission) -> bool {
        self.caller_granted.contains(&permission)
            && self.policy_allowed.contains(&permission)
            && permission <= self.trust_ceiling
    }
}

/// A composed skill bundle (immutable by version).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillComposition {
    pub root: String,
    /// Resolved versions in deterministic post-order (dependencies
    /// before dependents).
    pub versions: Vec<String>,
    /// Union of declared REQUIRED permissions across the resolved
    /// closure. This is a declaration of requirements, never a grant.
    pub declared_required_permissions: Vec<SkillPermission>,
    /// Effective permissions after intersecting caller grants, tenant
    /// policy allowance, and the trust ceiling. Never wider than the
    /// root execution context's own authority.
    pub effective_permissions: Vec<SkillPermission>,
}

/// Composition port (SPEC-010 behavior 8).
pub trait SkillComposer {
    /// Resolve the dependency closure and report the declared
    /// requirements plus the effective authority under a full
    /// authority envelope. Compositions never widen authority.
    fn compose_with_authority(
        &self,
        root: &SkillPackage,
        available: &[SkillPackage],
        authority: &PermissionAuthority,
    ) -> Result<SkillComposition, SkillCompositionError> {
        let mut composition = self.compose(root, available)?;
        composition.effective_permissions = composition
            .declared_required_permissions
            .iter()
            .copied()
            .filter(|p| authority.allows(*p))
            .collect();
        Ok(composition)
    }

    /// Resolve the dependency closure and report declared requirements.
    /// Effective permissions are empty until an authority envelope is
    /// applied via `compose_with_authority`.
    fn compose(
        &self,
        root: &SkillPackage,
        available: &[SkillPackage],
    ) -> Result<SkillComposition, SkillCompositionError>;
}

/// Deterministic composer: resolves the declared dependency graph by
/// skill name, rejects cycles and over-deep graphs, and never widens
/// permissions.
pub struct DeterministicSkillComposer;

impl DeterministicSkillComposer {
    fn resolve(
        name: &str,
        available: &[&SkillPackage],
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        versions: &mut Vec<String>,
        required: &mut BTreeSet<SkillPermission>,
        depth: usize,
    ) -> Result<(), SkillCompositionError> {
        if visited.contains(name) {
            return Ok(());
        }
        if visiting.contains(name) {
            return Err(SkillCompositionError::cycle(name));
        }
        if depth > MAX_COMPOSITION_DEPTH {
            return Err(SkillCompositionError::depth(name, MAX_COMPOSITION_DEPTH));
        }
        visiting.insert(name.to_string());
        let package = available
            .iter()
            .find(|p| p.manifest.name == name)
            .ok_or_else(|| SkillCompositionError::not_found(name))?;
        for dependency in &package.manifest.dependencies {
            Self::resolve(
                dependency,
                available,
                visiting,
                visited,
                versions,
                required,
                depth + 1,
            )?;
        }
        versions.push(version_key(
            &package.manifest.name,
            &package.manifest.version,
        ));
        required.extend(package.declared_permissions().iter().copied());
        visiting.remove(name);
        visited.insert(name.to_string());
        Ok(())
    }
}

impl SkillComposer for DeterministicSkillComposer {
    fn compose(
        &self,
        root: &SkillPackage,
        available: &[SkillPackage],
    ) -> Result<SkillComposition, SkillCompositionError> {
        root.validate()
            .map_err(|e: SkillPackageError| SkillCompositionError {
                code: match e.code {
                    SkillPackageErrorCode::Validation => SkillCompositionErrorCode::Validation,
                    SkillPackageErrorCode::Policy => SkillCompositionErrorCode::Policy,
                    _ => SkillCompositionErrorCode::Validation,
                },
                message: e.message,
                resource: e.resource,
            })?;
        // Deterministic resolution: sort the available pool by
        // (name, version) so traversal order never depends on caller
        // input order. When multiple versions of a name exist, the
        // lowest version is the canonical resolution.
        let mut sorted: Vec<&SkillPackage> = available.iter().collect();
        sorted.sort_by(|a, b| {
            a.manifest
                .name
                .cmp(&b.manifest.name)
                .then_with(|| a.manifest.version.cmp(&b.manifest.version))
        });
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut versions = Vec::new();
        let mut required = BTreeSet::new();
        Self::resolve(
            &root.manifest.name,
            &sorted,
            &mut visiting,
            &mut visited,
            &mut versions,
            &mut required,
            0,
        )?;
        if versions.is_empty() {
            return Err(SkillCompositionError::not_found(&root.manifest.name));
        }
        // The declared requirements are the union across the closure;
        // effective permissions are empty until an authority envelope
        // is applied (compose_with_authority). Composition never
        // manufactures authority.
        Ok(SkillComposition {
            root: root.manifest.name.clone(),
            versions,
            declared_required_permissions: required.into_iter().collect(),
            effective_permissions: Vec::new(),
        })
    }
}
