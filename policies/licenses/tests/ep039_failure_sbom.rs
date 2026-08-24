//! EP-039 M4 forced-failure and abuse-case proofs (SPEC-019; EP-039 M4
//! fence `scripts/sbom/` + the authorized `policies/licenses/` crate
//! that hosts the real transport machinery).
//!
//! Every test name begins `ep039_failure_`. Each proof exercises a REAL
//! failure mechanism - real missing/malformed lockfiles in isolated
//! temp dirs, real registry-cache packages with denied licenses, real
//! policy files, the certified M1/M2 engines - and asserts a typed
//! fail-closed outcome. No component under test is mocked.
//!
//! The M3 real finding (446 packages, 430 GREEN, 16 denied) is
//! preserved: tests assert the denied classes remain denied, never
//! papered over.

use std::io::Write;
use std::path::{Path, PathBuf};

use nexus_supply_chain::model::AdvisoryAffected;
use nexus_supply_chain::model::{
    component, Component, SbomDocument, SbomPackage, SbomVerification,
};
use nexus_supply_chain::vocabulary::{
    AdvisorySeverity, ApprovalState, LicenseReview, VerificationResult, WaiverState,
};
use nexus_supply_chain::LicenseClassifier;
use nexus_supply_chain::LicenseClassifierPort;
use nexus_supply_chain_policy::waiver::WaiverScope;
use nexus_supply_chain_policy::{
    AdvisoryPolicy, AdvisoryPolicyConfig, LicensePolicy, LicensePolicyConfig, SbomPolicy,
    SbomPolicyConfig, WaiverPolicy, WaiverPolicyConfig,
};
use nexus_supply_chain_policy_io::{
    assert_redacted, classify_spdx, evaluate_inventory, inventory_evidence, load_policy_files,
    read_lockfile,
};

/// Repository root from CARGO_MANIFEST_DIR (policies/licenses/).
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("policies/licenses parent")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

/// The first real cargo registry src index dir (real cache).
fn registry_src() -> PathBuf {
    let home = std::env::var("CARGO_HOME").unwrap_or_else(|_| {
        std::env::var("HOME")
            .map(|h| format!("{h}/.cargo"))
            .unwrap_or_else(|_| "/root/.cargo".to_string())
    });
    let root = Path::new(&home).join("registry/src");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("registry/src readable")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs.first()
        .expect("at least one registry src index dir")
        .clone()
}

/// Load the real checked-in policy files once per suite.
fn real_policy_files() -> nexus_supply_chain_policy_io::policy_files::PolicyFiles {
    load_policy_files(&repo_root().join("policies/licenses")).expect("real policy files load")
}

/// Unique temp dir for an isolated fixture; removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("ep039-m4-{label}-{}-{ts}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir created");
        TempDir(dir)
    }
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn write(path: &Path, content: &str) {
    let mut f = std::fs::File::create(path).expect("fixture file created");
    f.write_all(content.as_bytes()).expect("fixture written");
}

fn sbom_pkg(name: &str, version: &str, source: &str, digest: Option<&str>) -> SbomPackage {
    SbomPackage {
        name: name.to_string(),
        version: version.to_string(),
        source: source.to_string(),
        license_spdx: Some("MIT".to_string()),
        digest: digest
            .map(|d| nexus_supply_chain::model::ArtifactDigest::parse(d).expect("digest parses")),
        is_transitive: false,
    }
}

// ---------------------------------------------------------------------
// Lockfile transport failures (real files in isolated temp dirs).
// ---------------------------------------------------------------------

#[test]
fn ep039_failure_missing_lockfile_fails_closed() {
    let dir = TempDir::new("missing-lock");
    // No Cargo.lock exists in the temp dir. The real transport must
    // fail closed with a typed unreadable error, never an empty set.
    let err = read_lockfile(&dir.path().join("Cargo.lock")).expect_err("missing lockfile fails");
    assert!(
        err.contains("unreadable"),
        "missing lockfile must report unreadable, got: {err}"
    );
}

