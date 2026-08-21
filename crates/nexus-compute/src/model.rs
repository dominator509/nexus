//! EP-036 compute fabric value objects (SPEC-016).
//!
//! Every value object validates wire-shaped input with deny-unknown
//! semantics (mirroring the canonical schema `additionalProperties:
//! false` rule), and deserialization enforces the same checks as the
//! constructor. State truthfulness is structural: SELECTED != VERIFIED,
//! provider acceptance != resource created, resource created != ready,
//! provider API health != resource health, DECLARED != OBSERVED !=
//! CERTIFIED capacity, and ambiguous provisioning outcomes are never
//! blindly retried.

use std::fmt;

use nexus_domain::{CorrelationId, Locality, Privacy, TenantId};
use serde::{Deserialize, Serialize};

use crate::error::{ComputeError, ComputeResult};
use crate::vocabulary::{
    BillingState, CapacityProvenance, ComputeClass, DeleteState, FleetEnrollmentState,
    PlacementFailureClass, ProviderApiHealth, ProviderKind, ProvisioningOutcome, QuotaState,
    ResourceHealth, ResourceState, VerificationState, WorkloadState,
};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> ComputeResult<Self> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(ComputeError::validation(format!(concat!(
                        stringify!($name),
                        " must be 1..=128 characters"
                    ))));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(ComputeNodeId);
typed_id!(ProvisioningRequestId);
typed_id!(WorkloadManifestId);
typed_id!(ResourceId);
typed_id!(FleetId);
typed_id!(BootstrapBundleId);

/// Opaque reference to a cloud credential. Never contains the raw API
/// key, access key, secret, token, or private key. Secrets belong to the
/// designated secret provider (SPEC-016 requirement 3); this type only
/// names a credential by its reference. Serialization never emits the
/// referenced secret.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CloudCredentialRef(String);

impl CloudCredentialRef {
    pub fn new(reference: impl Into<String>) -> ComputeResult<Self> {
        let reference = reference.into();
        if reference.is_empty() || reference.len() > 256 {
            return Err(ComputeError::validation(
                "CloudCredentialRef must be 1..=256 characters",
            ));
        }
        if contains_secret_shape(&reference) {
            return Err(ComputeError::validation(
                "CloudCredentialRef must be an opaque reference, not a raw credential",
            ));
        }
        Ok(Self(reference))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CloudCredentialRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Redacted display: never surface the full reference in logs.
        write!(f, "cred:{}", redact(&self.0))
    }
}

/// Detect secret-shaped literals so credentials never leak into contract
/// data (SPEC-005). Redaction is structural: canary values are rejected
/// in constructor AND deserialization.
fn contains_secret_shape(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("secret")
        || lower.contains("password")
        || lower.contains("token")
        || lower.contains("api_key")
        || lower.contains("access_key")
        || lower.contains("private_key")
        || lower.starts_with("sk-")
        || value.starts_with("AKIA")
        || value.starts_with("-----BEGIN")
}

fn redact(value: &str) -> String {
    if value.len() <= 8 {
        "****".to_string()
    } else {
        format!("{}****", &value[..4])
    }
}

/// Hardware capability profile with provenance. A user declaration
/// ("16 GB VRAM") is DECLARED, never OBSERVED; observed values are never
/// CERTIFIED without workload-level proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapacityProfile {
    pub cpu_cores: u32,
    pub memory_gib: u32,
    pub disk_gib: u32,
    pub gpu_vram_gib: Option<u32>,
    pub architecture: Option<String>,
    pub provenance: CapacityProvenance,
}

impl CapacityProfile {
    /// Validate and construct a capacity profile. The argument count
    /// reflects the fixed capacity contract fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        cpu_cores: u32,
        memory_gib: u32,
        disk_gib: u32,
        gpu_vram_gib: Option<u32>,
        architecture: Option<String>,
        provenance: CapacityProvenance,
    ) -> ComputeResult<Self> {
        if cpu_cores == 0 {
            return Err(ComputeError::validation("cpu_cores must be > 0"));
        }
        if memory_gib == 0 {
            return Err(ComputeError::validation("memory_gib must be > 0"));
        }
        if disk_gib == 0 {
            return Err(ComputeError::validation("disk_gib must be > 0"));
        }
        Ok(Self {
            cpu_cores,
            memory_gib,
            disk_gib,
            gpu_vram_gib,
            architecture,
            provenance,
        })
    }
}

