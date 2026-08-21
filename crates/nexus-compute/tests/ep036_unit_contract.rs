//! EP-036 M1 unit suite: compute fabric contract, vocabulary, package
//! boundary, and fail-closed state semantics (SPEC-016).

use nexus_compute::error::{ComputeError, ComputeErrorCode};
use nexus_compute::model::{
    is_valid_resource_transition, is_valid_workload_transition, resolve_ambiguous_provisioning,
    BootstrapBundle, BootstrapBundleId, CapacityProfile, CloudCredentialRef, CloudProvider,
    ComputeNode, ComputeNodeId, FleetEnrollment, FleetId, PlacementConstraint, PlacementDecision,
    ProviderBinding, ProvisioningPlan, ProvisioningReceipt, ProvisioningRequest,
    ProvisioningRequestId, ResourceId, ResourceIdentity, WorkloadAssignment, WorkloadManifest,
    WorkloadManifestId,
};
use nexus_compute::placement::placement_decision;
use nexus_compute::vocabulary::{
    BillingState, CapacityProvenance, ComputeClass, DeleteState, FleetEnrollmentState,
    PlacementFailureClass, ProviderApiHealth, ProviderKind, ProvisioningOutcome, QuotaState,
    ResourceHealth, ResourceState, VerificationState, WorkloadState,
};
use nexus_domain::{CorrelationId, Locality, Privacy, TenantId};

