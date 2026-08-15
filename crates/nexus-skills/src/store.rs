//! EP-018 skill registry persistence (SPEC-010 behavior 6; ADR-025;
//! EP-018 M2).
//!
//! The registry state is a plain serializable snapshot of entries.
//! `SkillRegistryStore` is the I/O port; `JsonFileSkillRegistryStore`
//! is the real filesystem implementation (JSON file, atomic-enough by
//! write-on-temp + rename). Persistence never changes authority rules:
//! loading a state is exactly the set of installed entries, and
//! register/revoke still enforce the same fail-closed checks.

use crate::manifest::SkillPackageError;
use crate::registry::SkillRegistryEntry;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Serializable registry state (entries only; authority is enforced on
/// mutation, never reconstructed from state).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SkillRegistryState {
    pub entries: Vec<SkillRegistryEntry>,
}

/// The registry persistence port.
pub trait SkillRegistryStore {
    /// Load state. A missing store is an empty registry (fail-open on
    /// absence is safe here: nothing was installed yet).
    fn load(&self) -> Result<SkillRegistryState, SkillPackageError>;
    /// Persist state. Must be durable on success.
    fn save(&self, state: &SkillRegistryState) -> Result<(), SkillPackageError>;
}

/// Real JSON-file registry store.
#[derive(Debug, Clone)]
pub struct JsonFileSkillRegistryStore {
    path: PathBuf,
}

impl JsonFileSkillRegistryStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SkillRegistryStore for JsonFileSkillRegistryStore {
    fn load(&self) -> Result<SkillRegistryState, SkillPackageError> {
        if !self.path.is_file() {
            return Ok(SkillRegistryState {
                entries: Vec::new(),
            });
        }
        let text = std::fs::read_to_string(&self.path).map_err(|e| {
            SkillPackageError::unavailable(
                format!("cannot read skill registry state: {e}"),
                Some(self.path.to_string_lossy().into_owned()),
            )
        })?;
        serde_json::from_str(&text).map_err(|e| {
            SkillPackageError::validation(
                format!("malformed skill registry state: {e}"),
                Some(self.path.to_string_lossy().into_owned()),
            )
        })
    }

    fn save(&self, state: &SkillRegistryState) -> Result<(), SkillPackageError> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SkillPackageError::unavailable(
                    format!("cannot create registry state directory: {e}"),
                    Some(parent.to_string_lossy().into_owned()),
                )
            })?;
        }
        let json = serde_json::to_string_pretty(state).map_err(|e| {
            SkillPackageError::validation(
                format!("cannot serialize skill registry state: {e}"),
                Some("skill-registry-state".into()),
            )
        })?;
        // Write to a temp file in the same directory, then rename, so a
        // failed write never corrupts the previous state.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| {
            SkillPackageError::unavailable(
                format!("cannot write skill registry state: {e}"),
                Some(tmp.to_string_lossy().into_owned()),
            )
        })?;
        std::fs::rename(&tmp, &self.path).map_err(|e| {
            SkillPackageError::unavailable(
                format!("cannot commit skill registry state: {e}"),
                Some(self.path.to_string_lossy().into_owned()),
            )
        })
    }
}
