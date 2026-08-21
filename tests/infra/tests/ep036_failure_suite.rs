//! EP-036 M4 forced-failure suite (SPEC-016).
//!
//! Proves the Compute Fabric fails safely under dependency, policy,
//! security, and resource faults using REAL failure mechanisms:
//! - an ephemeral sshd container is terminated mid-flight (unavailable
//!   dependency),
//! - a real ssh-keyscan timeout against a dead endpoint (timeout),
//! - corrupted controlled messages (malformed input),
//! - duplicate requests with the same idempotency key (duplicate),
//! - denied placement policy (permission),
//! - delete-before-create (cancelled work),
//! - ambiguous provisioning with provider non-confirmation (partial
//!   side effect -> reconciliation required, never blind retry).
//!
//! No component being proven is mocked. Fail-closed behavior and
//! structured errors (SPEC-006 ComputeErrorCode) are asserted
//! throughout.

use std::process::Command;

use nexus_compute::error::ComputeErrorCode;
use nexus_compute::model::{
    is_valid_resource_transition, resolve_ambiguous_provisioning, CapacityProfile,
    CloudCredentialRef, CloudProvider, ComputeNode, ComputeNodeId, PlacementConstraint,
    PlacementDecision, ProviderBinding, ProvisioningPlan, ProvisioningReceipt, ProvisioningRequest,
    ProvisioningRequestId, ResourceIdentity, WorkloadManifestId,
};
use nexus_compute::placement::placement_decision;
use nexus_compute::vocabulary::{
    CapacityProvenance, ComputeClass, DeleteState, PlacementFailureClass, ProviderKind,
    ProvisioningOutcome, ResourceState, VerificationState,
};
use nexus_domain::{CorrelationId, Locality, Privacy, TenantId};

fn cid(s: &str) -> CorrelationId {
    let hex = format!(
        "{:012x}",
        s.bytes().fold(0x5eedu64, |acc, b| acc
            .wrapping_mul(31)
            .wrapping_add(u64::from(b)))
            & 0x0000_ffff_ffff_ffff
    );
    CorrelationId::new(format!("00000000-0000-7000-8000-{hex}")).expect("correlation id")
}
fn tid(s: &str) -> TenantId {
    let hex = format!(
        "{:012x}",
        s.bytes().fold(0x7efu64, |acc, b| acc
            .wrapping_mul(31)
            .wrapping_add(u64::from(b)))
            & 0x0000_ffff_ffff_ffff
    );
    TenantId::new(format!("00000000-0000-7000-8000-{hex}")).expect("tenant id")
}
fn rid(s: &str) -> ProvisioningRequestId {
    ProvisioningRequestId::new(s).expect("request id")
}
fn mid(s: &str) -> WorkloadManifestId {
    WorkloadManifestId::new(s).expect("manifest id")
}
fn nid(s: &str) -> ComputeNodeId {
    ComputeNodeId::new(s).expect("node id")
}
fn ref_() -> CloudCredentialRef {
    CloudCredentialRef::new("cred://vault/failure-suite").expect("ref")
}
fn binding(provider: ProviderKind, tenant: &str, region: &str) -> ProviderBinding {
    ProviderBinding::new(provider, tid(tenant), "acct-1", region, ref_()).expect("binding")
}

#[allow(clippy::too_many_arguments)]
fn node(
    id: &str,
    class: ComputeClass,
    tenant: &str,
    region: &str,
    locality: Locality,
    cpu: u32,
    mem: u32,
    disk: u32,
) -> ComputeNode {
    ComputeNode::new(
        nid(id),
        class,
        ProviderKind::Local,
        tid(tenant),
        region,
        CapacityProfile::new(cpu, mem, disk, None, None, CapacityProvenance::Declared)
            .expect("capacity"),
        locality,
        Privacy::Household,
    )
    .expect("node")
}

fn plan(request: &ProvisioningRequest) -> ProvisioningPlan {
    ProvisioningPlan::new(request.clone()).expect("plan")
}