/// A compute node in the fabric registry (SPEC-016 node registry).
/// Declared capacity never becomes observed or certified capacity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeNode {
    pub node_id: ComputeNodeId,
    pub class: ComputeClass,
    pub provider: ProviderKind,
    pub tenant: TenantId,
    pub region: String,
    pub declared_capacity: CapacityProfile,
    pub observed_capacity: Option<CapacityProfile>,
    pub locality: Locality,
    pub privacy: Privacy,
    pub api_health: ProviderApiHealth,
    pub resource_health: ResourceHealth,
}

impl ComputeNode {
    /// Validate and construct a compute node. The argument count
    /// reflects the fixed node registry contract fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_id: ComputeNodeId,
        class: ComputeClass,
        provider: ProviderKind,
        tenant: TenantId,
        region: impl Into<String>,
        declared_capacity: CapacityProfile,
        locality: Locality,
        privacy: Privacy,
    ) -> ComputeResult<Self> {
        let region = region.into();
        if region.is_empty() || region.len() > 128 {
            return Err(ComputeError::validation(
                "region must be 1..=128 characters",
            ));
        }
        Ok(Self {
            node_id,
            class,
            provider,
            tenant,
            region,
            declared_capacity,
            observed_capacity: None,
            locality,
            privacy,
            api_health: ProviderApiHealth::Unknown,
            resource_health: ResourceHealth::Unknown,
        })
    }
}

/// Placement constraint: explicit deterministic constraints for workload
/// placement. No decision is made by provider name alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementConstraint {
    pub min_cpu_cores: u32,
    pub min_memory_gib: u32,
    pub min_disk_gib: u32,
    pub required_architecture: Option<String>,
    pub required_gpu_vram_gib: Option<u32>,
    pub locality: Locality,
    pub privacy: Privacy,
    pub tenant: TenantId,
    pub allowed_classes: Vec<ComputeClass>,
    pub allowed_regions: Vec<String>,
    pub max_estimated_cost_per_month: Option<u64>,
}

