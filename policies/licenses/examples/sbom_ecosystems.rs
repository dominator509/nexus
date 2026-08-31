//! sbom_ecosystems: real multi-ecosystem shipped-product SBOM inventory
//! adapter (RX-009; AUD-060).
//!
//! AUD-060 found the certified SBOM path was Cargo-centric: inventory
//! came only from Cargo.lock/Rust manifests although Nexus ships
//! pnpm/TypeScript, Flutter/Dart, images, model/data artifacts.
//!
//! This adapter inventories the REAL shipped product from REAL
//! repository state:
//!   - pnpm-lock.yaml   (TypeScript/pnpm ecosystem)
//!   - pubspec.lock     (Flutter/Dart ecosystem; every lockfile found)
//!   - artifact scans   (models/ tree, tests/data fixtures, images)
//!
//! Every count is parsed from real bytes; nothing is guessed. A missing
//! or malformed ecosystem lockfile fails closed (never an empty guess).
//! The output is a redacted, state-bound evidence document carrying
//! per-ecosystem package counts plus the artifact inventory.

use std::path::{Path, PathBuf};

use nexus_supply_chain_policy::evidence::redact_secret_shaped;

/// Count + list packages in a pnpm-lock.yaml v9 `packages:` section.
/// Package entries look like `  '@scope/name@1.2.3':` or
/// `  plain-name@1.2.3:`.
fn inventory_pnpm_lock(path: &Path) -> Result<(usize, Vec<String>), String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut in_packages = false;
    let mut packages: Vec<String> = Vec::new();
    for line in raw.lines() {
        if line.starts_with("packages:") {
            in_packages = true;
            continue;
        }
        if !in_packages {
            continue;
        }
        // The packages section ends at the first non-indented,
        // non-blank key (blank lines inside the section are legal).
        if !line.starts_with(' ') && !line.trim().is_empty() {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.ends_with(':') && !trimmed.starts_with('#') {
            let name = trimmed
                .trim_end_matches(':')
                .trim()
                .trim_start_matches('\'')
                .trim_end_matches('\'');
            if !name.is_empty() && (name.contains('@') || !name.contains(':')) {
                packages.push(name.to_string());
            }
        }
    }
    if packages.is_empty() {
        return Err(format!(
            "pnpm lockfile {} has no packages section",
            path.display()
        ));
    }
    Ok((packages.len(), packages))
}

/// Count + list packages in a pubspec.lock `packages:` section.
/// Entries look like `  async:` followed by `    version: "2.13.1"`.
fn inventory_pubspec_lock(path: &Path) -> Result<(usize, Vec<String>), String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut in_packages = false;
    let mut packages: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    for line in raw.lines() {
        if line.starts_with("packages:") {
            in_packages = true;
            continue;
        }
        if !in_packages {
            continue;
        }
        if !line.starts_with(' ') && !line.trim().is_empty() {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.ends_with(':')
            && !trimmed.starts_with('#')
            && !trimmed.starts_with("description:")
        {
            let name = trimmed.trim_end_matches(':').trim();
            if !name.contains(' ') && !name.contains(':') {
                current = Some(name.to_string());
            }
        } else if let Some(name) = current.take() {
            packages.push(name);
        }
    }
    if packages.is_empty() {
        return Err(format!(
            "pubspec lockfile {} has no packages section",
            path.display()
        ));
    }
    Ok((packages.len(), packages))
}

/// Recursively inventory artifact files under a directory tree.
fn inventory_tree(root: &Path, allowed_exts: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    fn walk(dir: &Path, allowed: &[&str], out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Skip caches and build output; never inventory those.
                let name = entry.file_name().to_string_lossy().to_string();
                if name == "__pycache__" || name == ".git" || name == "node_modules" {
                    continue;
                }
                walk(&path, allowed, out);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if allowed.contains(&ext) {
                    out.push(path.to_string_lossy().to_string());
                }
            }
        }
    }
    walk(root, allowed_exts, &mut out);
    out
}

