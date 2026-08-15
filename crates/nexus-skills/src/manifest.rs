//! EP-018 skill package and manifest (SPEC-010 behaviors 6-8;
//! ADR-025).
//!
//! A `SkillManifest` is the Nexus metadata governing a skill's
//! permissions, network rules, license, provenance, and trust tier.
//! A `SkillPackage` is the immutable, signed, versioned bundle Nexus
//! registers. Skills are portable; a skill can never grant itself
//! tools or secrets (permissions are declared, never self-granted).

use crate::signature::SkillSignature;
use crate::vocabulary::{SkillPermission, SkillTrustLevel};
use nexus_domain::{ArtifactId, SkillId, TenantId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Canonical portable skill name: `namespace/skill-name` (lowercase
/// alphanumeric, '-', '_'; both parts non-empty; no '@', ':', '/').
pub fn is_valid_portable_name(name: &str) -> bool {
    let Some((namespace, skill)) = name.split_once('/') else {
        return false;
    };
    if namespace.is_empty() || skill.is_empty() {
        return false;
    }
    let valid_part = |part: &str| {
        !part.is_empty()
            && part
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
    };
    valid_part(namespace) && valid_part(skill)
}

/// Strict `major.minor.patch` semantic version: numeric parts, no
/// leading zeros beyond a single `0` (ADR-025).
pub fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 3 {
        return false;
    }
    parts.iter().all(|part| {
        !part.is_empty()
            && part.bytes().all(|b| b.is_ascii_digit())
            && !(part.len() > 1 && part.starts_with('0'))
    })
}

/// Hex-encoded byte string with even length (ADR-025 structural check).
pub fn is_hex_encoded(value: &str) -> bool {
    !value.is_empty()
        && value.len().is_multiple_of(2)
        && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// SPEC-006-style error for the skills plane. Messages are redacted by
/// construction; unknown vocabulary values are rejected at parse time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillPackageError {
    pub code: SkillPackageErrorCode,
    pub message: String,
    pub resource: Option<String>,
}

/// SPEC-006 error codes used by the skills plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SkillPackageErrorCode {
    Validation,
    Authorization,
    Policy,
    NotFound,
    Conflict,
    Unavailable,
    Verification,
    Vocabulary,
}

impl SkillPackageErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validation => "VALIDATION",
            Self::Authorization => "AUTHORIZATION",
            Self::Policy => "POLICY",
            Self::NotFound => "NOT_FOUND",
            Self::Conflict => "CONFLICT",
            Self::Unavailable => "UNAVAILABLE",
            Self::Verification => "VERIFICATION",
            Self::Vocabulary => "VOCABULARY",
        }
    }
}

impl SkillPackageError {
    pub fn new(
        code: SkillPackageErrorCode,
        message: impl Into<String>,
        resource: Option<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            resource,
        }
    }

    pub fn validation(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(SkillPackageErrorCode::Validation, message, resource)
    }

    pub fn policy(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(SkillPackageErrorCode::Policy, message, resource)
    }

    pub fn not_found(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(SkillPackageErrorCode::NotFound, message, resource)
    }

    pub fn conflict(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(SkillPackageErrorCode::Conflict, message, resource)
    }

    pub fn verification(message: impl Into<String>, resource: Option<String>) -> Self {
        Self::new(SkillPackageErrorCode::Verification, message, resource)
    }

    pub fn vocabulary(kind: &str, value: &str) -> Self {
        Self::new(
            SkillPackageErrorCode::Vocabulary,
            format!("unknown {kind} value: {value}"),
            Some(kind.to_string()),
        )
    }
}

impl fmt::Display for SkillPackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for SkillPackageError {}

/// Nexus metadata governing a skill package (SPEC-010 behavior 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillManifest {
    pub skill_id: SkillId,
    pub tenant_id: TenantId,
    /// Portable, versioned name (`namespace/skill-name`).
    pub name: String,
    /// Semantic version, immutable per package.
    pub version: String,
    pub description: String,
    /// Declared permissions (least privilege; never self-granted).
    pub permissions: Vec<SkillPermission>,
    /// Declared dependency skill names (composition resolves these by
    /// version key; cycles are rejected).
    pub dependencies: Vec<String>,
    /// Network rules (e.g. deny-all by default).
    pub network_rules: Vec<String>,
    pub license: String,
    /// Provenance: the artifact id of the canonical source bundle.
    pub provenance: ArtifactId,
    pub trust_level: SkillTrustLevel,
    /// The signature covering this manifest.
    pub signature: SkillSignature,
}

