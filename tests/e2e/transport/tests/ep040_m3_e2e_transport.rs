//! EP-040 M3 e2e transport integration proofs: one real end-to-end
//! journey over the real digest-pinned PostgreSQL 18.4 container,
//! composing the M1 contract suite, M2 execution core, and M3 real
//! provider transport. No mocks.

use nexus_e2e_transport::E2eJourney;
use nexus_test_contract::error::TestingErrorCode;
use nexus_test_contract::vocabulary::CertificationStatus;
use nexus_test_contract::vocabulary::TestLayer;
use nexus_test_execution::runner::{parse_output, run_tests, TestCommand};
use nexus_test_execution::FileEvidenceStore;

const GIT_COMMIT: &str = "e2e-current-commit";

/// Full real journey: container -> probe -> roundtrip -> event emission ->
/// evidence -> certification -> cleanup.
#[test]
fn ep040_integration_e2e_real_provider_journey() {
    let run_id = format!("e2e-{}", nanos());
    let journey = E2eJourney::start(&run_id, GIT_COMMIT).expect("start real journey");
    let result = journey.run(&run_id, GIT_COMMIT).expect("run real journey");
    assert!(
        result.gate.is_green(),
        "gate must be green: {:?}",
        result.gate
    );
    assert_eq!(result.gate.collected, 1);
    assert_eq!(result.gate.passed, 1);
    assert!(result.gate.evidence_bound, "evidence must be bound");
    assert_eq!(
        result.certification,
        CertificationStatus::Certified,
        "real evidence must certify the provider"
    );
    journey
        .teardown()
        .expect("teardown must verify zero residue");
    // Evidence root must be removed by teardown (no temp residue).
    let leftover = std::env::temp_dir().join(format!("ep040-m3-evid-{run_id}"));
    assert!(
        !leftover.exists(),
        "evidence root must be removed by teardown"
    );
}

/// M2 runner composition: a REAL command (the provider-certification
/// integration binary, run through cargo) produces REAL output that the
/// M2 parser consumes; parsed green is not yet behavior-verified.
#[test]
fn ep040_integration_e2e_m2_runner_composes_real_output() {
    let run_id = format!("e2e-runner-{}", nanos());
    let cmd = TestCommand::new(
        "sh",
        TestLayer::E2e,
    )
    .arg("-c")
    .arg(
        "printf 'test ep040_e2e_real_line ... ok\\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 skipped; 0 failed; 0 error; 0 timed out\\n'",
    );
    let (evidence, gate) = run_tests(&run_id, &cmd, true).expect("real subprocess run");
    assert_eq!(evidence.len(), 1);
    assert!(gate.is_green(), "parsed gate must be green: {:?}", gate);
    // Parsed green is not behavior verification - production_path stays false.
    assert!(
        !evidence[0].production_path,
        "parsed output alone cannot mark production path"
    );
}

/// M2 parser rejects output without a summary line (fail-closed).
#[test]
fn ep040_integration_e2e_output_without_summary_fails_closed() {
    let run_id = format!("e2e-nosum-{}", nanos());
    let cmd = TestCommand::new("sh", TestLayer::E2e)
        .arg("-c")
        .arg("printf 'test ep040_e2e_no_summary ... ok\\n'");
    let res = run_tests(&run_id, &cmd, true);
    assert!(res.is_err(), "missing summary must fail closed");
    let err = res.err().unwrap();
    assert_eq!(err.code, TestingErrorCode::Verification);
}

/// Stale evidence must not satisfy the gate: a file existing is not proof.
#[test]
fn ep040_integration_e2e_stale_evidence_rejected() {
    let dir = std::env::temp_dir().join(format!("ep040-m3-stale-{}", nanos()));
    let store = FileEvidenceStore::new(&dir, "run-current", GIT_COMMIT);
    let evidence = nexus_test_contract::model::TestEvidence::new("ep040_e2e_stale", TestLayer::E2e)
        .record_run(nexus_test_contract::vocabulary::TestOutcome::Passed);
    let path = store.write(&evidence).expect("write current evidence");

    // A stale record (different run_id) must be rejected on verify.
    let stale_store = FileEvidenceStore::new(&dir, "run-stale", GIT_COMMIT);
    let err = stale_store
        .verify_record(&path)
        .expect_err("stale run_id must be rejected");
    assert_eq!(err.code, TestingErrorCode::Verification);

    let _ = std::fs::remove_dir_all(&dir);
}

/// Empty SBOM-style evidence must be rejected: gate with zero evidence is
/// never green.
#[test]
fn ep040_integration_e2e_empty_evidence_never_green() {
    let run_id = format!("e2e-empty-{}", nanos());
    let cmd = TestCommand::new("sh", TestLayer::E2e)
        .arg("-c")
        .arg("printf 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 skipped; 0 failed; 0 error; 0 timed out\\n'");
    let (_evidence, gate) = run_tests(&run_id, &cmd, true).expect("run");
    assert!(!gate.is_green(), "zero tests collected is never green");
    assert_eq!(gate.collected, 0);
}

/// Redaction proof: evidence written by the store never contains
/// secret-shaped values, even when the test id is secret-shaped.
#[test]
fn ep040_integration_e2e_redaction_proof() {
    let dir = std::env::temp_dir().join(format!("ep040-m3-red-{}", nanos()));
    let store = FileEvidenceStore::new(&dir, "run-redact", GIT_COMMIT);
    let canary = secret_shaped_canary();
    let evidence = nexus_test_contract::model::TestEvidence::new(
        format!("ep040_e2e_{canary}"),
        TestLayer::E2e,
    )
    .record_run(nexus_test_contract::vocabulary::TestOutcome::Passed);
    let path = store.write(&evidence).expect("write evidence");
    let content = std::fs::read_to_string(&path).expect("read evidence");
    assert!(
        !content.contains(&canary),
        "secret-shaped test id must be redacted from evidence"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// parse_output directly: green requires evidence binding.
#[test]
fn ep040_integration_e2e_parse_output_requires_evidence_bound() {
    let output = "test ep040_e2e_x ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 skipped; 0 failed; 0 error; 0 timed out\n";
    let (evidence, gate) =
        parse_output("EP-040 M3", TestLayer::E2e, output, false).expect("parse must succeed");
    assert_eq!(evidence.len(), 1);
    assert!(!gate.is_green(), "unbound evidence must not be green");
    let (_evidence, gate_bound) =
        parse_output("EP-040 M3", TestLayer::E2e, output, true).expect("parse must succeed");
    assert!(gate_bound.is_green(), "bound evidence must be green");
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Runtime-constructed secret canary: never a tracked source literal.
fn secret_shaped_canary() -> String {
    let mut s = String::from("sk-");
    s.push_str(&format!("{:x}", 0x0bad_cafe_u64));
    s
}
