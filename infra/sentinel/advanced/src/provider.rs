//! EP-031 provider ports and services (node contract public
//! interfaces).
//!
//! Provider-neutral, versioned, and fail-closed: an unbound provider
//! returns Unavailable and never fabricates events, incidents,
//! triage, investigation, response, or verification. Provider
//! implementations live in connectors/suricata, connectors/zeek,
//! connectors/crowdsec, connectors/wazuh, connectors/osquery (M2+);
//! M1 owns the ports. Honeypots are optional high-signal sensors
//! isolated from real data (SPEC-013 behavior 7).

use crate::error::AdvancedSentinelError;
use crate::model::{
    HoneypotRecord, Incident, InvestigationCase, ResponsePlan, SecurityEvent, TriageCase,
    VerificationRecord,
};
use crate::vocabulary::{
    HoneypotId, HoneypotKind, IncidentCorrelationId, ResponseKind, ResponsePlanId, TriageCaseId,
    VerificationRecordId,
};
use nexus_domain::{ApprovalClass, TenantId};
use nexus_sentinel::SentinelCapabilityMap;

/// Network detection provider (SPEC-013 behavior 3: Suricata is the
/// Enhanced profile sensor; Zeek is the Advanced profile sensor).
/// Detection events are OBSERVED data with evidence references; an
/// unbound or failing provider never fabricates events.
pub trait NetworkDetectionProvider {
    /// The capabilities this provider actually advertises. Unbound
    /// and uncertified providers advertise nothing (fail closed).
    fn capabilities(&self) -> SentinelCapabilityMap;

    /// Read observed network detection events for the tenant.
    fn read_events(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<SecurityEvent>, AdvancedSentinelError>;
}

/// Endpoint telemetry provider (SPEC-013 behavior 3: Endpoint profile
/// adds Wazuh or osquery). Endpoint telemetry is OBSERVED data.
pub trait EndpointTelemetryProvider {
    fn capabilities(&self) -> SentinelCapabilityMap;

    /// Read observed endpoint security events for the tenant.
    fn read_telemetry(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<SecurityEvent>, AdvancedSentinelError>;
}

/// Threat intelligence provider (SPEC-013: CrowdSec is optional
/// reputation enforcement). A lookup returns observed reputation
/// evidence; unknown reputation is UNAVAILABLE, never fabricated.
pub trait ThreatIntelProvider {
    fn capabilities(&self) -> SentinelCapabilityMap;

    /// Look up reputation evidence for an observed indicator
    /// (provider-neutral indicator reference).
    fn lookup_reputation(
        &self,
        tenant_id: &TenantId,
        indicator: &str,
    ) -> Result<Option<SecurityEvent>, AdvancedSentinelError>;
}

/// Honeypot provider (SPEC-013 behavior 7: honeypots and honeytokens
/// are optional high-signal sensors isolated from real data). Trigger
/// records are OBSERVED data.
pub trait HoneypotProvider {
    fn capabilities(&self) -> SentinelCapabilityMap;

    /// List the tenant's honeypot records.
    fn list_honeypots(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<HoneypotRecord>, AdvancedSentinelError>;

    /// Arm a new honeypot record (state ARMED, no fabricated source).
    fn arm_honeypot(
        &self,
        tenant_id: &TenantId,
        honeypot_id: HoneypotId,
        kind: HoneypotKind,
    ) -> Result<HoneypotRecord, AdvancedSentinelError>;

    /// Disarm a previously armed honeypot (reversible).
    fn disarm_honeypot(
        &self,
        tenant_id: &TenantId,
        honeypot: &HoneypotRecord,
    ) -> Result<HoneypotRecord, AdvancedSentinelError>;
}

/// Security triage (SPEC-013: alerts correlate into incidents instead
/// of flooding users). Correlation is bounded and derived from
/// observed shared evidence; it never invents a root cause.
pub trait SecurityTriage {
    /// Triage observed events into a bounded incident.
    fn triage_events(
        &self,
        tenant_id: &TenantId,
        correlation_id: IncidentCorrelationId,
        events: &[SecurityEvent],
    ) -> Result<Incident, AdvancedSentinelError>;