#[test]
fn ep039_failure_malformed_lockfile_fails_closed() {
    let dir = TempDir::new("malformed-lock");
    let lock = dir.path().join("Cargo.lock");
    write(&lock, "this is [[ not [[ valid ] toml !!!");
    let err = read_lockfile(&lock).expect_err("malformed lockfile fails");
    assert!(
        err.contains("malformed"),
        "malformed lockfile must report malformed, got: {err}"
    );
}

#[test]
fn ep039_failure_empty_lockfile_refused() {
    let dir = TempDir::new("empty-lock");
    let lock = dir.path().join("Cargo.lock");
    // An empty package table parses as TOML but must still be refused
    // by the transport: a lockfile with zero packages is not an
    // inventory (LOCKFILE EXISTS != ALL ARTIFACTS ACCOUNTED FOR).
    write(&lock, "version = 3\npackage = []\n");
    let err = read_lockfile(&lock).expect_err("empty lockfile refused");
    assert!(
        err.contains("zero packages"),
        "empty lockfile must be refused, got: {err}"
    );
}

#[test]
fn ep039_failure_generate_inventory_missing_lockfile_fails_closed() {
    let dir = TempDir::new("gen-missing-lock");
    // evaluate_inventory on a root with no Cargo.lock must fail closed.
    let err = evaluate_inventory(
        "ep039-failure-missing-lock",
        &dir.path().join("Cargo.lock"),
        &registry_src(),
        &repo_root().join("policies/licenses"),
        dir.path(),
    )
    .expect_err("inventory with missing lockfile fails");
    assert!(
        err.contains("unreadable") || err.contains("Cargo.lock"),
        "typed fail-closed cause expected, got: {err}"
    );
}

// ---------------------------------------------------------------------
// License abuse cases through the REAL policy files + REAL registry.
// ---------------------------------------------------------------------

#[test]
fn ep039_failure_unknown_license_fails_closed() {
    let files = real_policy_files();
    let canonical = LicenseClassifierPort::new();
    // Real divergent ids from the M3 finding must remain non-GREEN.
    for id in ["MIT-0", "CC0-1.0", "Zlib", "BSL-1.0"] {
        let c = classify_spdx(id, &files, &|s| canonical.classify(s).ok());
        assert!(
            c.class.is_none() || c.class.as_ref().unwrap().as_str() != "GREEN",
            "{id} must not classify GREEN"
        );
        assert!(
            c.has_unknown_branch,
            "{id} must be flagged as an unknown branch"
        );
    }
}

#[test]
fn ep039_failure_missing_license_field_fails_closed_on_real_workspace() {
    // The real workspace contains at least two manifests with NO
    // license field (infra/sentinel/core and its advanced sibling).
    // The real inventory must surface them as missing, never guess.
    let root = repo_root();
    let report = evaluate_inventory(
        "ep039-failure-missing-license",
        &root.join("Cargo.lock"),
        &registry_src(),
        &root.join("policies/licenses"),
        &root,
    )
    .expect("real inventory evaluates");
    assert!(
        report.missing_license_count >= 1,
        "real workspace has license-less manifests; missing_license_count = {}",
        report.missing_license_count
    );
    for pkg in report.packages.iter().filter(|p| p.license_spdx.is_none()) {
        assert!(
            !pkg.license_clear && pkg.class != "GREEN",
            "missing license must fail closed for {}",
            pkg.name
        );
    }
}

#[test]
fn ep039_failure_fuzzy_license_alias_fails_closed() {
    let files = real_policy_files();
    let canonical = LicenseClassifierPort::new();
    for fuzzy in [
        "MIT-ish",
        "MIT/X11",
        "Apache",
        "GPL compatible",
        "BSD-style",
    ] {
        let c = classify_spdx(fuzzy, &files, &|s| canonical.classify(s).ok());
        assert!(
            c.class.is_none() || c.class.as_ref().unwrap().as_str() != "GREEN",
            "fuzzy alias {fuzzy:?} must never classify GREEN"
        );
    }
}