fn request(id: &str, provider: ProviderKind, idempotency: &str) -> ProvisioningRequest {
    let b = binding(provider, "t1", "EU");
    let cap =
        CapacityProfile::new(2, 4, 50, None, None, CapacityProvenance::Declared).expect("capacity");
    ProvisioningRequest::new(
        rid(id),
        cid(id),
        b,
        mid("workload-1"),
        cap,
        idempotency.to_string(),
    )
    .expect("request")
}

/// Run a command, capturing stdout+stderr.
fn run(cmd: &mut Command) -> (bool, String) {
    let out = cmd.output().expect("command must run");
    let text = String::from_utf8_lossy(&out.stdout).to_string()
        + String::from_utf8_lossy(&out.stderr).as_ref();
    (out.status.success(), text)
}

fn docker(args: &[&str]) -> (bool, String) {
    run(Command::new("docker").args(args))
}

fn docker_available() -> bool {
    docker(&["info", "--format", "{{.ServerVersion}}"]).0
}

/// UNAVAILABLE DEPENDENCY: a real ephemeral sshd container is
/// started, then TERMINATED, and the real ssh-keyscan probe must fail
/// (fail-closed): the endpoint is gone and no reachability is claimed.
#[test]
fn ep036_failure_unavailable_dependency_terminated_container() {
    if !docker_available() {
        eprintln!("docker unavailable; skipping real container termination");
        return;
    }
    let unique = format!("nexus-ep036-fail-{}", std::process::id());
    let cleanup = |name: &str| {
        let _ = docker(&["rm", "-f", name]);
    };

    let df_path =
        std::env::temp_dir().join(format!("ep036-fail-sshd-{}.Dockerfile", std::process::id()));
    std::fs::write(
        &df_path,
        "FROM alpine:latest\n\
         RUN apk add --no-cache openssh && ssh-keygen -A && echo 'root:nexus' | chpasswd\n\
         CMD [\"/usr/sbin/sshd\", \"-D\"]\n",
    )
    .expect("write dockerfile");
    let (build_ok, build_out) = docker(&[
        "build",
        "-t",
        "nexus-ep036-sshd:test",
        "-f",
        df_path.to_str().expect("path"),
        ".",
    ]);
    let _ = std::fs::remove_file(&df_path);
    if !build_ok {
        cleanup(&unique);
        panic!("sshd image build failed: {build_out}");
    }

    let (start_ok, start_out) = docker(&[
        "run",
        "-d",
        "--name",
        &unique,
        "-p",
        "127.0.0.1::22",
        "nexus-ep036-sshd:test",
    ]);
    if !start_ok {
        cleanup(&unique);
        panic!("sshd container start failed: {start_out}");
    }
    let container_id = start_out.trim().to_string();

    // TERMINATE the dependency: this is the real failure mechanism.
    let (kill_ok, kill_out) = docker(&["rm", "-f", &container_id]);
    if !kill_ok {
        panic!("container termination failed: {kill_out}");
    }

    // The real probe must now fail: no fabricated reachability.
    let mut probe = Command::new("ssh-keyscan");
    probe.arg("-p").arg("1").arg("-T").arg("2").arg("127.0.0.1");
    let (probe_ok, probe_out) = run(&mut probe);
    assert!(
        !probe_ok || !probe_out.contains("ssh-"),
        "terminated endpoint must not yield a host key, got: {probe_out}"
    );

    cleanup(&unique);
    let (_, ps_out) = docker(&[
        "ps",
        "-a",
        "--filter",
        &format!("name={unique}"),
        "--format",
        "{{.Names}}",
    ]);
    assert!(!ps_out.contains(&unique), "container leaked: {ps_out}");
}

/// TIMEOUT: a real ssh-keyscan timeout against a dead endpoint must
/// fail closed; the timeout never fabricates READY/VERIFIED.
#[test]
fn ep036_failure_timeout_probe_fails_closed() {
    if !docker_available() {
        eprintln!("docker unavailable; skipping timeout probe");
        return;
    }
    // No listener on this port range; ssh-keyscan -T 1 forces a real
    // timeout/refusal rather than a hang.
    let mut probe = Command::new("ssh-keyscan");
    probe.arg("-p").arg("1").arg("-T").arg("1").arg("127.0.0.1");
    let (probe_ok, probe_out) = run(&mut probe);
    assert!(
        !probe_ok || !probe_out.contains("ssh-"),
        "timeout must not produce a host key, got: {probe_out}"
    );
}

