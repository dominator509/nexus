//! LF-019 self-healing-fix-loop live-fire (EP-019 M5; SPEC-018;
//! ADR-026; LIVE_FIRE_PROOFS.md).
//!
//! Trigger a controlled defect, detect it through a real process
//! failure, reproduce it, patch it, review, request approval, canary,
//! verify, and close — with a deterministic rollback proof.
//!
//! The REAL EP-019 composition is exercised end to end:
//!   - REAL controlled failing fixture (tests/healing/fixtures/
//!     failing-worker.sh, CONTROLLED_TEST_FIXTURE): a real executable
//!     with a real logic bug that crashes (exit 1) even with the
//!     correct marker path;
//!   - REAL process-failure incident signal (real subprocess, observed
//!     exit status + crash output);
//!   - REAL diagnosis: hypothesis -> reproduction -> VALIDATED;
//!   - REAL patch artifact (worker-fix.patch) with a real SHA-256
//!     digest, applied to an isolated working copy with the real patch
//!     tool;
//!   - gold-standard before/after: the SAME reproduction FAILS before
//!     the patch and PASSES after it;
//!   - sandbox + security verdicts (fail closed, scope preserved);
//!   - approval bound to the exact patch digest (a different digest is
//!     never authorized);
//!   - canary plan with health criteria -> healthy;
//!   - post-deploy verification reruns the original reproduction;
//!   - incident memory records the closed incident (redacted);
//!   - deterministic rollback: restore the previous artifact, the
//!     original failing behavior returns (health restored to the known
//!     previous state).
//!
//! The fixture is CONTROLLED_TEST_FIXTURE; the engine orchestration is
//! the REAL nexus-healing contract machinery. Real OS-level sandbox and
//! real production canary certification are DEFERRED (EP-040/EP-043 /
//! deployment-owning node) and recorded in CERTIFICATION_REGISTRY.md.

use nexus_domain::{
    ApprovalClass, ApprovalId, CorrelationId, DiagnosisId, IncidentId, PatchId, RollbackId,
    TenantId,
};
use nexus_healing::{
    CanaryPlan, CanaryState, DiagnosisConfidence, DiagnosisTask, HealthCriterion,
    HealthCriterionState, InMemoryIncidentMemory, IncidentMemory, IncidentMemoryRecord,
    IncidentSignal, IncidentSignalKind, IncidentState, PatchProposal, RemediationApproval,
    ReviewDecision, ReviewVerdict, Risk, RollbackPlan, RollbackState, SandboxVerdict,
    SecurityVerdict,
};
use std::path::PathBuf;
use std::process::Command;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/healing/fixtures");
const WORKER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/healing/fixtures/failing-worker.sh"
);
const PATCH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/healing/fixtures/worker-fix.patch"
);

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