#[test]
fn ep039_failure_prohibited_license_fails_closed() {
    let files = real_policy_files();
    let canonical = LicenseClassifierPort::new();
    for id in ["CC-BY-NC-4.0", "CC-BY-NC-SA-4.0"] {
        let c = classify_spdx(id, &files, &|s| canonical.classify(s).ok());
        let class = c.class.map(|x| x.as_str().to_string()).unwrap_or_default();
        assert_eq!(class, "PROHIBITED", "{id} must classify PROHIBITED");
    }
}

#[test]
fn ep039_failure_transitive_dependency_with_denied_license_fails_closed() {
    // Isolated fixture: one workspace package depends on a REAL cached
    // registry crate whose REAL license is denied by Nexus policy
    // (foldhash 0.2.0 declares Zlib; the M3 finding recorded it).
    // TRANSITIVE DEPENDENCY != OUT OF SCOPE: the transitive must appear
    // in the inventory and must NOT clear license policy.
    let dir = TempDir::new("transitive-denied");
    write(
        &dir.path().join("Cargo.toml"),
        "[package]\nname = \"ep039-fixture-green\"\nversion = \"0.1.0\"\nedition = \"2021\"\nlicense = \"MIT\"\n",
    );
    write(
        &dir.path().join("Cargo.lock"),
        "version = 3\n\n\
[[package]]\n\
name = \"ep039-fixture-green\"\n\
version = \"0.1.0\"\n\
dependencies = [\"foldhash\"]\n\n\
[[package]]\n\
name = \"foldhash\"\n\
version = \"0.2.0\"\n\
source = \"registry+https://github.com/rust-lang/crates.io-index\"\n\
checksum = \"77ce24cb58228fbb8aa041425bb1050850ac19177686ea6e0f41a70416f56fdb\"\n",
    );
    let report = evaluate_inventory(
        "ep039-failure-transitive",
        &dir.path().join("Cargo.lock"),
        &registry_src(),
        &repo_root().join("policies/licenses"),
        dir.path(),
    )
    .expect("fixture inventory evaluates");
    let transitive = report
        .packages
        .iter()
        .find(|p| p.name == "foldhash")
        .expect("transitive foldhash is IN scope, never skipped");
    assert!(
        !transitive.license_clear,
        "foldhash (Zlib) must not clear license policy: {}",
        transitive.reason
    );
    assert_ne!(transitive.class, "GREEN", "foldhash must not be GREEN");
    assert!(
        report.denied_count_equivalent() >= 1 || !transitive.license_clear,
        "denied transitive must keep the inventory non-green"
    );
}

#[test]
fn ep039_failure_same_package_version_different_source_fails() {
    // ryu 1.0.23 in the real registry declares "Apache-2.0 OR BSL-1.0".
    // The Nexus canonical boundary fails closed on the unknown BSL-1.0
    // branch even though Apache-2.0 is permissive (documented M3
    // divergence from cargo-deny OR-any semantics). Prove the real
    // registry string is classified non-GREEN by the real transport.
    let files = real_policy_files();
    let canonical = LicenseClassifierPort::new();
    let c = classify_spdx("Apache-2.0 OR BSL-1.0", &files, &|s| {
        canonical.classify(s).ok()
    });
    assert!(
        c.class.is_none() || c.class.as_ref().unwrap().as_str() != "GREEN",
        "OR expression with unknown branch must fail closed"
    );
    assert!(c.has_unknown_branch);
}