    /// Produce a bounded priority case for an incident.
    fn prioritize(
        &self,
        tenant_id: &TenantId,
        case_id: TriageCaseId,
        incident: &Incident,
    ) -> Result<TriageCase, AdvancedSentinelError>;
}

/// Security investigator (SPEC-013: Sentinel detects and explains
/// controlled threats, preserves evidence). Investigation gathers
/// observed evidence; conclusions reference that evidence.
pub trait SecurityInvestigator {
    /// Open (or continue) an investigation case for an incident.
    fn investigate(
        &self,
        tenant_id: &TenantId,
        incident: &Incident,
    ) -> Result<InvestigationCase, AdvancedSentinelError>;
}

/// Response planner (SPEC-013 behavior 5/6). Automated containment is
/// limited to preauthorized high-confidence reversible rules;
/// destructive response (wipes, factory resets, broad lockouts,
/// credential rotation) requires human procedure and is never
/// auto-applicable.
pub trait ResponsePlanner {
    /// Plan a response for an incident. Fails closed unless the
    /// response kind is allowed under the approval class: destructive
    /// kinds require human procedure (ApprovalClass::Human or
    /// stronger) and are never preauthorized.
    fn plan_response(
        &self,
        tenant_id: &TenantId,
        plan_id: ResponsePlanId,
        incident: &Incident,
        kind: ResponseKind,
        approval_class: ApprovalClass,
    ) -> Result<ResponsePlan, AdvancedSentinelError>;
}

/// Security verifier (SPEC-013: returns the network to verified safe
/// state). Verification is only true when independent readback proves
/// the exact effect; it is never assumed.
pub trait SecurityVerifier {
    /// Verify a response plan by independent exact-target readback.
    fn verify_response(
        &self,
        tenant_id: &TenantId,
        verification_id: VerificationRecordId,
        plan: &ResponsePlan,
    ) -> Result<VerificationRecord, AdvancedSentinelError>;
}

/// Fail-closed unbound network detection provider. Every operation
/// returns Unavailable; it never fabricates events.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundNetworkDetectionProvider;

impl NetworkDetectionProvider for UnboundNetworkDetectionProvider {
    fn capabilities(&self) -> SentinelCapabilityMap {
        SentinelCapabilityMap::new()
    }

    fn read_events(
        &self,
        _tenant_id: &TenantId,
    ) -> Result<Vec<SecurityEvent>, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable(
            "no network detection provider bound",
        ))
    }
}

/// Fail-closed unbound endpoint telemetry provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundEndpointTelemetryProvider;

impl EndpointTelemetryProvider for UnboundEndpointTelemetryProvider {
    fn capabilities(&self) -> SentinelCapabilityMap {
        SentinelCapabilityMap::new()
    }

    fn read_telemetry(
        &self,
        _tenant_id: &TenantId,
    ) -> Result<Vec<SecurityEvent>, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable(
            "no endpoint telemetry provider bound",
        ))
    }
}

/// Fail-closed unbound threat intelligence provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundThreatIntelProvider;

impl ThreatIntelProvider for UnboundThreatIntelProvider {
    fn capabilities(&self) -> SentinelCapabilityMap {
        SentinelCapabilityMap::new()
    }

    fn lookup_reputation(
        &self,
        _tenant_id: &TenantId,
        _indicator: &str,
    ) -> Result<Option<SecurityEvent>, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable(
            "no threat intelligence provider bound",
        ))
    }
}

/// Fail-closed unbound honeypot provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundHoneypotProvider;

impl HoneypotProvider for UnboundHoneypotProvider {
    fn capabilities(&self) -> SentinelCapabilityMap {
        SentinelCapabilityMap::new()
    }

    fn list_honeypots(
        &self,
        _tenant_id: &TenantId,
    ) -> Result<Vec<HoneypotRecord>, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable(
            "no honeypot provider bound",
        ))
    }

    fn arm_honeypot(
        &self,
        _tenant_id: &TenantId,
        _honeypot_id: HoneypotId,
        _kind: HoneypotKind,
    ) -> Result<HoneypotRecord, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable(
            "no honeypot provider bound",
        ))
    }

    fn disarm_honeypot(
        &self,
        _tenant_id: &TenantId,
        _honeypot: &HoneypotRecord,
    ) -> Result<HoneypotRecord, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable(
            "no honeypot provider bound",
        ))
    }
}

/// Fail-closed unbound security triage. No correlation is invented.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundSecurityTriage;

impl SecurityTriage for UnboundSecurityTriage {
    fn triage_events(
        &self,
        _tenant_id: &TenantId,
        _correlation_id: IncidentCorrelationId,
        _events: &[SecurityEvent],
    ) -> Result<Incident, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable("no triage bound"))
    }

    fn prioritize(
        &self,
        _tenant_id: &TenantId,
        _case_id: TriageCaseId,
        _incident: &Incident,
    ) -> Result<TriageCase, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable("no triage bound"))
    }
}