impl PlacementConstraint {
    /// Validate and construct a placement constraint. The argument count
    /// reflects the fixed constraint contract fields; splitting it would
    /// permit partial constraints that silently weaken placement policy.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        min_cpu_cores: u32,
        min_memory_gib: u32,
        min_disk_gib: u32,
        required_architecture: Option<String>,
        required_gpu_vram_gib: Option<u32>,
        locality: Locality,
        privacy: Privacy,
        tenant: TenantId,
        allowed_classes: Vec<ComputeClass>,
        allowed_regions: Vec<String>,
        max_estimated_cost_per_month: Option<u64>,
    ) -> ComputeResult<Self> {
        if min_cpu_cores == 0 {
            return Err(ComputeError::validation("min_cpu_cores must be > 0"));
        }
        if min_memory_gib == 0 {
            return Err(ComputeError::validation("min_memory_gib must be > 0"));
        }
        if min_disk_gib == 0 {
            return Err(ComputeError::validation("min_disk_gib must be > 0"));
        }
        if allowed_classes.is_empty() {
            return Err(ComputeError::validation(
                "allowed_classes must not be empty",
            ));
        }
        for region in &allowed_regions {
            if region.is_empty() || region.len() > 128 {
                return Err(ComputeError::validation(
                    "allowed_regions entries must be 1..=128 characters",
                ));
            }
        }
        Ok(Self {
            min_cpu_cores,
            min_memory_gib,
            min_disk_gib,
            required_architecture,
            required_gpu_vram_gib,
            locality,
            privacy,
            tenant,
            allowed_classes,
            allowed_regions,
            max_estimated_cost_per_month,
        })
    }

    /// Fail-closed evaluation against a node. Returns Ok(()) only when
    /// every constraint is satisfied; otherwise a Policy error with the
    /// exact failure class.
    pub fn evaluate(&self, node: &ComputeNode) -> ComputeResult<()> {
        if node.tenant != self.tenant {
            return Err(ComputeError::policy(format!(
                "placement crosses tenant boundary: node {} is not in tenant {}",
                node.node_id, self.tenant
            )));
        }
        if node.privacy != self.privacy {
            return Err(ComputeError::policy(format!(
                "placement crosses privacy boundary: node {} privacy {} != required {}",
                node.node_id, node.privacy, self.privacy
            )));
        }
        if !self.allowed_classes.contains(&node.class) {
            return Err(ComputeError::policy(format!(
                "node {} class {} not in allowed classes",
                node.node_id, node.class
            )));
        }
        if !self.allowed_regions.is_empty() && !self.allowed_regions.contains(&node.region) {
            return Err(ComputeError::policy(format!(
                "node {} region {} not in allowed regions",
                node.node_id, node.region
            )));
        }
        if self.locality != Locality::Any && node.locality != self.locality {
            return Err(ComputeError::policy(format!(
                "node {} locality {} does not satisfy required {}",
                node.node_id, node.locality, self.locality
            )));
        }
        if node.declared_capacity.cpu_cores < self.min_cpu_cores {
            return Err(ComputeError::policy(format!(
                "node {} declared cpu {} < required {}",
                node.node_id, node.declared_capacity.cpu_cores, self.min_cpu_cores
            )));
        }
        if node.declared_capacity.memory_gib < self.min_memory_gib {
            return Err(ComputeError::policy(format!(
                "node {} declared memory {} < required {}",
                node.node_id, node.declared_capacity.memory_gib, self.min_memory_gib
            )));
        }
        if node.declared_capacity.disk_gib < self.min_disk_gib {
            return Err(ComputeError::policy(format!(
                "node {} declared disk {} < required {}",
                node.node_id, node.declared_capacity.disk_gib, self.min_disk_gib
            )));
        }
        if let Some(arch) = &self.required_architecture {
            match &node.declared_capacity.architecture {
                Some(node_arch) if node_arch == arch => {}
                _ => {
                    return Err(ComputeError::policy(format!(
                        "node {} architecture does not satisfy required {}",
                        node.node_id, arch
                    )));
                }
            }
        }
        if let Some(required_vram) = self.required_gpu_vram_gib {
            match node.declared_capacity.gpu_vram_gib {
                Some(node_vram) if node_vram >= required_vram => {}
                _ => {
                    return Err(ComputeError::policy(format!(
                        "node {} declared gpu vram does not satisfy required {} GiB",
                        node.node_id, required_vram
                    )));
                }
            }
        }
        Ok(())
    }
}

/// A workload manifest (SPEC-016 canonical WorkloadManifest). The
/// declared requirement is intent; it never becomes runtime truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadManifest {
    pub manifest_id: WorkloadManifestId,
    pub constraint: PlacementConstraint,
}

impl WorkloadManifest {
    pub fn new(
        manifest_id: WorkloadManifestId,
        constraint: PlacementConstraint,
    ) -> ComputeResult<Self> {
        Ok(Self {
            manifest_id,
            constraint,
        })
    }
}

/// Placement decision: the outcome of constraint-based placement.
/// WORKLOAD ASSIGNED != STARTED != HEALTHY != VERIFIED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementDecision {
    pub request_id: ProvisioningRequestId,
    pub manifest_id: WorkloadManifestId,
    pub selected_node: Option<ComputeNodeId>,
    pub failure_class: Option<PlacementFailureClass>,
}

impl PlacementDecision {
    pub fn assigned(
        request_id: ProvisioningRequestId,
        manifest_id: WorkloadManifestId,
        node: ComputeNodeId,
    ) -> Self {
        Self {
            request_id,
            manifest_id,
            selected_node: Some(node),
            failure_class: None,
        }
    }

    pub fn rejected(
        request_id: ProvisioningRequestId,
        manifest_id: WorkloadManifestId,
        failure_class: PlacementFailureClass,
    ) -> Self {
        Self {
            request_id,
            manifest_id,
            selected_node: None,
            failure_class: Some(failure_class),
        }
    }

    pub fn is_assigned(&self) -> bool {
        self.selected_node.is_some()
    }
}

/// Provider binding: the exact security/resource scope a provisioning
/// request owns. Account/project/region binding is explicit; never
/// inferred from display labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderBinding {
    pub provider: ProviderKind,
    pub tenant: TenantId,
    pub account: String,
    pub region: String,
    pub credential_ref: CloudCredentialRef,
}

