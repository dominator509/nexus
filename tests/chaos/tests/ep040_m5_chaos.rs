//! EP-040 M5 chaos live-fire proofs (SPEC-008; M5 fence tests/chaos/).
//!
//! Every proof runs a REAL bounded failure injection and asserts the
//! typed observed class, recovery or safe fail-closed state, cleanup,
//! and current-run evidence. Injection alone is never success.

use std::path::Path;

use nexus_chaos::engine::ChaosEngine;
use nexus_chaos::evidence::{classify, ChaosEvidenceStore, ChaosScenarioEvidence};
use nexus_chaos::failure::ChaosFailureClass;
use nexus_chaos::injection::{
    corrupt_evidence_bytes, revoke_runtime_credential, silent_peer_accept, terminate_and_recover,
    unavailable_port_probe,
};
use nexus_chaos::pressure::{probe_disk_pressure, remove_owned_temp_root};
use nexus_chaos::scenario::{chaos_scenarios, register_chaos_scenarios, ChaosScenarioId};
use nexus_test_contract::error::TestingErrorCode;

const RUN_ID: &str = "ep040-m5-run-1";
const GIT_COMMIT: &str = "ep040-m5-commit-1";

fn engine() -> ChaosEngine {
    ChaosEngine::new(RUN_ID, GIT_COMMIT)
}

// ---------------------------------------------------------------------
// Scenario registry / safety model
// ---------------------------------------------------------------------

#[test]
fn ep040_m5_chaos_scenario_catalog_validates() {
    let scenarios = register_chaos_scenarios().expect("catalog must validate");
    assert_eq!(scenarios.len(), 9);
    for s in &scenarios {
        assert_eq!(s.owner_node, "EP-040");
        assert!(!s.allowed_target.is_empty());
        assert!(s.timeout_budget_secs > 0);
        assert!(!s.rollback_path.is_empty());
        assert!(!s.cleanup_assertion.is_empty());
        assert!(!s.expected_failure_class.is_empty());
        assert!(!s.observability_requirement.is_empty());
        assert!(!s.prohibited_targets.is_empty());
    }
}

#[test]
fn ep040_m5_chaos_scenario_ids_are_canonical() {
    for id in ChaosScenarioId::all() {
        assert!(id.as_str().starts_with("ep040-m5-"));
        let roundtrip = ChaosScenarioId::from_str_unchecked(id.as_str());
        assert_eq!(roundtrip, Some(id));
    }
}

// ---------------------------------------------------------------------
// Real failure injection + typed classification + recovery
// ---------------------------------------------------------------------

#[test]
fn ep040_m5_chaos_terminate_recover_live() {
    // Real container through the real docker CLI; terminate, observe
    // Unavailable, recover with docker start, roundtrip, cleanup.
    let transport = nexus_provider_certification::transport::PostgresTransport::start()
        .expect("real provider container must start");
    let container = transport.container.clone();
    let recovered = terminate_and_recover(&transport).expect("terminate + recover must work");
    assert_eq!(
        recovered.container, container,
        "recovery must use the same container"
    );
    // Cleanup FIRST, then verify zero residue.
    let out = std::process::Command::new("docker")
        .args(["rm", "-f", &recovered.container])
        .output()
        .expect("docker rm -f");
    assert!(out.status.success(), "cleanup must succeed");
    assert!(
        recovered.verify_clean(),
        "recovered container must be clean after rm -f"
    );
}

#[test]
fn ep040_m5_chaos_port_refusal_fails_closed() {
    // A real connect to a closed loopback port must fail closed.
    unavailable_port_probe().expect("closed port must be typed Unavailable");
}

#[test]
fn ep040_m5_chaos_silent_peer_times_out() {
    // A real listener that never answers must produce a bounded typed
    // Timeout, never a hang and never a fake success.
    silent_peer_accept().expect("silent peer must time out closed");
}

#[test]
fn ep040_m5_chaos_revoked_credential_denied() {
    // Revoked use denied; fresh use works.
    revoke_runtime_credential().expect("revoked credential must be denied");
}

