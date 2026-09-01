//! EP-031 provider-neutral advanced sentinel value objects (SPEC-013).
//!
//! SPEC-013 canonical terms (SecurityEvent, Incident, Honeypot) are
//! vocabulary locked. This crate composes EP-031-owned objects;
//! nexus-wide identifiers (TenantId, DeviceId, IncidentId,
//! ApprovalId) and ApprovalClass come from nexus-domain and are never
//! redefined; sentinel core classes (FindingKind, FindingSeverity,
//! NetworkDeviceId) come from nexus-sentinel. Free-form provider
//! payloads are normalized at the infrastructure boundary and never
//! become domain contracts.

use nexus_domain::{ApprovalClass, IncidentId, TenantId};
use nexus_sentinel::{FindingKind, FindingSeverity, NetworkDeviceId};
use serde::{Deserialize, Serialize};

use crate::error::AdvancedSentinelError;

use crate::vocabulary::{
    AdvancedSensorProfile, AlertState, CorrelationConfidence, HoneypotId, HoneypotKind,
    HoneypotState, IncidentCorrelationId, IncidentState, InvestigationCaseId, InvestigationState,
    ResponseKind, ResponsePlanId, ResponsePlanState, SecurityEventId, TriageCaseId, TriagePriority,
    VerificationRecordId, VerificationState,
};

/// A security event observed by an advanced sensor (SPEC-013: Suricata,
/// Zeek, CrowdSec, Wazuh, osquery, honeypots). An event is OBSERVED
/// data with an evidence reference; it is never fabricated and never
/// promoted to a domain contract beyond its normalized fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub event_id: SecurityEventId,
    pub tenant_id: TenantId,
    /// Nexus-wide device reference when known.
    pub nexus_device_id: Option<nexus_domain::DeviceId>,
    /// Sentinel core device reference when known.
    pub device_id: Option<NetworkDeviceId>,
    /// The optional sensor profile that observed this event.
    pub profile: AdvancedSensorProfile,
    pub kind: FindingKind,
    pub severity: FindingSeverity,
    pub state: AlertState,
    /// Reference to the observed evidence (log ref, alert id).
    pub evidence_ref: String,
    /// Correlation reference to the originating detection.
    pub correlation: Option<String>,
    /// RFC3339 timestamp of observation.
    pub observed_at: String,
}

impl SecurityEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: SecurityEventId,
        tenant_id: TenantId,
        profile: AdvancedSensorProfile,
        kind: FindingKind,
        severity: FindingSeverity,
        evidence_ref: impl Into<String>,
        observed_at: impl Into<String>,
    ) -> Self {
        Self {
            event_id,
            tenant_id,
            nexus_device_id: None,
            device_id: None,
            profile,
            kind,
            severity,
            state: AlertState::Open,
            evidence_ref: evidence_ref.into(),
            correlation: None,
            observed_at: observed_at.into(),
        }
    }

    pub fn with_device(mut self, device_id: NetworkDeviceId) -> Self {
        self.device_id = Some(device_id);
        self
    }

    pub fn with_nexus_device(mut self, nexus_device_id: nexus_domain::DeviceId) -> Self {
        self.nexus_device_id = Some(nexus_device_id);
        self
    }

    pub fn with_correlation(mut self, correlation: impl Into<String>) -> Self {
        self.correlation = Some(correlation.into());
        self
    }

    pub fn with_state(mut self, state: AlertState) -> Self {
        self.state = state;
        self
    }
}

/// An incident: correlated security events grouped instead of
/// flooding users (SPEC-013: alerts correlate into incidents). The
/// nexus-wide IncidentId is reused, never redefined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incident {
    pub incident_id: IncidentId,
    pub tenant_id: TenantId,
    /// EP-031 correlation reference for this grouping.
    pub correlation_id: IncidentCorrelationId,
    pub state: IncidentState,
    pub severity: FindingSeverity,
    /// Confidence of the correlation grouping (bounded, derived from
    /// observed shared evidence).
    pub confidence: CorrelationConfidence,
    /// The correlated event ids in this incident.
    pub event_ids: Vec<SecurityEventId>,
    /// Summary line derived from observed events (never a prompt).
    pub summary: String,
    /// RFC3339 timestamp of first event in the incident.
    pub opened_at: String,
    /// RFC3339 timestamp of last update.
    pub updated_at: String,
}