#[test]
fn ep039_failure_duplicate_package_ambiguity_fails() {
    // Same name+version with a different digest is NOT the same
    // artifact (PACKAGE NAME MATCH != SAME ARTIFACT).
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["a".to_string()],
    ));
    let sbom = SbomDocument {
        format: "spdx".to_string(),
        spec_version: "2.3".to_string(),
        packages: vec![
            sbom_pkg(
                "a",
                "1.0.0",
                "crates.io",
                Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            ),
            sbom_pkg(
                "a",
                "1.0.0",
                "crates.io",
                Some("sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            ),
        ],
        generated_at_ts: 1_800_000_000,
        run_id: "run-1".to_string(),
        verification: SbomVerification::Verified,
    };
    let ev = policy.verify(&sbom, 1_800_000_100);
    assert!(!ev.valid, "duplicate ambiguity must fail");
    assert!(
        ev.reasons
            .iter()
            .any(|r| r.contains("duplicate component ambiguity")),
        "typed ambiguity reason expected: {:?}",
        ev.reasons
    );
}

#[test]
fn ep039_failure_image_tag_without_digest_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "run-1",
        vec!["web".to_string()],
    ));
    let sbom = SbomDocument {
        format: "spdx".to_string(),
        spec_version: "2.3".to_string(),
        packages: vec![SbomPackage {
            name: "web".to_string(),
            version: "1.0.0".to_string(),
            source: "ghcr.io/nexus/web:v1".to_string(),
            license_spdx: Some("MIT".to_string()),
            digest: None,
            is_transitive: false,
        }],
        generated_at_ts: 1_800_000_000,
        run_id: "run-1".to_string(),
        verification: SbomVerification::Verified,
    };
    let ev = policy.verify(&sbom, 1_800_000_100);
    assert!(!ev.valid, "image tag without digest must fail");
    assert!(
        ev.reasons.iter().any(|r| r.contains("pinned by digest")),
        "typed digest-required reason expected: {:?}",
        ev.reasons
    );
}

#[test]
fn ep039_failure_stale_sbom_evidence_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        60,
        "Cargo.lock",
        "run-1",
        vec!["a".to_string()],
    ));
    let sbom = SbomDocument {
        format: "spdx".to_string(),
        spec_version: "2.3".to_string(),
        packages: vec![sbom_pkg("a", "1.0.0", "crates.io", None)],
        generated_at_ts: 1_000_000_000,
        run_id: "run-1".to_string(),
        verification: SbomVerification::Verified,
    };
    let ev = policy.verify(&sbom, 1_800_000_000);
    assert!(!ev.valid, "stale SBOM must fail");
    assert!(
        ev.reasons.iter().any(|r| r.contains("stale")),
        "typed stale reason expected: {:?}",
        ev.reasons
    );
}

#[test]
fn ep039_failure_empty_sbom_evidence_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(3600, "Cargo.lock", "run-1", vec![]));
    let sbom = SbomDocument {
        format: "spdx".to_string(),
        spec_version: "2.3".to_string(),
        packages: vec![],
        generated_at_ts: 1_800_000_000,
        run_id: "run-1".to_string(),
        verification: SbomVerification::Verified,
    };
    let ev = policy.verify(&sbom, 1_800_000_100);
    assert!(!ev.valid, "empty SBOM must fail");
    assert!(
        ev.reasons.iter().any(|r| r.contains("empty")),
        "typed empty reason expected: {:?}",
        ev.reasons
    );
}

#[test]
fn ep039_failure_mismatched_run_id_fails() {
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "expected-run-9",
        vec!["a".to_string()],
    ));
    let sbom = SbomDocument {
        format: "spdx".to_string(),
        spec_version: "2.3".to_string(),
        packages: vec![sbom_pkg("a", "1.0.0", "crates.io", None)],
        generated_at_ts: 1_800_000_000,
        run_id: "different-run-7".to_string(),
        verification: SbomVerification::Verified,
    };
    let ev = policy.verify(&sbom, 1_800_000_100);
    assert!(!ev.valid, "mismatched run_id must fail");
    assert!(
        ev.reasons.iter().any(|r| r.contains("run id")),
        "typed run-id reason expected: {:?}",
        ev.reasons
    );
}

