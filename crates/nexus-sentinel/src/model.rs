//! EP-030 provider-neutral sentinel value objects (SPEC-013).
//!
//! SPEC-013 canonical terms (Sentinel, DeviceFingerprint, Baseline,
//! SecurityEvent, Incident, Quarantine) are vocabulary locked. This
//! crate composes EP-030-owned objects; nexus-wide identifiers come
//! from nexus-domain and are never redefined. Free-form provider
//! payloads are normalized at the infrastructure boundary and never
//! become domain contracts.

use nexus_domain::{ApprovalClass, BusinessId, DeviceId, TenantId};
use serde::{Deserialize, Serialize};

use crate::vocabulary::{
    BaselineId, BehaviorBaselineState, DeviceFingerprintId, FindingKind, FindingSeverity,
    FindingState, FirewallAction, NetworkDeviceId, NetworkFindingId, NetworkSegment,
    QuarantineProposalId, QuarantineState, TrustClass,
};

/// A device observed on the network (SPEC-013 behavior 4: every device
/// has expected protocols, destinations, internal access, baseline,
/// owner, firmware, provider, and trust class).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkDevice {
    pub device_id: NetworkDeviceId,
    /// Nexus-wide device reference when known (reference, never a
    /// fabricated identity).
    pub nexus_device_id: Option<DeviceId>,
    pub tenant_id: TenantId,
    pub segment: NetworkSegment,
    pub trust_class: TrustClass,
    /// Provider-neutral device label.
    pub label: String,
    /// Provider-neutral vendor label when observed.
    pub vendor: Option<String>,
    /// Provider-neutral firmware reference when observed.
    pub firmware_ref: Option<String>,
    /// Provider-neutral owner reference when known (person/household).
    pub owner_ref: Option<String>,
    /// Provider-neutral provider label (e.g. "opnsense", "openwrt").
    pub provider: String,
    /// RFC3339 timestamp of first observation.
    pub first_seen_at: String,
    /// RFC3339 timestamp of last observation.
    pub last_seen_at: String,
}

impl NetworkDevice {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device_id: NetworkDeviceId,
        tenant_id: TenantId,
        segment: NetworkSegment,
        trust_class: TrustClass,
        label: impl Into<String>,
        provider: impl Into<String>,
        first_seen_at: impl Into<String>,
        last_seen_at: impl Into<String>,
    ) -> Self {
        Self {
            device_id,
            nexus_device_id: None,
            tenant_id,
            segment,
            trust_class,
            label: label.into(),
            vendor: None,
            firmware_ref: None,
            owner_ref: None,
            provider: provider.into(),
            first_seen_at: first_seen_at.into(),
            last_seen_at: last_seen_at.into(),
        }
    }

    pub fn with_nexus_device(mut self, nexus_device_id: DeviceId) -> Self {
        self.nexus_device_id = Some(nexus_device_id);
        self
    }

    pub fn with_vendor(mut self, vendor: impl Into<String>) -> Self {
        self.vendor = Some(vendor.into());
        self
    }

    pub fn with_firmware(mut self, firmware_ref: impl Into<String>) -> Self {
        self.firmware_ref = Some(firmware_ref.into());
        self
    }

    pub fn with_owner(mut self, owner_ref: impl Into<String>) -> Self {
        self.owner_ref = Some(owner_ref.into());
        self
    }
}

/// A device fingerprint (SPEC-013: DeviceFingerprint is a locked
/// canonical term). A fingerprint is OBSERVED data, never fabricated;
/// unknown fingerprints fail closed rather than guessing identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceFingerprint {
    pub fingerprint_id: DeviceFingerprintId,
    pub device_id: NetworkDeviceId,
    /// Provider-neutral hardware/vendor class when observed.
    pub vendor: Option<String>,
    /// Provider-neutral operating/system class when observed.
    pub system: Option<String>,
    /// Provider-neutral MAC reference when observed.
    pub mac_ref: Option<String>,
    /// Provider-neutral IP reference when observed.
    pub ip_ref: Option<String>,
    /// RFC3339 timestamp of observation.
    pub observed_at: String,
}

impl DeviceFingerprint {
    pub fn new(
        fingerprint_id: DeviceFingerprintId,
        device_id: NetworkDeviceId,
        observed_at: impl Into<String>,
    ) -> Self {
        Self {
            fingerprint_id,
            device_id,
            vendor: None,
            system: None,
            mac_ref: None,
            ip_ref: None,
            observed_at: observed_at.into(),
        }
    }

