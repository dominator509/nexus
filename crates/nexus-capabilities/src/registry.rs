//! Capability registry port (SPEC-003 acceptance: a new client can
//! discover capabilities).
//!
//! The registry is the provider-neutral advertisement surface:
//! capabilities register with stable descriptors, and discovery returns
//! only capabilities that are available for the requesting tenant.
//! Unavailable provider features are never advertised (SPEC-022
//! behavior 5; node contract acceptance obligation 4).

use nexus_domain::TenantId;

use crate::context::InvocationContext;
use crate::descriptor::CapabilityDescriptor;
use crate::error::CapabilityError;

/// Provider-neutral capability discovery port (SPEC-003, SPEC-022).
pub trait CapabilityRegistry {
    /// Register or update a capability descriptor. Idempotent for the
    /// same descriptor version.
    fn register(
        &self,
        descriptor: CapabilityDescriptor,
        context: InvocationContext,
    ) -> Result<(), CapabilityError>;

    /// Remove a capability from the registry.
    fn unregister(
        &self,
        capability_id: &str,
        context: InvocationContext,
    ) -> Result<(), CapabilityError>;

    /// Discover capabilities available to the tenant. Returns only
    /// advertised, available capabilities; uncertified or unavailable
    /// features are omitted.
    fn discover(
        &self,
        tenant_id: &TenantId,
        context: InvocationContext,
    ) -> Result<Vec<CapabilityDescriptor>, CapabilityError>;

    /// Resolve a single capability by key for the tenant.
    fn resolve(
        &self,
        capability_id: &str,
        tenant_id: &TenantId,
        context: InvocationContext,
    ) -> Result<CapabilityDescriptor, CapabilityError>;
}
