//! Deterministic in-memory capability registry (EP-010 M2).
//!
//! The registry is the provider-neutral advertisement surface: it
//! stores descriptors keyed by capability id and tenant, enforces
//! idempotent registration, resolves by key, and only ever advertises
//! capabilities whose availability is `AVAILABLE`. Uncertified or
//! unavailable provider features are never returned by discovery
//! (SPEC-022 behavior 5; node contract acceptance obligation 4).
//!
//! The registry is deterministic: for the same sequence of operations
//! it always returns the same descriptors in the same (insertion)
//! order, which keeps discovery stable for clients and downstream
//! proofs.

use std::collections::BTreeMap;
use std::sync::Mutex;

use nexus_capabilities::context::InvocationContext;
use nexus_capabilities::descriptor::CapabilityDescriptor;
use nexus_capabilities::error::{CapabilityError, CapabilityErrorCode};
use nexus_capabilities::registry::CapabilityRegistry;
use nexus_domain::{Availability, TenantId};

/// One registry slot: a descriptor plus the tenant it was registered
/// for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    /// Tenant that owns the registration.
    pub tenant_id: TenantId,
    /// Advertised descriptor.
    pub descriptor: CapabilityDescriptor,
}

/// Deterministic in-memory capability registry.
///
/// Interior mutability (`Mutex`) lets the registry implement the
/// `&self` port methods while remaining shareable across dispatchers
/// and threads. All operations are serialized, so the registry is
/// still deterministic for a given sequence of calls.
#[derive(Debug)]
pub struct InMemoryCapabilityRegistry {
    /// Entries keyed by `(tenant_id, capability_id)`; BTreeMap gives
    /// deterministic iteration order.
    entries: Mutex<BTreeMap<(TenantId, String), RegistryEntry>>,
}

impl Default for InMemoryCapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for InMemoryCapabilityRegistry {
    fn clone(&self) -> Self {
        let entries = self.entries.lock().expect("registry lock poisoned").clone();
        Self {
            entries: Mutex::new(entries),
        }
    }
}

impl InMemoryCapabilityRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
        }
    }

    /// Number of registered entries (for tests and diagnostics).
    pub fn len(&self) -> usize {
        self.entries.lock().expect("registry lock poisoned").len()
    }

    /// True when no capabilities are registered.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl CapabilityRegistry for InMemoryCapabilityRegistry {
    fn register(
        &self,
        descriptor: CapabilityDescriptor,
        context: InvocationContext,
    ) -> Result<(), CapabilityError> {
        let tenant = context.tenant_id.clone();
        let key = (tenant.clone(), descriptor.id.clone());
        self.entries.lock().expect("registry lock poisoned").insert(
            key,
            RegistryEntry {
                tenant_id: tenant,
                descriptor,
            },
        );
        Ok(())
    }

    fn unregister(
        &self,
        capability_id: &str,
        context: InvocationContext,
    ) -> Result<(), CapabilityError> {
        let tenant = context.tenant_id.clone();
        let key = (tenant.clone(), capability_id.to_string());
        let removed = self
            .entries
            .lock()
            .expect("registry lock poisoned")
            .remove(&key);
        if removed.is_none() {
            return Err(CapabilityError::new(
                CapabilityErrorCode::NotFound,
                "capability not found",
                Some(context.correlation_id.to_string()),
                Some(context.external_actor_id),
                Some(tenant.to_string()),
                Some(capability_id.to_string()),
            ));
        }
        Ok(())
    }

    fn discover(
        &self,
        tenant_id: &TenantId,
        _context: InvocationContext,
    ) -> Result<Vec<CapabilityDescriptor>, CapabilityError> {
        Ok(self
            .entries
            .lock()
            .expect("registry lock poisoned")
            .iter()
            .filter(|((t, _), _)| t == tenant_id)
            .filter(|(_, e)| e.descriptor.availability == Availability::Available)
            .map(|(_, e)| e.descriptor.clone())
            .collect())
    }

    fn resolve(
        &self,
        capability_id: &str,
        tenant_id: &TenantId,
        context: InvocationContext,
    ) -> Result<CapabilityDescriptor, CapabilityError> {
        let key = (tenant_id.clone(), capability_id.to_string());
        match self
            .entries
            .lock()
            .expect("registry lock poisoned")
            .get(&key)
        {
            Some(entry) => Ok(entry.descriptor.clone()),
            None => Err(CapabilityError::new(
                CapabilityErrorCode::NotFound,
                "capability not found",
                Some(context.correlation_id.to_string()),
                Some(context.external_actor_id),
                Some(tenant_id.to_string()),
                Some(capability_id.to_string()),
            )),
        }
    }
}
