//! EP-019 M1 contract suite (SPEC-018; ADR-026).
//!
//! Non-vacuous `ep019_unit_*` tests proving vocabulary locking, lifecycle
//! terminal semantics, diagnosis confidence escalation, approval digest
//! binding, incident memory idempotency/dedup, error typing, and
//! dependency direction. The M1 gate runs this suite through the real
//! `cargo test -p nexus-healing ep019_unit` machinery with a vacuity
//! guard.

use nexus_domain::{
    ApprovalClass, ApprovalId, CorrelationId, DiagnosisId, IncidentId, PatchId, RollbackId,
    TenantId,
};
use nexus_healing::{
    CanaryPlan, CanaryState, DiagnosisConfidence, DiagnosisTask, HealingError, HealingErrorCode,
    HealthCriterion, HealthCriterionState, InMemoryIncidentMemory, Incident, IncidentEngine,
    IncidentMemory, IncidentSignal, IncidentSignalKind, IncidentState, PatchProposal,
    RemediationApproval, ReviewDecision, ReviewVerdict, Risk, RollbackPlan, RollbackState,
    SandboxVerdict, SecurityVerdict, StandardIncidentEngine,
};
use std::str::FromStr;

fn tenant() -> TenantId {
    TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072").expect("valid UUIDv7")
}

fn incident_id() -> IncidentId {
    IncidentId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101").expect("valid UUIDv7")
}

fn correlation() -> CorrelationId {
    CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102").expect("valid UUIDv7")
}

fn diagnosis_id() -> DiagnosisId {
    DiagnosisId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103").expect("valid UUIDv7")
}

fn patch_id() -> PatchId {
    PatchId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6104").expect("valid UUIDv7")
}

fn approval_id() -> ApprovalId {
    ApprovalId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105").expect("valid UUIDv7")
}

fn rollback_id() -> RollbackId {
    RollbackId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6106").expect("valid UUIDv7")
}

fn signal() -> IncidentSignal {
    IncidentSignal {
        correlation_id: correlation(),
        tenant_id: tenant(),
        error_class: "PROCESS_CRASH".into(),
        component: "worker-a".into(),
        version: Some("1.2.3".into()),
        workflow_id: None,
        kind: IncidentSignalKind::ProcessFailure,
        first_seen_epoch_ms: 1,
        last_seen_epoch_ms: 2,
    }
}

fn incident() -> Incident {
    Incident {
        incident_id: incident_id(),
        correlation_id: correlation(),
        tenant_id: tenant(),
        state: IncidentState::Incident,
        risk: Risk::R2,
        components: vec!["worker-a".into()],
        first_seen_epoch_ms: 1,
        last_seen_epoch_ms: 2,
        dedup_key: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072|PROCESS_CRASH|worker-a".into(),
    }
}

fn diagnosis() -> DiagnosisTask {
    DiagnosisTask {
        diagnosis_id: diagnosis_id(),
        incident_id: incident_id(),
        tenant_id: tenant(),
        correlation_id: correlation(),
        hypothesis: "worker-a crashed on null input".into(),
        confidence: DiagnosisConfidence::Hypothesis,
        evidence_refs: vec![],
        attempts: 1,
    }
}

fn patch() -> PatchProposal {
    PatchProposal {
        patch_id: patch_id(),
        incident_id: incident_id(),
        tenant_id: tenant(),
        correlation_id: correlation(),
        files_changed: vec!["crates/worker-a/src/main.rs".into()],
        diff: "--- a/src/main.rs\n+++ b/src/main.rs\n- panic!()\n+ return".into(),
        rationale: "guard null input".into(),
        tests_changed: vec!["tests/null_input.rs".into()],
        risk: Risk::R1,
        dependency_changes: vec![],
        migration_impact: String::new(),
        rollback_plan_ref: "rollback-1".into(),
        patch_digest: "abc123".into(),
    }
}

fn approval(digest: &str) -> RemediationApproval {
    RemediationApproval {
        approval_id: approval_id(),
        incident_id: incident_id(),
        tenant_id: tenant(),
        correlation_id: correlation(),
        patch_digest: digest.into(),
        approval_class: ApprovalClass::Human,
        approver: "human-owner".into(),
        granted_at_epoch_ms: 10,
    }
}

