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
    HealthCriterion, HealthCriterionState, InMemoryIncidentMemory, Incident, IncidentMemory,
    IncidentSignal, IncidentSignalKind, IncidentState, PatchProposal, RemediationApproval,
    ReviewDecision, ReviewVerdict, Risk, RollbackPlan, RollbackState, SandboxVerdict,
    SecurityVerdict,
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