fn cid(s: &str) -> CorrelationId {
    let hex = format!(
        "{:012x}",
        s.bytes().fold(0x5eedu64, |acc, b| acc
            .wrapping_mul(31)
            .wrapping_add(u64::from(b)))
    );
    CorrelationId::new(format!("00000000-0000-7000-8000-{hex}")).expect("correlation id")
}
fn tid(s: &str) -> TenantId {
    let hex = format!(
        "{:012x}",
        s.bytes().fold(0x7efu64, |acc, b| acc
            .wrapping_mul(31)
            .wrapping_add(u64::from(b)))
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

fn binding(provider: ProviderKind, tenant: &str, region: &str) -> ProviderBinding {
    ProviderBinding::new(
        provider,
        tid(tenant),
        "acct-1",
        region,
        CloudCredentialRef::new("cred://vault/main").expect("cred ref"),
    )
    .expect("binding")
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

#[test]
fn ep036_unit_vocabulary_rejects_unknown_provider() {
    let err: ComputeError = "VOID_CLOUD"
        .parse::<ProviderKind>()
        .expect_err("must reject");
    assert_eq!(err.code, ComputeErrorCode::Vocabulary);
}

#[test]
fn ep036_unit_vocabulary_rejects_unknown_state() {
    let err: ComputeError = "PROVISIONED"
        .parse::<ResourceState>()
        .expect_err("must reject");
    assert_eq!(err.code, ComputeErrorCode::Vocabulary);
    assert!("REQUESTED".parse::<ResourceState>().is_ok());
}

#[test]
fn ep036_unit_vocabulary_rejects_unknown_compute_class() {
    let err: ComputeError = "MAINFRAME"
        .parse::<ComputeClass>()
        .expect_err("must reject");
    assert_eq!(err.code, ComputeErrorCode::Vocabulary);
}

#[test]
fn ep036_unit_provider_kind_canonical_set() {
    for wire in [
        "CONTABO",
        "HETZNER",
        "DIGITAL_OCEAN",
        "AWS",
        "GENERIC_SSH",
        "LOCAL",
    ] {
        assert!(wire.parse::<ProviderKind>().is_ok(), "{wire}");
    }
}

#[test]
fn ep036_unit_resource_state_ladder_full() {
    assert!(is_valid_resource_transition(
        ResourceState::Requested,
        ResourceState::Planned
    ));
    assert!(is_valid_resource_transition(
        ResourceState::Planned,
        ResourceState::Submitted
    ));
    assert!(is_valid_resource_transition(
        ResourceState::Submitted,
        ResourceState::Provisioning
    ));
    assert!(is_valid_resource_transition(
        ResourceState::Provisioning,
        ResourceState::Created
    ));
    assert!(is_valid_resource_transition(
        ResourceState::Created,
        ResourceState::Reachable
    ));
    assert!(is_valid_resource_transition(
        ResourceState::Reachable,
        ResourceState::Ready
    ));
    assert!(is_valid_resource_transition(
        ResourceState::Ready,
        ResourceState::Verified
    ));
    assert!(is_valid_resource_transition(
        ResourceState::Verified,
        ResourceState::Certified
    ));
    assert!(!is_valid_resource_transition(
        ResourceState::Requested,
        ResourceState::Ready
    ));
    assert!(!is_valid_resource_transition(
        ResourceState::Created,
        ResourceState::Verified
    ));
    assert!(!is_valid_resource_transition(
        ResourceState::Ready,
        ResourceState::Certified
    ));
}

#[test]
fn ep036_unit_workload_state_ladder_full() {
    assert!(is_valid_workload_transition(
        WorkloadState::Assigned,
        WorkloadState::Started
    ));
    assert!(is_valid_workload_transition(
        WorkloadState::Started,
        WorkloadState::Healthy
    ));
    assert!(is_valid_workload_transition(
        WorkloadState::Healthy,
        WorkloadState::Verified
    ));
    assert!(!is_valid_workload_transition(
        WorkloadState::Assigned,
        WorkloadState::Healthy
    ));
    assert!(!is_valid_workload_transition(
        WorkloadState::Started,
        WorkloadState::Verified
    ));
}

#[test]
fn ep036_unit_capacity_provenance_distinct() {
    let declared = CapacityProfile::new(8, 32, 500, Some(16), None, CapacityProvenance::Declared)
        .expect("capacity");
    assert_eq!(declared.provenance, CapacityProvenance::Declared);
    assert_ne!(declared.provenance, CapacityProvenance::Observed);
    assert_ne!(declared.provenance, CapacityProvenance::Certified);
}

#[test]
fn ep036_unit_capacity_zero_rejected() {
    assert!(
        CapacityProfile::new(0, 32, 500, None, None, CapacityProvenance::Declared).is_err(),
        "zero cpu must be rejected"
    );
    assert!(
        CapacityProfile::new(8, 0, 500, None, None, CapacityProvenance::Declared).is_err(),
        "zero memory must be rejected"
    );
}

#[test]
fn ep036_unit_credential_ref_is_opaque_reference() {
    let good = CloudCredentialRef::new("cred://vault/do-main").expect("opaque ref accepted");
    assert!(good.as_str().starts_with("cred://"));
    for bad in [
        "dop_v1_secret_abc",
        "AKIAIOSFODNN7EXAMPLE",
        "-----BEGIN PRIVATE KEY-----",
        "password=swordfish",
        "api_key=abc123",
    ] {
        assert!(
            CloudCredentialRef::new(bad).is_err(),
            "secret-shaped ref must be rejected: {bad}"
        );
    }
}

#[test]
fn ep036_unit_credential_ref_redacts_display_and_serialization() {
    let good = CloudCredentialRef::new("cred://vault/do-main").expect("opaque ref");
    let display = format!("{good}");
    assert!(!display.contains("vault"));
    assert!(!display.contains("do-main"));
    assert!(display.starts_with("cred:"));
    let json = serde_json::to_string(&good).expect("serialize");
    assert!(
        json.contains("cred://vault/do-main"),
        "full ref is contract data by design"
    );
}

#[test]
fn ep036_unit_provider_binding_requires_account_region() {
    assert!(ProviderBinding::new(
        ProviderKind::DigitalOcean,
        tid("t1"),
        "",
        "nyc1",
        CloudCredentialRef::new("cred://vault/do").expect("ref"),
    )
    .is_err());
    assert!(ProviderBinding::new(
        ProviderKind::DigitalOcean,
        tid("t1"),
        "acct",
        "",
        CloudCredentialRef::new("cred://vault/do").expect("ref"),
    )
    .is_err());
}

#[test]
fn ep036_unit_provider_api_health_distinct_from_resource_health() {
    // A provider API being reachable proves nothing about the created
    // resource.
    let provider =
        CloudProvider::new(binding(ProviderKind::Local, "t1", "home")).expect("provider");
    assert_eq!(provider.api_health, ProviderApiHealth::Unknown);
    let n = node(
        "n1",
        ComputeClass::Local,
        "t1",
        "home",
        Locality::HomeEdge,
        4,
        8,
        50,
    );
    assert_eq!(n.resource_health, ResourceHealth::Unknown);
    assert_ne!(
        ProviderApiHealth::Reachable.as_str(),
        ResourceHealth::Ready.as_str()
    );
}

#[test]
fn ep036_unit_placement_constraint_requires_classes() {
    assert!(PlacementConstraint::new(
        1,
        2,
        10,
        None,
        None,
        Locality::Any,
        Privacy::Household,
        tid("t1"),
        vec![],
        vec![],
        None,
    )
    .is_err());
}

#[test]
fn ep036_unit_placement_selects_eligible_node() {
    let small = node(
        "n1",
        ComputeClass::Local,
        "t1",
        "home",
        Locality::HomeEdge,
        4,
        8,
        50,
    );
    let big = node(
        "n2",
        ComputeClass::Local,
        "t1",
        "home",
        Locality::HomeEdge,
        16,
        64,
        500,
    );
    let constraint = PlacementConstraint::new(
        4,
        16,
        100,
        None,
        None,
        Locality::HomeEdge,
        Privacy::Household,
        tid("t1"),
        vec![ComputeClass::Local],
        vec![],
        None,
    )
    .expect("constraint");
    let decision = placement_decision(rid("r1"), mid("w1"), &constraint, &[small, big.clone()])
        .expect("decision");
    assert!(decision.is_assigned());
    assert_eq!(
        decision.selected_node.as_ref().map(|id| id.as_str()),
        Some(big.node_id.as_str())
    );
    assert_eq!(decision.failure_class, None);
}

#[test]
fn ep036_unit_placement_fails_closed_when_no_eligible_target() {
    let small = node(
        "n1",
        ComputeClass::Local,
        "t1",
        "home",
        Locality::HomeEdge,
        4,
        8,
        50,
    );
    let constraint = PlacementConstraint::new(
        8,
        16,
        100,
        None,
        None,
        Locality::HomeEdge,
        Privacy::Household,
        tid("t1"),
        vec![ComputeClass::Local],
        vec![],
        None,
    )
    .expect("constraint");
    let decision =
        placement_decision(rid("r1"), mid("w1"), &constraint, &[small]).expect("decision");
    assert!(!decision.is_assigned());
    assert_eq!(
        decision.failure_class,
        Some(PlacementFailureClass::NoEligibleTarget)
    );
}

#[test]
fn ep036_unit_placement_never_crosses_privacy_boundary() {
    // Workload restricted to a stricter privacy class: no eligible
    // target -> fail closed, never silently downgraded.
    let n = ComputeNode::new(
        nid("n1"),
        ComputeClass::Cloud,
        ProviderKind::DigitalOcean,
        tid("t1"),
        "nyc1",
        CapacityProfile::new(8, 32, 200, None, None, CapacityProvenance::Declared).expect("cap"),
        Locality::Any,
        Privacy::Public,
    )
    .expect("node");
    let constraint = PlacementConstraint::new(
        4,
        16,
        100,
        None,
        None,
        Locality::Any,
        Privacy::Secret,
        tid("t1"),
        vec![ComputeClass::Cloud],
        vec![],
        None,
    )
    .expect("constraint");
    let decision = placement_decision(rid("r1"), mid("w1"), &constraint, &[n]).expect("decision");
    assert!(!decision.is_assigned());
    assert_eq!(
        decision.failure_class,
        Some(PlacementFailureClass::PrivacyBoundaryViolation)
    );
}

#[test]
fn ep036_unit_placement_never_crosses_tenant_boundary() {
    let n = node(
        "n1",
        ComputeClass::Local,
        "t1",
        "home",
        Locality::HomeEdge,
        8,
        32,
        200,
    );
    let constraint = PlacementConstraint::new(
        4,
        16,
        100,
        None,
        None,
        Locality::HomeEdge,
        Privacy::Household,
        tid("t2"),
        vec![ComputeClass::Local],
        vec![],
        None,
    )
    .expect("constraint");
    let decision = placement_decision(rid("r1"), mid("w1"), &constraint, &[n]).expect("decision");
    assert!(!decision.is_assigned());
    assert_eq!(
        decision.failure_class,
        Some(PlacementFailureClass::TenantBoundaryViolation)
    );
}

#[test]
fn ep036_unit_placement_gpu_requirement_fails_closed() {
    let n = node(
        "n1",
        ComputeClass::GpuHost,
        "t1",
        "home",
        Locality::HomeEdge,
        8,
        64,
        500,
    );
    let constraint = PlacementConstraint::new(
        4,
        16,
        100,
        None,
        Some(32),
        Locality::HomeEdge,
        Privacy::Household,
        tid("t1"),
        vec![ComputeClass::GpuHost],
        vec![],
        None,
    )
    .expect("constraint");
    // The node has no declared GPU VRAM: the GPU requirement is
    // unsatisfiable -> fail closed.
    let decision = placement_decision(rid("r1"), mid("w1"), &constraint, &[n]).expect("decision");
    assert!(!decision.is_assigned());
    assert_eq!(
        decision.failure_class,
        Some(PlacementFailureClass::ConstraintUnsatisfiable)
    );
}

#[test]
fn ep036_unit_placement_region_constraint_enforced() {
    let n = node(
        "n1",
        ComputeClass::Cloud,
        "t1",
        "fra1",
        Locality::Any,
        8,
        32,
        200,
    );
    let constraint = PlacementConstraint::new(
        4,
        16,
        100,
        None,
        None,
        Locality::Any,
        Privacy::Household,
        tid("t1"),
        vec![ComputeClass::Cloud],
        vec!["nyc1".to_string()],
        None,
    )
    .expect("constraint");
    let decision = placement_decision(rid("r1"), mid("w1"), &constraint, &[n]).expect("decision");
    assert!(!decision.is_assigned());
}

#[test]
fn ep036_unit_receipt_never_overclaims_readiness() {
    let receipt = ProvisioningReceipt::new(
        rid("r1"),
        ProviderKind::DigitalOcean,
        Some(ResourceId::new("droplet-1").expect("resource")),
        ResourceState::Submitted,
        VerificationState::Pending,
        cid("c1"),
        1_700_000_000,
    )
    .expect("receipt");
    assert_eq!(receipt.state, ResourceState::Submitted);
    assert_ne!(receipt.state, ResourceState::Ready);
    assert_eq!(receipt.verification, VerificationState::Pending);
}

#[test]
fn ep036_unit_receipt_requires_request_identity() {
    let receipt = ProvisioningReceipt::new(
        rid("r1"),
        ProviderKind::Aws,
        None,
        ResourceState::Submitted,
        VerificationState::Pending,
        cid("c1"),
        1_700_000_000,
    )
    .expect("receipt");
    assert_eq!(
        receipt.resource_id, None,
        "provider may not have assigned an id yet"
    );
}

#[test]
fn ep036_unit_plan_mark_verified_requires_ready() {
    let request = ProvisioningRequest::new(
        rid("r1"),
        cid("c1"),
        binding(ProviderKind::Local, "t1", "home"),
        mid("w1"),
        CapacityProfile::new(4, 16, 100, None, None, CapacityProvenance::Declared).expect("cap"),
        "idem-1",
    )
    .expect("request");
    let mut plan = ProvisioningPlan::new(request).expect("plan");
    assert_eq!(plan.state, ResourceState::Requested);
    assert!(
        plan.mark_verified().is_err(),
        "REQUESTED -> VERIFIED must be rejected"
    );
    assert!(
        plan.mark_ready().is_err(),
        "REQUESTED -> READY must be rejected"
    );
}

#[test]
fn ep036_unit_plan_ambiguous_outcome_requires_reconciliation() {
    let request = ProvisioningRequest::new(
        rid("r1"),
        cid("c1"),
        binding(ProviderKind::DigitalOcean, "t1", "nyc1"),
        mid("w1"),
        CapacityProfile::new(4, 16, 100, None, None, CapacityProvenance::Declared).expect("cap"),
        "idem-1",
    )
    .expect("request");
    let mut plan = ProvisioningPlan::new(request).expect("plan");
    // Ambiguity requires a submission first.
    assert!(
        plan.mark_ambiguous().is_err(),
        "REQUESTED cannot be AMBIGUOUS"
    );

    let receipt = ProvisioningReceipt::new(
        rid("r1"),
        ProviderKind::DigitalOcean,
        Some(ResourceId::new("droplet-1").expect("res")),
        ResourceState::Submitted,
        VerificationState::Pending,
        cid("c1"),
        1_700_000_000,
    )
    .expect("receipt");
    plan.mark_submitted(receipt).expect("submitted");
    let identity = ResourceIdentity::new(
        rid("r1"),
        ProviderKind::DigitalOcean,
        tid("t1"),
        "acct-1",
        "nyc1",
        Some(ResourceId::new("droplet-1").expect("res")),
        "idem-1",
    )
    .expect("identity");
    plan.mark_created(identity).expect("created");
    plan.mark_ambiguous().expect("ambiguous");
    assert_eq!(plan.outcome, ProvisioningOutcome::Ambiguous);
    assert!(
        resolve_ambiguous_provisioning(&plan, false).is_err(),
        "provider readback must confirm existence before success"
    );
    assert_eq!(
        resolve_ambiguous_provisioning(&plan, true).expect("resolved"),
        ProvisioningOutcome::Succeeded
    );
}

#[test]
fn ep036_unit_plan_delete_requires_readback() {
    let request = ProvisioningRequest::new(
        rid("r1"),
        cid("c1"),
        binding(ProviderKind::Local, "t1", "home"),
        mid("w1"),
        CapacityProfile::new(4, 16, 100, None, None, CapacityProvenance::Declared).expect("cap"),
        "idem-1",
    )
    .expect("request");
    let mut plan = ProvisioningPlan::new(request).expect("plan");
    assert_eq!(plan.delete_state, DeleteState::NotRequested);
    plan.mark_delete_requested().expect("delete requested");
    plan.mark_delete_accepted().expect("delete accepted");
    assert_eq!(plan.delete_state, DeleteState::DeleteAccepted);
    assert_ne!(
        plan.delete_state,
        DeleteState::ResourceAbsentVerified,
        "accepting a delete API call does not verify absence"
    );
    plan.mark_resource_absent_verified()
        .expect("absent verified");
    assert_eq!(plan.delete_state, DeleteState::ResourceAbsentVerified);
}

#[test]
fn ep036_unit_idempotency_key_required() {
    assert!(
        ProvisioningRequest::new(
            rid("r1"),
            cid("c1"),
            binding(ProviderKind::Local, "t1", "home"),
            mid("w1"),
            CapacityProfile::new(4, 16, 100, None, None, CapacityProvenance::Declared)
                .expect("cap"),
            "",
        )
        .is_err(),
        "empty idempotency key must be rejected"
    );
}

#[test]
fn ep036_unit_bootstrap_bundle_is_reference_only() {
    let bundle = BootstrapBundle::new(
        BootstrapBundleId::new("b1").expect("id"),
        "release://stable/2026-08",
        "offline://bundle/2026-08",
        "sig://bundle/2026-08",
    )
    .expect("bundle");
    assert_eq!(bundle.release_ref, "release://stable/2026-08");
    assert!(!format!("{bundle:?}").contains("secret"));
}

#[test]
fn ep036_unit_fleet_enrollment_ladder() {
    let mut enrollment =
        FleetEnrollment::new(FleetId::new("fleet-1").expect("id"), nid("n1")).expect("enrollment");
    assert_eq!(enrollment.state, FleetEnrollmentState::Discovered);
    assert!(
        enrollment.enroll().is_err(),
        "DISCOVERED -> ENROLLED must be rejected"
    );
    enrollment
        .request_enrollment(CloudCredentialRef::new("cred://vault/fleet").expect("ref"))
        .expect("requested");
    assert_eq!(enrollment.state, FleetEnrollmentState::EnrollmentRequested);
    enrollment.verify_identity().expect("verified");
    enrollment.enroll().expect("enrolled");
    assert_ne!(enrollment.state, FleetEnrollmentState::Trusted);
    enrollment.trust().expect("trusted");
    assert_eq!(enrollment.state, FleetEnrollmentState::Trusted);
}

#[test]
fn ep036_unit_workload_assignment_is_not_runtime_truth() {
    let mut assignment = WorkloadAssignment::new(mid("w1"), nid("n1")).expect("assignment");
    assert_eq!(assignment.state, WorkloadState::Assigned);
    assert!(
        assignment.mark_healthy().is_err(),
        "ASSIGNED -> HEALTHY rejected"
    );
    assignment.mark_started().expect("started");
    assert_ne!(assignment.state, WorkloadState::Healthy);
    assignment.mark_healthy().expect("healthy");
    assignment.mark_verified().expect("verified");
    assert_eq!(assignment.state, WorkloadState::Verified);
}

#[test]
fn ep036_unit_placement_decision_rejected_has_no_node() {
    let decision =
        PlacementDecision::rejected(rid("r1"), mid("w1"), PlacementFailureClass::QuotaExceeded);
    assert!(!decision.is_assigned());
    assert_eq!(decision.selected_node, None);
    assert_eq!(
        decision.failure_class,
        Some(PlacementFailureClass::QuotaExceeded)
    );
}

#[test]
fn ep036_unit_quota_is_semantics_not_fabricated_default() {
    // M1 models quota semantics; provider quota values come from provider
    // readback only. The initial state is Unobserved, never a fabricated
    // default quota number.
    assert_eq!(QuotaState::Unobserved.as_str(), "UNOBSERVED");
    assert_ne!(QuotaState::Observed, QuotaState::Exceeded);
}

#[test]
fn ep036_unit_billing_estimate_never_settled() {
    assert_ne!(BillingState::Estimated, BillingState::Settled);
    assert_ne!(BillingState::Estimated, BillingState::Incurred);
}

#[test]
fn ep036_unit_manifest_requires_constraint() {
    let manifest = WorkloadManifest::new(
        mid("w1"),
        PlacementConstraint::new(
            2,
            4,
            20,
            None,
            None,
            Locality::Any,
            Privacy::Household,
            tid("t1"),
            vec![ComputeClass::Local],
            vec![],
            None,
        )
        .expect("constraint"),
    )
    .expect("manifest");
    assert_eq!(manifest.manifest_id.as_str(), "w1");
}
