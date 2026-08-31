//! EP-030 provider ports (node contract public interfaces).
//!
//! Provider-neutral, versioned, and fail-closed: an unbound provider
//! returns Unavailable and never fabricates devices, baselines,
//! findings, telemetry, or containment. Provider implementations live
//! in connectors/opnsense, connectors/openwrt, and
//! connectors/adguard-home (M2+); M1 owns the ports. OPNsense and
//! OpenWrt share the canonical FirewallProvider contract (acceptance
//! obligation 1); AdGuard Home supplies DNS security and telemetry
//! through DnsSecurityProvider (acceptance obligation 2).

use nexus_domain::{BusinessId, TenantId};

use crate::capability::SentinelCapabilityMap;
use crate::error::SentinelError;
use crate::model::{
    BehaviorBaseline, ContainmentVerification, DeviceFingerprint, DnsBlocklistEntry, DnsTelemetry,
    NetworkDevice, NetworkFinding, QuarantineProposal,
};

/// Canonical firewall provider (SPEC-013: OPNsense is the primary
/// serious firewall; OpenWrt is supported for embedded and consumer
/// installations; they SHARE this canonical provider contract).
///
/// Automated containment is limited to preauthorized high-confidence
/// reversible rules (SPEC-013 behavior 5): `apply_containment` fails
/// closed for a proposal that is not auto-applicable. Destructive
/// remediation, credential rotation, wipes, factory resets, and broad
/// lockouts require human procedure (SPEC-013 behavior 6) and are not
/// offered here.
pub trait FirewallProvider {
    /// The capabilities this provider actually advertises. Unbound
    /// and uncertified providers advertise nothing (fail closed).
    fn capabilities(&self) -> SentinelCapabilityMap;

    /// Read observed firewall telemetry for the tenant.
    fn read_telemetry(&self, tenant_id: &TenantId) -> Result<Vec<NetworkFinding>, SentinelError>;

    /// Propose verified containment. The proposal is DATA, not an
    /// executed rule; it becomes containment only through the
    /// approved/applied/verified ladder.
    ///
    /// AUD-026: `observed_source` is the OBSERVED network identity
    /// (the device fingerprint's ip_ref) that the containment rule
    /// must bind to - NEVER the device display label. A proposal
    /// without an observed source fails closed.
    fn propose_containment(
        &self,
        tenant_id: &TenantId,
        business_id: Option<&BusinessId>,
        device: &NetworkDevice,
        observed_source: Option<&str>,
    ) -> Result<QuarantineProposal, SentinelError>;

    /// Apply a quarantine proposal. Fails closed unless the proposal
    /// is preauthorized high-confidence reversible AND approved; the
    /// applied rule is recorded with a rule reference.
    fn apply_containment(
        &self,
        proposal: &QuarantineProposal,
    ) -> Result<QuarantineProposal, SentinelError>;

    /// Verify containment by independent readback (exact-target: the
    /// verification binds to the exact proposal/device).
    fn verify_containment(
        &self,
        proposal: &QuarantineProposal,
    ) -> Result<ContainmentVerification, SentinelError>;

    /// Revoke a previously applied containment rule (reversible).
    fn revoke_containment(
        &self,
        proposal: &QuarantineProposal,
    ) -> Result<QuarantineProposal, SentinelError>;
}

/// DNS security provider (SPEC-013: AdGuard Home supplies DNS
/// security and telemetry; GPL-3.0 isolated-sidecar integration
/// mode per COMPONENT_REGISTRY).
pub trait DnsSecurityProvider {
    fn capabilities(&self) -> SentinelCapabilityMap;

    /// Read DNS security telemetry for the tenant window.
    fn read_telemetry(&self, tenant_id: &TenantId) -> Result<DnsTelemetry, SentinelError>;

    /// Read the current DNS blocklist state.
    fn read_blocklist(&self, tenant_id: &TenantId)
        -> Result<Vec<DnsBlocklistEntry>, SentinelError>;
}

/// Network inventory (SPEC-013: unknown-device inventory; every device
/// has a baseline, owner, firmware, provider, and trust class).
pub trait NetworkInventory {
    fn capabilities(&self) -> SentinelCapabilityMap;

    /// Enumerate the observed network inventory.
    fn list_devices(&self, tenant_id: &TenantId) -> Result<Vec<NetworkDevice>, SentinelError>;

    /// Fingerprint an observed device. A fingerprint is OBSERVED data,
    /// never fabricated; unknown fingerprints fail closed.
    fn fingerprint(&self, device: &NetworkDevice) -> Result<DeviceFingerprint, SentinelError>;

    /// Read (or begin learning) a device's behavior baseline.
    fn baseline(&self, device: &NetworkDevice) -> Result<BehaviorBaseline, SentinelError>;
}

/// Fail-closed unbound firewall provider. Every operation returns
/// Unavailable; it never fabricates telemetry, proposals, or
/// containment.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundFirewallProvider;