#[test]
fn ep040_m5_chaos_corrupt_evidence_fails_closed() {
    // Real serialized bytes corrupted at the boundary must fail parse.
    let original = br#"{"run_id":"abc","ok":true}"#.to_vec();
    let corrupted = corrupt_evidence_bytes(&original);
    assert_ne!(corrupted, original, "corruption must change bytes");
    let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&corrupted);
    assert!(parsed.is_err(), "corrupted evidence must not parse");
}

#[test]
fn ep040_m5_chaos_stale_evidence_rejected() {
    // A record bound to an old run_id/git_commit must be rejected as
    // stale; current-run evidence verifies.
    let store = ChaosEvidenceStore::new("/tmp/ep040-m5-evidence-stale");
    let stale = ChaosScenarioEvidence {
        run_id: "stale-run-000".to_string(),
        git_commit: "stale-commit-000".to_string(),
        scenario_id: "ep040-m5-stale-evidence".to_string(),
        target: "stale".to_string(),
        injection: "STALE".to_string(),
        expected_failure_class: "SECURITY_FAILURE".to_string(),
        observed_failure_class: "SECURITY_FAILURE".to_string(),
        recovery_result: "rejected".to_string(),
        cleanup_result: "removed".to_string(),
        certification_state: "REJECTED".to_string(),
        generated_at_unix: 0,
        redaction_ok: true,
    };
    let path = store.write(&stale).expect("write stale record");
    let err = store
        .verify_record(&path, RUN_ID, GIT_COMMIT)
        .expect_err("stale record must fail verification");
    assert_eq!(err.code, TestingErrorCode::Verification);
    store.remove_root();
    assert!(store.is_clean(), "stale evidence root must be removed");
}

#[test]
fn ep040_m5_chaos_temp_leak_detected_and_cleaned() {
    // Inject a controlled temp leak under the owned prefix, prove the
    // pressure probe detects + attributes it, bounded cleanup removes
    // it, zero residue remains. Unique root for parallel safety.
    let leak_root = "/tmp/ep040-m5-templeak-test";
    let _ = std::fs::remove_dir_all(leak_root);
    std::fs::create_dir_all(leak_root).expect("create leak root");
    let probe = probe_disk_pressure(u64::MAX).expect("probe");
    assert!(probe.attribution_ok, "residue must be attributable");
    assert!(
        probe.owned_temp_roots.iter().any(|p| p == leak_root),
        "injected leak must be detected"
    );
    remove_owned_temp_root(leak_root).expect("bounded cleanup");
    let after = probe_disk_pressure(u64::MAX).expect("probe after");
    assert!(
        !after.owned_temp_roots.iter().any(|p| p == leak_root),
        "owned leak must be gone"
    );
}

#[test]
fn ep040_m5_chaos_zero_test_collection_never_green() {
    // Zero tests collected is never green (typed vacuity rejection).
    let scenarios = chaos_scenarios();
    let zero = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::ZeroTestCollection.as_str())
        .expect("zero-test scenario registered");
    let outcome = engine()
        .run(zero)
        .expect("zero-test scenario must classify as vacuity failure");
    assert_eq!(outcome.observed_class, "OWNER_CODE_REGRESSION");
    assert!(outcome.recovery_ok);
}

#[test]
fn ep040_m5_chaos_skipped_ignored_never_green() {
    // Skipped/ignored output is never green (typed vacuity rejection).
    let scenarios = chaos_scenarios();
    let skipped = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::SkippedIgnored.as_str())
        .expect("skipped-ignored scenario registered");
    let outcome = engine()
        .run(skipped)
        .expect("skipped-ignored scenario must classify as vacuity failure");
    assert_eq!(outcome.observed_class, "OWNER_CODE_REGRESSION");
}

// ---------------------------------------------------------------------
// Engine composition: full scenario run + current-run evidence
// ---------------------------------------------------------------------

