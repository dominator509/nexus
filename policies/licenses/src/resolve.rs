//! Real license string resolution (EP-039 M3).
//!
//! For every locked package, resolve the REAL license declaration from
//! real sources:
//!
//! - registry packages: the crate's Cargo.toml in the real cargo
//!   registry cache (`$CARGO_HOME/registry/src/index.crates.io-*/`)
//! - workspace path packages: the package's Cargo.toml anywhere in the
//!   repository (crates/, connectors/, providers/, infra/, tests/,
//!   supply-chain/, dashboards/, policies/...), honoring
//!   `license.workspace = true` inheritance from the workspace root
//!
//! A package whose license cannot be resolved is reported as MISSING -
//! never guessed, never defaulted. MISSING LICENSE != SAFE.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::lockfile::LockedPackage;

/// Resolved license evidence for one locked package.
#[derive(Debug, Clone)]
pub struct ResolvedLicense {
    pub name: String,
    pub version: String,
    /// The exact SPDX string declared upstream (or None when missing).
    pub spdx: Option<String>,
    /// Human-readable source path of the declaration.
    pub source: String,
}

/// A name -> manifest path index of every workspace Cargo.toml found
/// under the repository (excluding target/ and .git).
#[derive(Debug, Clone, Default)]
pub struct WorkspaceManifestIndex {
    by_name: HashMap<String, PathBuf>,
    workspace_root_license: Option<String>,
}

impl WorkspaceManifestIndex {
    /// Build the index by scanning the repository for Cargo.toml files.
    /// This is the real transport: every workspace member's manifest is
    /// read, no member list is hard-coded.
    pub fn build(workspace_root: &Path) -> Self {
        let mut by_name = HashMap::new();
        let mut workspace_root_license = None;
        let mut stack = vec![workspace_root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let entries = match std::fs::read_dir(&dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = entry.file_name();
                let name = file_name.to_string_lossy().to_string();
                if path.is_dir() {
                    if name == "target" || name == ".git" || name == "node_modules" {
                        continue;
                    }
                    stack.push(path);
                } else if name == "Cargo.toml" {
                    if let Ok(text) = std::fs::read_to_string(&path) {
                        if let Some(pkg_name) = extract_package_name(&text) {
                            by_name.entry(pkg_name).or_insert_with(|| path.clone());
                        }
                        if path == workspace_root.join("Cargo.toml") {
                            workspace_root_license = extract_workspace_root_license(&text);
                        }
                    }
                }
            }
        }
        Self {
            by_name,
            workspace_root_license,
        }
    }

    /// Resolve a workspace member's license from its real manifest.
    fn resolve_member(&self, name: &str) -> Option<(String, String)> {
        let manifest = self.by_name.get(name)?;
        let text = std::fs::read_to_string(manifest).ok()?;
        if let Some(spdx) = extract_license(&text) {
            return Some((spdx, manifest.display().to_string()));
        }
        if has_workspace_license(&text) {
            if let Some(spdx) = &self.workspace_root_license {
                return Some((
                    spdx.clone(),
                    format!(
                        "{} (license.workspace -> {})",
                        manifest.display(),
                        "workspace root Cargo.toml"
                    ),
                ));
            }
        }
        None
    }
}

fn extract_package_name(toml_text: &str) -> Option<String> {
    for line in toml_text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("name") {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                let v = v.trim();
                if let Some(inner) = v.strip_prefix('"') {
                    if let Some(end) = inner.find('"') {
                        return Some(inner[..end].to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_workspace_root_license(toml_text: &str) -> Option<String> {
    let mut in_workspace_package = false;
    for line in toml_text.lines() {
        let t = line.trim();
        if t.starts_with("[workspace.package]") {
            in_workspace_package = true;
            continue;
        }
        if in_workspace_package && t.starts_with('[') && !t.starts_with("[[") {
            in_workspace_package = false;
        }
        if in_workspace_package {
            if let Some(rest) = t.strip_prefix("license") {
                let rest = rest.trim_start();
                if let Some(v) = rest.strip_prefix('=') {
                    let v = v.trim();
                    if let Some(inner) = v.strip_prefix('"') {
                        if let Some(end) = inner.find('"') {
                            return Some(inner[..end].to_string());
                        }
                    }
                }
            }
        }
    }
    None
}

/// Locate a registry crate's Cargo.toml inside a cargo registry src
/// root (e.g. `$CARGO_HOME/registry/src/index.crates.io-1949cf8c6b5b557f`).
fn registry_manifest(registry_src: &Path, name: &str, version: &str) -> Option<PathBuf> {
    let candidate = registry_src
        .join(format!("{name}-{version}"))
        .join("Cargo.toml");
    if candidate.is_file() {
        return Some(candidate);
    }
    // Fall back to scanning a single-level-deep layout if the exact
    // dir is not at the top (uncommon, but cheap to check).
    if let Ok(entries) = std::fs::read_dir(registry_src) {
        for e in entries.flatten() {
            let p = e
                .path()
                .join(format!("{name}-{version}"))
                .join("Cargo.toml");
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn extract_license(toml_text: &str) -> Option<String> {
    for line in toml_text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("license") {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                let v = v.trim();
                if let Some(inner) = v.strip_prefix('"') {
                    if let Some(end) = inner.find('"') {
                        return Some(inner[..end].to_string());
                    }
                }
            }
        }
    }
    None
}

fn has_workspace_license(toml_text: &str) -> bool {
    toml_text
        .lines()
        .any(|l| l.trim().starts_with("license.workspace"))
}

/// Resolve the real license for one locked package.
///
/// `registry_src` is the cargo registry src root; `workspace_root` is
/// the repository root whose manifests are indexed.
pub fn resolve_license(
    pkg: &LockedPackage,
    registry_src: &Path,
    workspace_root: &Path,
) -> ResolvedLicense {
    // Registry package: license lives in the cached crate manifest.
    if pkg.source.is_some() {
        if let Some(manifest) = registry_manifest(registry_src, &pkg.name, &pkg.version) {
            if let Ok(text) = std::fs::read_to_string(&manifest) {
                if let Some(spdx) = extract_license(&text) {
                    return ResolvedLicense {
                        name: pkg.name.clone(),
                        version: pkg.version.clone(),
                        spdx: Some(spdx),
                        source: manifest.display().to_string(),
                    };
                }
            }
            // Found the manifest but no license -> missing (fail closed).
            return ResolvedLicense {
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                spdx: None,
                source: manifest.display().to_string(),
            };
        }
        // Registry package whose manifest is not cached -> missing.
        return ResolvedLicense {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            spdx: None,
            source: format!(
                "registry cache miss for {}-{} under {}",
                pkg.name,
                pkg.version,
                registry_src.display()
            ),
        };
    }

    // Workspace path package: resolve through the real manifest index.
    let index = WorkspaceManifestIndex::build(workspace_root);
    if let Some((spdx, source)) = index.resolve_member(&pkg.name) {
        return ResolvedLicense {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            spdx: Some(spdx),
            source,
        };
    }

    ResolvedLicense {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        spdx: None,
        source: format!(
            "workspace manifest not found for {} under {}",
            pkg.name,
            workspace_root.display()
        ),
    }
}