impl Incident {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        incident_id: IncidentId,
        tenant_id: TenantId,
        correlation_id: IncidentCorrelationId,
        severity: FindingSeverity,
        confidence: CorrelationConfidence,
        summary: impl Into<String>,
        opened_at: impl Into<String>,
        updated_at: impl Into<String>,
    ) -> Self {
        Self {
            incident_id,
            tenant_id,
            correlation_id,
            state: IncidentState::Open,
            severity,
            confidence,
            event_ids: Vec::new(),
            summary: summary.into(),
            opened_at: opened_at.into(),
            updated_at: updated_at.into(),
        }
    }

    pub fn with_event(mut self, event_id: SecurityEventId) -> Self {
        if !self.event_ids.contains(&event_id) {
            self.event_ids.push(event_id);
        }
        self
    }

    pub fn with_state(mut self, state: IncidentState) -> Self {
        self.state = state;
        self
    }

    pub fn with_updated(mut self, updated_at: impl Into<String>) -> Self {
        self.updated_at = updated_at.into();
        self
    }
}

/// A honeypot or honeytoken record (SPEC-013 behavior 7: optional
/// high-signal sensors isolated from real data). Trigger records are
/// OBSERVED data, never fabricated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoneypotRecord {
    pub honeypot_id: HoneypotId,
    pub tenant_id: TenantId,
    pub kind: HoneypotKind,
    pub state: HoneypotState,
    /// Provider-neutral source reference when triggered.
    pub source_ref: Option<String>,
    /// RFC3339 timestamp of last state change.
    pub changed_at: String,
}

impl HoneypotRecord {
    pub fn new(
        honeypot_id: HoneypotId,
        tenant_id: TenantId,
        kind: HoneypotKind,
        changed_at: impl Into<String>,
    ) -> Self {
        Self {
            honeypot_id,
            tenant_id,
            kind,
            state: HoneypotState::Armed,
            source_ref: None,
            changed_at: changed_at.into(),
        }
    }

    pub fn with_state(mut self, state: HoneypotState, changed_at: impl Into<String>) -> Self {
        self.state = state;
        self.changed_at = changed_at.into();
        self
    }

    pub fn with_source(mut self, source_ref: impl Into<String>) -> Self {
        self.source_ref = Some(source_ref.into());
        self
    }
}

/// A triage case: bounded prioritization of an incident derived from
/// observed severity and correlation confidence (never invented).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriageCase {
    pub case_id: TriageCaseId,
    pub incident_id: IncidentId,
    pub tenant_id: TenantId,
    pub priority: TriagePriority,
    /// Derived rationale referencing observed evidence.
    pub rationale: String,
    /// RFC3339 timestamp of triage.
    pub triaged_at: String,
}

impl TriageCase {
    pub fn new(
        case_id: TriageCaseId,
        incident_id: IncidentId,
        tenant_id: TenantId,
        priority: TriagePriority,
        rationale: impl Into<String>,
        triaged_at: impl Into<String>,
    ) -> Self {
        Self {
            case_id,
            incident_id,
            tenant_id,
            priority,
            rationale: rationale.into(),
            triaged_at: triaged_at.into(),
        }
    }
}

/// An investigation case: bounded evidence gathering and analysis for
/// an incident (SPEC-013: preserves evidence).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestigationCase {
    pub case_id: InvestigationCaseId,
    pub incident_id: IncidentId,
    pub tenant_id: TenantId,
    pub state: InvestigationState,
    /// Evidence references gathered during investigation.
    pub evidence_refs: Vec<String>,
    /// Findings produced by analysis.
    pub findings: Vec<String>,
    /// RFC3339 timestamp of case creation.
    pub opened_at: String,
}

impl InvestigationCase {
    pub fn new(
        case_id: InvestigationCaseId,
        incident_id: IncidentId,
        tenant_id: TenantId,
        opened_at: impl Into<String>,
    ) -> Self {
        Self {
            case_id,
            incident_id,
            tenant_id,
            state: InvestigationState::Open,
            evidence_refs: Vec::new(),
            findings: Vec::new(),
            opened_at: opened_at.into(),
        }
    }

    pub fn with_evidence(mut self, evidence_ref: impl Into<String>) -> Self {
        self.evidence_refs.push(evidence_ref.into());
        self
    }

    pub fn with_finding(mut self, finding: impl Into<String>) -> Self {
        self.findings.push(finding.into());
        self
    }

    pub fn with_state(mut self, state: InvestigationState) -> Self {
        self.state = state;
        self
    }
}

