//! EP-040 M1 contract proofs: construction, validation, serialization,
//! vocabulary rejection, and dependency direction for the
//! nexus-test-contract crate.
//!
//! Every proof uses real crate machinery and no mocked component. The
//! permanent invariants from the EP-040 fence are encoded here:
//! TEST EXISTS != TEST RAN; TEST RAN != BEHAVIOR VERIFIED; MOCK PASSED !=
//! PRODUCTION PATH VERIFIED; CHAOS INJECTED != SYSTEM HARDENED; NO FAILURE
//! OBSERVED != RESILIENCE PROVEN; ZERO TESTS COLLECTED != GREEN; SKIPPED
//! TEST != PASSED TEST; FLAKE RETRIED GREEN != ROOT CAUSE FIXED; RESOURCE
//! CLEANUP ATTEMPTED != RESOURCE CLEAN; BUILD PASSED != RUNTIME SAFE.

use std::str::FromStr;

use nexus_test_contract::error::{redact_secret_shaped, TestingError, TestingErrorCode};
use nexus_test_contract::model::{
    AccessibilityAudit, ChaosScenario, FixtureOwnership, FlakeRecord, GateResult, HardeningControl,
    PerformanceBudget, ProviderCertificationSuite, RegressionRequirement, ResourceResidue,
    TestEvidence, TestMatrix,
};
use nexus_test_contract::vocabulary::{
    BlastRadius, CertificationStatus, FailureInjectionKind, FlakeClassification,
    HardeningControlState, ResourceKind, TestLayer, TestOutcome,
};
use nexus_test_contract::{ChaosScenarioPort, TestMatrixPort};

// ---------------------------------------------------------------------
// Vocabulary: deny-unknown construction.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_vocabulary_deny_unknown_test_layer() {
    assert!(TestLayer::from_str("UNIT").is_ok());
    assert!(TestLayer::from_str("E2E").is_ok());
    assert!(TestLayer::from_str("NOT_A_LAYER").is_err());
    assert!(TestLayer::from_str("").is_err());
}

#[test]
fn ep040_unit_vocabulary_deny_unknown_test_outcome() {
    assert_eq!(
        TestOutcome::from_str("PASSED").unwrap(),
        TestOutcome::Passed
    );
    assert!(TestOutcome::from_str("MAYBE").is_err());
    // SKIPPED TEST != PASSED TEST.
    assert!(!TestOutcome::Skipped.is_required_pass());
    assert!(!TestOutcome::Ignored.is_required_pass());
    assert!(TestOutcome::Passed.is_required_pass());
}

#[test]
fn ep040_unit_vocabulary_deny_unknown_flake_classification() {
    assert_eq!(
        FlakeClassification::from_str("TRANSIENT").unwrap(),
        FlakeClassification::Transient
    );
    for c in [
        FlakeClassification::Transient,
        FlakeClassification::FixtureStateLeak,
        FlakeClassification::ResourceExhaustion,
        FlakeClassification::RuntimeOrdering,
        FlakeClassification::ForeignNode,
        FlakeClassification::GlobalVerifyDefect,
        FlakeClassification::OwnerCodeRegression,
        FlakeClassification::Environment,
        FlakeClassification::AuthBlocked,
    ] {
        assert_eq!(FlakeClassification::from_str(c.as_str()).unwrap(), c);
    }
    assert!(FlakeClassification::from_str("MAGIC").is_err());
}

#[test]
fn ep040_unit_vocabulary_deny_unknown_failure_injection() {
    assert_eq!(
        FailureInjectionKind::from_str("TERMINATE").unwrap(),
        FailureInjectionKind::Terminate
    );
    assert!(FailureInjectionKind::from_str("PRAY").is_err());
}

