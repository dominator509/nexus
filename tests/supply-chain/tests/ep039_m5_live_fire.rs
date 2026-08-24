//! EP-039 M5 final live-fire proofs (SPEC-019; EP-039 M5 fence
//! `tests/supply-chain/`).
//!
//! Every test composes the REAL supply-chain journey on the REAL
//! workspace: real Cargo.lock, real cargo registry cache, checked-in
//! policies/licenses/ files, M1 contract, M2 deterministic engine, M3
//! transport, M4 scripts/sbom evidence semantics. No mocks, no
//! simulated providers, no in-memory dependency lists.
//!
//! The final decision is honest: the real 16-denied-package finding
//! keeps policy_passed=false and policy_verdict=NON_GREEN; the
//! evidence pipeline detects and reports that truthfully instead of
//! papering it over.

use std::io::Write;
use std::path::{Path, PathBuf};

use nexus_supply_chain_live_fire::{
    compose_live_fire, registry_src_root, verify_evidence, LiveFireReport,
};
use nexus_supply_chain_policy_io::assert_redacted;
use sha2::Digest;

/// Repository root from CARGO_MANIFEST_DIR (tests/supply-chain/).
fn repo_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .expect("tests parent")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn policy_dir() -> PathBuf {
    repo_root().join("policies/licenses")
}

/// Unique temp dir for evidence fixtures; removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("ep039-m5-{label}-{}-{ts}", std::process::id()));
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