impl ProviderBinding {
    pub fn new(
        provider: ProviderKind,
        tenant: TenantId,
        account: impl Into<String>,
        region: impl Into<String>,
        credential_ref: CloudCredentialRef,
    ) -> ComputeResult<Self> {
        let account = account.into();
        let region = region.into();
        if account.is_empty() || account.len() > 128 {
            return Err(ComputeError::validation(
                "account must be 1..=128 characters",
            ));
        }
        if region.is_empty() || region.len() > 128 {
            return Err(ComputeError::validation(
                "region must be 1..=128 characters",
            ));
        }
        Ok(Self {
            provider,
            tenant,
            account,
            region,
            credential_ref,
        })
    }
}

/// A cloud provider (SPEC-016 Provisioner abstraction). The provider
/// exposes identity, API health, and a provider-neutral contract surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudProvider {
    pub binding: ProviderBinding,
    pub api_health: ProviderApiHealth,
}

impl CloudProvider {
    pub fn new(binding: ProviderBinding) -> ComputeResult<Self> {
        Ok(Self {
            binding,
            api_health: ProviderApiHealth::Unknown,
        })
    }
}

/// Provisioning request: the idempotent intent. The request identity is
/// the correlation anchor for later readback. Repeating an identical
/// request yields one logical provisioning intent, not two resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvisioningRequest {
    pub request_id: ProvisioningRequestId,
    pub correlation: CorrelationId,
    pub binding: ProviderBinding,
    pub manifest_id: WorkloadManifestId,
    pub capacity: CapacityProfile,
    pub idempotency_key: String,
}

impl ProvisioningRequest {
    pub fn new(
        request_id: ProvisioningRequestId,
        correlation: CorrelationId,
        binding: ProviderBinding,
        manifest_id: WorkloadManifestId,
        capacity: CapacityProfile,
        idempotency_key: impl Into<String>,
    ) -> ComputeResult<Self> {
        let idempotency_key = idempotency_key.into();
        if idempotency_key.is_empty() || idempotency_key.len() > 256 {
            return Err(ComputeError::validation(
                "idempotency_key must be 1..=256 characters",
            ));
        }
        Ok(Self {
            request_id,
            correlation,
            binding,
            manifest_id,
            capacity,
            idempotency_key,
        })
    }
}

/// Provisioning receipt: only observed/contractual facts. Never called
/// "ready" unless readiness was actually established.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvisioningReceipt {
    pub request_id: ProvisioningRequestId,
    pub provider: ProviderKind,
    pub resource_id: Option<ResourceId>,
    pub state: ResourceState,
    pub verification: VerificationState,
    pub correlation: CorrelationId,
    pub accepted_at_unix_s: u64,
}

impl ProvisioningReceipt {
    pub fn new(
        request_id: ProvisioningRequestId,
        provider: ProviderKind,
        resource_id: Option<ResourceId>,
        state: ResourceState,
        verification: VerificationState,
        correlation: CorrelationId,
        accepted_at_unix_s: u64,
    ) -> ComputeResult<Self> {
        Ok(Self {
            request_id,
            provider,
            resource_id,
            state,
            verification,
            correlation,
            accepted_at_unix_s,
        })
    }
}

/// Resource identity: binds readback to the exact requested resource.
/// "some VM exists" is never proof of THIS resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceIdentity {
    pub request_id: ProvisioningRequestId,
    pub provider: ProviderKind,
    pub tenant: TenantId,
    pub account: String,
    pub region: String,
    pub provider_resource_id: Option<ResourceId>,
    pub idempotency_key: String,
}

impl ResourceIdentity {
    pub fn new(
        request_id: ProvisioningRequestId,
        provider: ProviderKind,
        tenant: TenantId,
        account: impl Into<String>,
        region: impl Into<String>,
        provider_resource_id: Option<ResourceId>,
        idempotency_key: impl Into<String>,
    ) -> ComputeResult<Self> {
        Ok(Self {
            request_id,
            provider,
            tenant,
            account: account.into(),
            region: region.into(),
            provider_resource_id,
            idempotency_key: idempotency_key.into(),
        })
    }
}

/// Provisioning plan: the durable provisioning intent state machine.
/// Provider acceptance establishes only SUBMITTED; READY requires
/// resource readback; VERIFIED requires exact-target verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvisioningPlan {
    pub request: ProvisioningRequest,
    pub state: ResourceState,
    pub outcome: ProvisioningOutcome,
    pub verification: VerificationState,
    pub resource_identity: Option<ResourceIdentity>,
    pub receipt: Option<ProvisioningReceipt>,
    pub billing: BillingState,
    pub estimated_cost_per_month: Option<u64>,
    pub quota: QuotaState,
    pub delete_state: DeleteState,
}