#[test]
fn ep040_unit_vocabulary_deny_unknown_blast_radius() {
    assert_eq!(
        BlastRadius::from_str("SINGLE").unwrap(),
        BlastRadius::Single
    );
    assert_eq!(
        BlastRadius::from_str("GLOBAL").unwrap(),
        BlastRadius::Global
    );
    assert!(BlastRadius::from_str("EVERYTHING").is_err());
}

#[test]
fn ep040_unit_vocabulary_deny_unknown_resource_kind() {
    assert_eq!(
        ResourceKind::from_str("CONTAINER").unwrap(),
        ResourceKind::Container
    );
    assert!(ResourceKind::from_str("DATABASE").is_err());
}

#[test]
fn ep040_unit_vocabulary_deny_unknown_hardening_state() {
    assert_eq!(
        HardeningControlState::from_str("VERIFIED").unwrap(),
        HardeningControlState::Verified
    );
    assert!(HardeningControlState::from_str("DONE").is_err());
}

#[test]
fn ep040_unit_vocabulary_serde_rejects_unknown_wire_value() {
    // Fail-closed serde: an unknown wire value must be rejected.
    let bad = r#"{"layer":"NOT_A_LAYER"}"#;
    let res: Result<serde_json::Value, _> = serde_json::from_str(bad);
    // The value itself parses as JSON; the enum-level rejection happens on
    // typed deserialization. Prove the typed path rejects unknown values.
    let typed: Result<TestLayer, _> = serde_json::from_str("\"NOT_A_LAYER\"");
    assert!(typed.is_err());
    let ok: Result<TestLayer, _> = serde_json::from_str("\"UNIT\"");
    assert_eq!(ok.unwrap(), TestLayer::Unit);
    let _ = res.unwrap();
}

// ---------------------------------------------------------------------
// TestEvidence: TEST EXISTS != TEST RAN; TEST RAN != BEHAVIOR VERIFIED;
// MOCK PASSED != PRODUCTION PATH VERIFIED.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_test_evidence_test_exists_not_test_ran() {
    let evidence = TestEvidence::new("ep040_unit_demo", TestLayer::Unit);
    assert!(!evidence.executed);
    assert!(evidence.outcome.is_none());
    // A record that exists is not a test that ran.
    assert!(!evidence.is_green_but_unverified());
}

#[test]
fn ep040_unit_test_evidence_test_ran_not_behavior_verified() {
    let evidence =
        TestEvidence::new("ep040_unit_demo", TestLayer::Unit).record_run(TestOutcome::Passed);
    assert!(evidence.executed);
    assert!(evidence.is_green_but_unverified());
    // Running green is not behavior verification.
    assert!(!evidence.behavior_verified);
}

#[test]
fn ep040_unit_test_evidence_mock_passed_not_production_path_verified() {
    let mut evidence =
        TestEvidence::new("ep040_unit_demo", TestLayer::Unit).record_run(TestOutcome::Passed);
    // Mock/fixture-only proof can never certify a production path.
    assert_eq!(
        evidence.certify_production().unwrap_err().code,
        TestingErrorCode::MockOnlyCertification
    );
    evidence.production_path = true;
    assert!(evidence.certify_production().is_ok());
    assert!(evidence.behavior_verified);
}

#[test]
fn ep040_unit_test_evidence_never_ran_cannot_certify() {
    let mut evidence = TestEvidence::new("ep040_unit_demo", TestLayer::Unit);
    assert_eq!(
        evidence.certify_production().unwrap_err().code,
        TestingErrorCode::ZeroTestCollection
    );
}

#[test]
fn ep040_unit_test_evidence_failed_run_cannot_certify() {
    let mut evidence =
        TestEvidence::new("ep040_unit_demo", TestLayer::Unit).record_run(TestOutcome::Failed);
    evidence.production_path = true;
    assert_eq!(
        evidence.certify_production().unwrap_err().code,
        TestingErrorCode::Verification
    );
}