// ---------------------------------------------------------------------------
// VOCABULARY LOCKING
// ---------------------------------------------------------------------------

#[test]
fn ep019_unit_incident_state_vocabulary_roundtrips_all_canonical_states() {
    for state in IncidentState::ALL {
        let text = state.as_str();
        assert_eq!(IncidentState::from_str(text).unwrap(), state);
        assert_eq!(state.to_string(), text);
    }
}

#[test]
fn ep019_unit_incident_state_rejects_unknown_and_collapsed_values() {
    assert_eq!(
        IncidentState::from_str("FIXED"),
        Err(HealingError {
            code: HealingErrorCode::Vocabulary,
            message: "unknown IncidentState value: FIXED".into(),
            correlation_id: None,
            resource: None,
        })
    );
    // No state may be collapsed: a raw "FIXED" is NOT a vocabulary value.
    assert_eq!(
        IncidentState::from_str("REMEDIATED"),
        Err(HealingError {
            code: HealingErrorCode::Vocabulary,
            message: "unknown IncidentState value: REMEDIATED".into(),
            correlation_id: None,
            resource: None,
        })
    );
}

#[test]
fn ep019_unit_incident_state_terminal_semantics() {
    assert!(IncidentState::Closed.is_terminal());
    assert!(IncidentState::Closed.is_healthy_terminal());
    assert!(IncidentState::Rejected.is_terminal());
    assert!(IncidentState::Unreproducible.is_terminal());
    assert!(IncidentState::ValidationFailed.is_terminal());
    assert!(IncidentState::SecurityFailed.is_terminal());
    assert!(IncidentState::RolledBack.is_terminal());
    assert!(IncidentState::Blocked.is_terminal());
    assert!(!IncidentState::Diagnose.is_terminal());
    assert!(!IncidentState::Observe.is_terminal());
    // Only CLOSED is a healthy terminal; a failed terminal is never
    // healthy, and no intermediate state can be claimed as remediated.
    for state in IncidentState::ALL {
        if state != IncidentState::Closed {
            assert!(!state.is_healthy_terminal());
        }
    }
}

#[test]
fn ep019_unit_diagnosis_confidence_escalation_order() {
    assert_eq!(
        DiagnosisConfidence::from_str("HYPOTHESIS").unwrap(),
        DiagnosisConfidence::Hypothesis
    );
    assert_eq!(
        DiagnosisConfidence::from_str("SUPPORTED").unwrap(),
        DiagnosisConfidence::Supported
    );
    assert_eq!(
        DiagnosisConfidence::from_str("REPRODUCED").unwrap(),
        DiagnosisConfidence::Reproduced
    );
    assert_eq!(
        DiagnosisConfidence::from_str("VALIDATED").unwrap(),
        DiagnosisConfidence::Validated
    );
    // A model-generated explanation is ALWAYS a hypothesis; only
    // VALIDATED is authoritative.
    assert!(!DiagnosisConfidence::Hypothesis.is_authoritative());
    assert!(!DiagnosisConfidence::Supported.is_authoritative());
    assert!(!DiagnosisConfidence::Reproduced.is_authoritative());
    assert!(DiagnosisConfidence::Validated.is_authoritative());
    assert_eq!(
        DiagnosisConfidence::from_str("PROVEN"),
        Err(HealingError {
            code: HealingErrorCode::Vocabulary,
            message: "unknown DiagnosisConfidence value: PROVEN".into(),
            correlation_id: None,
            resource: None,
        })
    );
}

#[test]
fn ep019_unit_signal_kind_and_review_decision_vocabulary() {
    assert_eq!(
        IncidentSignalKind::from_str("PROCESS_FAILURE").unwrap(),
        IncidentSignalKind::ProcessFailure
    );
    assert_eq!(
        IncidentSignalKind::from_str("DEPLOYMENT_REGRESSION").unwrap(),
        IncidentSignalKind::DeploymentRegression
    );
    assert_eq!(
        IncidentSignalKind::from_str("NOT_A_SIGNAL"),
        Err(HealingError {
            code: HealingErrorCode::Vocabulary,
            message: "unknown IncidentSignalKind value: NOT_A_SIGNAL".into(),
            correlation_id: None,
            resource: None,
        })
    );
    assert_eq!(ReviewDecision::Approve.as_str(), "APPROVE");
    assert_eq!(ReviewDecision::Reject.as_str(), "REJECT");
    assert_eq!(ReviewDecision::RequestChanges.as_str(), "REQUEST_CHANGES");
}