/// Fail-closed unbound security investigator.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundSecurityInvestigator;

impl SecurityInvestigator for UnboundSecurityInvestigator {
    fn investigate(
        &self,
        _tenant_id: &TenantId,
        _incident: &Incident,
    ) -> Result<InvestigationCase, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable("no investigator bound"))
    }
}

/// Fail-closed unbound response planner. No response is planned
/// without a bound planner, and destructive response always requires
/// human procedure.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundResponsePlanner;

impl ResponsePlanner for UnboundResponsePlanner {
    fn plan_response(
        &self,
        _tenant_id: &TenantId,
        _plan_id: ResponsePlanId,
        _incident: &Incident,
        _kind: ResponseKind,
        _approval_class: ApprovalClass,
    ) -> Result<ResponsePlan, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable(
            "no response planner bound",
        ))
    }
}

/// Fail-closed unbound security verifier.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundSecurityVerifier;

impl SecurityVerifier for UnboundSecurityVerifier {
    fn verify_response(
        &self,
        _tenant_id: &TenantId,
        _verification_id: VerificationRecordId,
        _plan: &ResponsePlan,
    ) -> Result<VerificationRecord, AdvancedSentinelError> {
        Err(AdvancedSentinelError::unavailable(
            "no security verifier bound",
        ))
    }
}

// The advanced ports reuse the sentinel core capability map
// (fail-closed) and the core finding severity without redefining
// them; provider implementations live in connectors (M2+).

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CorrelationConfidence;
    use nexus_domain::{IncidentId, TenantId};
    use nexus_sentinel::FindingSeverity;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    #[test]
    fn ep031_unit_unbound_network_detection_fails_closed() {
        let p = UnboundNetworkDetectionProvider;
        assert!(p.capabilities().is_empty());
        let err = p.read_events(&tenant()).unwrap_err();
        assert_eq!(err.code, nexus_sentinel::SentinelErrorCode::Unavailable);
    }

    #[test]
    fn ep031_unit_unbound_endpoint_telemetry_fails_closed() {
        let p = UnboundEndpointTelemetryProvider;
        assert!(p.capabilities().is_empty());
        let err = p.read_telemetry(&tenant()).unwrap_err();
        assert_eq!(err.code, nexus_sentinel::SentinelErrorCode::Unavailable);
    }

    #[test]
    fn ep031_unit_unbound_threat_intel_fails_closed() {
        let p = UnboundThreatIntelProvider;
        assert!(p.capabilities().is_empty());
        let err = p.lookup_reputation(&tenant(), "1.2.3.4").unwrap_err();
        assert_eq!(err.code, nexus_sentinel::SentinelErrorCode::Unavailable);
    }

    #[test]
    fn ep031_unit_unbound_honeypot_fails_closed() {
        let p = UnboundHoneypotProvider;
        assert!(p.capabilities().is_empty());
        let err = p.list_honeypots(&tenant()).unwrap_err();
        assert_eq!(err.code, nexus_sentinel::SentinelErrorCode::Unavailable);
    }

    #[test]
    fn ep031_unit_unbound_triage_investigator_planner_verifier_fail_closed() {
        let t = UnboundSecurityTriage;
        assert!(t
            .triage_events(&tenant(), IncidentCorrelationId::new("c-1").unwrap(), &[])
            .is_err());
        let i = UnboundSecurityInvestigator;
        let incident = Incident::new(
            IncidentId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            tenant(),
            IncidentCorrelationId::new("c-1").unwrap(),
            FindingSeverity::Medium,
            CorrelationConfidence::Medium,
            "s",
            "2026-08-20T00:00:00Z",
            "2026-08-20T00:00:00Z",
        );
        assert!(i.investigate(&tenant(), &incident).is_err());
        let p = UnboundResponsePlanner;
        assert!(p
            .plan_response(
                &tenant(),
                ResponsePlanId::new("plan-1").unwrap(),
                &incident,
                ResponseKind::Quarantine,
                ApprovalClass::Human,
            )
            .is_err());
        let v = UnboundSecurityVerifier;
        let plan = ResponsePlan::new(
            ResponsePlanId::new("plan-1").unwrap(),
            incident.incident_id.clone(),
            tenant(),
            ResponseKind::Quarantine,
            ApprovalClass::Human,
            "2026-08-20T00:00:00Z",
        );
        assert!(v
            .verify_response(&tenant(), VerificationRecordId::new("v-1").unwrap(), &plan)
            .is_err());
    }
}
