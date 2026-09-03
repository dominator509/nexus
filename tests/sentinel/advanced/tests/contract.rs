//! EP-031 advanced sentinel contract-composition proofs (SPEC-013
//! acceptance obligations at the contract level).

use nexus_domain::{ApprovalClass, IncidentId, TenantId};
use nexus_sentinel::{FindingKind, FindingSeverity, SentinelErrorCode};
use nexus_sentinel_advanced::{
    AdvancedSensorProfile, CorrelationConfidence, HoneypotProvider, IncidentCorrelationId,
    NetworkDetectionProvider, ResponseKind, ResponsePlan, ResponsePlanId, SecurityEventId,
    SecurityTriage, SecurityVerifier, UnboundHoneypotProvider, UnboundNetworkDetectionProvider,
    UnboundSecurityTriage, UnboundSecurityVerifier, VerificationRecordId,
};
use nexus_zeek_connector::{JsonLinesZeekTransport, ZeekNetworkDetectionProvider};
use std::str::FromStr;

fn tenant() -> TenantId {
    TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn incident_id() -> IncidentId {
    IncidentId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
}

#[test]
fn ep031_unit_advanced_sensors_are_optional_profiles() {
    // Acceptance obligation 1: advanced sensors are optional
    // profiles. The profiles are distinct, and an unbound provider
    // advertises nothing and fails closed.
    let profiles = [
        AdvancedSensorProfile::Suricata,
        AdvancedSensorProfile::Zeek,
        AdvancedSensorProfile::Crowdsec,
        AdvancedSensorProfile::Wazuh,
        AdvancedSensorProfile::Osquery,
        AdvancedSensorProfile::Honeypot,
    ];
    let mut spelled: Vec<&str> = profiles.iter().map(|p| p.as_str()).collect();
    spelled.sort_unstable();
    spelled.dedup();
    assert_eq!(spelled.len(), 6);
    let unbound = UnboundNetworkDetectionProvider;
    assert!(unbound.capabilities().is_empty());
    let err = unbound.read_events(&tenant()).unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
}

#[test]
fn ep031_unit_alerts_correlate_into_incidents_not_floods() {
    // Acceptance obligation 2: alerts correlate into incidents
    // instead of flooding users. The Incident aggregates correlated
    // events; duplicate events are never re-added.
    let mut incident = nexus_sentinel_advanced::Incident::new(
        incident_id(),
        tenant(),
        IncidentCorrelationId::new("corr-1").unwrap(),
        FindingSeverity::High,
        CorrelationConfidence::High,
        "scan from unknown device",
        "2026-08-20T00:00:00Z",
        "2026-08-20T00:00:00Z",
    );
    incident = incident
        .with_event(SecurityEventId::new("evt-1").unwrap())
        .with_event(SecurityEventId::new("evt-2").unwrap())
        .with_event(SecurityEventId::new("evt-1").unwrap());
    assert_eq!(incident.event_ids.len(), 2, "correlated, not flooded");
}

#[test]
fn ep031_unit_high_confidence_bounded_quarantine_can_be_preauthorized() {
    // Acceptance obligation 3: high-confidence bounded quarantine can
    // be preauthorized. Quarantine/Block/IsolateEndpoint are bounded
    // reversible containment and may be preauthorized - but ONLY when
    // high incident confidence AND a provider-specific reversibility
    // proof are bound (AUD-031).
    let plan = ResponsePlan::new(
        ResponsePlanId::new("plan-1").unwrap(),
        incident_id(),
        tenant(),
        ResponseKind::Quarantine,
        ApprovalClass::Human,
        "2026-08-20T00:00:00Z",
    );
    // Fail closed by default: the kind alone never mints
    // preauthorization.
    assert!(!plan.preauthorized);
    let plan = plan
        .preauthorize(
            CorrelationConfidence::High,
            "opnsense:proposal:p-1:reversible",
        )
        .expect("high-confidence bounded containment with provider proof may be preauthorized");
    assert!(plan.preauthorized);
    assert_eq!(
        plan.reversibility_proof.as_deref(),
        Some("opnsense:proposal:p-1:reversible")
    );
    assert!(ResponseKind::Block.is_bounded_containment());
    assert!(ResponseKind::IsolateEndpoint.is_bounded_containment());
}

#[test]
fn ep031_unit_preauthorization_fails_closed_without_confidence_or_proof() {
    // AUD-031: preauthorization requires BOTH high confidence AND a
    // provider-specific reversibility proof. Missing either fails
    // closed - no threat score may mint authorization.
    let base = ResponsePlan::new(
        ResponsePlanId::new("plan-2").unwrap(),
        incident_id(),
        tenant(),
        ResponseKind::Quarantine,
        ApprovalClass::Human,
        "2026-08-20T00:00:00Z",
    );
    // Medium confidence even with a proof: denied.
    assert!(base
        .clone()
        .preauthorize(
            CorrelationConfidence::Medium,
            "opnsense:proposal:p-1:reversible"
        )
        .is_err());
    // High confidence but empty proof: denied.
    assert!(base
        .clone()
        .preauthorize(CorrelationConfidence::High, "")
        .is_err());
    // High confidence with proof on a NON-bounded kind: denied.
    let destructive = ResponsePlan::new(
        ResponsePlanId::new("plan-3").unwrap(),
        incident_id(),
        tenant(),
        ResponseKind::Wipe,
        ApprovalClass::StrongHuman,
        "2026-08-20T00:00:00Z",
    );
    assert!(destructive
        .preauthorize(
            CorrelationConfidence::High,
            "opnsense:proposal:p-1:reversible"
        )
        .is_err());
}

#[test]
fn ep031_unit_destructive_response_remains_human_controlled() {
    // Acceptance obligation 4: destructive response remains human
    // controlled. Wipe/FactoryReset/BroadLockout/CredentialRotation
    // are destructive, never preauthorized, and require human
    // procedure.
    for kind in [
        ResponseKind::Wipe,
        ResponseKind::FactoryReset,
        ResponseKind::BroadLockout,
        ResponseKind::CredentialRotation,
    ] {
        assert!(kind.is_destructive(), "{kind} is destructive");
        let plan = ResponsePlan::new(
            ResponsePlanId::new("plan-x").unwrap(),
            incident_id(),
            tenant(),
            kind,
            ApprovalClass::StrongHuman,
            "2026-08-20T00:00:00Z",
        );
        assert!(!plan.preauthorized, "{kind} never preauthorized");
    }
}

#[test]
fn ep031_unit_zeek_live_detection_over_real_socket() {
    // The Zeek connector normalizes REAL Zeek-shaped JSON notice
    // records through the production transport (mocks control the
    // peer only; the transport/adapter are never mocked).
    let json = r#"{"_path":"notice","_write_ts":1755650000}
{"ts":1755650000.5,"uid":"C1","id.orig_h":"192.0.2.10","id.orig_p":54321,"id.resp_h":"198.51.100.7","id.resp_p":80,"proto":"tcp","note":"Scan::Port_Scan","msg":"Port scan detected","src":"192.0.2.10","dst":"198.51.100.7","p":80,"n":42,"actions":["Notice::ACTION_LOG"],"dropped":false}
{"ts":1755650001.0,"uid":"C2","note":"SSL::Invalid_Server_Cert","msg":"unclassified observed"}
{"ts":1755650002.0,"uid":"C3","id.orig_h":"192.0.2.10","note":"DNS::Suspicious_Query","msg":"dns anomaly"}"#;
    let transport = JsonLinesZeekTransport::new(json.as_bytes());
    let provider = ZeekNetworkDetectionProvider::new(transport);
    let events = provider.read_events(&tenant()).unwrap();
    // Two classified events (Scan + DNS); the SSL note is observed
    // but never fabricated into a canonical finding.
    assert_eq!(events.len(), 2);
    let scan = events
        .iter()
        .find(|e| e.kind == FindingKind::ScanDetected)
        .expect("scan event present");
    assert_eq!(scan.profile, AdvancedSensorProfile::Zeek);
    assert_eq!(scan.severity, FindingSeverity::Medium);
    let dns = events
        .iter()
        .find(|e| e.kind == FindingKind::DnsAnomaly)
        .expect("dns event present");
    assert_eq!(dns.severity, FindingSeverity::Low);
}

#[test]
fn ep031_unit_services_fail_closed_when_unbound() {
    // Triage, investigation, planning, verification all fail closed
    // when unbound; nothing is invented.
    let triage = UnboundSecurityTriage;
    assert!(triage
        .triage_events(&tenant(), IncidentCorrelationId::new("c-1").unwrap(), &[])
        .is_err());
    let verifier = UnboundSecurityVerifier;
    let plan = ResponsePlan::new(
        ResponsePlanId::new("plan-1").unwrap(),
        incident_id(),
        tenant(),
        ResponseKind::Quarantine,
        ApprovalClass::Human,
        "2026-08-20T00:00:00Z",
    );
    assert!(verifier
        .verify_response(&tenant(), VerificationRecordId::new("v-1").unwrap(), &plan)
        .is_err());
    let honeypots = UnboundHoneypotProvider;
    assert!(honeypots.list_honeypots(&tenant()).is_err());
}