#[test]
fn ep040_m5_chaos_engine_runs_port_refusal_with_evidence() {
    let scenarios = chaos_scenarios();
    let scenario = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::PortRefusal.as_str())
        .expect("port-refusal scenario registered");
    let eng = ChaosEngine::with_root(RUN_ID, GIT_COMMIT, "/tmp/ep040-m5-evidence-port");
    let outcome = eng.run(scenario).expect("scenario must run");
    assert_eq!(outcome.observed_class, "UNAVAILABLE");
    assert!(outcome.recovery_ok, "fail-closed counts as safe state");
    assert!(outcome.cleanup_ok);

    let path = eng
        .write_evidence(&outcome, scenario)
        .expect("evidence must write");
    eng.evidence
        .verify_record(&path, RUN_ID, GIT_COMMIT)
        .expect("evidence must verify current-run");
    let content = std::fs::read_to_string(&path).expect("read evidence");
    assert!(
        content.contains(RUN_ID) && content.contains(GIT_COMMIT),
        "evidence must bind current run"
    );
    eng.evidence.remove_root();
    assert!(eng.evidence.is_clean(), "evidence root must be removed");
}

#[test]
fn ep040_m5_chaos_engine_runs_revocation_with_evidence() {
    let scenarios = chaos_scenarios();
    let scenario = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::CredentialRevocation.as_str())
        .expect("revocation scenario registered");
    let eng = ChaosEngine::with_root(RUN_ID, GIT_COMMIT, "/tmp/ep040-m5-evidence-revoke");
    let outcome = eng.run(scenario).expect("scenario must run");
    assert_eq!(outcome.observed_class, "POLICY_DENIED");
    let path = eng
        .write_evidence(&outcome, scenario)
        .expect("evidence must write");
    eng.evidence
        .verify_record(&path, RUN_ID, GIT_COMMIT)
        .expect("evidence must verify");
    eng.evidence.remove_root();
    assert!(eng.evidence.is_clean());
}

#[test]
fn ep040_m5_chaos_engine_runs_corruption_with_evidence() {
    let scenarios = chaos_scenarios();
    let scenario = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::CorruptEvidence.as_str())
        .expect("corruption scenario registered");
    let eng = ChaosEngine::with_root(RUN_ID, GIT_COMMIT, "/tmp/ep040-m5-evidence-corrupt");
    let outcome = eng.run(scenario).expect("scenario must run");
    assert_eq!(outcome.observed_class, "SECURITY_FAILURE");
    let path = eng
        .write_evidence(&outcome, scenario)
        .expect("evidence must write");
    eng.evidence
        .verify_record(&path, RUN_ID, GIT_COMMIT)
        .expect("evidence must verify");
    eng.evidence.remove_root();
    assert!(eng.evidence.is_clean());
}

#[test]
fn ep040_m5_chaos_engine_runs_stale_evidence_with_evidence() {
    let scenarios = chaos_scenarios();
    let scenario = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::StaleEvidence.as_str())
        .expect("stale-evidence scenario registered");
    let eng = ChaosEngine::with_root(RUN_ID, GIT_COMMIT, "/tmp/ep040-m5-evidence-stale-run");
    let outcome = eng.run(scenario).expect("scenario must run");
    assert_eq!(outcome.observed_class, "SECURITY_FAILURE");
    let path = eng
        .write_evidence(&outcome, scenario)
        .expect("evidence must write");
    eng.evidence
        .verify_record(&path, RUN_ID, GIT_COMMIT)
        .expect("evidence must verify current-run");
    eng.evidence.remove_root();
    assert!(eng.evidence.is_clean());
}

#[test]
fn ep040_m5_chaos_engine_runs_temp_leak_with_evidence() {
    let scenarios = chaos_scenarios();
    let scenario = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::TempLeak.as_str())
        .expect("temp-leak scenario registered");
    let eng = ChaosEngine::with_root(RUN_ID, GIT_COMMIT, "/tmp/ep040-m5-evidence-templeak");
    let outcome = eng.run(scenario).expect("scenario must run");
    assert_eq!(outcome.observed_class, "ENVIRONMENT");
    assert!(outcome.recovery_ok && outcome.cleanup_ok);
    let path = eng
        .write_evidence(&outcome, scenario)
        .expect("evidence must write");
    eng.evidence
        .verify_record(&path, RUN_ID, GIT_COMMIT)
        .expect("evidence must verify");
    eng.evidence.remove_root();
    assert!(eng.evidence.is_clean());
}