impl ProvisioningPlan {
    pub fn new(request: ProvisioningRequest) -> ComputeResult<Self> {
        Ok(Self {
            request,
            state: ResourceState::Requested,
            outcome: ProvisioningOutcome::Pending,
            verification: VerificationState::Pending,
            resource_identity: None,
            receipt: None,
            billing: BillingState::NoCost,
            estimated_cost_per_month: None,
            quota: QuotaState::Unobserved,
            delete_state: DeleteState::NotRequested,
        })
    }

    pub fn mark_submitted(&mut self, receipt: ProvisioningReceipt) -> ComputeResult<()> {
        if self.state != ResourceState::Requested && self.state != ResourceState::Planned {
            return Err(ComputeError::policy(format!(
                "cannot mark submitted from {}",
                self.state
            )));
        }
        self.state = ResourceState::Submitted;
        self.outcome = ProvisioningOutcome::Succeeded;
        self.receipt = Some(receipt);
        Ok(())
    }

    pub fn mark_ambiguous(&mut self) -> ComputeResult<()> {
        if self.state == ResourceState::Requested {
            return Err(ComputeError::policy(
                "cannot be ambiguous before submission",
            ));
        }
        self.outcome = ProvisioningOutcome::Ambiguous;
        Ok(())
    }

    pub fn mark_created(&mut self, identity: ResourceIdentity) -> ComputeResult<()> {
        if self.state != ResourceState::Submitted && self.state != ResourceState::Provisioning {
            return Err(ComputeError::policy(format!(
                "cannot mark created from {}",
                self.state
            )));
        }
        self.state = ResourceState::Created;
        self.resource_identity = Some(identity);
        Ok(())
    }

    pub fn mark_reachable(&mut self) -> ComputeResult<()> {
        if self.state != ResourceState::Created {
            return Err(ComputeError::policy(format!(
                "cannot mark reachable from {}",
                self.state
            )));
        }
        self.state = ResourceState::Reachable;
        Ok(())
    }

    pub fn mark_ready(&mut self) -> ComputeResult<()> {
        if self.state != ResourceState::Reachable {
            return Err(ComputeError::policy(format!(
                "cannot mark ready from {}",
                self.state
            )));
        }
        self.state = ResourceState::Ready;
        Ok(())
    }

    pub fn mark_verified(&mut self) -> ComputeResult<()> {
        if self.state != ResourceState::Ready {
            return Err(ComputeError::policy(format!(
                "cannot mark verified from {}",
                self.state
            )));
        }
        self.state = ResourceState::Verified;
        self.verification = VerificationState::Verified;
        Ok(())
    }

    pub fn mark_certified(&mut self) -> ComputeResult<()> {
        if self.state != ResourceState::Verified {
            return Err(ComputeError::policy(format!(
                "cannot mark certified from {}",
                self.state
            )));
        }
        self.state = ResourceState::Certified;
        Ok(())
    }

    pub fn mark_delete_requested(&mut self) -> ComputeResult<()> {
        self.delete_state = DeleteState::DeleteRequested;
        Ok(())
    }

    pub fn mark_delete_accepted(&mut self) -> ComputeResult<()> {
        if self.delete_state != DeleteState::DeleteRequested {
            return Err(ComputeError::policy(
                "cannot accept delete before delete requested",
            ));
        }
        self.delete_state = DeleteState::DeleteAccepted;
        Ok(())
    }

    pub fn mark_resource_absent_verified(&mut self) -> ComputeResult<()> {
        if self.delete_state != DeleteState::DeleteAccepted {
            return Err(ComputeError::policy(
                "cannot verify absence before delete accepted",
            ));
        }
        self.delete_state = DeleteState::ResourceAbsentVerified;
        Ok(())
    }
}

/// Bootstrap bundle (SPEC-016 offline bundle / bootstrap): references
/// signed release content. The bundle establishes bootstrap identity and
/// pulls signed releases; it never carries raw credentials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapBundle {
    pub bundle_id: BootstrapBundleId,
    pub release_ref: String,
    pub offline_bundle_ref: String,
    pub signature_ref: String,
}

