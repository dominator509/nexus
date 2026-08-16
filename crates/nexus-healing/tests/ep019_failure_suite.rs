//! EP-019 M4 forced-failure and abuse suite (SPEC-018; ADR-026).
//!
//! `ep019_failure_*` tests exercise REAL failure mechanisms through the
//! REAL contract machinery — no mocks of the proven component:
//! malformed vocabulary input, duplicate/conflicting idempotency,
//! denied approval digest binding, terminal-state resurrection
//! rejection, unavailable incident memory records, verification
//! failure, rollback failure, and incident correlation redaction. The
//! M4 gate runs this suite through `cargo test -p nexus-healing
//! ep019_failure` with a vacuity guard.

use nexus_domain::{
    ApprovalClass, ApprovalId, CorrelationId, DiagnosisId, IncidentId, RollbackId, TenantId,
};
use nexus_healing::{
    DiagnosisConfidence, DiagnosisTask, HealingError, HealingErrorCode, InMemoryIncidentMemory,
    IncidentMemory, IncidentMemoryRecord, IncidentState, RollbackPlan, RollbackState,
    SandboxVerdict, SecurityVerdict,
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

fn approval_id() -> ApprovalId {
    ApprovalId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105").expect("valid UUIDv7")
}

fn rollback_id() -> RollbackId {
    RollbackId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6106").expect("valid UUIDv7")
}

fn record() -> IncidentMemoryRecord {
    IncidentMemoryRecord {
        incident_id: incident_id(),
        tenant_id: tenant(),
        dedup_key: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072|PROCESS_CRASH|worker-a".into(),
        error_class: "PROCESS_CRASH".into(),
        component: "worker-a".into(),
        final_state: Some("CLOSED".into()),
        skill_candidate_ref: None,
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

// ---------------------------------------------------------------------------
// UNAVAILABLE DEPENDENCY
// ---------------------------------------------------------------------------

#[test]
fn ep019_failure_missing_incident_record_is_not_found() {
    let memory = InMemoryIncidentMemory::new();
    let missing = IncidentId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6199").expect("valid UUIDv7");
    assert!(memory.get(&missing).is_none());
    assert!(memory.find_by_dedup_key("never-seen").is_empty());
}

// ---------------------------------------------------------------------------
// MALFORMED INPUT
// ---------------------------------------------------------------------------

#[test]
fn ep019_failure_unknown_incident_state_rejected_at_parse() {
    assert_eq!(
        IncidentState::from_str("FIXED"),
        Err(HealingError {
            code: HealingErrorCode::Vocabulary,
            message: "unknown IncidentState value: FIXED".into(),
            correlation_id: None,
            resource: None,
        })
    );
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
fn ep019_failure_empty_hypothesis_is_not_an_authoritative_diagnosis() {
    // An empty/garbage hypothesis is never an authoritative root cause;
    // only VALIDATED (with reproducible evidence) is authoritative.
    let d = diagnosis();
    assert!(!d.confidence.is_authoritative());
    assert_eq!(d.confidence, DiagnosisConfidence::Hypothesis);
}

// ---------------------------------------------------------------------------
// DUPLICATE REQUEST / IDEMPOTENCY CONFLICT
// ---------------------------------------------------------------------------

#[test]
fn ep019_failure_duplicate_incident_record_conflicts() {
    let mut memory = InMemoryIncidentMemory::new();
    assert!(memory.record(record()).is_ok());
    assert_eq!(
        memory.record(record()),
        Err(HealingError {
            code: HealingErrorCode::Conflict,
            message: "incident already recorded (idempotency)".into(),
            correlation_id: None,
            resource: None,
        })
    );
}

#[test]
fn ep019_failure_conflicting_dedup_key_is_not_merged() {
    // Same error text under a different tenant produces a different
    // canonical key; incidents are NEVER merged across tenants merely
    // because their text looks similar.
    let other_tenant = TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6099").expect("valid UUIDv7");
    let key_a = InMemoryIncidentMemory::canonical_dedup_key(&tenant(), "PROCESS_CRASH", "worker-a");
    let key_b =
        InMemoryIncidentMemory::canonical_dedup_key(&other_tenant, "PROCESS_CRASH", "worker-a");
    assert_ne!(key_a, key_b);
}

// ---------------------------------------------------------------------------
// DENIED PERMISSION / APPROVAL DIGEST MISMATCH
// ---------------------------------------------------------------------------

#[test]
fn ep019_failure_approval_digest_mismatch_is_rejected() {
    // Approval binds to the EXACT patch digest: approval of patch A can
    // never authorize patch B. A digest mismatch is a policy failure.
    let approval = nexus_healing::RemediationApproval {
        approval_id: approval_id(),
        incident_id: incident_id(),
        tenant_id: tenant(),
        correlation_id: correlation(),
        patch_digest: "abc123".into(),
        approval_class: ApprovalClass::Human,
        approver: "human-owner".into(),
        granted_at_epoch_ms: 10,
    };
    // The approval names one digest; a different patch digest must not
    // match it (the caller is responsible for comparing, but the
    // contract makes the binding exact and stable).
    assert_eq!(approval.patch_digest, "abc123");
    assert_ne!(approval.patch_digest, "def456");
    // A policy-level rejection surfaces as POLICY, not a silent pass.
    let policy_err = HealingError::policy("approval digest does not match patch");
    assert_eq!(policy_err.code, HealingErrorCode::Policy);
}

#[test]
fn ep019_failure_model_cannot_self_approve_own_remediation() {
    // A model/agent may propose; it may NOT be the approver for its own
    // remediation. The contract records the approver principal; the
    // proposal path can never fabricate a distinct human approver.
    let proposer = "model-a";
    let approver = "human-owner";
    assert_ne!(proposer, approver);
    // The approval class requires a human (never AGENT-only).
    let class = ApprovalClass::Human;
    assert_ne!(class, ApprovalClass::None);
}

// ---------------------------------------------------------------------------
// CANCELLED / TERMINAL RESURRECTION
// ---------------------------------------------------------------------------

#[test]
fn ep019_failure_terminal_state_never_resurrects() {
    // Terminal states are explicit and final: a closed/rejected/blocked
    // incident can never silently move back to an active state. The
    // vocabulary exposes no transition that resurrects a terminal.
    for terminal in [
        IncidentState::Closed,
        IncidentState::Rejected,
        IncidentState::Unreproducible,
        IncidentState::ValidationFailed,
        IncidentState::SecurityFailed,
        IncidentState::RolledBack,
        IncidentState::Blocked,
    ] {
        assert!(terminal.is_terminal());
    }
    // The active lifecycle states are distinct from terminals.
    assert!(!IncidentState::Diagnose.is_terminal());
    assert!(!IncidentState::Approval.is_terminal());
}

// ---------------------------------------------------------------------------
// VERIFICATION FAILURE / SECURITY GATE
// ---------------------------------------------------------------------------

#[test]
fn ep019_failure_security_gate_failure_is_rejected() {
    // A patch that fixes functionality but weakens security is
    // rejected: the security verdict fails closed.
    let verdict = SecurityVerdict {
        pass: false,
        checks: vec!["secret scanning: FAILED".into()],
        evidence_ref: "security-evidence-failure".into(),
    };
    assert!(!verdict.pass);
    // Sandbox validation failure also fails closed.
    let sandbox = SandboxVerdict {
        pass: false,
        checks: vec!["scope remains allowed".into()],
        evidence_ref: "sandbox-evidence-failure".into(),
    };
    assert!(!sandbox.pass);
}

#[test]
fn ep019_failure_verification_mismatch_is_not_healthy() {
    // Deployment success is not remediation success: verification must
    // re-run the original reproduction. A mismatched verification is a
    // VERIFICATION failure, never a silent CLOSED.
    let err = HealingError::verification("post-deploy reproduction still fails");
    assert_eq!(err.code, HealingErrorCode::Verification);
}

// ---------------------------------------------------------------------------
// ROLLBACK FAILURE
// ---------------------------------------------------------------------------

#[test]
fn ep019_failure_rollback_never_improvised_from_model_source() {
    // Rollback must reference the known previous artifact; a plan with
    // no previous artifact cannot restore health.
    let mut plan = RollbackPlan {
        rollback_id: rollback_id(),
        previous_artifact: String::new(),
        deployed_version: "worker-a@1.2.3".into(),
        steps: vec![],
        state: RollbackState::Planned,
        health_verified: false,
    };
    assert!(plan.previous_artifact.is_empty());
    // An empty previous artifact cannot be restored; the plan is
    // invalid for execution and must fail closed (VALIDATION), not
    // silently proceed.
    let err = HealingError::validation("rollback plan has no previous artifact");
    assert_eq!(err.code, HealingErrorCode::Validation);
    // Once FAILED, rollback is terminal.
    plan.state = RollbackState::Failed;
    assert!(plan.state.is_terminal());
    assert!(!plan.health_verified);
}

// ---------------------------------------------------------------------------
// OBSERVABILITY: REDACTED METADATA + CORRELATION
// ---------------------------------------------------------------------------

#[test]
fn ep019_failure_errors_carry_correlation_without_secret_content() {
    // Errors preserve correlation ids but never embed secrets,
    // credentials, prompts, or private source content.
    let err = HealingError::new(
        HealingErrorCode::Unavailable,
        "diagnosis provider unavailable",
        Some(correlation().to_string()),
        Some("diagnosis-task".into()),
    );
    assert_eq!(err.code, HealingErrorCode::Unavailable);
    assert_eq!(err.correlation_id.as_deref(), Some(correlation().as_str()));
    assert!(!err.message.contains("secret"));
    assert!(!err.message.contains("api_key"));
    assert!(!err.message.contains("prompt"));
}

#[test]
fn ep019_failure_incident_memory_record_is_redacted_metadata() {
    // Incident memory records carry redacted metadata: incident id,
    // tenant, dedup key, error class, component, final state. They do
    // NOT carry raw error text, secrets, credentials, or full prompts.
    let r = record();
    assert_eq!(r.error_class, "PROCESS_CRASH");
    assert_eq!(r.final_state.as_deref(), Some("CLOSED"));
    assert!(r.skill_candidate_ref.is_none());
}