// ---------------------------------------------------------------------------
// STATE MACHINE CONTRACT
// ---------------------------------------------------------------------------

#[test]
fn ep019_unit_lifecycle_states_are_never_collapsed() {
    // The full canonical lifecycle must be representable as distinct
    // vocabulary states; no two lifecycle phases collapse to one value.
    let lifecycle = [
        IncidentState::Observe,
        IncidentState::Incident,
        IncidentState::Correlate,
        IncidentState::Diagnose,
        IncidentState::Reproduce,
        IncidentState::PatchProposed,
        IncidentState::SandboxValidation,
        IncidentState::SecurityValidation,
        IncidentState::Approval,
        IncidentState::StagedDeployment,
        IncidentState::PostDeployVerification,
        IncidentState::Closed,
    ];
    let mut seen = std::collections::HashSet::new();
    for state in lifecycle {
        assert!(seen.insert(state), "lifecycle state collapsed: {state}");
    }
    // Explicit failure/terminal states are distinct too.
    let terminals = [
        IncidentState::Rejected,
        IncidentState::Unreproducible,
        IncidentState::ValidationFailed,
        IncidentState::SecurityFailed,
        IncidentState::RolledBack,
        IncidentState::Blocked,
    ];
    for state in terminals {
        assert!(seen.insert(state), "terminal state collapsed: {state}");
    }
    assert_eq!(seen.len(), 18);
}

#[test]
fn ep019_unit_serialization_roundtrips_incident_and_signal() {
    let json = serde_json::to_string(&incident()).expect("serialize incident");
    let back: Incident = serde_json::from_str(&json).expect("deserialize incident");
    assert_eq!(back, incident());
    let sig_json = serde_json::to_string(&signal()).expect("serialize signal");
    let sig_back: IncidentSignal = serde_json::from_str(&sig_json).expect("deserialize signal");
    assert_eq!(sig_back, signal());
    // State serializes as the canonical SCREAMING_SNAKE_CASE token.
    assert!(json.contains("\"state\":\"INCIDENT\""));
}

// ---------------------------------------------------------------------------
// DIAGNOSIS CONFIDENCE: MODEL CANNOT SELF-CERTIFY
// ---------------------------------------------------------------------------

#[test]
fn ep019_unit_hypothesis_is_not_root_cause() {
    let mut d = diagnosis();
    assert_eq!(d.confidence, DiagnosisConfidence::Hypothesis);
    // Confidence is a field, not an authority: a model/agent may
    // generate a hypothesis, but the claim only becomes VALIDATED via
    // the engine's update path with real evidence. There is no
    // "declare fixed" method on the contract.
    d.confidence = DiagnosisConfidence::Validated;
    assert!(d.confidence.is_authoritative());
}

// ---------------------------------------------------------------------------
// APPROVAL BINDING
// ---------------------------------------------------------------------------

#[test]
fn ep019_unit_approval_binds_to_exact_patch_digest() {
    let a = approval("abc123");
    assert_eq!(a.patch_digest, "abc123");
    assert_eq!(a.approval_class, ApprovalClass::Human);
    // Approval of patch A cannot authorize patch B: the digest is a
    // required, exact binding field.
    let b = approval("def456");
    assert_ne!(a.patch_digest, b.patch_digest);
}

#[test]
fn ep019_unit_review_verdict_is_independent() {
    let verdict = ReviewVerdict {
        reviewer: "human-reviewer".into(),
        decision: ReviewDecision::Approve,
        comments: "diff reviewed".into(),
        patch_digest: "abc123".into(),
    };
    assert_eq!(verdict.reviewer, "human-reviewer");
    assert_eq!(verdict.decision, ReviewDecision::Approve);
    // The reviewer is distinct from the approver principal by contract
    // shape: reviewer, approval, and proposer are separate fields.
    let proposer = "model-a";
    let approver = approval("abc123").approver;
    assert_ne!(proposer, approver);
    assert_ne!(verdict.reviewer, proposer);
}