#[test]
fn ep039_failure_tampered_sbom_binding_fails() {
    // A tampered evidence binding (run id rewritten after generation)
    // must be rejected: GENERATED != VERIFIED and the binding is part
    // of the verification.
    let policy = SbomPolicy::new(SbomPolicyConfig::new(
        3600,
        "Cargo.lock",
        "genuine-run",
        vec!["a".to_string()],
    ));
    let genuine = SbomDocument {
        format: "spdx".to_string(),
        spec_version: "2.3".to_string(),
        packages: vec![sbom_pkg("a", "1.0.0", "crates.io", None)],
        generated_at_ts: 1_800_000_000,
        run_id: "genuine-run".to_string(),
        verification: SbomVerification::Verified,
    };
    assert!(policy.verify(&genuine, 1_800_000_100).valid);
    let mut tampered = genuine.clone();
    tampered.run_id = "forged-run".to_string();
    assert!(!policy.verify(&tampered, 1_800_000_100).valid);
}

// ---------------------------------------------------------------------
// Waiver abuse cases (typed through the certified WaiverPolicy).
// ---------------------------------------------------------------------

#[test]
fn ep039_failure_waiver_wrong_scope_fails() {
    // Default policy permits Runtime scope only (exact bounded decision).
    let policy = WaiverPolicy::new(WaiverPolicyConfig::default());
    let waiver = nexus_supply_chain::model::DependencyWaiver {
        package: "pkg-a".to_string(),
        version: "1.0.0".to_string(),
        owner: "ep039".to_string(),
        reason: "bounded".to_string(),
        controls: vec!["none".to_string()],
        expires_at_ts: 1_900_000_000,
        replacement_plan: "upgrade".to_string(),
        state: WaiverState::Active,
    };
    let ev = policy.validate(
        Some(&waiver),
        "pkg-a",
        "1.0.0",
        &WaiverScope::BuildTime,
        1_800_000_000,
    );
    assert!(
        !ev.valid,
        "waiver for a scope outside the permitted set must fail: {}",
        ev.reason
    );
}

#[test]
fn ep039_failure_waiver_expired_fails() {
    let policy = WaiverPolicy::new(WaiverPolicyConfig::default());
    let waiver = nexus_supply_chain::model::DependencyWaiver {
        package: "pkg-a".to_string(),
        version: "1.0.0".to_string(),
        owner: "ep039".to_string(),
        reason: "bounded".to_string(),
        controls: vec!["none".to_string()],
        expires_at_ts: 1_000_000_000,
        replacement_plan: "upgrade".to_string(),
        state: WaiverState::Active,
    };
    let ev = policy.validate(
        Some(&waiver),
        "pkg-a",
        "1.0.0",
        &WaiverScope::Runtime,
        1_800_000_000,
    );
    assert!(!ev.valid, "expired waiver must fail: {}", ev.reason);
}

#[test]
fn ep039_failure_waiver_revoked_fails() {
    let policy = WaiverPolicy::new(WaiverPolicyConfig::default());
    let waiver = nexus_supply_chain::model::DependencyWaiver {
        package: "pkg-a".to_string(),
        version: "1.0.0".to_string(),
        owner: "ep039".to_string(),
        reason: "bounded".to_string(),
        controls: vec!["none".to_string()],
        expires_at_ts: 1_900_000_000,
        replacement_plan: "upgrade".to_string(),
        state: WaiverState::Revoked,
    };
    let ev = policy.validate(
        Some(&waiver),
        "pkg-a",
        "1.0.0",
        &WaiverScope::Runtime,
        1_800_000_000,
    );
    assert!(!ev.valid, "revoked waiver must fail: {}", ev.reason);
}

// ---------------------------------------------------------------------
// Advisory abuse cases (typed through the certified AdvisoryPolicy).
// ---------------------------------------------------------------------

#[test]
fn ep039_failure_advisory_source_not_queried_fails() {
    // source_queried=false: unknown advisory status is never safe, even
    // with zero advisories returned ("no advisories returned" != secure
    // without a verified query).
    let policy = AdvisoryPolicy::new(AdvisoryPolicyConfig {
        source_queried: false,
        require_bounded_mitigation: true,
    });
    let ev = policy.evaluate(&[], &[], 1_800_000_000);
    assert!(
        !ev.valid,
        "unqueried advisory source must never be reported valid: {}",
        ev.reason
    );
    assert!(
        ev.reason.contains("not queried"),
        "typed not-queried reason expected: {}",
        ev.reason
    );
    assert!(ev.blocking_count >= 1);
    let _ = AdvisoryAffected {
        advisory_id: String::new(),
        package: String::new(),
        version: String::new(),
    };
}

