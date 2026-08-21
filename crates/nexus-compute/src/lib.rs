//! EP-036 Compute Fabric and Cloud Provisioning contracts (SPEC-016).
//!
//! Provider-neutral compute fabric behavior: ComputeNode, PlacementConstraint,
//! PlacementDecision, CloudProvider, ProvisioningPlan, BootstrapBundle, and
//! FleetEnrollment. State truthfulness is structural - REQUESTED != PLANNED !=
//! SUBMITTED != PROVISIONING != CREATED != REACHABLE != READY != VERIFIED !=
//! CERTIFIED, WORKLOAD ASSIGNED != STARTED != HEALTHY != VERIFIED, PROVIDER API
//! HEALTH != RESOURCE HEALTH != WORKLOAD HEALTH, DECLARED CAPACITY != OBSERVED
//! CAPACITY != CERTIFIED CAPACITY - and invalid states fail closed.
//!
//! Ambiguous provisioning outcomes (provider may have succeeded but the client
//! lost confirmation) are UNKNOWN/VERIFICATION_REQUIRED, never blindly retried.
//! Cloud credentials are opaque references, never raw contract data.
//!
//! Dependency direction: this crate depends only on nexus-domain and
//! serde/serde_json. No provider SDK, transport, or framework crate appears.

#![forbid(unsafe_code)]

pub mod error;
pub mod model;
pub mod placement;
pub mod port;
pub mod vocabulary;

pub use error::{ComputeError, ComputeErrorCode, ComputeResult};
pub use model::{
    is_valid_resource_transition, is_valid_workload_transition, resolve_ambiguous_provisioning,
    BootstrapBundle, BootstrapBundleId, CapacityProfile, CloudCredentialRef, CloudProvider,
    ComputeNode, ComputeNodeId, FleetEnrollment, FleetId, PlacementConstraint, PlacementDecision,
    ProviderBinding, ProvisioningPlan, ProvisioningReceipt, ProvisioningRequest,
    ProvisioningRequestId, ResourceId, ResourceIdentity, WorkloadAssignment, WorkloadManifest,
    WorkloadManifestId,
};
pub use nexus_domain::{CorrelationId, Locality, Privacy, TenantId};
pub use placement::placement_decision;
pub use port::{CloudProviderPort, ComputeFabricPort};
pub use vocabulary::{
    BillingState, CapacityProvenance, ComputeClass, DeleteState, FleetEnrollmentState,
    PlacementFailureClass, ProviderApiHealth, ProviderKind, ProvisioningOutcome, QuotaState,
    ResourceHealth, ResourceState, VerificationState, WorkloadState,
};