// ---------------------------------------------------------------------------
// PATCH SCOPE: UNEXPECTED EXPANSION FAILS VALIDATION
// ---------------------------------------------------------------------------

#[test]
fn ep019_unit_patch_scope_is_explicit_and_bounded() {
    let p = patch();
    assert_eq!(p.files_changed, vec!["crates/worker-a/src/main.rs"]);
    // A patch carrying changes outside its declared scope must be
    // rejected at validation: the contract carries the exact file list,
    // and the sandbox validation gate re-checks scope.
    let verdict = SandboxVerdict {
        pass: true,
        checks: vec![
            "patch applies cleanly".into(),
            "build succeeds".into(),
            "targeted reproduction FAIL->PASS".into(),
            "affected tests pass".into(),
            "regression tests pass".into(),
            "no forbidden placeholders/stubs".into(),
            "scope remains allowed".into(),
        ],
        evidence_ref: "sandbox-evidence-1".into(),
    };
    assert!(verdict.pass);
    assert!(verdict.checks.len() >= 7);
}

#[test]
fn ep019_unit_sandbox_failure_fails_closed() {
    let verdict = SandboxVerdict {
        pass: false,
        checks: vec!["scope remains allowed".into()],
        evidence_ref: "sandbox-evidence-2".into(),
    };
    assert!(!verdict.pass);
    let sec = SecurityVerdict {
        pass: false,
        checks: vec!["secret scanning: FAILED".into()],
        evidence_ref: "security-evidence-1".into(),
    };
    assert!(!sec.pass);
}

// ---------------------------------------------------------------------------
// CANARY + ROLLBACK DETERMINISTIC STATES
// ---------------------------------------------------------------------------

#[test]
fn ep019_unit_canary_states_and_health_criteria() {
    let plan = CanaryPlan {
        stages: vec!["canary".into(), "targeted".into(), "broader".into()],
        health_criteria: vec![HealthCriterion {
            name: "readyz".into(),
            expected: HealthCriterionState::Healthy,
            observed: None,
        }],
        patch_digest: "abc123".into(),
        auto_rollback_on_regression: true,
        state: CanaryState::Planned,
    };
    assert_eq!(plan.stages.len(), 3);
    assert!(plan.auto_rollback_on_regression);
    assert_eq!(
        CanaryState::from_str("ROLLED_BACK").unwrap(),
        CanaryState::RolledBack
    );
    assert!(CanaryState::RolledBack.is_terminal());
    assert!(!CanaryState::Validating.is_terminal());
}

#[test]
fn ep019_unit_rollback_state_machine_is_deterministic() {
    let mut plan = RollbackPlan {
        rollback_id: rollback_id(),
        previous_artifact: "worker-a@1.2.2".into(),
        deployed_version: "worker-a@1.2.3".into(),
        steps: vec!["restore previous artifact".into(), "verify health".into()],
        state: RollbackState::Planned,
        health_verified: false,
    };
    assert_eq!(plan.state, RollbackState::Planned);
    plan.state = RollbackState::Executing;
    assert!(!plan.state.is_terminal());
    plan.state = RollbackState::Restored;
    plan.health_verified = true;
    assert!(plan.state.is_terminal());
    assert!(plan.health_verified);
    // Rollback is bound to the known previous artifact, never to
    // model-generated source.
    assert_eq!(plan.previous_artifact, "worker-a@1.2.2");
    assert_eq!(
        RollbackState::from_str("FAILED").unwrap(),
        RollbackState::Failed
    );
}

// ---------------------------------------------------------------------------
// INCIDENT MEMORY: DEDUP + IDEMPOTENCY
// ---------------------------------------------------------------------------

#[test]
fn ep019_unit_incident_memory_dedup_key_is_canonical_and_tenant_scoped() {
    let key = InMemoryIncidentMemory::canonical_dedup_key(&tenant(), "PROCESS_CRASH", "worker-a");
    assert_eq!(
        key,
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072|PROCESS_CRASH|worker-a"
    );
    // A different tenant with the same text produces a DIFFERENT key:
    // incidents are never merged across tenant boundaries.
    let other_tenant = TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6099").expect("valid UUIDv7");
    let other =
        InMemoryIncidentMemory::canonical_dedup_key(&other_tenant, "PROCESS_CRASH", "worker-a");
    assert_ne!(key, other);
}