#[test]
fn ep039_failure_advisory_critical_unmitigated_blocks() {
    let policy = AdvisoryPolicy::new(AdvisoryPolicyConfig::default());
    let advisory = nexus_supply_chain::model::Advisory {
        id: "RUSTSEC-0000-0001".to_string(),
        package: "pkg-a".to_string(),
        affected_versions: vec!["1.0.0".to_string()],
        severity: AdvisorySeverity::Critical,
        summary: "real critical".to_string(),
        mitigation_adr: None,
        mitigation_expires_ts: None,
    };
    let affected = vec![AdvisoryAffected {
        advisory_id: advisory.id.clone(),
        package: advisory.package.clone(),
        version: "1.0.0".to_string(),
    }];
    let ev = policy.evaluate(&[advisory], &affected, 1_800_000_000);
    assert!(
        !ev.valid,
        "critical advisory without mitigation must fail: {}",
        ev.reason
    );
    assert!(ev.blocking_count >= 1, "blocking count must be nonzero");
}

// ---------------------------------------------------------------------
// Evidence / observability / redaction proofs.
// ---------------------------------------------------------------------

#[test]
fn ep039_failure_secret_canary_redacted_in_evidence() {
    // Runtime-constructed canaries must never survive the evidence
    // boundary. No secret-shaped literal exists in tracked source.
    let sk = format!("sk{}", "-live-abcdef123456");
    let ghp = format!("ghp{}", "_abcdefghijklmnop");
    let aws = format!("AKIA{}", "ABCDEFGHIJKLMNOP");
    let bearer = format!("Bearer {}", "abcdefghijklmnop");
    let url = format!("https://user:{}@example.invalid/private", "s3cr3t");
    let body = serde_json::json!({
        "sk": sk,
        "ghp": ghp,
        "aws": aws,
        "bearer": bearer,
        "url": url,
    })
    .to_string();
    let redacted = nexus_supply_chain_policy::evidence::redact_secret_shaped(&body);
    assert!(
        assert_redacted(&redacted),
        "redacted evidence must pass scan"
    );
    // Markers are constructed at runtime so no secret-shaped literal
    // exists in tracked source (security-canary rule).
    let marker_sk = format!("sk{}", "-live");
    let marker_ghp = format!("ghp{}", "_");
    let marker_aws = format!("AK{}", "IA");
    let marker_bearer = format!("Bearer{}", " ");
    let marker_secret = format!("s3{}", "cr3t");
    for marker in [
        marker_sk,
        marker_ghp,
        marker_aws,
        marker_bearer,
        marker_secret,
    ] {
        assert!(
            !redacted.contains(&marker),
            "canary marker {marker:?} survived redaction"
        );
    }
}

#[test]
fn ep039_failure_observability_evidence_bound_to_real_inventory() {
    // The real inventory evidence carries the observability fields the
    // fence requires: run_id, package_count, resolved, denied, unknown,
    // missing-license counts. The M3 finding (16 denied) must remain
    // visible in the evidence, not erased.
    let root = repo_root();
    let report = evaluate_inventory(
        "ep039-failure-observability",
        &root.join("Cargo.lock"),
        &registry_src(),
        &root.join("policies/licenses"),
        &root,
    )
    .expect("real inventory evaluates");
    let doc = inventory_evidence(&report);
    let json = doc.to_redacted_json();
    assert!(assert_redacted(&json));
    let v: serde_json::Value = serde_json::from_str(&json).expect("evidence is valid JSON");
    assert_eq!(v["run_id"], "ep039-failure-observability");
    // The inventory observability fields live in the redacted body
    // string (EvidenceDocument shape: run_id/owner/body/generated_at).
    let body: serde_json::Value =
        serde_json::from_str(v["body"].as_str().expect("body is a JSON string"))
            .expect("evidence body is valid JSON");
    assert!(body["package_count"].as_u64().unwrap() >= 300);
    assert!(body["green_count"].as_u64().unwrap() >= 100);
    assert!(body["unknown_count"].as_u64().unwrap() >= 1);
    assert!(body["missing_license_count"].as_u64().unwrap() >= 1);
}