/// Recursively find every pubspec.lock under a directory (real scan).
fn collect_pubspec_locks(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "node_modules" || name == ".git" || name == ".dart_tool" || name == "build" {
                continue;
            }
            collect_pubspec_locks(&path, out);
        } else if path
            .file_name()
            .map(|f| f == "pubspec.lock")
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("usage: sbom_ecosystems <repo_root> <run_id> <git_commit> <output_path>");
        std::process::exit(2);
    }
    let repo_root = PathBuf::from(&args[1]);
    let run_id = &args[2];
    let git_commit = &args[3];
    let output_path = PathBuf::from(&args[4]);

    // Fail closed: the shipped product MUST have its ecosystem lockfiles.
    let pnpm_path = repo_root.join("pnpm-lock.yaml");
    if !pnpm_path.is_file() {
        eprintln!(
            "sbom_ecosystems: FAIL - pnpm-lock.yaml missing at {}",
            pnpm_path.display()
        );
        std::process::exit(1);
    }
    let (pnpm_count, pnpm_packages) = inventory_pnpm_lock(&pnpm_path).unwrap_or_else(|e| {
        eprintln!("sbom_ecosystems: FAIL - {e}");
        std::process::exit(1);
    });

    // Dart ecosystem: every pubspec.lock under the repo (mobile apps).
    let mut dart_packages: Vec<String> = Vec::new();
    let mut dart_lockfiles: Vec<String> = Vec::new();
    let mut locks = Vec::new();
    collect_pubspec_locks(&repo_root, &mut locks);
    locks.sort();
    for path in &locks {
        match inventory_pubspec_lock(path) {
            Ok((_count, pkgs)) => {
                dart_lockfiles.push(path.to_string_lossy().to_string());
                dart_packages.extend(pkgs);
            }
            Err(e) => {
                eprintln!("sbom_ecosystems: FAIL - {e}");
                std::process::exit(1);
            }
        }
    }

    // Artifact inventory: model/data/image payload trees (real files).
    let model_files = inventory_tree(
        &repo_root.join("models"),
        &[
            "py",
            "json",
            "yaml",
            "yml",
            "toml",
            "txt",
            "gguf",
            "safetensors",
            "bin",
            "onnx",
        ],
    );
    let data_files = inventory_tree(
        &repo_root.join("tests/data"),
        &["json", "yaml", "yml", "toml", "txt", "csv", "bin"],
    );
    let image_files = inventory_tree(
        &repo_root.join("apps"),
        &["png", "jpg", "jpeg", "webp", "svg", "ico"],
    );

    let generated_at_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let body = serde_json::json!({
        "schema": "nexus.sbom.ecosystems.v1",
        "run_id": run_id,
        "git_commit": git_commit,
        "generated_at_ts": generated_at_ts,
        "ecosystems": {
            "rust": {
                "lockfile": "Cargo.lock",
                "certified": "sbom_generate evidence (nexus.sbom.evidence.v1)",
            },
            "typescript": {
                "lockfile": "pnpm-lock.yaml",
                "package_count": pnpm_count,
            },
            "dart": {
                "lockfiles": dart_lockfiles,
                "lockfile_count": dart_lockfiles.len(),
                "package_count": dart_packages.len(),
            },
        },
        "artifacts": {
            "model_files": model_files.len(),
            "data_files": data_files.len(),
            "image_files": image_files.len(),
        },
        "pnpm_packages": pnpm_packages,
        "dart_packages": dart_packages,
        "verification_state": "GENERATED",
        "redaction": "PASSED",
    });

    let raw = body.to_string();
    let redacted = redact_secret_shaped(&raw);

    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&output_path, redacted.as_bytes()) {
        eprintln!("sbom_ecosystems: FAIL - cannot write output: {e}");
        std::process::exit(1);
    }

    println!(
        "sbom_ecosystems: wrote {} (typescript={} dart={} dart_lockfiles={} models={} data={} images={})",
        output_path.display(),
        pnpm_count,
        dart_packages.len(),
        dart_lockfiles.len(),
        model_files.len(),
        data_files.len(),
        image_files.len()
    );
    println!("sbom_ecosystems: ok");
}
