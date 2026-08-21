//! EP-036 provider-neutral ports (SPEC-016).
//!
//! The contract layer defines what a provider must prove; later
//! milestones implement how. These ports are intentionally
//! provider-neutral: no concrete SDK, transport, or framework type
//! appears here.

use crate::error::ComputeResult;
use crate::model::{
    CloudProvider, ProvisioningPlan, ProvisioningReceipt, ProvisioningRequest, ResourceIdentity,
};

/// Port a cloud provider adapter must satisfy. Provider acceptance
/// establishes only a receipt at SUBMITTED; the plan advances through
/// resource readback and verification. Readback is exact-target bound to
/// the request identity, never "some VM exists".
pub trait CloudProviderPort {
    /// Submit a provisioning request. Returns a receipt that never
    /// overclaims: state is at most SUBMITTED unless the adapter has
    /// independently established more.
    fn submit(&self, request: &ProvisioningRequest) -> ComputeResult<ProvisioningReceipt>;

    /// Exact-target readback of the resource identified by identity.
    /// Returns the plan with the observed state advanced only as far as
    /// actually verified.
    fn readback(&self, identity: &ResourceIdentity) -> ComputeResult<ProvisioningPlan>;

    /// Request deletion. DELETE ACCEPTED != RESOURCE ABSENT VERIFIED;
    /// absence must be read back independently.
    fn delete(&self, identity: &ResourceIdentity) -> ComputeResult<ProvisioningPlan>;
}

/// Port the compute fabric exposes to callers: node registry, placement,
/// and provisioning orchestration. M1 encodes the contract; the durable
/// implementation and provider adapters are owned by later milestones.
pub trait ComputeFabricPort {
    /// Register a compute node in the fabric registry.
    fn register_node(&mut self, node: crate::model::ComputeNode) -> ComputeResult<()>;

    /// List currently registered nodes.
    fn nodes(&self) -> ComputeResult<Vec<crate::model::ComputeNode>>;

    /// List provider bindings currently known to the fabric.
    fn providers(&self) -> ComputeResult<Vec<CloudProvider>>;
}
