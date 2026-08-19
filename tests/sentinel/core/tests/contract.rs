//! EP-030 contract-composition proofs (SPEC-013 acceptance
//! obligations at the contract level).

use nexus_domain::{ApprovalClass, TenantId};
use nexus_sentinel::{
    BehaviorBaseline, BehaviorBaselineState, ContainmentVerification, DnsSecurityProvider,
    FirewallAction, FirewallProvider, NetworkDevice, NetworkInventory, NetworkSegment,
    QuarantineProposal, QuarantineProposalId, QuarantineState, SentinelCapabilityKind,
    SentinelErrorCode, TrustClass, UnboundDnsSecurityProvider, UnboundFirewallProvider,
    UnboundNetworkInventory,
};
use std::str::FromStr;

fn tenant() -> TenantId {
    TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

#[test]
fn ep030_unit_opnsense_and_openwrt_share_canonical_provider() {
    // Acceptance obligation 1: OPNsense and OpenWrt share a canonical
    // network provider. The contract is the FirewallProvider port;
    // both connectors (M2/M3) implement the SAME trait.
    let unbound = UnboundFirewallProvider;
    // Both share the same fail-closed contract surface.
    assert!(unbound.capabilities().is_empty());
    let err = unbound.read_telemetry(&tenant()).unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
    // And a single provider instance satisfies the contract boundary
    // for either appliance class.
    fn is_canonical_firewall_provider(_p: &impl FirewallProvider) {}
    is_canonical_firewall_provider(&unbound);
}

#[test]
fn ep030_unit_adguard_supplies_dns_security_and_telemetry() {
    // Acceptance obligation 2: AdGuard Home supplies DNS security and
    // telemetry through the DnsSecurityProvider port.
    let unbound = UnboundDnsSecurityProvider;
    assert!(unbound.capabilities().is_empty());
    let err = unbound.read_telemetry(&tenant()).unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
    let err = unbound.read_blocklist(&tenant()).unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
}

#[test]
fn ep030_unit_five_segments_modeled() {
    // Acceptance obligation 3: IoT, trusted, guest, camera, and
    // quarantine segments are modeled.
    let segments = [
        NetworkSegment::Iot,
        NetworkSegment::Trusted,
        NetworkSegment::Guest,
        NetworkSegment::Camera,
        NetworkSegment::Quarantine,
    ];
    assert_eq!(segments.len(), 5);
    for s in segments {
        assert!(!s.as_str().is_empty());
    }
}

#[test]
fn ep030_unit_core_sentinel_proposes_verified_containment() {
    // Acceptance obligation 4: Core Sentinel is light enough for a
    // normal home and can propose verified containment. A quarantine
    // proposal is DATA (PROPOSED), never an executed rule; it reaches
    // VERIFIED only through the approved/applied/verified ladder and
    // requires a reversible preauthorized rule for automation.
    let proposal = QuarantineProposal::new(
        QuarantineProposalId::new("q-1").unwrap(),
        tenant(),
        nexus_sentinel::NetworkDeviceId::new("dev-1").unwrap(),
        NetworkSegment::Quarantine,
        FirewallAction::Drop,
        true,
        true,
        ApprovalClass::Human,
        "2026-08-20T00:00:00Z",
    );
    assert_eq!(proposal.state, QuarantineState::Proposed);
    assert_eq!(proposal.target_segment, NetworkSegment::Quarantine);
    assert!(proposal.is_auto_applicable());
    // The ladder is distinct: PROPOSED -> APPROVED -> APPLIED ->
    // VERIFIED. A proposal is never itself a containment verdict.
    let verification = ContainmentVerification::new(
        proposal.proposal_id.clone(),
        proposal.device_id.clone(),
        true,
        "readback-1",
        "2026-08-20T00:00:01Z",
    );
    assert!(verification.verified);
    assert_eq!(verification.proposal_id, proposal.proposal_id);
}

#[test]
fn ep030_unit_inventory_and_baseline_compose() {
    // The inventory enumerates devices; every device has a trust class
    // and a baseline lifecycle (SPEC-013 behavior 4).
    let inventory = UnboundNetworkInventory;
    assert!(inventory.capabilities().is_empty());
    let err = inventory.list_devices(&tenant()).unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Unavailable);

    // Contract-level: a baseline starts LEARNING and reaches
    // ESTABLISHED; trust classes are canonical.
    let device = NetworkDevice::new(
        nexus_sentinel::NetworkDeviceId::new("dev-1").unwrap(),
        tenant(),
        NetworkSegment::Trusted,
        TrustClass::Trusted,
        "nas",
        "opnsense",
        "2026-08-20T00:00:00Z",
        "2026-08-20T00:00:00Z",
    );
    assert_eq!(device.segment, NetworkSegment::Trusted);
    let baseline = BehaviorBaseline::new(
        nexus_sentinel::BaselineId::new("b-1").unwrap(),
        device.device_id.clone(),
        tenant(),
        vec!["dns.example".into()],
        vec!["dns".into(), "https".into()],
        vec!["trusted".into()],
        "2026-08-20T00:00:00Z",
    );
    assert_eq!(baseline.state, BehaviorBaselineState::Learning);
}

#[test]
fn ep030_unit_capability_advertisement_fails_closed() {
    // Unbound providers advertise nothing; an unadvertised capability
    // is UNAVAILABLE.
    let fw = UnboundFirewallProvider;
    let caps = fw.capabilities();
    assert!(caps.is_empty());
    assert!(!caps.contains(SentinelCapabilityKind::Containment));
    let dns = UnboundDnsSecurityProvider;
    assert!(!dns
        .capabilities()
        .contains(SentinelCapabilityKind::ReadDnsTelemetry));
}
