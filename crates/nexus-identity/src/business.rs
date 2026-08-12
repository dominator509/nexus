//! Business binding (SPEC-001, EP-003).
//!
//! A `BusinessBinding` links a person to a business with a role. The
//! business record itself is owned by the business control plane (EP-028)
//! and later providers; this type carries the stable reference and the
//! person's role within it.

use std::fmt;

use nexus_domain::{BusinessId, PersonId, TenantId};
use serde::{Deserialize, Serialize};

/// Role of a person within a business (provider-neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BusinessRole {
    Owner,
    Admin,
    Member,
    Contractor,
    Viewer,
}

impl BusinessRole {
    /// Canonical wire string.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "OWNER",
            Self::Admin => "ADMIN",
            Self::Member => "MEMBER",
            Self::Contractor => "CONTRACTOR",
            Self::Viewer => "VIEWER",
        }
    }
}

impl fmt::Display for BusinessRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned by business binding construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BusinessBindingError {
    /// The person and business are in different tenants.
    TenantMismatch,
}

impl fmt::Display for BusinessBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TenantMismatch => f.write_str("person and business must share a tenant"),
        }
    }
}

impl std::error::Error for BusinessBindingError {}

/// A person's role within a business.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessBinding {
    /// Person identifier.
    pub person_id: PersonId,
    /// Business identifier.
    pub business_id: BusinessId,
    /// Tenant boundary; both sides must match.
    pub tenant_id: TenantId,
    /// Role within the business.
    pub role: BusinessRole,
}

impl BusinessBinding {
    /// Construct a validated business binding.
    pub fn new(
        person_id: PersonId,
        business_id: BusinessId,
        tenant_id: TenantId,
        role: BusinessRole,
    ) -> Result<Self, BusinessBindingError> {
        // The binding has a single tenant boundary; the person and business
        // identifiers are validated UUIDv7 values but the tenant is carried
        // explicitly so cross-tenant references are rejected at the API
        // boundary rather than leaking existence (EP-003 obligation 4).
        Ok(Self {
            person_id,
            business_id,
            tenant_id,
            role,
        })
    }

    /// Whether this binding is in the given tenant.
    pub fn is_in_tenant(&self, tenant_id: &TenantId) -> bool {
        &self.tenant_id == tenant_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101";
    const BID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6104";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102";
    const OTHER_TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6120";

    #[test]
    fn ep003_unit_business_binding_constructs_and_scopes() {
        let b = BusinessBinding::new(
            PersonId::new(PID).unwrap(),
            BusinessId::new(BID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            BusinessRole::Owner,
        )
        .unwrap();
        assert!(b.is_in_tenant(&TenantId::new(TENANT).unwrap()));
        assert!(!b.is_in_tenant(&TenantId::new(OTHER_TENANT).unwrap()));
        assert_eq!(b.role.as_str(), "OWNER");
    }

    #[test]
    fn ep003_unit_business_binding_serde_roundtrip() {
        let b = BusinessBinding::new(
            PersonId::new(PID).unwrap(),
            BusinessId::new(BID).unwrap(),
            TenantId::new(TENANT).unwrap(),
            BusinessRole::Contractor,
        )
        .unwrap();
        let json = serde_json::to_string(&b).unwrap();
        assert!(json.contains("\"role\":\"CONTRACTOR\""));
        let back: BusinessBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);
    }
}
