//! Tenant binding (directive F).
//!
//! The sidecar is provisioned with exactly one bound tenant (the
//! strongest tenant-binding mechanism that exists in EP-011 today:
//! per-instance provisioning). A request whose envelope tenant differs
//! from the bound tenant is denied before any provider invocation.
//!
//! EP-008 authenticated outer-context composition is NOT composed in
//! EP-011; that authority is recorded as NOT ASSERTED, never
//! fabricated here.

use crate::error::{SidecarError, SidecarErrorKind};

/// Immutable tenant binding for one sidecar instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TenantBinding {
    tenant_id: String,
}

impl TenantBinding {
    /// Construct a tenant binding; rejects empty tenant ids.
    pub fn new(tenant_id: impl Into<String>) -> Result<Self, SidecarError> {
        let tenant_id = tenant_id.into();
        if tenant_id.trim().is_empty() {
            return Err(SidecarError::validation(
                "bound tenant must not be empty",
                None,
            ));
        }
        Ok(Self { tenant_id })
    }

    /// The bound tenant id.
    pub fn tenant(&self) -> &str {
        &self.tenant_id
    }

    /// Enforce the binding (directive F): a request for any other
    /// tenant is denied before provider invocation.
    pub fn enforce(
        &self,
        envelope_tenant: &str,
        correlation_id: Option<&str>,
    ) -> Result<(), SidecarError> {
        if envelope_tenant == self.tenant_id {
            Ok(())
        } else {
            Err(SidecarError::new(
                SidecarErrorKind::Validation,
                "tenant mismatch: request tenant does not match the sidecar's bound tenant",
                correlation_id.map(str::to_string),
                Some(envelope_tenant.to_string()),
                None,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT_A: &str = "018f0f6f-9c1e-7b6e-8000-000000000003";
    const TENANT_B: &str = "018f0f6f-9c1e-7b6e-8000-000000000099";

    #[test]
    fn ep011_unit_sidecar_tenant_binding_accepts_bound_tenant() {
        let binding = TenantBinding::new(TENANT_A).unwrap();
        assert!(binding.enforce(TENANT_A, None).is_ok());
    }

    #[test]
    fn ep011_unit_sidecar_tenant_binding_denies_other_tenant() {
        let binding = TenantBinding::new(TENANT_A).unwrap();
        let err = binding.enforce(TENANT_B, None).unwrap_err();
        assert_eq!(err.kind, SidecarErrorKind::Validation);
        assert!(err.message.contains("tenant mismatch"));
    }

    #[test]
    fn ep011_unit_sidecar_tenant_binding_rejects_empty() {
        assert!(TenantBinding::new("").is_err());
    }
}