impl SkillManifest {
    pub fn validate(&self) -> Result<(), SkillPackageError> {
        if self.name.is_empty() {
            return Err(SkillPackageError::validation(
                "skill name must not be empty",
                Some("skill-manifest".into()),
            ));
        }
        // Portable versioned name is `namespace/skill-name`; both parts
        // non-empty, no whitespace, no '@' ':' or extra '/'.
        if !is_valid_portable_name(&self.name) {
            return Err(SkillPackageError::validation(
                "skill name must be namespace/skill-name (lowercase alphanumeric, '-', '_')",
                Some("skill-manifest".into()),
            ));
        }
        if !is_valid_semver(&self.version) {
            return Err(SkillPackageError::validation(
                "skill version must be semantic major.minor.patch",
                Some("skill-manifest".into()),
            ));
        }
        if self.license.is_empty() {
            return Err(SkillPackageError::validation(
                "skill license must not be empty",
                Some("skill-manifest".into()),
            ));
        }
        // Duplicate declared permissions are a validation error: the
        // declaration set is canonical and must not be ambiguous.
        let mut seen_permissions = BTreeSet::new();
        for permission in &self.permissions {
            if !seen_permissions.insert(*permission) {
                return Err(SkillPackageError::validation(
                    "duplicate skill permission declaration",
                    Some("skill-manifest".into()),
                ));
            }
        }
        // Duplicate dependencies are rejected (canonical policy), and a
        // skill cannot depend on itself.
        let mut seen_dependencies = BTreeSet::new();
        for dependency in &self.dependencies {
            if dependency == &self.name {
                return Err(SkillPackageError::validation(
                    "skill must not depend on itself",
                    Some("skill-manifest".into()),
                ));
            }
            if !seen_dependencies.insert(dependency.clone()) {
                return Err(SkillPackageError::validation(
                    "duplicate skill dependency",
                    Some("skill-manifest".into()),
                ));
            }
        }
        // Duplicate network rules are a validation error.
        let mut seen_network = BTreeSet::new();
        for rule in &self.network_rules {
            if !seen_network.insert(rule.clone()) {
                return Err(SkillPackageError::validation(
                    "duplicate network rule declaration",
                    Some("skill-manifest".into()),
                ));
            }
        }
        if self.trust_level == SkillTrustLevel::System && !self.permissions.is_empty() {
            // SYSTEM skills are built-in; a SYSTEM-trusted skill that
            // declares permissions must be reviewed (never implicit).
            return Err(SkillPackageError::policy(
                "SYSTEM skill must not declare permissions implicitly",
                Some("skill-manifest".into()),
            ));
        }
        self.signature.validate()?;
        Ok(())
    }

    /// Declared network access REQUESTS (ADR-025). These are
    /// declarations only: they describe what the skill would like, they
    /// do not open the network. The execution/sandbox policy enforces
    /// them; a manifest can never grant itself network access.
    pub fn requested_network_rules(&self) -> &[String] {
        &self.network_rules
    }
}

/// An immutable, signed, versioned skill package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillPackage {
    pub manifest: SkillManifest,
    /// Immutable content bundle (the payload Nexus scans before
    /// install). Never executable by itself.
    pub content_hash: String,
    pub created_at_epoch_ms: u64,
}

impl SkillPackage {
    /// Canonical invariants. Fails closed on empty names, versions, or
    /// a malformed content hash (64-char hex).
    pub fn validate(&self) -> Result<(), SkillPackageError> {
        self.manifest.validate()?;
        if self.content_hash.is_empty() || self.content_hash.len() != 64 {
            return Err(SkillPackageError::validation(
                "skill content_hash must be a 64-char sha256 hex",
                Some("skill-package".into()),
            ));
        }
        Ok(())
    }

    /// A skill can never grant itself tools or secrets: the declared
    /// permissions are the ONLY authority this package carries.
    pub fn declared_permissions(&self) -> &[SkillPermission] {
        &self.manifest.permissions
    }

    /// Canonical package identity (ADR-025). Immutable by version:
    /// `name@version:content_hash`. The same skill id + version +
    /// content always produce the same identity; changed content under
    /// the same name/version is a conflict, never a silent mutation.
    /// There is no mutable "latest" content under an immutable version.
    pub fn canonical_identity(&self) -> String {
        format!(
            "{}@{}:{}",
            self.manifest.name, self.manifest.version, self.content_hash
        )
    }
}

/// Version-immutability helper: `SkillPackage` is immutable by
/// version; a new version is a NEW package, never a mutation.
pub fn version_key(name: &str, version: &str) -> String {
    format!("{name}@{version}")
}
