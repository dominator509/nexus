//! EP-018 skill bundle loading and scan-before-install (SPEC-010
//! behavior 6; ADR-025; EP-018 M2).
//!
//! A portable skill bundle is a directory:
//!
//! ```text
//! <root>/<namespace>/<skill-name>/<version>/
//!   manifest.json   # canonical SkillManifest JSON
//!   SKILL.md        # payload; sha256 of these bytes is content_hash
//! ```
//!
//! `SkillBundleLoader` performs real filesystem I/O and the real
//! scan-before-install content hash: the payload is hashed with SHA-256
//! at load time and the package is validated fail-closed. Tampered or
//! missing payloads are rejected; the manifest path must agree with the
//! manifest's own name/version (spoofing rejection).

use crate::manifest::SkillPackageErrorCode;
use crate::manifest::{SkillManifest, SkillPackage, SkillPackageError};
use std::fmt;
use std::path::{Path, PathBuf};

/// A loaded skill bundle: the validated package plus its payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillBundle {
    pub package: SkillPackage,
    pub payload: Vec<u8>,
}

/// Loads skill bundles from a bundle root directory.
#[derive(Debug, Clone)]
pub struct SkillBundleLoader {
    root: PathBuf,
}

impl SkillBundleLoader {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Load and scan one bundle. The payload is hashed (SHA-256) and
    /// the package is validated; a mismatch, malformed manifest, or
    /// spoofed path fails closed.
    pub fn load(&self, name: &str, version: &str) -> Result<SkillBundle, SkillPackageError> {
        let bundle_dir = self.root.join(name).join(version);
        let manifest_path = bundle_dir.join("manifest.json");
        if !manifest_path.is_file() {
            return Err(SkillPackageError::new(
                SkillPackageErrorCode::NotFound,
                format!("skill bundle manifest not found: {name}@{version}"),
                Some(manifest_path.to_string_lossy().into_owned()),
            ));
        }
        let manifest_text = std::fs::read_to_string(&manifest_path).map_err(|e| {
            SkillPackageError::new(
                SkillPackageErrorCode::Unavailable,
                format!("cannot read skill bundle manifest: {e}"),
                Some(manifest_path.to_string_lossy().into_owned()),
            )
        })?;
        let manifest: SkillManifest = serde_json::from_str(&manifest_text).map_err(|e| {
            SkillPackageError::new(
                SkillPackageErrorCode::Validation,
                format!("malformed skill bundle manifest: {e}"),
                Some(manifest_path.to_string_lossy().into_owned()),
            )
        })?;
        // Path/contract consistency: the bundle path must agree with
        // the manifest's own identity (spoofing rejection).
        if manifest.name != name || manifest.version != version {
            return Err(SkillPackageError::new(
                SkillPackageErrorCode::Validation,
                format!(
                    "skill bundle path {name}@{version} does not match manifest {}@{}",
                    manifest.name, manifest.version
                ),
                Some("skill-bundle".into()),
            ));
        }
        let payload_path = bundle_dir.join("SKILL.md");
        let payload = std::fs::read(&payload_path).map_err(|e| {
            SkillPackageError::new(
                SkillPackageErrorCode::NotFound,
                format!("skill bundle payload missing: {e}"),
                Some(payload_path.to_string_lossy().into_owned()),
            )
        })?;
        let content_hash = sha256_hex(&payload);
        let package = SkillPackage {
            manifest,
            content_hash,
            created_at_epoch_ms: 0,
        };
        package.validate()?;
        Ok(SkillBundle { package, payload })
    }

    /// List available bundles as `(name, version)` in deterministic
    /// order (namespace, skill, version).
    pub fn list_available(&self) -> Result<Vec<(String, String)>, SkillPackageError> {
        let mut out = Vec::new();
        let entries = std::fs::read_dir(&self.root).map_err(|e| {
            SkillPackageError::new(
                SkillPackageErrorCode::Unavailable,
                format!("cannot read skill bundle root: {e}"),
                Some(self.root.to_string_lossy().into_owned()),
            )
        })?;
        for namespace in entries.flatten() {
            if !namespace.path().is_dir() {
                continue;
            }
            let skills = std::fs::read_dir(namespace.path()).map_err(|e| {
                SkillPackageError::new(
                    SkillPackageErrorCode::Unavailable,
                    format!("cannot read skill namespace: {e}"),
                    Some(namespace.path().to_string_lossy().into_owned()),
                )
            })?;
            for skill in skills.flatten() {
                if !skill.path().is_dir() {
                    continue;
                }
                let versions = std::fs::read_dir(skill.path()).map_err(|e| {
                    SkillPackageError::new(
                        SkillPackageErrorCode::Unavailable,
                        format!("cannot read skill versions: {e}"),
                        Some(skill.path().to_string_lossy().into_owned()),
                    )
                })?;
                for version in versions.flatten() {
                    if !version.path().join("manifest.json").is_file() {
                        continue;
                    }
                    let name = format!(
                        "{}/{}",
                        namespace.file_name().to_string_lossy(),
                        skill.file_name().to_string_lossy()
                    );
                    out.push((name, version.file_name().to_string_lossy().into_owned()));
                }
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Load and scan every available bundle.
    pub fn load_all(&self) -> Result<Vec<SkillBundle>, SkillPackageError> {
        let mut out = Vec::new();
        for (name, version) in self.list_available()? {
            out.push(self.load(&name, &version)?);
        }
        Ok(out)
    }
}

impl fmt::Display for SkillBundleLoader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SkillBundleLoader(root={})", self.root.display())
    }
}

/// Real SHA-256 content hash in lowercase hex (scan-before-install).
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(data);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
