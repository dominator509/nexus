//! Cross-tenant and cross-business read guard (EP-003 obligation 4).
//!
//! Reads outside the caller's tenant or business boundary fail with a
//! uniform `NotFound` - the same error whether the resource exists in
//! another tenant or does not exist at all. This prevents existence
//! disclosure through error differentiation.

use nexus_domain::{BusinessId, PersonId, TenantId};

use crate::error::PresenceError;

/// Guard for tenant- and business-scoped reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct TenantGuard;

impl TenantGuard {
    /// Validate that the requested resource's tenant is the caller's.
    ///
    /// On mismatch returns `PresenceError::NotFound` (no disclosure that
    /// the resource exists). The caller's tenant is never echoed.
    pub fn check_tenant(
        &self,
        caller_tenant: &TenantId,
        resource_tenant: &TenantId,
    ) -> Result<(), PresenceError> {
        if caller_tenant == resource_tenant {
            Ok(())
        } else {
            Err(PresenceError::NotFound)
        }
    }

    /// Validate that a person is visible to the caller's business scope.
    ///
    /// A person is visible when the caller's tenant matches. Business scope
    /// membership is a separate relationship check; a mismatch yields the
    /// uniform NotFound without saying why.
    pub fn check_person_visible(
        &self,
        caller_tenant: &TenantId,
        person_tenant: &TenantId,
        _person: &PersonId,
        _caller_business: Option<&BusinessId>,
    ) -> Result<(), PresenceError> {
        self.check_tenant(caller_tenant, person_tenant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT_A: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const TENANT_B: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6120";
    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";
    const BID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6104";

    #[test]
    fn ep003_unit_tenant_guard_allows_same_tenant() {
        let guard = TenantGuard;
        let t = TenantId::new(TENANT_A).unwrap();
        assert_eq!(guard.check_tenant(&t, &t), Ok(()));
    }

    #[test]
    fn ep003_unit_tenant_guard_hides_cross_tenant_existence() {
        let guard = TenantGuard;
        let a = TenantId::new(TENANT_A).unwrap();
        let b = TenantId::new(TENANT_B).unwrap();
        // Cross-tenant read: uniform NotFound, no disclosure.
        let err = guard.check_tenant(&a, &b).unwrap_err();
        assert_eq!(err, PresenceError::NotFound);
        assert_eq!(err.code(), "not_found");
        // The message carries no tenant identifiers.
        assert!(!err.to_string().contains(TENANT_A));
        assert!(!err.to_string().contains(TENANT_B));
    }

    #[test]
    fn ep003_unit_tenant_guard_person_visibility_is_uniform() {
        let guard = TenantGuard;
        let a = TenantId::new(TENANT_A).unwrap();
        let b = TenantId::new(TENANT_B).unwrap();
        let person = PersonId::new(PID).unwrap();
        let biz = BusinessId::new(BID).unwrap();
        // Same error whether the person exists in tenant B or not at all:
        // the guard only ever sees the tenant pair and returns NotFound.
        let cross = guard
            .check_person_visible(&a, &b, &person, Some(&biz))
            .unwrap_err();
        assert_eq!(cross, PresenceError::NotFound);
    }
}