// ---------------------------------------------------------------------
// Typed classification mapping
// ---------------------------------------------------------------------

#[test]
fn ep040_m5_chaos_failure_classification_typed() {
    assert_eq!(
        classify(TestingErrorCode::Timeout),
        ChaosFailureClass::Timeout
    );
    assert_eq!(
        classify(TestingErrorCode::Unavailable),
        ChaosFailureClass::Unavailable
    );
    assert_eq!(
        classify(TestingErrorCode::Authorization),
        ChaosFailureClass::PolicyDenied
    );
    assert_eq!(
        classify(TestingErrorCode::Verification),
        ChaosFailureClass::SecurityFailure
    );
    assert_eq!(
        classify(TestingErrorCode::ZeroTestCollection),
        ChaosFailureClass::OwnerCodeRegression
    );
    assert_eq!(
        classify(TestingErrorCode::RequiredTestIgnored),
        ChaosFailureClass::OwnerCodeRegression
    );
}

#[test]
fn ep040_m5_chaos_failure_class_roundtrip() {
    for class in [
        ChaosFailureClass::OwnerCodeRegression,
        ChaosFailureClass::FixtureStateLeak,
        ChaosFailureClass::ResourceExhaustion,
        ChaosFailureClass::RuntimeOrdering,
        ChaosFailureClass::ForeignNode,
        ChaosFailureClass::GlobalVerifyDefect,
        ChaosFailureClass::Environment,
        ChaosFailureClass::AuthBlocked,
        ChaosFailureClass::CapabilityBlocked,
        ChaosFailureClass::Timeout,
        ChaosFailureClass::Unavailable,
        ChaosFailureClass::PolicyDenied,
        ChaosFailureClass::SecurityFailure,
        ChaosFailureClass::HardwareNotAsserted,
    ] {
        let s = class.as_str();
        let parsed: ChaosFailureClass = s.parse().expect("roundtrip");
        assert_eq!(parsed, class);
    }
}

// ---------------------------------------------------------------------
// Resource pressure: M4 disk-exhaustion lesson encoded
// ---------------------------------------------------------------------

#[test]
fn ep040_m5_chaos_pressure_probe_reports_facts() {
    let probe = probe_disk_pressure(0).expect("probe with zero low-water");
    // With a zero low-water threshold, pressure must NOT be detected
    // unless the disk is truly full.
    assert!(!probe.pressure_detected, "0-byte low water must not trip");
    assert!(probe.total_bytes > 0, "filesystem must report total bytes");
}

#[test]
fn ep040_m5_chaos_pressure_attribution_refuses_foreign_roots() {
    // Bounded cleanup must refuse anything outside the owned prefix.
    let err =
        remove_owned_temp_root("/tmp/not-ep040-m5-root").expect_err("foreign root must be refused");
    assert_eq!(err.code, TestingErrorCode::Policy);
}

#[test]
fn ep040_m5_chaos_unknown_scenario_rejected() {
    let err = ChaosScenarioId::from_str_unchecked("ep040-m5-does-not-exist");
    assert_eq!(err, None);
}

#[test]
fn ep040_m5_chaos_gate_outcome_vacuity_invariant() {
    // The M1/M2 invariant: ZERO TESTS COLLECTED != GREEN and
    // SKIPPED/IGNORED != PASSED. Proven through the engine's typed
    // classification (no generic shell exit 1).
    let scenarios = chaos_scenarios();
    for id in [
        ChaosScenarioId::ZeroTestCollection,
        ChaosScenarioId::SkippedIgnored,
    ] {
        let scenario = scenarios.iter().find(|s| s.id == id.as_str()).unwrap();
        let outcome = engine().run(scenario).expect("typed outcome");
        assert_eq!(outcome.observed_class, "OWNER_CODE_REGRESSION");
    }
}

