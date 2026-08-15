//! EP-018 skill registry (SPEC-010 behaviors 6-8; ADR-025).
//!
//! Nexus owns the skill registry. Skills are portable, signed,
//! immutable by version, and scanned before install. The registry
//! fails closed: an unsigned or invalid package is never registered;
//! a skill cannot be installed unless its declared permissions are
//! within the caller's authority; community skills begin inspect-only
//! or sandboxed.

use crate::manifest::{version_key, SkillPackage, SkillPackageError};
use crate::vocabulary::SkillTrustLevel;
use nexus_domain::TenantId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The installed skill registry entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRegistryEntry {
    pub name: String,
    pub version: String,
    pub package: SkillPackage,
    pub installed_at_epoch_ms: u64,
}

/// The canonical skill registry.
#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    entries: HashMap<String, SkillRegistryEntry>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a package. Fails closed: the package must validate
    /// (signed, immutable, well-formed) and the declared permissions
    /// must be within the caller's trust authority. A duplicate with
    /// the exact same canonical identity is idempotent (returns the
    /// existing entry); the same name/version with changed content is
    /// a conflict (immutable by version, ADR-025).
    pub fn register(
        &mut self,
        package: SkillPackage,
        caller_trust: SkillTrustLevel,
        now_epoch_ms: u64,
    ) -> Result<SkillRegistryEntry, SkillPackageError> {
        package.validate()?;
        // A skill can never grant itself authority: the declared
        // permissions must be within the caller's trust tier (the
        // deterministic ceiling, ADR-025).
        let max_allowed = caller_trust.permission_ceiling();
        for permission in package.declared_permissions() {
            if permission > &max_allowed {
                return Err(SkillPackageError::policy(
                    "declared permission exceeds caller trust",
                    Some("skill-registry".into()),
                ));
            }
        }
        // Community skills begin inspect-only or sandboxed.
        let trust = package.manifest.trust_level;
        if trust == SkillTrustLevel::Trusted || trust == SkillTrustLevel::System {
            // Only a SYSTEM caller may register a SYSTEM/TRUSTED
            // skill; otherwise it must be lowered to sandboxed.
            if caller_trust != SkillTrustLevel::System {
                return Err(SkillPackageError::policy(
                    "trusted/system skill requires SYSTEM caller",
                    Some("skill-registry".into()),
                ));
            }
        }
        let key = version_key(&package.manifest.name, &package.manifest.version);
        if let Some(existing) = self.entries.get(&key) {
            if existing.package.canonical_identity() == package.canonical_identity() {
                // Exact duplicate: idempotent, returns the installed
                // entry. There is no mutable "latest" content under an
                // immutable version.
                return Ok(existing.clone());
            }
            return Err(SkillPackageError::conflict(
                "skill version already registered with different content",
                Some(key),
            ));
        }
        let entry = SkillRegistryEntry {
            name: package.manifest.name.clone(),
            version: package.manifest.version.clone(),
            package: package.clone(),
            installed_at_epoch_ms: now_epoch_ms,
        };
        self.entries.insert(key, entry.clone());
        Ok(entry)
    }

    pub fn get(&self, name: &str, version: &str) -> Option<&SkillRegistryEntry> {
        self.entries.get(&version_key(name, version))
    }

    pub fn list(&self, _tenant_id: &TenantId) -> Vec<&SkillRegistryEntry> {
        let mut out: Vec<&SkillRegistryEntry> = self.entries.values().collect();
        out.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
        out
    }
}