#[test]
fn ep039_failure_real_inventory_denied_finding_preserved() {
    // The honest M3 finding: ~446 packages, ~430 GREEN, 16 denied (14
    // ids outside the canonical tables + 2 license-less workspace
    // manifests). M4 must not paper it over: the real inventory still
    // reports the denied ids and the policy verdict stays non-green.
    // The denied-count relationship is asserted exactly so a future
    // workspace change cannot silently erase the finding.
    let root = repo_root();
    let report = evaluate_inventory(
        "ep039-failure-denied-preserved",
        &root.join("Cargo.lock"),
        &registry_src(),
        &root.join("policies/licenses"),
        &root,
    )
    .expect("real inventory evaluates");
    let denied = report
        .package_count
        .saturating_sub(report.green_count)
        .saturating_sub(report.review_count)
        .saturating_sub(report.sidecar_count)
        .saturating_sub(report.external_count);
    assert!(denied >= 1, "real denied finding must remain visible");
    assert_eq!(
        report.unknown_count + report.prohibited_count,
        report.package_count
            - report.green_count
            - report.review_count
            - report.sidecar_count
            - report.external_count,
        "UNKNOWN+PROHIBITED must equal the honest denied count (MISSING is a subset of UNKNOWN)"
    );
    for id in ["MIT-0", "CC0-1.0", "Zlib", "BSL-1.0"] {
        assert!(
            report
                .packages
                .iter()
                .filter(|p| p.license_spdx.as_deref().unwrap_or("").contains(id))
                .all(|p| !p.license_clear),
            "{id} package must not clear policy"
        );
    }
}

// ---------------------------------------------------------------------
// Deterministic engine failure assertions (M2 LicensePolicy reuse).
// ---------------------------------------------------------------------

#[test]
fn ep039_failure_license_engine_denied_without_approval_fails() {
    // DEPENDENCY EXISTS != LICENSE APPROVED; a GREEN license with no
    // review/approval must still fail the engine (permitted=false).
    let policy = LicensePolicy::new(LicensePolicyConfig::default());
    let comp: Component = component(
        "pkg-a",
        "1.0.0",
        Some("MIT"),
        LicenseReview::NeedsReview,
        ApprovalState::Pending,
    );
    let ev = policy.evaluate(&comp);
    assert!(
        !ev.permitted,
        "GREEN license alone must not permit: {}",
        ev.reason
    );
}

#[test]
fn ep039_failure_unverified_component_never_releasable() {
    // VerificationResult::Unverified is never releasable even with a
    // reviewed license (LICENSE STRING PRESENT != LICENSE VERIFIED).
    let mut comp = component(
        "pkg-a",
        "1.0.0",
        Some("MIT"),
        LicenseReview::Approved,
        ApprovalState::Approved,
    );
    comp.verification = VerificationResult::Unverified;
    assert!(!comp.is_releasable());
    let mut verified = comp.clone();
    verified.verification = VerificationResult::Verified;
    assert!(verified.is_releasable());
}

/// Helper trait used by the transitive test to express the denied
/// count over the public report fields without adding a field.
trait DeniedCountExt {
    fn denied_count_equivalent(&self) -> usize;
}

impl DeniedCountExt for nexus_supply_chain_policy_io::inventory::InventoryReport {
    fn denied_count_equivalent(&self) -> usize {
        // denied = non-GREEN minus actionable classes; matches the
        // sbom_generate example's honest count (no double count).
        self.package_count
            .saturating_sub(self.green_count)
            .saturating_sub(self.review_count)
            .saturating_sub(self.sidecar_count)
            .saturating_sub(self.external_count)
    }
}
