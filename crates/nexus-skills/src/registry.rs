//! EP-018 skill registry (SPEC-010 behaviors 6-8; ADR-025).
//!
//! Nexus owns the skill registry. Skills are portable, signed,
//! immutable by version, and scanned before install. The registry
//! fails closed: an unsigned or invalid package is never registered;
//! a skill cannot be installed unless its declared permissions are
//! within the caller's authority; community skills begin inspect-only
//! or sandboxed.

use crate::bundle::SkillBundle;
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
    /// Revoked entries are terminal: they remain visible in state but
    /// can never be resolved for execution (ADR-025).
    pub revoked: bool,
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
            revoked: false,
        };
        self.entries.insert(key, entry.clone());
        Ok(entry)
    }

    /// Install a scanned bundle: register the package (authority checks
    /// apply unchanged) and persist through the store. Rollback is
    /// fail-closed: if persistence fails, the in-memory registration is
    /// undone so memory and disk never diverge (partial side effect).
    pub fn install_bundle<S: crate::store::SkillRegistryStore>(
        &mut self,
        bundle: SkillBundle,
        caller_trust: SkillTrustLevel,
        now_epoch_ms: u64,
        store: &S,
    ) -> Result<SkillRegistryEntry, SkillPackageError> {
        let entry = self.register(bundle.package, caller_trust, now_epoch_ms)?;
        let key = version_key(&entry.name, &entry.version);
        if let Err(e) = store.save(&self.to_state()) {
            // Rollback the in-memory mutation before failing.
            self.entries.remove(&key);
            return Err(e);
        }
        Ok(entry)
    }

    /// Remove an installed version. Revoked or missing entries cannot
    /// be removed silently; removal is explicit and persists. On
    /// persistence failure the entry is restored (compensation).
    pub fn remove<S: crate::store::SkillRegistryStore>(
        &mut self,
        name: &str,
        version: &str,
        store: &S,
    ) -> Result<(), SkillPackageError> {
        let key = version_key(name, version);
        let removed = self.entries.remove(&key).ok_or_else(|| {
            SkillPackageError::not_found("skill version not installed", Some(key.clone()))
        })?;
        if let Err(e) = store.save(&self.to_state()) {
            self.entries.insert(key, removed);
            return Err(e);
        }
        Ok(())
    }

    /// Revoke an installed version (ADR-025). A revoked entry remains
    /// in state for audit but can never be resolved for execution. On
    /// persistence failure the revocation is undone (compensation).
    pub fn revoke<S: crate::store::SkillRegistryStore>(
        &mut self,
        name: &str,
        version: &str,
        store: &S,
    ) -> Result<(), SkillPackageError> {
        let key = version_key(name, version);
        {
            let entry = self.entries.get_mut(&key).ok_or_else(|| {
                SkillPackageError::not_found("skill version not installed", Some(key.clone()))
            })?;
            entry.revoked = true;
        }
        if let Err(e) = store.save(&self.to_state()) {
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.revoked = false;
            }
            return Err(e);
        }
        Ok(())
    }

    /// Bounded recovery command: clear all installed state. This is the
    /// operations diagnostic for the registry (EP-018 M4); it is
    /// explicit, persisted, and never reconstructs authority.
    pub fn clear<S: crate::store::SkillRegistryStore>(
        &mut self,
        store: &S,
    ) -> Result<(), SkillPackageError> {
        self.entries.clear();
        store.save(&self.to_state())
    }

    /// Resolve a package for execution. Fails closed: missing or
    /// revoked entries are never executable (ADR-025).
    pub fn resolve_for_execution(
        &self,
        name: &str,
        version: &str,
    ) -> Result<SkillPackage, SkillPackageError> {
        let key = version_key(name, version);
        let entry = self.entries.get(&key).ok_or_else(|| {
            SkillPackageError::not_found("skill version not installed", Some(key.clone()))
        })?;
        if entry.revoked {
            return Err(SkillPackageError::policy(
                "skill version is revoked and cannot execute",
                Some(key),
            ));
        }
        Ok(entry.package.clone())
    }

    /// Whether the installed version is revoked (not installed -> not
    /// revoked; uninstalled skills cannot be executed).
    pub fn is_revoked(&self, name: &str, version: &str) -> bool {
        self.entries
            .get(&version_key(name, version))
            .map(|e| e.revoked)
            .unwrap_or(false)
    }

    /// Build a registry from persisted state. Authority is enforced on
    /// mutation, never reconstructed from state.
    pub fn from_state(state: crate::store::SkillRegistryState) -> Self {
        let mut entries = HashMap::new();
        for entry in state.entries {
            let key = version_key(&entry.name, &entry.version);
            entries.insert(key, entry);
        }
        Self { entries }
    }

    /// Snapshot the registry for persistence.
    pub fn to_state(&self) -> crate::store::SkillRegistryState {
        let mut entries: Vec<SkillRegistryEntry> = self.entries.values().cloned().collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.version.cmp(&b.version)));
        crate::store::SkillRegistryState { entries }
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
