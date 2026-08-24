//! sbom_generate: real SBOM evidence generator adapter (EP-039 M4).
//!
//! Invoked by scripts/sbom/generate.sh with:
//!
//!   sbom_generate <repo_root> <run_id> <git_commit>
//!                <lockfile_fingerprint> <policy_fingerprint> <output_path>
//!
//! The adapter uses the REAL certified transport (evaluate_inventory)
//! against the REAL workspace Cargo.lock, the REAL cargo registry
//! cache, and the checked-in policies/licenses/ files. It writes a
//! redacted, state-bound SBOM evidence document and exits non-zero
//! (fail closed) whenever the inventory cannot be evaluated - e.g.
//! missing Cargo.lock, malformed Cargo.lock, or empty inventory.
//!
//! The evidence document distinguishes GENERATED (written) from
//! VERIFIED (proven by scripts/sbom/verify.sh against the current
//! repository state) and NEVER claims COMPLETE, POLICY_PASSED, or
//! LEGAL_APPROVED. The real 16-denied-package finding (M3) is carried
//! forward verbatim through denied_count / policy_verdict.

use std::path::{Path, PathBuf};

use nexus_supply_chain_policy::evidence::redact_secret_shaped;
use nexus_supply_chain_policy_io::{assert_redacted, evaluate_inventory};

fn registry_src_root() -> PathBuf {
    let home = std::env::var("CARGO_HOME").unwrap_or_else(|_| {
        std::env::var("HOME")
            .map(|h| format!("{h}/.cargo"))
            .unwrap_or_else(|_| "/root/.cargo".to_string())
    });
    let root = Path::new(&home).join("registry/src");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .collect()
        })
        .unwrap_or_default();
    dirs.sort();
    dirs.first()
        .cloned()
        .unwrap_or_else(|| root.join("index.crates.io-missing"))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 7 {
        eprintln!(
            "usage: sbom_generate <repo_root> <run_id> <git_commit> <lockfile_fingerprint> <policy_fingerprint> <output_path>"
        );
        std::process::exit(2);
    }
    let repo_root = PathBuf::from(&args[1]);
    let run_id = &args[2];
    let git_commit = &args[3];
    let lockfile_fingerprint = &args[4];
    let policy_fingerprint = &args[5];
    let output_path = PathBuf::from(&args[6]);

    // Fail closed on missing repository inputs: a generator that cannot
    // see the real lockfile must not emit an empty or guessed SBOM.
    if !repo_root.join("Cargo.lock").is_file() {
        eprintln!(
            "sbom_generate: FAIL - Cargo.lock missing at {}",
            repo_root.join("Cargo.lock").display()
        );
        std::process::exit(1);
    }

    let registry_src = registry_src_root();
    let report = match evaluate_inventory(
        run_id,
        &repo_root.join("Cargo.lock"),
        &registry_src,
        &repo_root.join("policies/licenses"),
        &repo_root,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("sbom_generate: FAIL - inventory evaluation failed closed: {e}");
            std::process::exit(1);
        }
    };

    // Honest counts. denied = packages whose license cannot clear
    // policy: UNKNOWN + PROHIBITED + MISSING. REVIEW/SIDECAR/EXTERNAL
    // are non-GREEN actionable classes, reported separately, never
    // silently promoted to green. (missing_license_count is a subset
    // of the UNKNOWN class in this transport, so denied never
    // double-counts it.)
    let non_green_count = report.package_count.saturating_sub(report.green_count);
    let denied_count = non_green_count
        .saturating_sub(report.review_count)
        .saturating_sub(report.sidecar_count)
        .saturating_sub(report.external_count);
    let policy_verdict = if report.green_count > 0 && denied_count == 0 && non_green_count == 0 {
        "GREEN"
    } else if denied_count > 0 {
        "NON_GREEN"
    } else {
        "ACTION_REQUIRED"
    };

    let generated_at_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let packages: Vec<serde_json::Value> = report
        .packages
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "version": p.version,
                "license_spdx": p.license_spdx,
                "class": p.class,
                "license_clear": p.license_clear,
                "permitted_default": p.permitted_default,
                "reason": p.reason,
            })
        })
        .collect();

    let body = serde_json::json!({
        "schema": "nexus.sbom.evidence.v1",
        "format": "nexus-sbom-evidence",
        "spec_version": "1.0",
        "run_id": run_id,
        "git_commit": git_commit,
        "lockfile": "Cargo.lock",
        "lockfile_fingerprint": lockfile_fingerprint,
        "policy_fingerprint": policy_fingerprint,
        "generated_at_ts": generated_at_ts,
        "package_count": report.package_count,
        "resolved_count": report.resolved_license_count,
        "transitive_count": report.transitive_count,
        "workspace_count": report.workspace_count,
        "green_count": report.green_count,
        "review_count": report.review_count,
        "sidecar_count": report.sidecar_count,
        "external_count": report.external_count,
        "prohibited_count": report.prohibited_count,
        "unknown_count": report.unknown_count,
        "missing_license_count": report.missing_license_count,
        "denied_count": denied_count,
        "non_green_count": non_green_count,
        "permitted_default_count": report.permitted_default_count,
        "policy_verdict": policy_verdict,
        "verification_state": "GENERATED",
        "verification_state_explanation": "GENERATED != VERIFIED: verify.sh must bind this document to the current repository state",
        "completeness_state": "NOT_ASSERTED",
        "policy_passed": false,
        "legal_approved": false,
        "provenance_state": "NOT_VERIFIED",
        "advisory_source_status": "NOT_QUERIED",
        "redaction": "PASSED",
        "packages": packages,
    });

    let raw = body.to_string();
    let redacted = redact_secret_shaped(&raw);

    if !assert_redacted(&redacted) {
        eprintln!("sbom_generate: FAIL - evidence contains secret-shaped content after redaction");
        std::process::exit(1);
    }

    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Err(e) = std::fs::write(&output_path, redacted.as_bytes()) {
        eprintln!("sbom_generate: FAIL - cannot write evidence: {e}");
        std::process::exit(1);
    }

    println!(
        "sbom_generate: wrote {} ({} packages, {} resolved, {} green, {} denied, verdict {})",
        output_path.display(),
        report.package_count,
        report.resolved_license_count,
        report.green_count,
        denied_count,
        policy_verdict
    );
}