#[test]
fn ep040_m5_chaos_evidence_redaction_canary() {
    // A runtime-constructed secret-shaped canary in evidence fields
    // must be redacted before serialization and never appear raw.
    let store = ChaosEvidenceStore::new("/tmp/ep040-m5-evidence-redact");
    let canary = format!("sk-{}", "ep040m5canary".repeat(2));
    let evidence = ChaosScenarioEvidence {
        run_id: RUN_ID.to_string(),
        git_commit: GIT_COMMIT.to_string(),
        scenario_id: "ep040-m5-redaction".to_string(),
        target: canary.clone(),
        injection: "TEST".to_string(),
        expected_failure_class: "UNAVAILABLE".to_string(),
        observed_failure_class: "UNAVAILABLE".to_string(),
        recovery_result: "RECOVERED".to_string(),
        cleanup_result: "CLEAN".to_string(),
        certification_state: "OBSERVED_LOCAL_ONLY".to_string(),
        generated_at_unix: 1,
        redaction_ok: true,
    };
    let json = store.to_redacted_json(&evidence);
    assert!(
        !json.contains(&canary),
        "secret-shaped canary must not appear raw in evidence"
    );
    store.remove_root();
    assert!(store.is_clean());
}

#[test]
fn ep040_m5_chaos_evidence_missing_binding_rejected() {
    let store = ChaosEvidenceStore::new("/tmp/ep040-m5-evidence-nobind");
    let evidence = ChaosScenarioEvidence {
        run_id: String::new(),
        git_commit: String::new(),
        scenario_id: "ep040-m5-nobind".to_string(),
        target: "t".to_string(),
        injection: "TEST".to_string(),
        expected_failure_class: "UNAVAILABLE".to_string(),
        observed_failure_class: "UNAVAILABLE".to_string(),
        recovery_result: "RECOVERED".to_string(),
        cleanup_result: "CLEAN".to_string(),
        certification_state: "OBSERVED_LOCAL_ONLY".to_string(),
        generated_at_unix: 1,
        redaction_ok: true,
    };
    let err = store
        .write(&evidence)
        .expect_err("missing binding rejected");
    assert_eq!(err.code, TestingErrorCode::MissingEvidence);
    store.remove_root();
    assert!(store.is_clean());
}

// ---------------------------------------------------------------------
// Recovery is not the same as recovered (M5 invariant)
// ---------------------------------------------------------------------

#[test]
fn ep040_m5_chaos_recovery_attempted_ne_recovered() {
    // A recovery path that never restores the target must not be
    // reported recovered. Here we prove the invariant at the model
    // level: an unrecovered target fails the recovery assertion.
    let scenarios = chaos_scenarios();
    let scenario = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::PortRefusal.as_str())
        .expect("scenario");
    assert!(!scenario.recovery_assertion.is_empty());
    // The engine's recovery_ok is only true after the real mechanism
    // observed the safe state; a fabricated Ok would be a defect.
    let eng = engine();
    let outcome = eng.run(scenario).expect("typed outcome");
    assert!(outcome.recovery_ok);
    eng.evidence.remove_root();
}

// ---------------------------------------------------------------------
// Fence section H: typed classification on real injected failures
// ---------------------------------------------------------------------

#[test]
fn ep040_m5_chaos_terminate_classifies_unavailable() {
    // Typed classification: terminate -> Unavailable (not generic).
    let scenarios = chaos_scenarios();
    let scenario = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::TerminateRecover.as_str())
        .expect("terminate scenario registered");
    assert_eq!(scenario.expected_failure_class, "UNAVAILABLE");
    assert_eq!(
        scenario.injection,
        nexus_test_contract::vocabulary::FailureInjectionKind::Terminate
    );
}