/// A response plan (SPEC-013 behavior 5/6). Automated containment is
/// limited to preauthorized high-confidence reversible rules;
/// destructive response (wipes, factory resets, broad lockouts,
/// credential rotation) always requires human procedure and is never
/// auto-applicable.
/// A response plan (SPEC-013: response planning; AUD-031).
///
/// Preauthorization is NEVER derived from the kind alone. A plan is
/// preauthorized only after `preauthorize()` binds BOTH the incident
/// confidence (High) AND a provider-specific reversibility proof -
/// only preauthorized high-confidence reversible containment may
/// auto-execute.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponsePlan {
    pub plan_id: ResponsePlanId,
    pub incident_id: IncidentId,
    pub tenant_id: TenantId,
    pub kind: ResponseKind,
    pub state: ResponsePlanState,
    /// True when the plan is preauthorized high-confidence reversible
    /// (SPEC-013 behavior 5). Destructive kinds are never
    /// preauthorized; bounded kinds are preauthorized ONLY when
    /// `preauthorize()` bound High confidence AND a provider-specific
    /// reversibility proof (AUD-031).
    pub preauthorized: bool,
    /// Approval class required to execute (SPEC-013 behavior 6:
    /// destructive remediation requires human procedure).
    pub approval_class: ApprovalClass,
    /// Provider-neutral quarantine proposal reference when this plan
    /// carries containment.
    pub quarantine_proposal_ref: Option<String>,
    /// Provider-specific reversibility proof bound at preauthorization
    /// (AUD-031). None until the provider proved the containment is
    /// reversible.
    pub reversibility_proof: Option<String>,
    /// RFC3339 timestamp of plan creation.
    pub proposed_at: String,
}

impl ResponsePlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plan_id: ResponsePlanId,
        incident_id: IncidentId,
        tenant_id: TenantId,
        kind: ResponseKind,
        approval_class: ApprovalClass,
        proposed_at: impl Into<String>,
    ) -> Self {
        // AUD-031: fail closed. Preauthorization is never derived from
        // the kind alone; it is bound explicitly by preauthorize().
        Self {
            plan_id,
            incident_id,
            tenant_id,
            kind,
            state: ResponsePlanState::Proposed,
            preauthorized: false,
            approval_class,
            quarantine_proposal_ref: None,
            reversibility_proof: None,
            proposed_at: proposed_at.into(),
        }
    }

    /// Bind preauthorization ONLY when the containment is bounded
    /// reversible, the incident is high confidence, and the provider
    /// supplied a reversibility proof (AUD-031). Fails closed
    /// (Policy) otherwise - no threat score may mint authorization.
    pub fn preauthorize(
        mut self,
        confidence: CorrelationConfidence,
        reversibility_proof: impl Into<String>,
    ) -> Result<Self, AdvancedSentinelError> {
        if !self.kind.is_bounded_containment() {
            return Err(AdvancedSentinelError::policy(
                "only bounded reversible containment may be preauthorized",
            ));
        }
        if confidence != CorrelationConfidence::High {
            return Err(AdvancedSentinelError::policy(
                "preauthorization requires high incident confidence",
            ));
        }
        let proof = reversibility_proof.into();
        if proof.trim().is_empty() {
            return Err(AdvancedSentinelError::policy(
                "preauthorization requires a provider-specific reversibility proof",
            ));
        }
        self.preauthorized = true;
        self.reversibility_proof = Some(proof);
        Ok(self)
    }

    pub fn with_quarantine(mut self, proposal_ref: impl Into<String>) -> Self {
        self.quarantine_proposal_ref = Some(proposal_ref.into());
        self
    }

    pub fn with_state(mut self, state: ResponsePlanState) -> Self {
        self.state = state;
        self
    }
}

/// A verification record (SPEC-013: returns the network to verified
/// safe state; exact-target evidence). Verification is only true when
/// independent readback proves the effect; it is never assumed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRecord {
    pub verification_id: VerificationRecordId,
    pub plan_id: ResponsePlanId,
    pub tenant_id: TenantId,
    pub state: VerificationState,
    /// Independent readback evidence reference.
    pub evidence_ref: String,
    /// RFC3339 timestamp of verification.
    pub verified_at: String,
}

impl VerificationRecord {
    pub fn new(
        verification_id: VerificationRecordId,
        plan_id: ResponsePlanId,
        tenant_id: TenantId,
        evidence_ref: impl Into<String>,
        verified_at: impl Into<String>,
    ) -> Self {
        Self {
            verification_id,
            plan_id,
            tenant_id,
            state: VerificationState::Pending,
            evidence_ref: evidence_ref.into(),
            verified_at: verified_at.into(),
        }
    }

