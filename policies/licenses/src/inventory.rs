//! Real inventory evaluation (EP-039 M3).
//!
//! Builds the deterministic inventory report from the REAL Cargo.lock,
//! REAL resolved licenses, and the checked-in policy files. Every
//! locked package (including transitives) is evaluated through the M2
//! engine; nothing is skipped, nothing is guessed.
//!
//! Honest semantics:
//! - `class`: the transport's canonical license class for the real
//!   expression (GREEN/REVIEW/SIDECAR/EXTERNAL/PROHIBITED/UNKNOWN)
//! - `license_clear`: TRUE only when class == GREEN (the license itself
//!   clears policy; a real component still needs review + approval)
//! - `permitted_default`: the M2 engine's decision on the package as
//!   scanned (review Denied, approval Pending) - ALWAYS false, which
//!   proves ALLOWLIST ENTRY != APPROVAL on real data
//! - `engine_permits_when_approved`: for a GREEN-class package with a
//!   canonical expression the M1 table knows, a fully reviewed +
//!   approved component IS permitted (positive wiring proof)

use std::path::Path;

use nexus_supply_chain::model::{Component, ComponentBoundary, ComponentIdentity, SourceOffer};
use nexus_supply_chain::vocabulary::{
    ApprovalState, IntegrationMode, LicenseClass, LicenseReview, RiskClass, VerificationResult,
};
use nexus_supply_chain::LicenseClassifier;
use nexus_supply_chain::LicenseClassifierPort;

use crate::lockfile::{read_lockfile, LockedPackage};
use crate::policy_files::{load_policy_files, PolicyFiles};
use crate::resolve::{resolve_license, ResolvedLicense};
use crate::spdx::classify_spdx;

/// Per-package evaluation result.
#[derive(Debug, Clone)]
pub struct PackageEvaluation {
    pub name: String,
    pub version: String,
    pub license_spdx: Option<String>,
    pub class: String,
    pub license_clear: bool,
    pub permitted_default: bool,
    pub reason: String,
    pub source: String,
}

/// Aggregate inventory report.
#[derive(Debug, Clone)]
pub struct InventoryReport {
    pub run_id: String,
    pub package_count: usize,
    pub transitive_count: usize,
    pub workspace_count: usize,
    pub resolved_license_count: usize,
    pub missing_license_count: usize,
    pub green_count: usize,
    pub review_count: usize,
    pub sidecar_count: usize,
    pub external_count: usize,
    pub prohibited_count: usize,
    pub unknown_count: usize,
    pub permitted_default_count: usize,
    pub packages: Vec<PackageEvaluation>,
}

/// Run the full real inventory evaluation.
///
/// `lockfile_path` is the real Cargo.lock; `registry_src` the real
/// cargo registry src root; `policy_dir` the checked-in policies/
/// licenses/ directory; `workspace_root` the repo root.
pub fn evaluate_inventory(
    run_id: &str,
    lockfile_path: &Path,
    registry_src: &Path,
    policy_dir: &Path,
    workspace_root: &Path,
) -> Result<InventoryReport, String> {
    let files = load_policy_files(policy_dir)?;
    let lock = read_lockfile(lockfile_path)?;

    let mut packages = Vec::new();
    let mut resolved = 0usize;
    let mut missing = 0usize;
    let mut green = 0usize;
    let mut review = 0usize;
    let mut sidecar = 0usize;
    let mut external = 0usize;
    let mut prohibited = 0usize;
    let mut unknown = 0usize;
    let mut permitted_default = 0usize;
    let mut transitive = 0usize;
    let mut workspace_pkgs = 0usize;

    for pkg in &lock.package {
        if pkg.source.is_none() {
            workspace_pkgs += 1;
        }
        if !pkg.dependencies.is_empty() {
            transitive += 1;
        }

        let resolved_license = resolve_license(pkg, registry_src, workspace_root);
        let eval = evaluate_package(pkg, &resolved_license, &files);
        if resolved_license.spdx.is_some() {
            resolved += 1;
        } else {
            missing += 1;
        }
        match eval.class.as_str() {
            "GREEN" => green += 1,
            "REVIEW" => review += 1,
            "SIDECAR" => sidecar += 1,
            "EXTERNAL" => external += 1,
            "PROHIBITED" => prohibited += 1,
            _ => unknown += 1,
        }
        if eval.permitted_default {
            permitted_default += 1;
        }
        packages.push(eval);
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name).then(a.version.cmp(&b.version)));

    Ok(InventoryReport {
        run_id: run_id.to_string(),
        package_count: lock.package.len(),
        transitive_count: transitive,
        workspace_count: workspace_pkgs,
        resolved_license_count: resolved,
        missing_license_count: missing,
        green_count: green,
        review_count: review,
        sidecar_count: sidecar,
        external_count: external,
        prohibited_count: prohibited,
        unknown_count: unknown,
        permitted_default_count: permitted_default,
        packages,
    })
}