    pub fn with_vendor(mut self, vendor: impl Into<String>) -> Self {
        self.vendor = Some(vendor.into());
        self
    }

    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    pub fn with_mac(mut self, mac_ref: impl Into<String>) -> Self {
        self.mac_ref = Some(mac_ref.into());
        self
    }

    pub fn with_ip(mut self, ip_ref: impl Into<String>) -> Self {
        self.ip_ref = Some(ip_ref.into());
        self
    }
}

/// A device behavior baseline (SPEC-013: flow baselines; every device
/// has expected protocols, destinations, and internal access).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BehaviorBaseline {
    pub baseline_id: BaselineId,
    pub device_id: NetworkDeviceId,
    pub tenant_id: TenantId,
    pub state: BehaviorBaselineState,
    /// Expected destination references (hosts, CIDRs, FQDNs).
    pub expected_destinations: Vec<String>,
    /// Expected protocol references (provider-neutral labels).
    pub expected_protocols: Vec<String>,
    /// Expected internal-access references (segments/hosts).
    pub expected_internal_access: Vec<String>,
    /// RFC3339 timestamp of last update.
    pub updated_at: String,
}

impl BehaviorBaseline {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        baseline_id: BaselineId,
        device_id: NetworkDeviceId,
        tenant_id: TenantId,
        expected_destinations: Vec<String>,
        expected_protocols: Vec<String>,
        expected_internal_access: Vec<String>,
        updated_at: impl Into<String>,
    ) -> Self {
        Self {
            baseline_id,
            device_id,
            tenant_id,
            state: BehaviorBaselineState::Learning,
            expected_destinations,
            expected_protocols,
            expected_internal_access,
            updated_at: updated_at.into(),
        }
    }

    pub fn with_state(mut self, state: BehaviorBaselineState) -> Self {
        self.state = state;
        self
    }
}

/// A network finding (SPEC-013 evidence correlation). Findings are
/// derived from OBSERVED telemetry; a finding never fabricates
/// evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkFinding {
    pub finding_id: NetworkFindingId,
    pub tenant_id: TenantId,
    pub device_id: Option<NetworkDeviceId>,
    pub kind: FindingKind,
    pub severity: FindingSeverity,
    pub state: FindingState,
    /// Reference to the observed evidence (telemetry id, log ref).
    pub evidence_ref: String,
    /// Correlation reference to the originating event.
    pub correlation: Option<String>,
    /// RFC3339 timestamp of first observation.
    pub observed_at: String,
}

impl NetworkFinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        finding_id: NetworkFindingId,
        tenant_id: TenantId,
        kind: FindingKind,
        severity: FindingSeverity,
        evidence_ref: impl Into<String>,
        observed_at: impl Into<String>,
    ) -> Self {
        Self {
            finding_id,
            tenant_id,
            device_id: None,
            kind,
            severity,
            state: FindingState::Open,
            evidence_ref: evidence_ref.into(),
            correlation: None,
            observed_at: observed_at.into(),
        }
    }

    pub fn for_device(mut self, device_id: NetworkDeviceId) -> Self {
        self.device_id = Some(device_id);
        self
    }

    pub fn with_correlation(mut self, correlation: impl Into<String>) -> Self {
        self.correlation = Some(correlation.into());
        self
    }

    pub fn with_state(mut self, state: FindingState) -> Self {
        self.state = state;
        self
    }
}

/// A quarantine proposal (SPEC-013: automated containment is limited
/// to preauthorized high-confidence reversible rules and always
/// notifies the owner; quarantine is a proposal until approved,
/// applied, and verified). A proposal is NOT an executed rule; it
/// becomes containment only through the approved/verified ladder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuarantineProposal {
    pub proposal_id: QuarantineProposalId,
    pub tenant_id: TenantId,
    pub business_id: Option<BusinessId>,
    pub device_id: NetworkDeviceId,
    /// Target segment for containment.
    pub target_segment: NetworkSegment,
    /// Proposed firewall action (reversible rules only).
    pub action: FirewallAction,
    /// Provider-neutral rule reference once applied.
    pub rule_ref: Option<String>,
    pub state: QuarantineState,
    /// True when the rule is preauthorized high-confidence reversible
    /// (SPEC-013 behavior 5).
    pub preauthorized: bool,
    /// True when the rule is reversible (rollback exists).
    pub reversible: bool,
    /// Approval class required to apply (SPEC-013 behavior 5/6:
    /// destructive remediation requires human procedure).
    pub approval_class: ApprovalClass,
    /// Owner notification reference once notified.
    pub notified_owner: bool,
    /// Correlation reference to the originating finding.
    pub correlation: Option<String>,
    /// RFC3339 timestamp of proposal creation.
    pub proposed_at: String,
}