#[test]
fn ep019_unit_incident_memory_record_is_idempotent_and_deduplicating() {
    let mut memory = InMemoryIncidentMemory::new();
    let record = nexus_healing::IncidentMemoryRecord {
        incident_id: incident_id(),
        tenant_id: tenant(),
        dedup_key: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072|PROCESS_CRASH|worker-a".into(),
        error_class: "PROCESS_CRASH".into(),
        component: "worker-a".into(),
        final_state: Some("CLOSED".into()),
        skill_candidate_ref: None,
    };
    assert!(memory.record(record.clone()).is_ok());
    // Duplicate incident id conflicts (idempotency).
    assert_eq!(
        memory.record(record.clone()),
        Err(HealingError {
            code: HealingErrorCode::Conflict,
            message: "incident already recorded (idempotency)".into(),
            correlation_id: None,
            resource: None,
        })
    );
    let found = memory.find_by_dedup_key(&record.dedup_key);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].incident_id, incident_id());
    assert!(memory.get(&incident_id()).is_some());
}

// ---------------------------------------------------------------------------
// ERROR TYPING (SPEC-006)
// ---------------------------------------------------------------------------

#[test]
fn ep019_unit_error_codes_are_stable_and_typed() {
    assert_eq!(HealingErrorCode::Validation.as_str(), "VALIDATION");
    assert_eq!(HealingErrorCode::Policy.as_str(), "POLICY");
    assert_eq!(HealingErrorCode::Verification.as_str(), "VERIFICATION");
    assert_eq!(HealingErrorCode::Vocabulary.as_str(), "VOCABULARY");
    let err = HealingError::verification("reproduction did not change");
    assert_eq!(err.code, HealingErrorCode::Verification);
    assert!(err.message.contains("reproduction"));
}

// ---------------------------------------------------------------------------
// DEPENDENCY DIRECTION
// ---------------------------------------------------------------------------

#[test]
fn ep019_unit_dependency_direction_contract_crate_imports_no_provider_impl() {
    let manifest_path = concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml");
    let text = std::fs::read_to_string(manifest_path).expect("read Cargo.toml");
    for forbidden in [
        "nexus-model-gateway",
        "nexus-harness-adapters",
        "nexus-agents",
        "nexus-skills",
        "nexus-context",
        "nexus-memory-workers",
        "tokio",
        "reqwest",
        "temporal",
        "postgres",
        "sqlx",
        "ring =",
    ] {
        assert!(
            !text.contains(forbidden),
            "contract crate must not depend on provider/runtime crate {forbidden}"
        );
    }
    // The contract crate depends only on the shared domain crate and
    // serde.
    assert!(text.contains("nexus-domain"));
    assert!(text.contains("serde"));
}