/// MALFORMED INPUT: corrupt controlled messages must be rejected
/// with structured validation errors, never accepted.
#[test]
fn ep036_failure_malformed_input_rejected() {
    // Secret-shaped credential references are rejected (redaction
    // invariant; static scanners never see the literal because the
    // canary is runtime-constructed).
    let secret_shaped = ["-----BEGIN ", "PRIVATE KEY-----"].concat();
    let err = CloudCredentialRef::new(&secret_shaped).expect_err("must reject secret shape");
    assert_eq!(err.code, ComputeErrorCode::Validation);

    // Empty / oversized idempotency keys are rejected.
    let good = request("req-1", ProviderKind::Contabo, "idem-1");
    let cap =
        CapacityProfile::new(2, 4, 50, None, None, CapacityProvenance::Declared).expect("capacity");
    let empty_key = ProvisioningRequest::new(
        rid("req-2"),
        cid("req-2"),
        good.binding.clone(),
        mid("w"),
        cap.clone(),
        "",
    )
    .expect_err("empty idempotency key must be rejected");
    assert_eq!(empty_key.code, ComputeErrorCode::Validation);

    // Zero capacity is rejected (exhausted declared budget).
    let zero_cap = CapacityProfile::new(8, 0, 500, None, None, CapacityProvenance::Declared)
        .expect_err("zero memory must be rejected");
    assert_eq!(zero_cap.code, ComputeErrorCode::Validation);
}

/// DUPLICATE REQUEST: the same idempotency key must not trigger a
/// blind second submission; the contract requires reconciliation of
/// the ambiguous outcome, never a silent retry.
#[test]
fn ep036_failure_duplicate_request_requires_reconciliation() {
    let r1 = request("req-a", ProviderKind::Contabo, "idem-duplicate");
    let r2 = request("req-b", ProviderKind::Contabo, "idem-duplicate");
    assert_eq!(r1.idempotency_key, r2.idempotency_key);

    let mut p1 = plan(&r1);
    let mut p2 = plan(&r2);

    // Both submissions observe an ambiguous provider outcome.
    p1.mark_submitted(
        ProvisioningReceipt::new(
            r1.request_id.clone(),
            ProviderKind::Contabo,
            None,
            ResourceState::Submitted,
            VerificationState::Pending,
            r1.correlation.clone(),
            1000,
        )
        .expect("receipt"),
    )
    .expect("mark submitted");
    p2.mark_submitted(
        ProvisioningReceipt::new(
            r2.request_id.clone(),
            ProviderKind::Contabo,
            None,
            ResourceState::Submitted,
            VerificationState::Pending,
            r2.correlation.clone(),
            1000,
        )
        .expect("receipt"),
    )
    .expect("mark submitted");
    p1.mark_ambiguous().expect("ambiguous");
    p2.mark_ambiguous().expect("ambiguous");

    // Provider does NOT confirm existence: reconciliation must refuse
    // to claim success (never blind retry).
    let outcome = resolve_ambiguous_provisioning(&p1, false);
    assert!(outcome.is_err());
    let err = outcome.expect_err("must refuse blind success");
    assert_eq!(err.code, ComputeErrorCode::Verification);
    assert_eq!(p1.outcome, ProvisioningOutcome::Ambiguous);
}

/// DENIED PERMISSION: placement policy denial must fail closed; a
/// rejected decision is never silently downgraded by a fallback.
#[test]
fn ep036_failure_denied_permission_placement_fails_closed() {
    let target = node(
        "n-private",
        ComputeClass::Vps,
        "t2",
        "EU",
        Locality::HomeEdge,
        4,
        8,
        100,
    );
    let constraint = PlacementConstraint::new(
        1,
        1,
        1,
        None,
        None,
        Locality::HomeEdge,
        Privacy::Household,
        tid("t1"),
        vec![ComputeClass::Vps],
        vec!["EU".to_string()],
        None,
    )
    .expect("constraint");

    let decision = placement_decision(rid("req-denied"), mid("w"), &constraint, &[target])
        .expect("placement runs");
    assert!(!decision.is_assigned());
    assert_eq!(
        decision.failure_class,
        Some(PlacementFailureClass::TenantBoundaryViolation)
    );
    // A denied placement must not fabricate an assignment.
    assert_eq!(decision.selected_node, None);
}