impl QuarantineProposal {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        proposal_id: QuarantineProposalId,
        tenant_id: TenantId,
        device_id: NetworkDeviceId,
        target_segment: NetworkSegment,
        action: FirewallAction,
        preauthorized: bool,
        reversible: bool,
        approval_class: ApprovalClass,
        proposed_at: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id,
            tenant_id,
            business_id: None,
            device_id,
            target_segment,
            action,
            rule_ref: None,
            state: QuarantineState::Proposed,
            preauthorized,
            reversible,
            approval_class,
            notified_owner: false,
            correlation: None,
            proposed_at: proposed_at.into(),
        }
    }

    pub fn with_business(mut self, business_id: BusinessId) -> Self {
        self.business_id = Some(business_id);
        self
    }

    pub fn with_correlation(mut self, correlation: impl Into<String>) -> Self {
        self.correlation = Some(correlation.into());
        self
    }

    pub fn with_rule_ref(mut self, rule_ref: impl Into<String>) -> Self {
        self.rule_ref = Some(rule_ref.into());
        self
    }

    /// SPEC-013 behavior 5: automated containment is limited to
    /// preauthorized high-confidence reversible rules. A proposal that
    /// is not both preauthorized and reversible cannot be applied by
    /// automation; it fails closed.
    pub fn is_auto_applicable(&self) -> bool {
        self.preauthorized && self.reversible
    }
}

/// Result of verifying a containment rule (exact-target verification:
/// the verification binds to the exact proposal/device; an unrelated
/// change never satisfies it). VERIFIED only after independent
/// readback observes the rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainmentVerification {
    pub proposal_id: QuarantineProposalId,
    pub device_id: NetworkDeviceId,
    pub verified: bool,
    /// Reference to the independent readback evidence.
    pub evidence_ref: String,
    /// RFC3339 timestamp of verification.
    pub verified_at: String,
}

impl ContainmentVerification {
    pub fn new(
        proposal_id: QuarantineProposalId,
        device_id: NetworkDeviceId,
        verified: bool,
        evidence_ref: impl Into<String>,
        verified_at: impl Into<String>,
    ) -> Self {
        Self {
            proposal_id,
            device_id,
            verified,
            evidence_ref: evidence_ref.into(),
            verified_at: verified_at.into(),
        }
    }
}

/// DNS security telemetry (SPEC-013: AdGuard Home supplies DNS
/// security and telemetry). Telemetry is OBSERVED data, never
/// fabricated; unknown values are normalized at the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsTelemetry {
    pub tenant_id: TenantId,
    /// Total queries observed in the window.
    pub total_queries: u64,
    /// Blocked queries observed in the window.
    pub blocked_queries: u64,
    /// RFC3339 window start.
    pub window_start: String,
    /// RFC3339 window end.
    pub window_end: String,
}

impl DnsTelemetry {
    pub fn new(
        tenant_id: TenantId,
        total_queries: u64,
        blocked_queries: u64,
        window_start: impl Into<String>,
        window_end: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id,
            total_queries,
            blocked_queries,
            window_start: window_start.into(),
            window_end: window_end.into(),
        }
    }

    /// Blocked-query ratio bounded to [0,1]. A window with zero
    /// queries has no anomaly signal.
    pub fn blocked_ratio(&self) -> f64 {
        if self.total_queries == 0 {
            0.0
        } else {
            self.blocked_queries as f64 / self.total_queries as f64
        }
    }
}

/// A DNS blocklist entry (AdGuard; provider-neutral reference).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsBlocklistEntry {
    pub tenant_id: TenantId,
    /// Provider-neutral domain reference.
    pub domain_ref: String,
    /// True when the entry is currently active.
    pub active: bool,
    /// RFC3339 timestamp of last update.
    pub updated_at: String,
}