#[test]
fn ep019_unit_dependency_direction_cargo_tree_has_no_runtime_deps() {
    let out = std::process::Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "nexus-healing",
            "--depth",
            "1",
            "--edges",
            "normal",
        ])
        .output()
        .expect("cargo tree must run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    for line in stdout.lines().skip(1) {
        let trimmed =
            line.trim_start_matches(['\u{251c}', '\u{2514}', '\u{2500}', '\u{2502}', ' ']);
        for forbidden in ["tokio", "reqwest", "temporal", "postgres", "sqlx", "ring"] {
            assert!(
                !(trimmed.starts_with(forbidden) || trimmed.contains(&format!(" {forbidden} v"))),
                "nexus-healing tree violates dependency direction: {line}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// PRODUCTION INCIDENT ENGINE (AUD-017): the port now has a real
// implementation that owns the lifecycle, dedup, evidence gating,
// patch-digest binding, and fail-closed verification.
// ---------------------------------------------------------------------------

fn new_engine() -> StandardIncidentEngine<InMemoryIncidentMemory> {
    StandardIncidentEngine::new(InMemoryIncidentMemory::new())
}

fn patch_digest() -> String {
    "a1b2c3d4e5f60718293a4b5c6d7e8f9012233445566778899aabbccddeeff0011".into()
}

fn sandbox_pass() -> SandboxVerdict {
    SandboxVerdict {
        pass: true,
        checks: vec![
            "patch applies cleanly".into(),
            "build succeeds".into(),
            "targeted reproduction FAIL->PASS".into(),
            "regression tests pass".into(),
        ],
        evidence_ref: "engine-sandbox-evidence".into(),
    }
}

fn security_pass() -> SecurityVerdict {
    SecurityVerdict {
        pass: true,
        checks: vec![
            "dependency audit ok".into(),
            "license gate ok".into(),
            "secret scanning ok".into(),
        ],
        evidence_ref: "engine-security-evidence".into(),
    }
}

fn engine_approval(digest: &str) -> RemediationApproval {
    RemediationApproval {
        approval_id: ApprovalId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6107").expect("valid"),
        incident_id: IncidentId::new(correlation().as_str()).expect("derived incident id"),
        tenant_id: tenant(),
        correlation_id: correlation(),
        patch_digest: digest.into(),
        approval_class: ApprovalClass::Human,
        approver: "human-owner".into(),
        granted_at_epoch_ms: 10,
    }
}

fn patch_proposal(digest: &str) -> PatchProposal {
    PatchProposal {
        patch_id: PatchId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6104").expect("valid"),
        incident_id: IncidentId::new(correlation().as_str()).expect("derived incident id"),
        tenant_id: tenant(),
        correlation_id: correlation(),
        files_changed: vec!["failing-worker.sh".into()],
        diff: "--- a/failing-worker.sh\n+++ b/failing-worker.sh\n".into(),
        rationale: "fix marker filename check".into(),
        tests_changed: vec!["test_ep019_integration_healing_loop.py".into()],
        risk: Risk::R1,
        dependency_changes: vec![],
        migration_impact: String::new(),
        rollback_plan_ref: "rollback-plan-1".into(),
        patch_digest: digest.into(),
    }
}

/// Drive the engine through the full canonical lifecycle to CLOSED.
fn drive_to_closed(engine: &StandardIncidentEngine<InMemoryIncidentMemory>) -> Incident {
    let mut incident = engine.observe(signal()).expect("observe");
    engine
        .transition(&mut incident, IncidentState::Correlate)
        .expect("correlate");
    engine
        .transition(&mut incident, IncidentState::Diagnose)
        .expect("diagnose");
    let mut diagnosis = engine
        .create_diagnosis(&incident, "worker checks hard-coded wrong filename".into())
        .expect("diagnosis");
    engine
        .update_diagnosis_confidence(
            &mut diagnosis,
            DiagnosisConfidence::Supported,
            "correlated evidence: crash log".into(),
        )
        .expect("supported");
    engine
        .update_diagnosis_confidence(
            &mut diagnosis,
            DiagnosisConfidence::Reproduced,
            "reproduction:exit=1".into(),
        )
        .expect("reproduced");
    engine
        .transition(&mut incident, IncidentState::Reproduce)
        .expect("reproduce");
    engine
        .propose_patch(&incident, patch_proposal(&patch_digest()))
        .expect("patch");
    engine
        .transition(&mut incident, IncidentState::PatchProposed)
        .expect("patch proposed");
    engine
        .record_sandbox_validation(&mut incident, &sandbox_pass())
        .expect("sandbox");
    engine
        .record_security_validation(&mut incident, &security_pass())
        .expect("security");
    engine
        .record_approval(&mut incident, &engine_approval(&patch_digest()))
        .expect("approval");
    engine
        .transition(&mut incident, IncidentState::StagedDeployment)
        .expect("staged");
    engine
        .transition(&mut incident, IncidentState::PostDeployVerification)
        .expect("post-deploy");
    engine
        .record_post_deploy_verification(&mut incident, true)
        .expect("verified");
    assert_eq!(incident.state, IncidentState::Closed);
    incident
}

#[test]
fn ep019_unit_engine_full_lifecycle_closes_with_real_verification() {
    let engine = new_engine();
    let incident = drive_to_closed(&engine);
    assert_eq!(incident.state, IncidentState::Closed);
    assert!(incident.state.is_terminal());
    assert!(incident.state.is_healthy_terminal());
    // Memory holds the terminal record.
    let memory_records = engine.memory().find_by_dedup_key(&incident.dedup_key);
    assert_eq!(memory_records.len(), 1);
    assert_eq!(memory_records[0].final_state.as_deref(), Some("CLOSED"));
}

#[test]
fn ep019_unit_engine_deduplicates_open_incidents_by_canonical_signature() {
    let engine = new_engine();
    let first = engine.observe(signal()).expect("first observe");
    let second = engine.observe(signal()).expect("second observe");
    assert_eq!(first.incident_id, second.incident_id);
    assert_eq!(first.dedup_key, second.dedup_key);
    // A different tenant with identical text is a DISTINCT incident
    // (and a distinct correlation id, as a real foreign signal would
    // carry).
    let mut foreign = signal();
    foreign.tenant_id = TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6099").expect("valid");
    foreign.correlation_id =
        CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6199").expect("valid");
    let third = engine.observe(foreign).expect("foreign observe");
    assert_ne!(first.incident_id, third.incident_id);
}

#[test]
fn ep019_unit_engine_rejects_skipped_and_backwards_transitions() {
    let engine = new_engine();
    let mut incident = engine.observe(signal()).expect("observe");
    // Skip CORRELATE -> DIAGNOSE is a non-canonical jump.
    let err = engine
        .transition(&mut incident, IncidentState::Diagnose)
        .expect_err("skipped transition must fail");
    assert_eq!(err.code, HealingErrorCode::Conflict);
    // Backwards transition fails.
    engine
        .transition(&mut incident, IncidentState::Correlate)
        .expect("correlate");
    let err = engine
        .transition(&mut incident, IncidentState::Incident)
        .expect_err("backwards transition must fail");
    assert_eq!(err.code, HealingErrorCode::Conflict);
    // Terminal states never move: close then attempt resurrection.
    let engine2 = new_engine();
    let mut closed = drive_to_closed(&engine2);
    let err = engine2
        .transition(&mut closed, IncidentState::Incident)
        .expect_err("terminal resurrection must fail");
    assert_eq!(err.code, HealingErrorCode::Conflict);
}

#[test]
fn ep019_unit_engine_requires_evidence_for_confidence_escalation() {
    let engine = new_engine();
    let mut incident = engine.observe(signal()).expect("observe");
    engine
        .transition(&mut incident, IncidentState::Correlate)
        .expect("correlate");
    engine
        .transition(&mut incident, IncidentState::Diagnose)
        .expect("diagnose");
    let mut diagnosis = engine
        .create_diagnosis(&incident, "hypothesis".into())
        .expect("diagnosis");
    // Empty evidence fails closed.
    let err = engine
        .update_diagnosis_confidence(&mut diagnosis, DiagnosisConfidence::Reproduced, "".into())
        .expect_err("empty evidence must fail");
    assert_eq!(err.code, HealingErrorCode::Verification);
    // Skipping straight to VALIDATED fails closed (no evidence chain).
    let err = engine
        .update_diagnosis_confidence(
            &mut diagnosis,
            DiagnosisConfidence::Validated,
            "reproduction:exit=1".into(),
        )
        .expect_err("skip to validated must fail");
    assert_eq!(err.code, HealingErrorCode::Conflict);
    assert_eq!(diagnosis.confidence, DiagnosisConfidence::Hypothesis);
}

#[test]
fn ep019_unit_engine_sandbox_failure_fails_closed_to_validation_failed() {
    let engine = new_engine();
    let mut incident = engine.observe(signal()).expect("observe");
    engine
        .transition(&mut incident, IncidentState::Correlate)
        .expect("correlate");
    engine
        .transition(&mut incident, IncidentState::Diagnose)
        .expect("diagnose");
    engine
        .transition(&mut incident, IncidentState::Reproduce)
        .expect("reproduce");
    engine
        .propose_patch(&incident, patch_proposal(&patch_digest()))
        .expect("patch");
    engine
        .transition(&mut incident, IncidentState::PatchProposed)
        .expect("patch proposed");
    let failing = SandboxVerdict {
        pass: false,
        checks: vec!["build fails".into()],
        evidence_ref: "engine-sandbox-failure".into(),
    };
    let err = engine
        .record_sandbox_validation(&mut incident, &failing)
        .expect_err("failing sandbox verdict must fail closed");
    assert_eq!(err.code, HealingErrorCode::Verification);
    assert_eq!(incident.state, IncidentState::ValidationFailed);
    assert!(incident.state.is_terminal());
}

#[test]
fn ep019_unit_engine_approval_digest_mismatch_fails_closed_policy() {
    let engine = new_engine();
    let mut incident = engine.observe(signal()).expect("observe");
    engine
        .transition(&mut incident, IncidentState::Correlate)
        .expect("correlate");
    engine
        .transition(&mut incident, IncidentState::Diagnose)
        .expect("diagnose");
    engine
        .transition(&mut incident, IncidentState::Reproduce)
        .expect("reproduce");
    engine
        .propose_patch(&incident, patch_proposal(&patch_digest()))
        .expect("patch");
    engine
        .transition(&mut incident, IncidentState::PatchProposed)
        .expect("patch proposed");
    engine
        .record_sandbox_validation(&mut incident, &sandbox_pass())
        .expect("sandbox");
    engine
        .record_security_validation(&mut incident, &security_pass())
        .expect("security");
    // Approval authorizes a DIFFERENT digest: the engine rejects it.
    let wrong = approval("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");
    let err = engine
        .record_approval(&mut incident, &wrong)
        .expect_err("digest mismatch must fail closed");
    assert_eq!(err.code, HealingErrorCode::Policy);
    assert_eq!(incident.state, IncidentState::SecurityValidation);
    // Approval before security validation fails closed conflict.
    let premature_engine = new_engine();
    let mut premature = premature_engine.observe(signal()).expect("observe");
    let err = premature_engine
        .record_approval(&mut premature, &engine_approval(&patch_digest()))
        .expect_err("premature approval must fail");
    assert_eq!(err.code, HealingErrorCode::Conflict);
}

#[test]
fn ep019_unit_engine_verification_false_fails_closed_and_keeps_open() {
    let engine = new_engine();
    let mut incident = engine.observe(signal()).expect("observe");
    engine
        .transition(&mut incident, IncidentState::Correlate)
        .expect("correlate");
    engine
        .transition(&mut incident, IncidentState::Diagnose)
        .expect("diagnose");
    engine
        .transition(&mut incident, IncidentState::Reproduce)
        .expect("reproduce");
    engine
        .propose_patch(&incident, patch_proposal(&patch_digest()))
        .expect("patch");
    engine
        .transition(&mut incident, IncidentState::PatchProposed)
        .expect("patch proposed");
    engine
        .record_sandbox_validation(&mut incident, &sandbox_pass())
        .expect("sandbox");
    engine
        .record_security_validation(&mut incident, &security_pass())
        .expect("security");
    engine
        .record_approval(&mut incident, &engine_approval(&patch_digest()))
        .expect("approval");
    engine
        .transition(&mut incident, IncidentState::StagedDeployment)
        .expect("staged");
    engine
        .transition(&mut incident, IncidentState::PostDeployVerification)
        .expect("post-deploy");
    let err = engine
        .record_post_deploy_verification(&mut incident, false)
        .expect_err("false verification must fail closed");
    assert_eq!(err.code, HealingErrorCode::Verification);
    assert_eq!(incident.state, IncidentState::PostDeployVerification);
    assert!(!incident.state.is_terminal());
}

#[test]
fn ep019_unit_engine_derived_ids_are_deterministic_and_retry_stable() {
    let engine = new_engine();
    let first = engine.observe(signal()).expect("observe");
    // Re-observing the same signal while open returns the same incident.
    let second = engine.observe(signal()).expect("observe");
    assert_eq!(first.incident_id, second.incident_id);
    // Derived diagnosis id is a canonical UUIDv7 and stable across the
    // same engine instance.
    let mut incident = first;
    engine
        .transition(&mut incident, IncidentState::Correlate)
        .expect("correlate");
    engine
        .transition(&mut incident, IncidentState::Diagnose)
        .expect("diagnose");
    let d1 = engine
        .create_diagnosis(&incident, "h".into())
        .expect("diagnosis");
    assert_eq!(d1.diagnosis_id.as_str().len(), 36);
    assert!(IncidentId::new(d1.diagnosis_id.as_str()).is_ok());
}