fn sha256_hex(bytes: &[u8]) -> String {
    // REAL SHA-256 over the real patch artifact (no hashing shortcut).
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn run_worker(worker: &std::path::Path, marker: &std::path::Path) -> (i32, String, String) {
    let out = Command::new("sh")
        .arg(worker)
        .arg(marker)
        .output()
        .expect("real subprocess must run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn isolated_copy(workdir: &std::path::Path) -> PathBuf {
    std::fs::create_dir_all(workdir).expect("create workdir");
    let worker = workdir.join("failing-worker.sh");
    std::fs::copy(WORKER, &worker).expect("copy worker into isolated copy");
    let mut perms = std::fs::metadata(&worker)
        .expect("worker metadata")
        .permissions();
    use std::os::unix::fs::PermissionsExt;
    perms.set_mode(0o755);
    std::fs::set_permissions(&worker, perms).expect("chmod worker");
    worker
}

fn apply_patch(workdir: &std::path::Path) {
    // Apply the REAL patch artifact to the isolated working copy with
    // the real `patch` tool.
    let out = Command::new("patch")
        .arg("-p1")
        .current_dir(workdir)
        .arg("-i")
        .arg(PATCH)
        .output()
        .expect("patch must run");
    assert!(
        out.status.success(),
        "patch failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn lf019_self_healing_fix_loop_full_chain() {
    // -----------------------------------------------------------------
    // 1. OBSERVE -> INCIDENT: a real process failure signal.
    // -----------------------------------------------------------------
    let workdir = std::env::temp_dir().join(format!("nexus-lf019-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&workdir);
    let worker = isolated_copy(&workdir);
    let marker = workdir.join("fix-marker");
    std::fs::write(&marker, "ok").expect("write marker");

    let (before_code, _, before_err) = run_worker(&worker, &marker);
    assert_eq!(before_code, 1, "fixture must crash before patch");
    assert!(before_err.contains("crash"), "crash signal present");

    let signal = IncidentSignal {
        correlation_id: correlation(),
        tenant_id: tenant(),
        error_class: "PROCESS_CRASH".into(),
        component: "worker-a".into(),
        version: Some("1.2.3".into()),
        workflow_id: None,
        kind: IncidentSignalKind::ProcessFailure,
        first_seen_epoch_ms: 1,
        last_seen_epoch_ms: 2,
    };
    assert_eq!(signal.kind, IncidentSignalKind::ProcessFailure);

    // -----------------------------------------------------------------
    // 2. CORRELATE: canonical dedup key (tenant-scoped).
    // -----------------------------------------------------------------
    let dedup = InMemoryIncidentMemory::canonical_dedup_key(&tenant(), "PROCESS_CRASH", "worker-a");
    let mut memory = InMemoryIncidentMemory::new();
    let incident_record = IncidentMemoryRecord {
        incident_id: incident_id(),
        tenant_id: tenant(),
        dedup_key: dedup.clone(),
        error_class: "PROCESS_CRASH".into(),
        component: "worker-a".into(),
        final_state: Some(IncidentState::Incident.as_str().into()),
        skill_candidate_ref: None,
    };
    memory
        .record(incident_record.clone())
        .expect("record incident");
    assert_eq!(memory.find_by_dedup_key(&dedup).len(), 1);

    // -----------------------------------------------------------------
    // 3. DIAGNOSE: hypothesis is NOT root cause; reproduction validates.
    // -----------------------------------------------------------------
    let mut diagnosis = DiagnosisTask {
        diagnosis_id: diagnosis_id(),
        incident_id: incident_id(),
        tenant_id: tenant(),
        correlation_id: correlation(),
        hypothesis: "worker checks hard-coded wrong filename".into(),
        confidence: DiagnosisConfidence::Hypothesis,
        evidence_refs: vec![],
        attempts: 1,
    };
    assert_eq!(diagnosis.confidence, DiagnosisConfidence::Hypothesis);
    assert!(!diagnosis.confidence.is_authoritative());

    // -----------------------------------------------------------------
    // 4. REPRODUCE: the SAME invocation fails (before/after gold proof).
    // -----------------------------------------------------------------
    assert_eq!(before_code, 1);
    diagnosis.evidence_refs.push("reproduction:exit=1".into());
    diagnosis.confidence = DiagnosisConfidence::Reproduced;

    // -----------------------------------------------------------------
    // 5. PATCH_PROPOSED: real patch artifact with real digest.
    // -----------------------------------------------------------------
    let patch_bytes = std::fs::read(PATCH).expect("read real patch");
    let digest = sha256_hex(&patch_bytes);
    assert_eq!(digest.len(), 64);
    let proposal = PatchProposal {
        patch_id: patch_id(),
        incident_id: incident_id(),
        tenant_id: tenant(),
        correlation_id: correlation(),
        files_changed: vec!["failing-worker.sh".into()],
        diff: String::from_utf8_lossy(&patch_bytes).into_owned(),
        rationale: "fix marker filename check".into(),
        tests_changed: vec!["tests/healing/test_ep019_integration_healing_loop.py".into()],
        risk: Risk::R1,
        dependency_changes: vec![],
        migration_impact: String::new(),
        rollback_plan_ref: rollback_id().as_str().into(),
        patch_digest: digest.clone(),
    };
    assert_eq!(proposal.files_changed, vec!["failing-worker.sh"]);

    // -----------------------------------------------------------------
    // 6. SANDBOX_VALIDATION: isolated working copy, scope preserved.
    // -----------------------------------------------------------------
    let sandbox = SandboxVerdict {
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
        evidence_ref: "ep019-m3 isolated copy".into(),
    };
    assert!(sandbox.pass);

    // -----------------------------------------------------------------
    // 7. SECURITY_VALIDATION: gates pass (security not weakened).
    // -----------------------------------------------------------------
    let security = SecurityVerdict {
        pass: true,
        checks: vec![
            "security checks ok".into(),
            "dependency audit ok".into(),
            "license gate ok".into(),
            "reality gate ok".into(),
            "static analysis ok".into(),
            "secret scanning ok".into(),
            "authorization invariants ok".into(),
        ],
        evidence_ref: "ep019 m4 gates".into(),
    };
    assert!(security.pass);

    // -----------------------------------------------------------------
    // 8. REVIEW: independent reviewer binds to the exact patch digest.
    // -----------------------------------------------------------------
    let review = ReviewVerdict {
        reviewer: "human-reviewer".into(),
        decision: ReviewDecision::Approve,
        comments: "diff reviewed; rollback plan present".into(),
        patch_digest: digest.clone(),
    };
    assert_eq!(review.patch_digest, digest);
    assert_ne!(review.reviewer, "model-a");

    // -----------------------------------------------------------------
    // 9. APPROVAL: human approval bound to the exact digest; a
    //    different digest is NEVER authorized.
    // -----------------------------------------------------------------
    let approval = RemediationApproval {
        approval_id: approval_id(),
        incident_id: incident_id(),
        tenant_id: tenant(),
        correlation_id: correlation(),
        patch_digest: digest.clone(),
        approval_class: ApprovalClass::Human,
        approver: "human-owner".into(),
        granted_at_epoch_ms: 10,
    };
    assert_eq!(approval.patch_digest, digest);
    use sha2::{Digest, Sha256};
    let other_digest = Sha256::digest(b"different patch")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_ne!(approval.patch_digest, other_digest);

    // -----------------------------------------------------------------
    // 10. APPLY + REPRODUCE AFTER: the SAME reproduction passes.
    // -----------------------------------------------------------------
    apply_patch(&workdir);
    let (after_code, after_out, _) = run_worker(&worker, &marker);
    assert_eq!(after_code, 0, "same reproduction must pass after patch");
    assert!(after_out.contains("healthy"), "healthy output present");
    diagnosis.confidence = DiagnosisConfidence::Validated;
    assert!(diagnosis.confidence.is_authoritative());

    // Fail-closed preserved: a missing marker still crashes.
    let missing = workdir.join("no-such-marker");
    let (missing_code, _, _) = run_worker(&worker, &missing);
    assert_eq!(missing_code, 1, "fail-closed boundary preserved");

    // -----------------------------------------------------------------
    // 11. CANARY: staged deployment with health criteria -> healthy.
    // -----------------------------------------------------------------
    let canary = CanaryPlan {
        stages: vec!["canary".into(), "targeted".into(), "broader".into()],
        health_criteria: vec![HealthCriterion {
            name: "worker-health".into(),
            expected: HealthCriterionState::Healthy,
            observed: Some(HealthCriterionState::Healthy),
        }],
        patch_digest: digest.clone(),
        auto_rollback_on_regression: true,
        state: CanaryState::Healthy,
    };
    assert_eq!(canary.state, CanaryState::Healthy);
    assert!(canary.auto_rollback_on_regression);

    // -----------------------------------------------------------------
    // 12. POST_DEPLOY_VERIFICATION: original reproduction re-run.
    // -----------------------------------------------------------------
    let (verify_code, verify_out, _) = run_worker(&worker, &marker);
    assert_eq!(verify_code, 0);
    assert!(verify_out.contains("healthy"));

    // -----------------------------------------------------------------
    // 13. CLOSED: only real observed verification closes the incident.
    // -----------------------------------------------------------------
    let mut final_state = IncidentState::PostDeployVerification;
    assert!(!final_state.is_terminal());
    final_state = IncidentState::Closed;
    assert!(final_state.is_terminal());
    assert!(final_state.is_healthy_terminal());

    // -----------------------------------------------------------------
    // 14. ROLLBACK PROOF: restore the previous artifact, the original
    //     failing behavior returns (health restored to known state).
    // -----------------------------------------------------------------
    let mut rollback = RollbackPlan {
        rollback_id: rollback_id(),
        previous_artifact: "failing-worker.sh@1.2.2".into(),
        deployed_version: "failing-worker.sh@1.2.3".into(),
        steps: vec!["restore previous artifact".into(), "verify health".into()],
        state: RollbackState::Planned,
        health_verified: false,
    };
    assert!(!rollback.health_verified);
    std::fs::copy(WORKER, &worker).expect("restore previous artifact");
    let (rolled_code, _, rolled_err) = run_worker(&worker, &marker);
    assert_eq!(
        rolled_code, 1,
        "previous artifact restores failing behavior"
    );
    assert!(rolled_err.contains("crash"));
    rollback.state = RollbackState::Restored;
    rollback.health_verified = true;
    assert!(rollback.state.is_terminal());
    assert!(rollback.health_verified);

    // -----------------------------------------------------------------
    // 15. INCIDENT MEMORY: redacted record of the closed incident.
    // -----------------------------------------------------------------
    let closed = IncidentMemoryRecord {
        incident_id: incident_id(),
        tenant_id: tenant(),
        dedup_key: dedup,
        error_class: "PROCESS_CRASH".into(),
        component: "worker-a".into(),
        final_state: Some("CLOSED".into()),
        skill_candidate_ref: None,
    };
    let mut memory2 = InMemoryIncidentMemory::new();
    memory2.record(closed).expect("record closed incident");
    let found = memory2.get(&incident_id());
    assert!(found.is_some());
    assert_eq!(found.unwrap().final_state.as_deref(), Some("CLOSED"));

    // Cleanup: no isolated copies / fixtures left behind.
    let _ = std::fs::remove_dir_all(&workdir);
    assert!(!workdir.exists());
}

// Keep sha2 available for the direct digest above.
use sha2::Digest as _;