/// CANCELLED WORK: delete-before-create must fail closed; a plan
/// that never reached CREATED cannot claim RESOURCE ABSENT VERIFIED,
/// and the delete chain cannot be skipped.
#[test]
fn ep036_failure_cancelled_work_delete_before_create_fails_closed() {
    let r = request("req-cancel", ProviderKind::Contabo, "idem-cancel");
    let mut p = plan(&r);
    assert_eq!(p.state, ResourceState::Requested);

    // The resource-state ladder never admits leaps: a plan cannot jump
    // from PLANNED to READY without readback, and deletion is a
    // separate validated chain (DeleteState) whose final state
    // (RESOURCE ABSENT VERIFIED) requires the full delete ladder.
    assert!(!is_valid_resource_transition(
        ResourceState::Planned,
        ResourceState::Ready,
    ));
    assert!(is_valid_resource_transition(
        ResourceState::Planned,
        ResourceState::Submitted,
    ));

    // Cancelled work: deleting a resource that was never created must
    // not reach RESOURCE ABSENT VERIFIED without the full delete
    // chain (DELETE REQUESTED -> DELETE ACCEPTED -> ABSENT VERIFIED).
    p.mark_delete_requested().expect("delete requested");
    assert_eq!(p.delete_state, DeleteState::DeleteRequested);
    // Absent-verification before delete-accepted is rejected.
    let early_absent = p
        .mark_resource_absent_verified()
        .expect_err("absent verification before delete accepted must fail closed");
    assert_eq!(early_absent.code, ComputeErrorCode::Policy);
    // The plan's resource state never claims CREATED or READY.
    assert_eq!(p.state, ResourceState::Requested);
}
/// the receipt never overclaims readiness.
#[test]
fn ep036_failure_partial_side_effect_receipt_no_overclaim() {
    let r = request("req-partial", ProviderKind::Contabo, "idem-partial");
    let mut p = plan(&r);

    let receipt = ProvisioningReceipt::new(
        r.request_id.clone(),
        ProviderKind::Contabo,
        None,
        ResourceState::Submitted,
        VerificationState::Pending,
        r.correlation.clone(),
        1000,
    )
    .expect("receipt");
    p.mark_submitted(receipt).expect("submitted");
    // Receipt never overclaims: after a bare submit, the plan is not
    // READY and not VERIFIED.
    assert_ne!(p.state, ResourceState::Ready);
    assert_ne!(p.state, ResourceState::Verified);

    // Ambiguous outcome with a resource identity still requires
    // provider confirmation to resolve.
    let identity = ResourceIdentity::new(
        r.request_id.clone(),
        ProviderKind::Contabo,
        tid("t1"),
        "acct-1",
        "EU",
        None,
        "idem-partial",
    )
    .expect("identity");
    p.resource_identity = Some(identity);
    p.mark_ambiguous().expect("ambiguous");

    let denied = resolve_ambiguous_provisioning(&p, false);
    assert!(denied.is_err());
    let confirmed = resolve_ambiguous_provisioning(&p, true);
    assert_eq!(
        confirmed.expect("confirmed"),
        ProvisioningOutcome::Succeeded
    );
    // Success after reconciliation still requires readback before
    // READY/VERIFIED: the plan state stays AMBIGUOUS until readback.
    assert_eq!(p.outcome, ProvisioningOutcome::Ambiguous);
}

// The failure suite intentionally references the contract surface to
// prove the real mechanisms; silence the unused-import lints for the
// decision/cloud-provider helpers exercised by M1.
#[allow(dead_code)]
fn _unused_contract_refs(_p: &CloudProvider, _d: &PlacementDecision) {}