    pub fn with_state(mut self, state: VerificationState) -> Self {
        self.state = state;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    fn incident_id() -> IncidentId {
        IncidentId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
    }

    #[test]
    fn ep031_unit_security_event_is_observed_data_with_evidence() {
        let evt = SecurityEvent::new(
            SecurityEventId::new("evt-1").unwrap(),
            tenant(),
            AdvancedSensorProfile::Suricata,
            FindingKind::ScanDetected,
            FindingSeverity::Medium,
            "eve-log:1234",
            "2026-08-20T00:00:00Z",
        )
        .with_device(NetworkDeviceId::new("dev-1").unwrap())
        .with_correlation("corr-1");
        assert_eq!(evt.state, AlertState::Open);
        assert_eq!(evt.profile, AdvancedSensorProfile::Suricata);
        assert_eq!(evt.evidence_ref, "eve-log:1234");
        assert_eq!(evt.correlation.as_deref(), Some("corr-1"));
    }

    #[test]
    fn ep031_unit_incident_correlates_events_not_floods() {
        // Alerts correlate into incidents instead of flooding users.
        let mut inc = Incident::new(
            incident_id(),
            tenant(),
            IncidentCorrelationId::new("corr-1").unwrap(),
            FindingSeverity::High,
            CorrelationConfidence::High,
            "scan from unknown device",
            "2026-08-20T00:00:00Z",
            "2026-08-20T00:00:00Z",
        );
        inc = inc
            .with_event(SecurityEventId::new("evt-1").unwrap())
            .with_event(SecurityEventId::new("evt-2").unwrap())
            .with_event(SecurityEventId::new("evt-1").unwrap());
        assert_eq!(inc.event_ids.len(), 2, "duplicate event not re-added");
        assert_eq!(inc.state, IncidentState::Open);
    }

    #[test]
    fn ep031_unit_honeypot_record_is_isolated_observed_data() {
        let rec = HoneypotRecord::new(
            HoneypotId::new("hp-1").unwrap(),
            tenant(),
            HoneypotKind::HoneyToken,
            "2026-08-20T00:00:00Z",
        )
        .with_state(HoneypotState::Triggered, "2026-08-20T00:00:05Z")
        .with_source("192.0.2.10");
        assert_eq!(rec.state, HoneypotState::Triggered);
        assert_eq!(rec.source_ref.as_deref(), Some("192.0.2.10"));
    }

    #[test]
    fn ep031_unit_response_plan_destructive_never_preauthorized() {
        // AUD-031: preauthorization requires high confidence AND a
        // provider-specific reversibility proof. A bounded plan is NOT
        // preauthorized from the kind alone; a destructive plan is
        // never preauthorized under any conditions.
        let containment = ResponsePlan::new(
            ResponsePlanId::new("plan-1").unwrap(),
            incident_id(),
            tenant(),
            ResponseKind::Quarantine,
            ApprovalClass::Human,
            "2026-08-20T00:00:00Z",
        );
        assert!(
            !containment.preauthorized,
            "kind alone never mints preauthorization (AUD-031)"
        );
        let preauthorized = containment
            .clone()
            .preauthorize(
                CorrelationConfidence::High,
                "opnsense:proposal:p-1:reversible",
            )
            .expect("high-confidence bounded with provider proof may preauthorize");
        assert!(preauthorized.preauthorized);
        let destructive = ResponsePlan::new(
            ResponsePlanId::new("plan-2").unwrap(),
            incident_id(),
            tenant(),
            ResponseKind::Wipe,
            ApprovalClass::StrongHuman,
            "2026-08-20T00:00:00Z",
        );
        assert!(
            !destructive.preauthorized,
            "destructive never preauthorized"
        );
        assert!(
            destructive
                .preauthorize(
                    CorrelationConfidence::High,
                    "opnsense:proposal:p-1:reversible"
                )
                .is_err(),
            "destructive cannot be preauthorized even with proof"
        );
        assert!(ResponseKind::Wipe.is_destructive());
    }

    #[test]
    fn ep031_unit_verification_requires_exact_evidence() {
        let ver = VerificationRecord::new(
            VerificationRecordId::new("ver-1").unwrap(),
            ResponsePlanId::new("plan-1").unwrap(),
            tenant(),
            "readback-rule-ref-1",
            "2026-08-20T00:00:01Z",
        );
        assert_eq!(ver.state, VerificationState::Pending);
        assert_eq!(ver.evidence_ref, "readback-rule-ref-1");
    }

    #[test]
    fn ep031_unit_advanced_models_roundtrip_serde() {
        let evt = SecurityEvent::new(
            SecurityEventId::new("evt-1").unwrap(),
            tenant(),
            AdvancedSensorProfile::Zeek,
            FindingKind::DnsAnomaly,
            FindingSeverity::Low,
            "conn-log:9",
            "2026-08-20T00:00:00Z",
        );
        let json = serde_json::to_string(&evt).unwrap();
        let back: SecurityEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, evt);
    }
}