fn sha256_hex(path: &Path) -> String {
    let bytes = std::fs::read(path).expect("readable");
    let mut hasher = sha2::Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// Compose the live-fire journey on the real repo with a unique run id
/// and a temp evidence path.
fn compose(label: &str) -> (LiveFireReport, TempDir) {
    let dir = TempDir::new(label);
    let run_id = format!("ep039-m5-{label}-{}", std::process::id());
    let git_commit = "live-fire-commit";
    let evidence = dir.path().join("evidence.json");
    let report = compose_live_fire(
        &repo_root(),
        &registry_src_root(),
        &policy_dir(),
        &run_id,
        git_commit,
        &evidence,
    )
    .expect("live-fire composition succeeds on the real repo");
    (report, dir)
}

#[test]
fn ep039_live_fire_full_composition_on_real_repo() {
    let (report, dir) = compose("full");
    let root = repo_root();

    // The real inventory is large and non-vacuous.
    assert!(report.package_count >= 300, "real lockfile inventory");
    assert_eq!(
        report.resolved_count + report.missing_license_count,
        report.package_count,
        "every package resolved or explicitly missing"
    );
    assert!(report.green_count >= 100, "real green majority");

    // The honest decision: non-green policy with a truthful verdict.
    assert!(report.denied_count >= 1, "real denied finding remains");
    assert_eq!(report.policy_verdict, "NON_GREEN");
    assert!(report.sbom_generated, "SBOM was generated");
    assert!(
        report.sbom_verified,
        "SBOM evidence binds to the current tree"
    );
    assert!(
        !report.policy_passed,
        "policy must NOT pass while denied findings stand"
    );
    assert_eq!(report.legal_clearance, "NOT_ASSERTED");
    assert!(
        !report.ship_approved,
        "ship must be blocked while policy is non-green"
    );

    // Observability fields present.
    assert!(report.verification_state == "VERIFIED");
    assert!(report.provenance_state == "NOT_VERIFIED");
    assert!(report.advisory_source_status == "NOT_QUERIED");
    assert!(report.redaction == "PASSED");

    // Fingerprints are real and bound to the tree.
    assert_eq!(
        report.lockfile_fingerprint,
        sha256_hex(&root.join("Cargo.lock"))
    );
    assert!(!report.policy_fingerprint.is_empty());
    assert!(!report.inventory_fingerprint.is_empty());
    assert!(!report.evidence_fingerprint.is_empty());

    // Evidence file + seal exist and are consistent.
    let evidence_path = dir.path().join("evidence.json");
    assert!(evidence_path.is_file());
    let seal = std::fs::read_to_string(dir.path().join("evidence.json.sha256")).expect("seal");
    assert_eq!(seal.trim(), report.evidence_fingerprint);
    assert_eq!(report.evidence_fingerprint, sha256_hex(&evidence_path));
}

#[test]
fn ep039_live_fire_evidence_verifies_against_current_tree() {
    let (report, dir) = compose("verify");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let v = verify_evidence(
        &dir.path().join("evidence.json"),
        &report.run_id,
        &report.git_commit,
        &report.lockfile_fingerprint,
        &report.policy_fingerprint,
        now,
        86_400,
    );
    assert!(v.is_verified(), "fresh evidence verifies: {:?}", v.reasons);
    assert_eq!(v.failure_class, "NONE");
}

#[test]
fn ep039_live_fire_stale_evidence_rejected() {
    let (report, dir) = compose("stale");
    let evidence_path = dir.path().join("evidence.json");
    // Rewrite generated_at to an ancient timestamp and reseal.
    let raw = std::fs::read_to_string(&evidence_path).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["generated_at_ts"] = serde_json::json!(1_000_000_000u64);
    write(&evidence_path, &v.to_string());
    write(
        &dir.path().join("evidence.json.sha256"),
        &format!("{}\n", sha256_hex(&evidence_path)),
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let v = verify_evidence(
        &evidence_path,
        &report.run_id,
        &report.git_commit,
        &report.lockfile_fingerprint,
        &report.policy_fingerprint,
        now,
        86_400,
    );
    assert!(!v.is_verified(), "stale evidence must be rejected");
    assert_eq!(
        v.failure_class, "STALE_EVIDENCE",
        "typed cause: {:?}",
        v.reasons
    );
}

#[test]
fn ep039_live_fire_tampered_evidence_rejected() {
    let (report, dir) = compose("tampered");
    let evidence_path = dir.path().join("evidence.json");
    // Tamper one field WITHOUT resealing: the seal must catch it.
    let raw = std::fs::read_to_string(&evidence_path).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["package_count"] = serde_json::json!(v["package_count"].as_u64().unwrap() + 1);
    write(&evidence_path, &v.to_string());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let v = verify_evidence(
        &evidence_path,
        &report.run_id,
        &report.git_commit,
        &report.lockfile_fingerprint,
        &report.policy_fingerprint,
        now,
        86_400,
    );
    assert!(!v.is_verified(), "tampered evidence must be rejected");
    assert_eq!(
        v.failure_class, "TAMPERED_EVIDENCE",
        "typed cause: {:?}",
        v.reasons
    );
}

#[test]
fn ep039_live_fire_mismatched_run_id_rejected() {
    let (report, dir) = compose("mismatch");
    let evidence_path = dir.path().join("evidence.json");
    let raw = std::fs::read_to_string(&evidence_path).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["run_id"] = serde_json::json!("ep039-m5-foreign-run");
    write(&evidence_path, &v.to_string());
    write(
        &dir.path().join("evidence.json.sha256"),
        &format!("{}\n", sha256_hex(&evidence_path)),
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // All bindings except run_id are genuine, so the typed failure
    // class is exactly MISMATCHED_RUN_ID.
    let v = verify_evidence(
        &evidence_path,
        "ep039-m5-expected-run",
        &report.git_commit,
        &report.lockfile_fingerprint,
        &report.policy_fingerprint,
        now,
        86_400,
    );
    assert!(!v.is_verified(), "mismatched run_id must be rejected");
    assert_eq!(
        v.failure_class, "MISMATCHED_RUN_ID",
        "typed cause: {:?}",
        v.reasons
    );
}

#[test]
fn ep039_live_fire_empty_evidence_rejected() {
    let dir = TempDir::new("empty");
    let evidence_path = dir.path().join("evidence.json");
    write(
        &evidence_path,
        r#"{"schema":"nexus.sbom.livefire.v1","package_count":0,"packages":[]}"#,
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let v = verify_evidence(
        &evidence_path,
        "run",
        "commit",
        "lock-fp",
        "policy-fp",
        now,
        86_400,
    );
    assert!(!v.is_verified(), "empty evidence must be rejected");
}

#[test]
fn ep039_live_fire_redaction_never_leaks_canaries() {
    // Runtime-constructed canaries must never survive the evidence
    // boundary; no secret-shaped literal exists in tracked source.
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
    assert!(assert_redacted(&redacted));
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
        assert!(!redacted.contains(&marker), "canary {marker:?} survived");
    }
}

#[test]
fn ep039_live_fire_inventory_deterministic() {
    let (r1, _d1) = compose("det-a");
    let (r2, _d2) = compose("det-b");
    assert_eq!(r1.package_count, r2.package_count);
    assert_eq!(r1.green_count, r2.green_count);
    assert_eq!(r1.denied_count, r2.denied_count);
    assert_eq!(r1.policy_verdict, r2.policy_verdict);
    assert_eq!(r1.inventory_fingerprint, r2.inventory_fingerprint);
}

#[test]
fn ep039_live_fire_writes_current_evidence() {
    // The gate sets EP039_M5_EVIDENCE/RUN_ID/GIT_COMMIT so the final
    // composition writes the canonical current-run evidence. Without
    // the env (workspace battery) the test still RUNS against a temp
    // path - it is never skipped, so a zero-evidence green is
    // impossible.
    let evidence_path = std::env::var("EP039_M5_EVIDENCE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::temp_dir().join(format!("ep039-m5-current-{}.json", std::process::id()))
        });
    let run_id = std::env::var("EP039_M5_RUN_ID")
        .unwrap_or_else(|_| format!("ep039-m5-local-{}", std::process::id()));
    let git_commit =
        std::env::var("EP039_M5_GIT_COMMIT").unwrap_or_else(|_| "unknown-commit".to_string());

    let report = compose_live_fire(
        &repo_root(),
        &registry_src_root(),
        &policy_dir(),
        &run_id,
        &git_commit,
        &evidence_path,
    )
    .expect("composition succeeds on the real repo");

    // The evidence must verify against the CURRENT tree (same run id,
    // commit, lockfile, policy) and must be honest.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let v = verify_evidence(
        &evidence_path,
        &run_id,
        &git_commit,
        &report.lockfile_fingerprint,
        &report.policy_fingerprint,
        now,
        86_400,
    );
    assert!(
        v.is_verified(),
        "current-run evidence must verify: {:?}",
        v.reasons
    );
    assert!(report.sbom_generated);
    assert!(!report.policy_passed, "policy stays non-green");
    assert_eq!(report.policy_verdict, "NON_GREEN");
    assert!(!report.ship_approved, "ship stays blocked");
    assert_eq!(report.legal_clearance, "NOT_ASSERTED");
}

#[test]
fn ep039_live_fire_real_denied_finding_preserved() {
    // The honest M3/M4 finding must survive the final composition: the
    // denied ids stay non-green, the license-less manifests stay
    // missing, and the denied-count relationship holds exactly.
    let (report, _dir) = compose("denied");
    assert!(report.denied_count >= 1);
    assert_eq!(
        report.unknown_count + report.prohibited_count,
        report.denied_count,
        "UNKNOWN+PROHIBITED == denied (MISSING is a subset of UNKNOWN)"
    );
    // Read the evidence package list and assert the denied ids cannot
    // clear policy.
    let evidence_path = report.evidence_path.clone();
    let raw = std::fs::read_to_string(&evidence_path).unwrap();
    let evidence: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let packages = evidence["packages"].as_array().unwrap();
    for id in ["MIT-0", "CC0-1.0", "Zlib", "BSL-1.0"] {
        let hits: Vec<&serde_json::Value> = packages
            .iter()
            .filter(|p| p["license_spdx"].as_str().unwrap_or("").contains(id))
            .collect();
        assert!(
            !hits.is_empty(),
            "{id} must still be present in the real inventory"
        );
        for p in hits {
            assert_eq!(
                p["license_clear"],
                serde_json::json!(false),
                "{id} must not clear"
            );
        }
    }
    assert_eq!(evidence["policy_passed"], serde_json::json!(false));
    assert_eq!(evidence["policy_verdict"], serde_json::json!("NON_GREEN"));
    assert_eq!(evidence["ship_approved"], serde_json::json!(false));
}