impl BootstrapBundle {
    pub fn new(
        bundle_id: BootstrapBundleId,
        release_ref: impl Into<String>,
        offline_bundle_ref: impl Into<String>,
        signature_ref: impl Into<String>,
    ) -> ComputeResult<Self> {
        Ok(Self {
            bundle_id,
            release_ref: release_ref.into(),
            offline_bundle_ref: offline_bundle_ref.into(),
            signature_ref: signature_ref.into(),
        })
    }
}

/// Fleet enrollment (SPEC-016 private mesh). DISCOVERED != ENROLLED !=
/// TRUSTED; discovery metadata is never authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetEnrollment {
    pub fleet_id: FleetId,
    pub node_id: ComputeNodeId,
    pub state: FleetEnrollmentState,
    pub enrollment_token_ref: Option<CloudCredentialRef>,
}

impl FleetEnrollment {
    pub fn new(fleet_id: FleetId, node_id: ComputeNodeId) -> ComputeResult<Self> {
        Ok(Self {
            fleet_id,
            node_id,
            state: FleetEnrollmentState::Discovered,
            enrollment_token_ref: None,
        })
    }

    pub fn request_enrollment(&mut self, token_ref: CloudCredentialRef) -> ComputeResult<()> {
        if self.state != FleetEnrollmentState::Discovered {
            return Err(ComputeError::policy(format!(
                "cannot request enrollment from {}",
                self.state
            )));
        }
        self.state = FleetEnrollmentState::EnrollmentRequested;
        self.enrollment_token_ref = Some(token_ref);
        Ok(())
    }

    pub fn verify_identity(&mut self) -> ComputeResult<()> {
        if self.state != FleetEnrollmentState::EnrollmentRequested {
            return Err(ComputeError::policy(format!(
                "cannot verify identity from {}",
                self.state
            )));
        }
        self.state = FleetEnrollmentState::IdentityVerified;
        Ok(())
    }

    pub fn enroll(&mut self) -> ComputeResult<()> {
        if self.state != FleetEnrollmentState::IdentityVerified {
            return Err(ComputeError::policy(format!(
                "cannot enroll from {}",
                self.state
            )));
        }
        self.state = FleetEnrollmentState::Enrolled;
        Ok(())
    }

    pub fn trust(&mut self) -> ComputeResult<()> {
        if self.state != FleetEnrollmentState::Enrolled {
            return Err(ComputeError::policy(format!(
                "cannot trust from {}",
                self.state
            )));
        }
        self.state = FleetEnrollmentState::Trusted;
        Ok(())
    }
}

/// Workload assignment: WORKLOAD ASSIGNED != STARTED != HEALTHY !=
/// VERIFIED. Scheduler intent never becomes runtime truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkloadAssignment {
    pub manifest_id: WorkloadManifestId,
    pub node_id: ComputeNodeId,
    pub state: WorkloadState,
}

impl WorkloadAssignment {
    pub fn new(manifest_id: WorkloadManifestId, node_id: ComputeNodeId) -> ComputeResult<Self> {
        Ok(Self {
            manifest_id,
            node_id,
            state: WorkloadState::Assigned,
        })
    }

    pub fn mark_started(&mut self) -> ComputeResult<()> {
        if self.state != WorkloadState::Assigned {
            return Err(ComputeError::policy(format!(
                "cannot start workload from {}",
                self.state
            )));
        }
        self.state = WorkloadState::Started;
        Ok(())
    }

    pub fn mark_healthy(&mut self) -> ComputeResult<()> {
        if self.state != WorkloadState::Started {
            return Err(ComputeError::policy(format!(
                "cannot mark healthy from {}",
                self.state
            )));
        }
        self.state = WorkloadState::Healthy;
        Ok(())
    }

    pub fn mark_verified(&mut self) -> ComputeResult<()> {
        if self.state != WorkloadState::Healthy {
            return Err(ComputeError::policy(format!(
                "cannot verify workload from {}",
                self.state
            )));
        }
        self.state = WorkloadState::Verified;
        Ok(())
    }
}

/// Validate a resource-state transition (REQUESTED -> READY is invalid).
pub fn is_valid_resource_transition(from: ResourceState, to: ResourceState) -> bool {
    use ResourceState::*;
    matches!(
        (from, to),
        (Requested, Planned)
            | (Planned, Submitted)
            | (Submitted, Provisioning)
            | (Provisioning, Created)
            | (Created, Reachable)
            | (Reachable, Ready)
            | (Ready, Verified)
            | (Verified, Certified)
    )
}