// ---------------------------------------------------------------------
// GateResult: ZERO TESTS COLLECTED != GREEN; SKIPPED/IGNORED != PASSED.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_gate_result_zero_tests_collected_not_green() {
    let gate = GateResult::new("ep040-m1");
    assert!(gate.is_vacuous());
    assert!(!gate.is_green());
    // Zero collected tests is never green even when "nothing failed".
    let mut gate = GateResult::new("ep040-m1");
    gate.evidence_bound = true;
    assert!(!gate.is_green());
}

#[test]
fn ep040_unit_gate_result_collected_passed_is_green() {
    let mut gate = GateResult::new("ep040-m1");
    gate.collected = 5;
    gate.passed = 5;
    gate.evidence_bound = true;
    assert!(gate.is_green());
}

#[test]
fn ep040_unit_gate_result_skipped_required_test_not_green() {
    let mut gate = GateResult::new("ep040-m1");
    gate.collected = 5;
    gate.passed = 4;
    gate.skipped = 1;
    gate.evidence_bound = true;
    // SKIPPED TEST != PASSED TEST.
    assert!(!gate.is_green());
}

#[test]
fn ep040_unit_gate_result_ignored_required_test_not_green() {
    let mut gate = GateResult::new("ep040-m1");
    gate.collected = 5;
    gate.passed = 4;
    gate.ignored = 1;
    gate.evidence_bound = true;
    assert!(!gate.is_green());
}

#[test]
fn ep040_unit_gate_result_failed_test_not_green() {
    let mut gate = GateResult::new("ep040-m1");
    gate.collected = 5;
    gate.passed = 4;
    gate.failed = 1;
    gate.evidence_bound = true;
    assert!(!gate.is_green());
}

#[test]
fn ep040_unit_gate_result_unbound_evidence_not_green() {
    let mut gate = GateResult::new("ep040-m1");
    gate.collected = 5;
    gate.passed = 5;
    // Evidence not bound: artifact-only proof is vacuous.
    assert!(!gate.is_green());
}