/// Canonicalize a real SPDX expression for the M1 classifier where
/// possible: single id or a simple two-branch OR sorted + uppercased
/// (M1 knows `APACHE-2.0 OR MIT` but not `MIT OR Apache-2.0`).
fn canonical_for_engine(spdx: &str, _class: LicenseClass) -> Option<String> {
    let upper = spdx.trim().to_ascii_uppercase();
    // Single canonical id.
    let canonical = LicenseClassifierPort::new();
    if canonical.classify(&upper).is_ok() {
        return Some(upper);
    }
    // Two-branch OR: sort branches.
    let branches: Vec<&str> = upper.split(" OR ").map(str::trim).collect();
    if branches.len() == 2 {
        let mut sorted = branches.clone();
        sorted.sort();
        let key = sorted.join(" OR ");
        if canonical.classify(&key).is_ok() {
            return Some(key);
        }
    }
    // Complex expression: M1 table cannot verify it; the transport's
    // classification is authoritative and the engine stays fail-closed
    // (unknown string) unless class is GREEN via our boundary parser.
    None
}

/// Evaluate one package: resolve license -> classify via transport SPDX
/// boundary -> run through the M2 LicensePolicy for the exact policy
/// match.
pub fn evaluate_package(
    pkg: &LockedPackage,
    resolved: &ResolvedLicense,
    files: &PolicyFiles,
) -> PackageEvaluation {
    let canonical = LicenseClassifierPort::new();
    let canonical_fn = |id: &str| canonical.classify(id).ok();

    let spdx = resolved.spdx.clone();
    let class = match &spdx {
        Some(s) => classify_spdx(s, files, &canonical_fn),
        None => crate::spdx::SpdxClassification {
            class: None,
            reason: "missing license fails closed".to_string(),
            has_unknown_branch: true,
        },
    };

    // M2 engine on the scanned package (review Denied, approval
    // Pending). This is ALWAYS denied - ALLOWLIST ENTRY != APPROVAL.
    let raw_component = component_for(pkg, spdx.clone(), class.class, false);
    let policy = nexus_supply_chain_policy::license::LicensePolicy::new(
        crate::policy_files::license_policy_config(files),
    );
    let raw_eval = policy.evaluate(&raw_component);
    let permitted_default = raw_eval.permitted;

    // Positive wiring: a GREEN-class package with a canonical expression
    // IS permitted once reviewed + approved.
    let engine_permits_when_approved = class.class == Some(LicenseClass::Green)
        && spdx
            .as_deref()
            .and_then(|s| canonical_for_engine(s, LicenseClass::Green))
            .map(|canon| {
                let approved = component_for(pkg, Some(canon), Some(LicenseClass::Green), true);
                policy.evaluate(&approved).permitted
            })
            .unwrap_or(false);

    let license_clear = class.class == Some(LicenseClass::Green);
    let reason = if license_clear {
        if engine_permits_when_approved {
            format!(
                "GREEN: {} (engine permits once reviewed+approved)",
                class.reason
            )
        } else {
            format!(
                "GREEN: {} (complex expression beyond M1 table; transport boundary is authoritative)",
                class.reason
            )
        }
    } else {
        format!("denied: {}", class.reason)
    };

    PackageEvaluation {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        license_spdx: spdx,
        class: class
            .class
            .map(|c| c.as_str().to_string())
            .unwrap_or_else(|| "UNKNOWN".to_string()),
        license_clear,
        permitted_default,
        reason,
        source: resolved.source.clone(),
    }
}

fn component_for(
    pkg: &LockedPackage,
    spdx: Option<String>,
    class: Option<LicenseClass>,
    approved: bool,
) -> Component {
    Component {
        identity: ComponentIdentity {
            name: pkg.name.clone(),
            version: pkg.version.clone(),
            source: pkg
                .source
                .clone()
                .unwrap_or_else(|| "workspace".to_string()),
            registry: "crates.io".to_string(),
            lockfile: "Cargo.lock".to_string(),
            digest: None,
        },
        license_spdx: spdx,
        license_class: class,
        review: if approved {
            LicenseReview::Approved
        } else {
            LicenseReview::Denied
        },
        approval: if approved {
            ApprovalState::Approved
        } else {
            ApprovalState::Pending
        },
        integration_mode: IntegrationMode::Embedded,
        risk: RiskClass::Low,
        owner: "ep039-m3".to_string(),
        verification: VerificationResult::Unverified,
        evidence_ts: 1_700_000_000,
        run_id: "ep039-m3".to_string(),
    }
}

/// Declare a sidecar boundary record for SIDECAR-class components (used
/// by integration tests to prove boundary enforcement on real data).
pub fn sidecar_boundary(
    component: &str,
    process: &str,
    api: &str,
    class: LicenseClass,
) -> ComponentBoundary {
    ComponentBoundary {
        component: component.to_string(),
        sidecar_process: process.to_string(),
        api_contract: api.to_string(),
        license_class: class,
        source_offer: SourceOffer {
            url: "https://example.invalid/source".to_string(),
            version: "1.0.0".to_string(),
            valid_through: None,
        },
    }
}

/// Return the canonical M1 classifier (exposed for the gate's alignment
/// proof between policy files and the M1 contract).
pub fn canonical_classifier() -> LicenseClassifierPort {
    LicenseClassifierPort::new()
}
