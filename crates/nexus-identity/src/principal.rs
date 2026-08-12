//! Principal: the authenticated actor at the boundary (SPEC-005).
//!
//! A `Principal` is a typed reference to an identity. It carries the
//! canonical `PrincipalType` (HUMAN, SERVICE, AGENT, DEVICE, SYSTEM), a
//! tenant boundary, and an opaque identifier. `Principal` is a reference,
//! never the identity record itself (SPEC-005: "Keycloak owns identities,
//! Nexus owns references").

use std::fmt;
use std::str::FromStr;

use nexus_domain::{IdError, NexusId, PrincipalType, TenantId};
use serde::{Deserialize, Serialize};

/// Error returned when a principal reference is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalError {
    /// The embedded identifier is not a canonical UUIDv7.
    InvalidId(IdError),
    /// The tenant boundary is missing or empty.
    MissingTenant,
}

impl fmt::Display for PrincipalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(e) => write!(f, "invalid principal id: {e}"),
            Self::MissingTenant => f.write_str("principal requires a tenant boundary"),
        }
    }
}

impl std::error::Error for PrincipalError {}

/// A typed reference to an authenticated actor.
///
/// The `principal_id` is the opaque Nexus-wide identifier of the identity
/// record; the `principal_type` determines which identity kind it refers to
/// (person, service, agent, device, or system).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    /// Opaque Nexus-wide identifier of the underlying identity record.
    pub principal_id: NexusId,
    /// Canonical actor class.
    pub principal_type: PrincipalType,
    /// Tenant boundary; principals are always tenant-scoped.
    pub tenant_id: TenantId,
}

impl Principal {
    /// Construct a principal reference.
    pub fn new(principal_id: NexusId, principal_type: PrincipalType, tenant_id: TenantId) -> Self {
        Self {
            principal_id,
            principal_type,
            tenant_id,
        }
    }

    /// The canonical actor class.
    pub fn principal_type(&self) -> PrincipalType {
        self.principal_type
    }

    /// The tenant boundary for this principal.
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    /// Opaque identifier of the underlying identity record.
    pub fn principal_id(&self) -> &NexusId {
        &self.principal_id
    }
}

impl TryFrom<&str> for Principal {
    type Error = PrincipalError;

    /// Parse `principal_type:tenant_id:principal_id` from canonical strings.
    ///
    /// Format: `<PRINCIPAL_TYPE>:<tenant-id>:<principal-id>`. This is a
    /// deterministic textual representation used at provider boundaries.
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let mut parts = s.splitn(3, ':');
        let ptype = parts.next().ok_or(PrincipalError::MissingTenant)?;
        let tenant = parts.next().ok_or(PrincipalError::MissingTenant)?;
        let pid = parts.next().ok_or(PrincipalError::MissingTenant)?;
        let principal_type =
            PrincipalType::from_str(ptype).map_err(|_| PrincipalError::MissingTenant)?;
        let tenant_id = TenantId::new(tenant).map_err(PrincipalError::InvalidId)?;
        let principal_id = NexusId::new(pid).map_err(PrincipalError::InvalidId)?;
        Ok(Self {
            principal_id,
            principal_type,
            tenant_id,
        })
    }
}

impl fmt::Display for Principal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}:{}",
            self.principal_type.as_str(),
            self.tenant_id,
            self.principal_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_ID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6071";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072";

    #[test]
    fn ep003_unit_principal_constructs_with_typed_fields() {
        let tenant = TenantId::new(TENANT).unwrap();
        let id = NexusId::new(VALID_ID).unwrap();
        let p = Principal::new(id.clone(), PrincipalType::Human, tenant.clone());
        assert_eq!(p.principal_type(), PrincipalType::Human);
        assert_eq!(p.tenant_id(), &tenant);
        assert_eq!(p.principal_id(), &id);
    }

    #[test]
    fn ep003_unit_principal_roundtrips_textual_form() {
        let tenant = TenantId::new(TENANT).unwrap();
        let id = NexusId::new(VALID_ID).unwrap();
        let p = Principal::new(id.clone(), PrincipalType::Agent, tenant.clone());
        let text = p.to_string();
        let back = Principal::try_from(text.as_str()).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn ep003_unit_principal_rejects_malformed_textual_form() {
        assert!(Principal::try_from("HUMAN:nope:also-nope").is_err());
        assert!(Principal::try_from("HUMAN").is_err());
        assert!(Principal::try_from("HUMAN:0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072").is_err());
        assert!(Principal::try_from("").is_err());
    }

    #[test]
    fn ep003_unit_principal_serde_roundtrip() {
        let tenant = TenantId::new(TENANT).unwrap();
        let id = NexusId::new(VALID_ID).unwrap();
        let p = Principal::new(id, PrincipalType::Device, tenant);
        let json = serde_json::to_string(&p).unwrap();
        let back: Principal = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn ep003_unit_principal_rejects_bad_serde() {
        let res: Result<Principal, _> = serde_json::from_str(
            r#"{"principal_id":"bogus","principal_type":"HUMAN","tenant_id":"0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072"}"#,
        );
        assert!(res.is_err());
    }
}