impl FirewallProvider for UnboundFirewallProvider {
    fn capabilities(&self) -> SentinelCapabilityMap {
        SentinelCapabilityMap::new()
    }

    fn read_telemetry(&self, _tenant_id: &TenantId) -> Result<Vec<NetworkFinding>, SentinelError> {
        Err(SentinelError::unavailable("no firewall provider bound"))
    }

    fn propose_containment(
        &self,
        _tenant_id: &TenantId,
        _business_id: Option<&BusinessId>,
        _device: &NetworkDevice,
        _observed_source: Option<&str>,
    ) -> Result<QuarantineProposal, SentinelError> {
        Err(SentinelError::unavailable("no firewall provider bound"))
    }

    fn apply_containment(
        &self,
        _proposal: &QuarantineProposal,
    ) -> Result<QuarantineProposal, SentinelError> {
        Err(SentinelError::unavailable("no firewall provider bound"))
    }

    fn verify_containment(
        &self,
        _proposal: &QuarantineProposal,
    ) -> Result<ContainmentVerification, SentinelError> {
        Err(SentinelError::unavailable("no firewall provider bound"))
    }

    fn revoke_containment(
        &self,
        _proposal: &QuarantineProposal,
    ) -> Result<QuarantineProposal, SentinelError> {
        Err(SentinelError::unavailable("no firewall provider bound"))
    }
}

/// Fail-closed unbound DNS security provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundDnsSecurityProvider;

impl DnsSecurityProvider for UnboundDnsSecurityProvider {
    fn capabilities(&self) -> SentinelCapabilityMap {
        SentinelCapabilityMap::new()
    }

    fn read_telemetry(&self, _tenant_id: &TenantId) -> Result<DnsTelemetry, SentinelError> {
        Err(SentinelError::unavailable("no DNS security provider bound"))
    }

    fn read_blocklist(
        &self,
        _tenant_id: &TenantId,
    ) -> Result<Vec<DnsBlocklistEntry>, SentinelError> {
        Err(SentinelError::unavailable("no DNS security provider bound"))
    }
}

/// Fail-closed unbound network inventory.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundNetworkInventory;

impl NetworkInventory for UnboundNetworkInventory {
    fn capabilities(&self) -> SentinelCapabilityMap {
        SentinelCapabilityMap::new()
    }

    fn list_devices(&self, _tenant_id: &TenantId) -> Result<Vec<NetworkDevice>, SentinelError> {
        Err(SentinelError::unavailable("no network inventory bound"))
    }

    fn fingerprint(&self, _device: &NetworkDevice) -> Result<DeviceFingerprint, SentinelError> {
        Err(SentinelError::unavailable("no network inventory bound"))
    }

    fn baseline(&self, _device: &NetworkDevice) -> Result<BehaviorBaseline, SentinelError> {
        Err(SentinelError::unavailable("no network inventory bound"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SentinelErrorCode;
    use crate::vocabulary::{
        FirewallAction, NetworkDeviceId, NetworkSegment, QuarantineProposalId,
    };
    use nexus_domain::ApprovalClass;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    fn device() -> NetworkDevice {
        NetworkDevice::new(
            NetworkDeviceId::new("dev-1").unwrap(),
            tenant(),
            NetworkSegment::Iot,
            crate::vocabulary::TrustClass::Unknown,
            "thermostat",
            "opnsense",
            "2026-08-20T00:00:00Z",
            "2026-08-20T00:00:00Z",
        )
    }

    #[test]
    fn ep030_unit_unbound_firewall_fails_closed() {
        let provider = UnboundFirewallProvider;
        assert!(provider.capabilities().is_empty());
        let err = provider.read_telemetry(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        let err = provider
            .propose_containment(&tenant(), None, &device(), None)
            .unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        let proposal = QuarantineProposal::new(
            QuarantineProposalId::new("q-1").unwrap(),
            tenant(),
            NetworkDeviceId::new("dev-1").unwrap(),
            NetworkSegment::Quarantine,
            FirewallAction::Drop,
            true,
            true,
            ApprovalClass::Human,
            "2026-08-20T00:00:00Z",
        );
        let err = provider.apply_containment(&proposal).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        let err = provider.verify_containment(&proposal).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        let err = provider.revoke_containment(&proposal).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
    }

    #[test]
    fn ep030_unit_unbound_dns_fails_closed() {
        let provider = UnboundDnsSecurityProvider;
        assert!(provider.capabilities().is_empty());
        let err = provider.read_telemetry(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        let err = provider.read_blocklist(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
    }

    #[test]
    fn ep030_unit_unbound_inventory_fails_closed() {
        let provider = UnboundNetworkInventory;
        assert!(provider.capabilities().is_empty());
        let err = provider.list_devices(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        let err = provider.fingerprint(&device()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        let err = provider.baseline(&device()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
    }
}