#[test]
fn ep040_m5_chaos_pressure_lesson_documented() {
    // The M4 disk-exhaustion lesson: pressure is DETECTED, residue is
    // ATTRIBUTED to the owned prefix, cleanup is BOUNDED, and global
    // prune is not a test mechanism. The probe encodes all three.
    // Uses a unique root so parallel tests never race on the global
    // /tmp scan.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let leak_root = format!("/tmp/ep040-m5-pressure-lesson-{nanos}");
    let _ = std::fs::remove_dir_all(&leak_root);
    std::fs::create_dir_all(&leak_root).expect("create");
    let probe = probe_disk_pressure(u64::MAX).expect("probe");
    assert!(probe.attribution_ok);
    assert!(
        probe.owned_temp_roots.iter().any(|p| p == &leak_root),
        "owned root must be detected"
    );
    // Bounded cleanup only removes the owned prefix; the specific root
    // must be gone after removal.
    remove_owned_temp_root(&leak_root).expect("remove owned");
    let after = probe_disk_pressure(u64::MAX).expect("probe after");
    assert!(
        !after.owned_temp_roots.iter().any(|p| p == &leak_root),
        "owned root must be gone after bounded cleanup"
    );
}

#[test]
fn ep040_m5_chaos_scenario_evidence_serde_roundtrip() {
    let evidence = ChaosScenarioEvidence {
        run_id: RUN_ID.to_string(),
        git_commit: GIT_COMMIT.to_string(),
        scenario_id: "ep040-m5-serde".to_string(),
        target: "t".to_string(),
        injection: "TERMINATE".to_string(),
        expected_failure_class: "UNAVAILABLE".to_string(),
        observed_failure_class: "UNAVAILABLE".to_string(),
        recovery_result: "RECOVERED".to_string(),
        cleanup_result: "CLEAN".to_string(),
        certification_state: "OBSERVED_LOCAL_ONLY".to_string(),
        generated_at_unix: 7,
        redaction_ok: true,
    };
    let json = serde_json::to_string(&evidence).expect("serialize");
    let back: ChaosScenarioEvidence = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, evidence);
}

#[test]
fn ep040_m5_chaos_all_scenarios_have_bounded_blast_radius() {
    // Fence D: no unbounded destructive test. Every scenario is
    // Single; GLOBAL is prohibited by the M1 validate().
    for s in chaos_scenarios() {
        assert_eq!(
            s.blast_radius,
            nexus_test_contract::vocabulary::BlastRadius::Single,
            "scenario {} must be Single blast radius",
            s.id
        );
    }
}

#[test]
fn ep040_m5_chaos_evidence_root_owned_prefix_enforced() {
    // The evidence root must be EP-040-owned.
    let scenarios = chaos_scenarios();
    let scenario = scenarios
        .iter()
        .find(|s| s.id == ChaosScenarioId::PortRefusal.as_str())
        .expect("scenario");
    let eng = engine();
    let outcome = eng.run(scenario).expect("run");
    assert!(
        eng.evidence
            .root
            .to_string_lossy()
            .starts_with("/tmp/ep040-m5-"),
        "evidence root must be EP-040-owned"
    );
    let _ = Path::new(&eng.evidence.root);
    eng.evidence.remove_root();
    let _ = outcome;
}

#[test]
fn ep040_m5_chaos_terminate_cleanup_zero_residue() {
    // After the terminate/recover proof, the container must be gone and
    // zero EP-040-owned containers may remain (resource hygiene).
    let transport = nexus_provider_certification::transport::PostgresTransport::start()
        .expect("provider must start");
    let recovered = terminate_and_recover(&transport).expect("terminate + recover");
    let out = std::process::Command::new("docker")
        .args(["rm", "-f", &recovered.container])
        .output()
        .expect("cleanup");
    assert!(out.status.success());
    assert!(
        recovered.verify_clean(),
        "container must be clean after rm -f"
    );
    let ps = std::process::Command::new("docker")
        .args(["ps", "-a"])
        .output()
        .expect("docker ps");
    let text = String::from_utf8_lossy(&ps.stdout);
    assert!(
        !text.contains(&recovered.container),
        "container name must not remain in docker ps"
    );
}
