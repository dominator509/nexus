//! Guest and unknown principal bounds (EP-003 acceptance obligation 3).
//!
//! Unknown and guest users receive bounded local permissions. This module
//! encodes those bounds as a deterministic policy: guests operate at home
//! edge and client device locality only, never control plane, never with
//! privileged capability classes.

use nexus_domain::Locality;
use nexus_identity::Principal;

/// What a guest or unknown principal may access.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalAccess {
    /// Principal is a normal, tenant-scoped actor.
    Full,
    /// Principal is a guest or unknown actor with bounded local access.
    Bounded,
    /// Principal is not allowed to act at all.
    Denied,
}

/// Deterministic bounds for unknown and guest principals.
#[derive(Debug, Clone, Copy, Default)]
pub struct GuestPolicy;

impl GuestPolicy {
    /// Classify a principal's access.
    ///
    /// Rule: unknown and guest principals are Bounded. A principal whose
    /// type is HUMAN or DEVICE but carries no tenant relationship is
    /// treated as a guest. SYSTEM and SERVICE principals are never guests
    /// (they are provisioned actors); if they lack a tenant they are
    /// Denied rather than silently bounded.
    pub fn classify(&self, principal: &Principal) -> PrincipalAccess {
        match principal.principal_type() {
            nexus_domain::PrincipalType::Human | nexus_domain::PrincipalType::Device => {
                PrincipalAccess::Bounded
            }
            _ => PrincipalAccess::Denied,
        }
    }

    /// Whether a bounded principal may use the given locality.
    ///
    /// Bounded principals operate at HOME_EDGE and CLIENT_DEVICE only.
    /// CONTROL_PLANE, HARDWARE_NODE, and ANY are out of reach.
    pub fn allows_locality(&self, access: PrincipalAccess, locality: Locality) -> bool {
        if access != PrincipalAccess::Bounded {
            return access == PrincipalAccess::Full;
        }
        matches!(locality, Locality::HomeEdge | Locality::ClientDevice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{Locality, NexusId, PrincipalType, TenantId};

    const VALID_ID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6071";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072";

    fn principal(ptype: PrincipalType) -> Principal {
        Principal::new(
            NexusId::new(VALID_ID).unwrap(),
            ptype,
            TenantId::new(TENANT).unwrap(),
        )
    }

    #[test]
    fn ep003_unit_guest_policy_bounds_human_and_device() {
        let policy = GuestPolicy;
        assert_eq!(
            policy.classify(&principal(PrincipalType::Human)),
            PrincipalAccess::Bounded
        );
        assert_eq!(
            policy.classify(&principal(PrincipalType::Device)),
            PrincipalAccess::Bounded
        );
    }

    #[test]
    fn ep003_unit_guest_policy_denies_unprovisioned_actors() {
        let policy = GuestPolicy;
        assert_eq!(
            policy.classify(&principal(PrincipalType::Service)),
            PrincipalAccess::Denied
        );
        assert_eq!(
            policy.classify(&principal(PrincipalType::Agent)),
            PrincipalAccess::Denied
        );
        assert_eq!(
            policy.classify(&principal(PrincipalType::System)),
            PrincipalAccess::Denied
        );
    }

    #[test]
    fn ep003_unit_guest_policy_bounds_localities() {
        let policy = GuestPolicy;
        let bounded = PrincipalAccess::Bounded;
        assert!(policy.allows_locality(bounded, Locality::HomeEdge));
        assert!(policy.allows_locality(bounded, Locality::ClientDevice));
        assert!(!policy.allows_locality(bounded, Locality::ControlPlane));
        assert!(!policy.allows_locality(bounded, Locality::HardwareNode));
        assert!(!policy.allows_locality(bounded, Locality::Any));
    }
}