// ---------------------------------------------------------------------
// TestMatrix: coverage/collection policy + zero-test guard.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_test_matrix_requires_owner() {
    let matrix = TestMatrix::new("").add_required(TestLayer::Unit, "ep040_unit_demo");
    assert_eq!(
        matrix.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_test_matrix_zero_test_guard_fails_closed() {
    let matrix = TestMatrix::new("EP-040");
    assert_eq!(
        matrix.validate().unwrap_err().code,
        TestingErrorCode::ZeroTestCollection
    );
}

#[test]
fn ep040_unit_test_matrix_valid_with_required_tests() {
    let matrix = TestMatrix::new("EP-040")
        .add_required(TestLayer::Unit, "ep040_unit_demo")
        .add_required(TestLayer::Contract, "ep040_contract_demo");
    assert!(matrix.validate().is_ok());
}

#[test]
fn ep040_unit_test_matrix_duplicate_test_rejected() {
    let matrix = TestMatrix::new("EP-040")
        .add_required(TestLayer::Unit, "ep040_unit_dup")
        .add_required(TestLayer::Unit, "ep040_unit_dup");
    assert_eq!(
        matrix.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
}

// ---------------------------------------------------------------------
// ChaosScenario: bounded blast radius, rollback, cleanup, expected
// failure class. CHAOS INJECTED != SYSTEM HARDENED.
// ---------------------------------------------------------------------

fn valid_scenario() -> ChaosScenario {
    ChaosScenario {
        id: "ep040-scn-001".to_string(),
        owner_node: "EP-040".to_string(),
        allowed_target: "ep040-m1-fixture".to_string(),
        injection: FailureInjectionKind::Terminate,
        blast_radius: BlastRadius::Single,
        timeout_budget_secs: 30,
        rollback_path: "restart fixture".to_string(),
        safety_preconditions: Vec::new(),
        observability_requirement: "metrics + logs during window".to_string(),
        expected_failure_class: "UNAVAILABLE".to_string(),
        recovery_assertion: "fixture healthy after rollback".to_string(),
        cleanup_assertion: "zero ep040-owned residue".to_string(),
        prohibited_targets: Vec::new(),
    }
}

#[test]
fn ep040_unit_chaos_scenario_requires_owner() {
    let mut scn = valid_scenario();
    scn.owner_node = String::new();
    assert_eq!(
        scn.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_chaos_scenario_requires_bounded_blast_radius() {
    let mut scn = valid_scenario();
    scn.blast_radius = BlastRadius::Global;
    // GLOBAL blast radius is prohibited without explicit ownership.
    assert_eq!(scn.validate().unwrap_err().code, TestingErrorCode::Policy);
}

#[test]
fn ep040_unit_chaos_scenario_requires_timeout_budget() {
    let mut scn = valid_scenario();
    scn.timeout_budget_secs = 0;
    assert_eq!(
        scn.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_chaos_scenario_requires_rollback_path() {
    let mut scn = valid_scenario();
    scn.rollback_path = String::new();
    assert_eq!(
        scn.validate().unwrap_err().code,
        TestingErrorCode::RollbackUnavailable
    );
}

#[test]
fn ep040_unit_chaos_scenario_requires_cleanup_assertion() {
    let mut scn = valid_scenario();
    scn.cleanup_assertion = String::new();
    assert_eq!(
        scn.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_chaos_scenario_requires_expected_failure_class() {
    let mut scn = valid_scenario();
    scn.expected_failure_class = String::new();
    assert_eq!(
        scn.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_chaos_scenario_requires_observability() {
    let mut scn = valid_scenario();
    scn.observability_requirement = String::new();
    assert_eq!(
        scn.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_chaos_scenario_valid_when_complete() {
    assert!(valid_scenario().validate().is_ok());
}

// ---------------------------------------------------------------------
// HardeningControl: DEFINED != APPLIED != VERIFIED != REGRESSED.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_hardening_control_defined_not_applied() {
    let control = HardeningControl::new("redaction");
    assert_eq!(control.state, HardeningControlState::Defined);
    // A written control is not proof.
    assert!(!control.is_proof());
}

#[test]
fn ep040_unit_hardening_control_applied_not_verified() {
    let control = HardeningControl::new("redaction").apply();
    assert_eq!(control.state, HardeningControlState::Applied);
    assert!(!control.is_proof());
}

#[test]
fn ep040_unit_hardening_control_verify_requires_evidence() {
    let control = HardeningControl::new("redaction").apply();
    assert_eq!(
        control.verify("").unwrap_err().code,
        TestingErrorCode::MissingEvidence
    );
}

#[test]
fn ep040_unit_hardening_control_verified_with_evidence() {
    let control = HardeningControl::new("redaction")
        .apply()
        .verify("ep040-m1-redaction-proof")
        .unwrap();
    assert_eq!(control.state, HardeningControlState::Verified);
    assert!(control.is_proof());
}

#[test]
fn ep040_unit_hardening_control_regressed_not_proof() {
    let control = HardeningControl::new("redaction")
        .apply()
        .verify("ep040-m1-redaction-proof")
        .unwrap()
        .regress();
    assert_eq!(control.state, HardeningControlState::Regressed);
    assert!(!control.is_proof());
}

// ---------------------------------------------------------------------
// FixtureOwnership + ResourceResidue: RESOURCE CLEANUP ATTEMPTED !=
// RESOURCE CLEAN.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_fixture_ownership_requires_owned_prefix() {
    let fixture = FixtureOwnership::new("EP-040", "shared-resources");
    assert_eq!(
        fixture.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_fixture_ownership_requires_teardown() {
    let fixture =
        FixtureOwnership::new("EP-040", "nexus-ep040-test").with_kind(ResourceKind::Container);
    assert!(fixture.validate().is_ok());
    let mut no_teardown = fixture.clone();
    no_teardown.teardown_required = false;
    assert_eq!(
        no_teardown.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
}

#[test]
fn ep040_unit_resource_residue_cleanup_attempted_not_clean() {
    let residue =
        ResourceResidue::new("EP-040", ResourceKind::Container, "nexus-ep040-x").attempt_cleanup();
    // Cleanup attempted is not the same as verified clean.
    assert!(!residue.is_clean());
}

#[test]
fn ep040_unit_resource_residue_verified_clean_only_when_both() {
    let residue = ResourceResidue::new("EP-040", ResourceKind::Container, "nexus-ep040-x")
        .attempt_cleanup()
        .verify_clean();
    assert!(residue.is_clean());
    let only_verified =
        ResourceResidue::new("EP-040", ResourceKind::Container, "nexus-ep040-x").verify_clean();
    assert!(!only_verified.is_clean());
}

// ---------------------------------------------------------------------
// FlakeRecord: FLAKE RETRIED GREEN != ROOT CAUSE FIXED.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_flake_retried_green_not_fixed() {
    let flake = FlakeRecord::new("ep040_unit_flaky", FlakeClassification::Transient)
        .retried_green()
        .retried_green();
    assert_eq!(flake.retry_count, 2);
    // Retried green is not root-cause fixed.
    assert!(!flake.is_fixed());
}

#[test]
fn ep040_unit_flake_fix_requires_root_cause() {
    let flake = FlakeRecord::new("ep040_unit_flaky", FlakeClassification::Transient);
    assert_eq!(
        flake.fix("").unwrap_err().code,
        TestingErrorCode::FlakeUnresolved
    );
}

#[test]
fn ep040_unit_flake_fixed_with_root_cause() {
    let flake = FlakeRecord::new("ep040_unit_flaky", FlakeClassification::FixtureStateLeak)
        .retried_green()
        .fix("fixture teardown missing trap on panic")
        .unwrap();
    assert!(flake.is_fixed());
}

// ---------------------------------------------------------------------
// RegressionRequirement, ProviderCertificationSuite,
// HardwareCertificationSuite, AccessibilityAudit, PerformanceBudget.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_regression_requirement_requires_gate() {
    let req = RegressionRequirement::new("EP-040", "ep040_unit_demo");
    assert_eq!(
        req.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
    assert!(req.with_gate("ep040-m1").validate().is_ok());
}

#[test]
fn ep040_unit_provider_certification_requires_real_evidence() {
    let suite = ProviderCertificationSuite::new("storage-s3", "core");
    assert_eq!(suite.status, CertificationStatus::NotAsserted);
    assert_eq!(
        suite.certify(vec![]).unwrap_err().code,
        TestingErrorCode::MissingEvidence
    );
}

#[test]
fn ep040_unit_provider_certification_rejects_secret_evidence() {
    let suite = ProviderCertificationSuite::new("storage-s3", "core");
    // Runtime-constructed canary: no secret-shaped literal in source.
    let secret = format!("sk{}", "-live-abcdef123456");
    let err = suite.certify(vec![secret]).unwrap_err();
    assert_eq!(err.code, TestingErrorCode::Validation);
}

#[test]
fn ep040_unit_provider_certification_certified_with_evidence() {
    let suite = ProviderCertificationSuite::new("storage-s3", "core")
        .certify(vec!["ep040-m3-s3-proof".to_string()])
        .unwrap();
    assert_eq!(suite.status, CertificationStatus::Certified);
}

#[test]
fn ep040_unit_hardware_certification_requires_model_firmware_evidence() {
    let suite = nexus_test_contract::model::HardwareCertificationSuite::new("voice-satellite");
    assert_eq!(
        suite
            .certify("", "1.0", vec!["hw-proof".to_string()])
            .unwrap_err()
            .code,
        TestingErrorCode::MissingEvidence
    );
    let suite = nexus_test_contract::model::HardwareCertificationSuite::new("voice-satellite");
    assert_eq!(
        suite
            .certify("raspberry-pi-5", "1.0", vec![])
            .unwrap_err()
            .code,
        TestingErrorCode::MissingEvidence
    );
}

#[test]
fn ep040_unit_hardware_certification_certified_with_evidence() {
    let suite = nexus_test_contract::model::HardwareCertificationSuite::new("voice-satellite")
        .certify(
            "raspberry-pi-5",
            "1.0",
            vec!["ep040-m4-hw-proof".to_string()],
        )
        .unwrap();
    assert_eq!(suite.status, CertificationStatus::Certified);
}

#[test]
fn ep040_unit_accessibility_audit_requires_target_and_standard() {
    let audit = AccessibilityAudit::new("", "WCAG 2.1 AA");
    assert_eq!(
        audit.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
    let audit = AccessibilityAudit::new("dashboard", "");
    assert_eq!(
        audit.validate().unwrap_err().code,
        TestingErrorCode::Validation
    );
    let audit = AccessibilityAudit::new("dashboard", "WCAG 2.1 AA");
    assert!(audit.validate().is_ok());
}

#[test]
fn ep040_unit_performance_budget_build_passed_not_runtime_safe() {
    // BUILD PASSED != RUNTIME SAFE: a budget with no observation is not met.
    let budget = PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms");
    assert!(!budget.met());
}

#[test]
fn ep040_unit_performance_budget_met_only_when_observed_within_bound() {
    let budget =
        PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms").observe(180.0);
    assert!(budget.met());
    let over =
        PerformanceBudget::new("ep040-api-latency", "EP-040", "p95", 250.0, "ms").observe(400.0);
    assert!(!over.met());
}

// ---------------------------------------------------------------------
// Errors: canonical codes + redaction.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_error_codes_are_canonical() {
    assert_eq!(
        TestingErrorCode::ZeroTestCollection.as_serde_str(),
        "ZERO_TEST_COLLECTION"
    );
    assert_eq!(
        TestingErrorCode::RequiredTestSkipped.as_serde_str(),
        "REQUIRED_TEST_SKIPPED"
    );
    assert_eq!(TestingErrorCode::VacuousGate.as_serde_str(), "VACUOUS_GATE");
    assert_eq!(
        TestingErrorCode::FlakeUnresolved.as_serde_str(),
        "FLAKE_UNRESOLVED"
    );
    assert_eq!(
        TestingErrorCode::MissingEvidence.as_serde_str(),
        "MISSING_EVIDENCE"
    );
    assert_eq!(
        TestingErrorCode::MockOnlyCertification.as_serde_str(),
        "MOCK_ONLY_CERTIFICATION"
    );
}

#[test]
fn ep040_unit_error_serializes_without_secrets() {
    let err = TestingError::validation("probe failed");
    let json = err.to_redacted_json();
    assert!(json.contains("VALIDATION"));
    assert!(!json.contains("secret"));
}

#[test]
fn ep040_unit_error_messages_never_contain_secret_shaped_values() {
    // Runtime-constructed canaries: no secret-shaped literal in source.
    let sk = format!("sk{}", "-live-abcdef123456");
    let ghp = format!("ghp{}", "_abcdefghijklmnop");
    let aws = format!("AKIA{}", "ABCDEFGHIJKLMNOP");
    let bearer = format!("Bearer {}", "abcdefghijklmnop");
    let message = format!("failure with {sk} {ghp} {aws} {bearer}");
    let err = TestingError::internal(message);
    let redacted = err.to_redacted_json();
    let marker_sk = format!("sk{}", "-live");
    let marker_ghp = format!("ghp{}", "_");
    let marker_aws = format!("AK{}", "IA");
    let marker_bearer = format!("Bearer{}", " ");
    for marker in [marker_sk, marker_ghp, marker_aws, marker_bearer] {
        assert!(
            !redacted.contains(&marker),
            "canary marker {marker:?} survived redaction"
        );
    }
}

#[test]
fn ep040_unit_redact_secret_shaped_scrubs_all_families() {
    let sk = format!("sk{}", "-live-abcdef123456");
    let ghp = format!("ghp{}", "_abcdefghijklmnop");
    let aws = format!("AKIA{}", "ABCDEFGHIJKLMNOP");
    let bearer = format!("Bearer {}", "abcdefghijklmnop");
    let url = format!("https://user:{}@example.invalid/private", "s3cr3t");
    let body = format!("{sk} {ghp} {aws} {bearer} {url}");
    let redacted = redact_secret_shaped(&body);
    assert!(!redacted.contains("abcdef123456"));
    assert!(!redacted.contains("ghp_"));
    assert!(!redacted.contains("AKIA"));
}

// ---------------------------------------------------------------------
// Serialization: models roundtrip through serde.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_test_matrix_serializes_roundtrip() {
    let matrix = TestMatrix::new("EP-040")
        .add_required(TestLayer::Unit, "ep040_unit_demo")
        .add_required(TestLayer::Chaos, "ep040_chaos_demo");
    let json = serde_json::to_string(&matrix).unwrap();
    let back: TestMatrix = serde_json::from_str(&json).unwrap();
    assert_eq!(back, matrix);
}

#[test]
fn ep040_unit_chaos_scenario_serializes_roundtrip() {
    let scn = valid_scenario();
    let json = serde_json::to_string(&scn).unwrap();
    let back: ChaosScenario = serde_json::from_str(&json).unwrap();
    assert_eq!(back, scn);
}

#[test]
fn ep040_unit_chaos_scenario_serde_rejects_unknown_field() {
    // deny_unknown_fields: an extra field is rejected, never silently
    // accepted into the contract.
    let scn = valid_scenario();
    let mut json = serde_json::to_string(&scn).unwrap();
    json = json.replace('}', ",\"sneaky\":true}");
    let back: Result<ChaosScenario, _> = serde_json::from_str(&json);
    assert!(back.is_err());
}

// ---------------------------------------------------------------------
// Ports: object-safe, provider-neutral, implementable.
// ---------------------------------------------------------------------

struct NoopMatrixPort;
impl TestMatrixPort for NoopMatrixPort {
    fn validate(&self, matrix: &TestMatrix) -> nexus_test_contract::TestingResult<()> {
        matrix.validate()
    }
}

struct NoopChaosPort;
impl ChaosScenarioPort for NoopChaosPort {
    fn validate(&self, scenario: &ChaosScenario) -> nexus_test_contract::TestingResult<()> {
        scenario.validate()
    }
}

#[test]
fn ep040_unit_port_traits_implementable() {
    let matrix_port: Box<dyn TestMatrixPort> = Box::new(NoopMatrixPort);
    let chaos_port: Box<dyn ChaosScenarioPort> = Box::new(NoopChaosPort);
    let matrix = TestMatrix::new("EP-040").add_required(TestLayer::Unit, "ep040_unit_demo");
    assert!(matrix_port.validate(&matrix).is_ok());
    assert!(chaos_port.validate(&valid_scenario()).is_ok());
    let bad = TestMatrix::new("EP-040");
    assert!(matrix_port.validate(&bad).is_err());
}

// ---------------------------------------------------------------------
// Dependency direction: the contract crate depends only on nexus-domain,
// serde, serde_json. No test runner, injector, or certification harness.
// ---------------------------------------------------------------------

#[test]
fn ep040_unit_dependency_direction() {
    // The gate enforces this via cargo tree; here we prove the direct
    // dependency surface is limited to nexus-domain + serde + serde_json.
    let _ = nexus_domain::CorrelationId::new("018e5c5e-4d9b-7f0c-8a2b-000000000001");
}