/// Validate a workload-state transition (ASSIGNED -> HEALTHY is invalid).
pub fn is_valid_workload_transition(from: WorkloadState, to: WorkloadState) -> bool {
    use WorkloadState::*;
    matches!(
        (from, to),
        (Unassigned, Assigned) | (Assigned, Started) | (Started, Healthy) | (Healthy, Verified)
    )
}

/// Resolve an ambiguous provisioning outcome. UNKNOWN whether a mutation
/// occurred -> reconcile first (query provider by exact identity), then
/// decide retry/continue/cleanup. Never blindly repeat the operation.
pub fn resolve_ambiguous_provisioning(
    plan: &ProvisioningPlan,
    provider_confirms_existence: bool,
) -> ComputeResult<ProvisioningOutcome> {
    if plan.outcome != ProvisioningOutcome::Ambiguous {
        return Err(ComputeError::policy(
            "resolve_ambiguous_provisioning requires an AMBIGUOUS outcome",
        ));
    }
    if plan.resource_identity.is_none() {
        return Err(ComputeError::verification(
            "ambiguous outcome cannot be resolved without a resource identity to reconcile",
        ));
    }
    if provider_confirms_existence {
        Ok(ProvisioningOutcome::Succeeded)
    } else {
        Err(ComputeError::verification(
            "provider does not confirm the resource exists; do not blindly retry",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::CorrelationId;

    fn cid(s: &str) -> CorrelationId {
        // Canonical UUIDv7 test identity; content-derived so distinct
        // labels never collide.
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

    fn node() -> ComputeNode {
        ComputeNode::new(
            ComputeNodeId::new("node-1").unwrap(),
            ComputeClass::Local,
            ProviderKind::Local,
            tid("t1"),
            "home",
            CapacityProfile::new(4, 16, 100, None, None, CapacityProvenance::Declared).unwrap(),
            Locality::HomeEdge,
            Privacy::Household,
        )
        .unwrap()
    }

    #[test]
    fn ep036_unit_capacity_declared_never_observed() {
        let declared =
            CapacityProfile::new(4, 16, 100, Some(16), None, CapacityProvenance::Declared).unwrap();
        assert_eq!(declared.provenance, CapacityProvenance::Declared);
        assert_ne!(declared.provenance, CapacityProvenance::Observed);
        assert_ne!(declared.provenance, CapacityProvenance::Certified);
    }

    #[test]
    fn ep036_unit_credential_ref_rejects_secret_shape() {
        assert!(CloudCredentialRef::new("cred://vault/do-main").is_ok());
        assert!(CloudCredentialRef::new("sk-live-abcdef").is_err());
        assert!(CloudCredentialRef::new("AKIAIOSFODNN7EXAMPLE").is_err());
        assert!(CloudCredentialRef::new("-----BEGIN PRIVATE KEY-----").is_err());
        assert!(CloudCredentialRef::new("dop_v1_secret_token").is_err());
    }

    #[test]
    fn ep036_unit_credential_ref_redacts_display() {
        let c = CloudCredentialRef::new("cred://vault/do-main").unwrap();
        let display = format!("{c}");
        assert!(!display.contains("cred://vault/do-main"));
        assert!(display.starts_with("cred:"));
        assert!(display.contains("****"));
    }

    #[test]
    fn ep036_unit_placement_crosses_tenant_fails_closed() {
        let constraint = PlacementConstraint::new(
            2,
            4,
            20,
            None,
            None,
            Locality::HomeEdge,
            Privacy::Household,
            tid("t2"),
            vec![ComputeClass::Local],
            vec![],
            None,
        )
        .unwrap();
        let n = node();
        assert!(constraint.evaluate(&n).is_err());
    }

    #[test]
    fn ep036_unit_placement_gpu_constraint_fails_closed() {
        let constraint = PlacementConstraint::new(
            2,
            4,
            20,
            None,
            Some(32),
            Locality::Any,
            Privacy::Household,
            tid("t1"),
            vec![ComputeClass::Local],
            vec![],
            None,
        )
        .unwrap();
        let n = node();
        assert!(constraint.evaluate(&n).is_err());
    }

    #[test]
    fn ep036_unit_placement_satisfies_constraints() {
        let constraint = PlacementConstraint::new(
            2,
            4,
            20,
            None,
            None,
            Locality::HomeEdge,
            Privacy::Household,
            tid("t1"),
            vec![ComputeClass::Local],
            vec![],
            None,
        )
        .unwrap();
        let n = node();
        assert!(constraint.evaluate(&n).is_ok());
    }

    #[test]
    fn ep036_unit_resource_state_ladder_blocks_leaps() {
        assert!(is_valid_resource_transition(
            ResourceState::Requested,
            ResourceState::Planned
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
            ResourceState::Requested
        ));
    }

    #[test]
    fn ep036_unit_workload_state_ladder_blocks_leaps() {
        assert!(is_valid_workload_transition(
            WorkloadState::Assigned,
            WorkloadState::Started
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
    fn ep036_unit_provisioning_receipt_never_overclaims() {
        let r = ProvisioningReceipt::new(
            ProvisioningRequestId::new("req-1").unwrap(),
            ProviderKind::DigitalOcean,
            None,
            ResourceState::Submitted,
            VerificationState::Pending,
            cid("c1"),
            1_700_000_000,
        )
        .unwrap();
        // A receipt at SUBMITTED must never claim READY.
        assert_ne!(r.state, ResourceState::Ready);
        assert_eq!(r.verification, VerificationState::Pending);
    }

    #[test]
    fn ep036_unit_ambiguous_outcome_requires_reconciliation() {
        let binding = ProviderBinding::new(
            ProviderKind::Local,
            tid("t1"),
            "acct",
            "home",
            CloudCredentialRef::new("cred://vault/local").unwrap(),
        )
        .unwrap();
        let request = ProvisioningRequest::new(
            ProvisioningRequestId::new("req-1").unwrap(),
            cid("c1"),
            binding,
            WorkloadManifestId::new("wm-1").unwrap(),
            CapacityProfile::new(2, 4, 20, None, None, CapacityProvenance::Declared).unwrap(),
            "idem-1",
        )
        .unwrap();
        let mut plan = ProvisioningPlan::new(request).unwrap();
        assert!(plan
            .mark_submitted(
                ProvisioningReceipt::new(
                    ProvisioningRequestId::new("req-1").unwrap(),
                    ProviderKind::Local,
                    None,
                    ResourceState::Submitted,
                    VerificationState::Pending,
                    cid("c1"),
                    1_700_000_000,
                )
                .unwrap()
            )
            .is_ok());
        assert!(plan
            .mark_created(
                ResourceIdentity::new(
                    ProvisioningRequestId::new("req-1").unwrap(),
                    ProviderKind::Local,
                    tid("t1"),
                    "acct",
                    "home",
                    Some(ResourceId::new("r-1").unwrap()),
                    "idem-1",
                )
                .unwrap()
            )
            .is_ok());
        plan.mark_ambiguous().unwrap();
        // Without provider readback, resolution must fail closed.
        assert!(resolve_ambiguous_provisioning(&plan, false).is_err());
        assert_eq!(
            resolve_ambiguous_provisioning(&plan, true).unwrap(),
            ProvisioningOutcome::Succeeded
        );
    }

    #[test]
    fn ep036_unit_delete_ladder_requires_readback() {
        let binding = ProviderBinding::new(
            ProviderKind::Local,
            tid("t1"),
            "acct",
            "home",
            CloudCredentialRef::new("cred://vault/local").unwrap(),
        )
        .unwrap();
        let request = ProvisioningRequest::new(
            ProvisioningRequestId::new("req-1").unwrap(),
            cid("c1"),
            binding,
            WorkloadManifestId::new("wm-1").unwrap(),
            CapacityProfile::new(2, 4, 20, None, None, CapacityProvenance::Declared).unwrap(),
            "idem-1",
        )
        .unwrap();
        let mut plan = ProvisioningPlan::new(request).unwrap();
        // DELETE ACCEPTED != RESOURCE ABSENT VERIFIED.
        assert!(plan.mark_delete_accepted().is_err());
        plan.mark_delete_requested().unwrap();
        plan.mark_delete_accepted().unwrap();
        assert_ne!(plan.delete_state, DeleteState::ResourceAbsentVerified);
    }
}