impl DnsBlocklistEntry {
    pub fn new(
        tenant_id: TenantId,
        domain_ref: impl Into<String>,
        active: bool,
        updated_at: impl Into<String>,
    ) -> Self {
        Self {
            tenant_id,
            domain_ref: domain_ref.into(),
            active,
            updated_at: updated_at.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    #[test]
    fn ep030_unit_device_carries_trust_and_segment() {
        let device = NetworkDevice::new(
            NetworkDeviceId::new("dev-1").unwrap(),
            tenant(),
            NetworkSegment::Iot,
            TrustClass::Unknown,
            "thermostat",
            "opnsense",
            "2026-08-20T00:00:00Z",
            "2026-08-20T00:00:00Z",
        )
        .with_vendor("vendor-a");
        assert_eq!(device.segment, NetworkSegment::Iot);
        assert_eq!(device.trust_class, TrustClass::Unknown);
        assert_eq!(device.vendor.as_deref(), Some("vendor-a"));
        assert_eq!(device.nexus_device_id, None);
    }

    #[test]
    fn ep030_unit_fingerprint_is_observed_not_fabricated() {
        let fp = DeviceFingerprint::new(
            DeviceFingerprintId::new("fp-1").unwrap(),
            NetworkDeviceId::new("dev-1").unwrap(),
            "2026-08-20T00:00:00Z",
        )
        .with_vendor("vendor-a")
        .with_ip("192.0.2.10");
        assert_eq!(fp.vendor.as_deref(), Some("vendor-a"));
        assert_eq!(fp.ip_ref.as_deref(), Some("192.0.2.10"));
        assert_eq!(fp.system, None, "unobserved system never fabricated");
    }

    #[test]
    fn ep030_unit_baseline_starts_learning() {
        let baseline = BehaviorBaseline::new(
            BaselineId::new("b-1").unwrap(),
            NetworkDeviceId::new("dev-1").unwrap(),
            tenant(),
            vec!["dns.example".into()],
            vec!["dns".into(), "https".into()],
            vec!["trusted".into()],
            "2026-08-20T00:00:00Z",
        );
        assert_eq!(baseline.state, BehaviorBaselineState::Learning);
        let established = baseline.with_state(BehaviorBaselineState::Established);
        assert_eq!(established.state, BehaviorBaselineState::Established);
    }

    #[test]
    fn ep030_unit_finding_false_positive_state_locked() {
        let finding = NetworkFinding::new(
            NetworkFindingId::new("f-1").unwrap(),
            tenant(),
            FindingKind::ScanDetected,
            FindingSeverity::High,
            "evidence-1",
            "2026-08-20T00:00:00Z",
        )
        .for_device(NetworkDeviceId::new("dev-1").unwrap())
        .with_state(FindingState::FalsePositive);
        assert_eq!(finding.state, FindingState::FalsePositive);
        assert_eq!(finding.device_id.as_ref().unwrap().as_str(), "dev-1");
        assert_eq!(finding.kind, FindingKind::ScanDetected);
    }

    #[test]
    fn ep030_unit_quarantine_proposal_is_not_containment() {
        // PROPOSED != APPLIED: a proposal is data, not an executed
        // rule. Automated containment requires preauthorized +
        // reversible.
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
        assert_eq!(proposal.state, QuarantineState::Proposed);
        assert!(proposal.is_auto_applicable());

        let non_reversible = QuarantineProposal::new(
            QuarantineProposalId::new("q-2").unwrap(),
            tenant(),
            NetworkDeviceId::new("dev-2").unwrap(),
            NetworkSegment::Quarantine,
            FirewallAction::Drop,
            true,
            false,
            ApprovalClass::StrongHuman,
            "2026-08-20T00:00:00Z",
        );
        assert!(
            !non_reversible.is_auto_applicable(),
            "non-reversible rule never auto-applies"
        );
    }

    #[test]
    fn ep030_unit_dns_telemetry_ratio_bounded() {
        let t = DnsTelemetry::new(
            tenant(),
            100,
            5,
            "2026-08-20T00:00:00Z",
            "2026-08-20T01:00:00Z",
        );
        assert_eq!(t.blocked_ratio(), 0.05);
        let empty = DnsTelemetry::new(
            tenant(),
            0,
            0,
            "2026-08-20T00:00:00Z",
            "2026-08-20T01:00:00Z",
        );
        assert_eq!(empty.blocked_ratio(), 0.0);
    }

    #[test]
    fn ep030_unit_models_roundtrip_serde() {
        let device = NetworkDevice::new(
            NetworkDeviceId::new("dev-1").unwrap(),
            tenant(),
            NetworkSegment::Trusted,
            TrustClass::Trusted,
            "nas",
            "opnsense",
            "2026-08-20T00:00:00Z",
            "2026-08-20T00:00:00Z",
        );
        let json = serde_json::to_string(&device).unwrap();
        let back: NetworkDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(back, device);
    }

    #[test]
    fn ep030_unit_verification_binds_exact_proposal() {
        let v = ContainmentVerification::new(
            QuarantineProposalId::new("q-1").unwrap(),
            NetworkDeviceId::new("dev-1").unwrap(),
            true,
            "readback-1",
            "2026-08-20T00:00:00Z",
        );
        assert!(v.verified);
        assert_eq!(v.proposal_id.as_str(), "q-1");
        assert_eq!(v.device_id.as_str(), "dev-1");
    }
}
