//! EP-031 LF-009 sentinel-quarantine live-fire services (M5).
//!
//! Production implementations of the advanced sentinel ports used by
//! the LF-009 journey:
//!
//! - `SentinelTriageService`: correlates observed events into
//!   incidents over COMPATIBLE OBSERVED FACTS (a shared observed
//!   source indicator), never over raw sensor count. Confidence is
//!   derived from corroboration across independent observation planes
//!   (network detection, reputation, endpoint), never from "N sensors
//!   agree".
//! - `SentinelInvestigationService`: gathers observed evidence
//!   references into an investigation case.
//! - `SentinelResponsePlanner`: bounded containment
//!   (Quarantine/Block/IsolateEndpoint) may be preauthorized;
//!   destructive kinds (Wipe/FactoryReset/BroadLockout/
//!   CredentialRotation) are NEVER preauthorized and always require
//!   human procedure; planning a destructive kind under an
//!   insufficient approval class fails closed.
//! - `SentinelVerificationService`: independent exact-target readback
//!   through the firewall containment engine; a verification is only
//!   `Verified` when the readback proves the effect.
//!
//! These implementations preserve the permanent invariant: a raw
//! sensor event can never become an executed destructive response
//! without explicit human approval and independent verification.

use std::str::FromStr;
use std::sync::{Arc, Mutex};

use nexus_domain::{ApprovalClass, TenantId};
use nexus_sentinel::{
    FindingSeverity, QuarantineProposal, SentinelCapabilityMap, SentinelError, SentinelErrorCode,
};
use nexus_sentinel_advanced::{
    CorrelationConfidence, Incident, IncidentCorrelationId, InvestigationCase, InvestigationCaseId,
    ResponseKind, ResponsePlan, ResponsePlanId, ResponsePlanner, SecurityEvent, SecurityEventId,
    SecurityInvestigator, SecurityTriage, SecurityVerifier, TriageCase, TriageCaseId,
    TriagePriority, VerificationRecord, VerificationRecordId, VerificationState,
};

/// Triage service: correlate observed events over a shared observed
/// source indicator, then prioritize the resulting incident.
#[derive(Debug, Clone, Default)]
pub struct SentinelTriageService;

impl SentinelTriageService {
    /// Extract a candidate IPv4 source indicator from an observed
    /// event's correlation or evidence reference. Returns None when
    /// the event carries no IPv4 indicator (endpoint-context events
    /// still join the incident window without inflating confidence).
    fn source_indicator(event: &SecurityEvent) -> Option<String> {
        let mut candidates = Vec::new();
        if let Some(c) = &event.correlation {
            if let Some(v) = c.strip_prefix("src=") {
                candidates.push(v.to_string());
            }
        }
        for token in event.evidence_ref.split(':') {
            if token.parse::<std::net::Ipv4Addr>().is_ok() {
                candidates.push(token.to_string());
            }
        }
        candidates.pop()
    }

    /// Independent observation plane of an event (derived from the
    /// profile, never from raw count).
    fn plane(profile: nexus_sentinel_advanced::AdvancedSensorProfile) -> &'static str {
        match profile {
            nexus_sentinel_advanced::AdvancedSensorProfile::Suricata
            | nexus_sentinel_advanced::AdvancedSensorProfile::Zeek => "network",
            nexus_sentinel_advanced::AdvancedSensorProfile::Crowdsec => "reputation",
            nexus_sentinel_advanced::AdvancedSensorProfile::Wazuh
            | nexus_sentinel_advanced::AdvancedSensorProfile::Osquery => "endpoint",
            nexus_sentinel_advanced::AdvancedSensorProfile::Honeypot => "honeypot",
        }
    }
}

impl SecurityTriage for SentinelTriageService {
    fn triage_events(
        &self,
        tenant_id: &TenantId,
        correlation_id: IncidentCorrelationId,
        events: &[SecurityEvent],
    ) -> Result<Incident, SentinelError> {
        if events.is_empty() {
            return Err(SentinelError::new(
                SentinelErrorCode::Validation,
                "no events to triage",
                None,
                None,
                None,
                None,
            ));
        }
        // Compatible observed facts: group events by a shared observed
        // IPv4 source indicator. Events without an indicator are
        // window context, never the correlation key.
        let mut by_indicator: Vec<(String, Vec<&SecurityEvent>)> = Vec::new();
        let mut context: Vec<&SecurityEvent> = Vec::new();
        for event in events {
            match Self::source_indicator(event) {
                Some(ind) => {
                    if let Some(entry) = by_indicator.iter_mut().find(|(k, _)| *k == ind) {
                        entry.1.push(event);
                    } else {
                        by_indicator.push((ind, vec![event]));
                    }
                }
                None => context.push(event),
            }
        }
        let Some((indicator, group)) = by_indicator
            .iter()
            .max_by_key(|(_, g)| g.len())
            .map(|(k, g)| (k.clone(), g.clone()))
        else {
            return Err(SentinelError::new(
                SentinelErrorCode::Validation,
                "no shared observed source indicator; correlation refused",
                None,
                None,
                None,
                None,
            ));
        };
        let mut severity = FindingSeverity::Low;
        // AUD-032: confidence derives ONLY from independent planes
        // corroborating the SAME observed indicator. Context events
        // (no shared indicator) join the incident window and
        // contribute observed severity, but they NEVER inflate
        // confidence - an unrelated endpoint/reputation event cannot
        // corroborate a network-scanner indicator.
        let mut corroborating_planes: Vec<&'static str> = Vec::new();
        let mut event_ids: Vec<SecurityEventId> = Vec::new();
        for event in group.iter() {
            if event.severity > severity {
                severity = event.severity;
            }
            let plane = Self::plane(event.profile);
            if !corroborating_planes.contains(&plane) {
                corroborating_planes.push(plane);
            }
            if !event_ids.contains(&event.event_id) {
                event_ids.push(event.event_id.clone());
            }
        }
        for event in context.iter() {
            if event.severity > severity {
                severity = event.severity;
            }
            if !event_ids.contains(&event.event_id) {
                event_ids.push(event.event_id.clone());
            }
        }
        // Confidence derives from independent observation planes
        // corroborating the SAME observed indicator: 2+ planes ->
        // High, else Medium. Raw event count never inflates it;
        // unrelated context never does either (AUD-032).
        let confidence = if corroborating_planes.len() >= 2 {
            CorrelationConfidence::High
        } else {
            CorrelationConfidence::Medium
        };
        let summary = format!(
            "observed source {} corroborated across {} independent plane(s) ({}), severity {}, {} event(s) correlated",
            indicator,
            corroborating_planes.len(),
            corroborating_planes.join("+"),
            severity.as_str(),
            event_ids.len()
        );
        let mut incident = Incident::new(
            nexus_domain::IncidentId::from_str("018f0f6f-9c1e-7b6e-8000-0000000000aa").map_err(
                |_| {
                    SentinelError::new(
                        SentinelErrorCode::Validation,
                        "incident id",
                        None,
                        None,
                        None,
                        None,
                    )
                },
            )?,
            tenant_id.clone(),
            correlation_id,
            severity,
            confidence,
            summary,
            "2026-08-20T00:00:00Z",
            "2026-08-20T00:00:00Z",
        )
        .with_state(nexus_sentinel_advanced::IncidentState::Open);
        for id in event_ids {
            incident = incident.with_event(id);
        }
        Ok(incident)
    }

    fn prioritize(
        &self,
        tenant_id: &TenantId,
        case_id: TriageCaseId,
        incident: &Incident,
    ) -> Result<TriageCase, SentinelError> {
        let priority = match (incident.severity, &incident.confidence) {
            (FindingSeverity::Critical, _) => TriagePriority::Critical,
            (FindingSeverity::High, CorrelationConfidence::High) => TriagePriority::Critical,
            (FindingSeverity::High, _) => TriagePriority::High,
            (FindingSeverity::Medium, _) => TriagePriority::Medium,
            _ => TriagePriority::Low,
        };
        Ok(TriageCase::new(
            case_id,
            incident.incident_id.clone(),
            tenant_id.clone(),
            priority,
            format!(
                "severity {} with {} confidence (observed indicator corroboration)",
                incident.severity.as_str(),
                incident.confidence.as_str()
            ),
            "2026-08-20T00:00:00Z",
        ))
    }
}

/// Investigation service: gathers observed evidence references.
#[derive(Debug, Clone, Default)]
pub struct SentinelInvestigationService;

impl SecurityInvestigator for SentinelInvestigationService {
    fn investigate(
        &self,
        tenant_id: &TenantId,
        incident: &Incident,
    ) -> Result<InvestigationCase, SentinelError> {
        let mut case = InvestigationCase::new(
            InvestigationCaseId::new("invest-lf009").map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::Validation,
                    "investigation id",
                    None,
                    None,
                    None,
                    None,
                )
            })?,
            incident.incident_id.clone(),
            tenant_id.clone(),
            "2026-08-20T00:00:00Z",
        );
        for id in &incident.event_ids {
            case = case.with_evidence(format!("event:{id}"));
        }
        case = case.with_finding(incident.summary.clone());
        Ok(case)
    }
}

/// Response planner: bounded containment may be preauthorized;
/// destructive response always requires human procedure and is never
/// preauthorized.
///
/// AUD-031: preauthorization is bound only when the incident is high
/// confidence AND a provider-specific reversibility proof is supplied.
/// A bounded plan without the proof is NOT preauthorized - it may be
/// executed under explicit human approval, but never auto-executed.
#[derive(Debug, Clone, Default)]
pub struct SentinelResponsePlanner;

impl ResponsePlanner for SentinelResponsePlanner {
    fn plan_response(
        &self,
        tenant_id: &TenantId,
        plan_id: ResponsePlanId,
        incident: &Incident,
        kind: ResponseKind,
        approval_class: ApprovalClass,
        reversibility_proof: Option<&str>,
    ) -> Result<ResponsePlan, SentinelError> {
        if kind.is_destructive() {
            // SPEC-013 behavior 6: destructive remediation requires
            // human procedure (ApprovalClass::Human or stronger). An
            // approval class that cannot supply a human fails closed -
            // no threat score may mint authorization.
            match approval_class {
                ApprovalClass::Human | ApprovalClass::StrongHuman | ApprovalClass::FourEyes => {}
                ApprovalClass::None | ApprovalClass::Policy => {
                    return Err(SentinelError::new(
                        SentinelErrorCode::Authorization,
                        "destructive response requires human procedure",
                        None,
                        None,
                        None,
                        None,
                    ));
                }
            }
        }
        let plan = ResponsePlan::new(
            plan_id,
            incident.incident_id.clone(),
            tenant_id.clone(),
            kind,
            approval_class,
            "2026-08-20T00:00:00Z",
        );
        // AUD-031: bounded containment is preauthorized ONLY with high
        // incident confidence AND a provider-specific reversibility
        // proof. Without the proof the plan fails closed (not
        // preauthorized); it may still execute under human approval.
        if plan.kind.is_bounded_containment() {
            if let Some(proof) = reversibility_proof {
                return plan.preauthorize(incident.confidence, proof);
            }
        }
        Ok(plan)
    }
}

/// Verification service: independent exact-target readback through
/// the firewall containment engine. A verification is only `Verified`
/// when the readback proves the effect; it is never assumed.
#[derive(Clone, Default)]
pub struct SentinelVerificationService {
    /// The applied containment proposal (registered after execution).
    applied: Arc<Mutex<Option<QuarantineProposal>>>,
    /// The firewall containment engine used for exact-target readback.
    firewall: Arc<Mutex<Option<Arc<dyn nexus_sentinel::FirewallProvider + Send + Sync>>>>,
}

impl SentinelVerificationService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind_firewall(&self, provider: Arc<dyn nexus_sentinel::FirewallProvider + Send + Sync>) {
        *self.firewall.lock().unwrap() = Some(provider);
    }

    pub fn register_applied(&self, proposal: QuarantineProposal) {
        *self.applied.lock().unwrap() = Some(proposal);
    }
}

impl SecurityVerifier for SentinelVerificationService {
    fn verify_response(
        &self,
        tenant_id: &TenantId,
        verification_id: VerificationRecordId,
        plan: &ResponsePlan,
    ) -> Result<VerificationRecord, SentinelError> {
        if !plan.kind.is_bounded_containment() {
            return Err(SentinelError::new(
                SentinelErrorCode::Unavailable,
                "verification requires a containment plan",
                None,
                None,
                None,
                None,
            ));
        }
        let applied = self
            .applied
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| SentinelError::unavailable("no applied containment registered"))?;
        let firewall = self
            .firewall
            .lock()
            .unwrap()
            .as_ref()
            .cloned()
            .ok_or_else(|| SentinelError::unavailable("no firewall engine bound"))?;
        let readback = firewall.verify_containment(&applied)?;
        let evidence = format!(
            "opnsense:readback:{}:verified={}",
            applied.proposal_id.as_str(),
            readback.verified
        );
        let state = if readback.verified {
            VerificationState::Verified
        } else {
            VerificationState::Failed
        };
        Ok(VerificationRecord::new(
            verification_id,
            plan.plan_id.clone(),
            tenant_id.clone(),
            evidence,
            "2026-08-20T00:00:00Z",
        )
        .with_state(state))
    }
}

// Sentinel capability maps are not advertised by services (services
// are not providers); kept for API symmetry with the contract.
#[allow(dead_code)]
fn _cap(_: &SentinelCapabilityMap) {}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::TenantId;
    use nexus_sentinel_advanced::SecurityEventId;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    fn evt(
        id: &str,
        profile: nexus_sentinel_advanced::AdvancedSensorProfile,
        correlation: Option<&str>,
    ) -> SecurityEvent {
        let mut e = SecurityEvent::new(
            SecurityEventId::new(id).unwrap(),
            tenant(),
            profile,
            nexus_sentinel::FindingKind::ScanDetected,
            nexus_sentinel::FindingSeverity::Medium,
            format!("evidence:{id}"),
            "2026-08-20T00:00:00Z",
        );
        if let Some(c) = correlation {
            e = e.with_correlation(c);
        }
        e
    }

    #[test]
    fn aud032_unit_unrelated_context_events_never_inflate_confidence() {
        // AUD-032: confidence derives ONLY from planes corroborating
        // the SAME observed source indicator. Unrelated context events
        // (endpoint/reputation planes with NO shared indicator) join
        // the incident window but must NOT raise confidence to High.
        let triage = SentinelTriageService;
        let events = vec![
            // One plane corroborates the scanner indicator (network).
            evt(
                "evt-net-1",
                nexus_sentinel_advanced::AdvancedSensorProfile::Zeek,
                Some("src=192.168.40.77"),
            ),
            // Unrelated context: endpoint event with NO indicator.
            evt(
                "evt-end-1",
                nexus_sentinel_advanced::AdvancedSensorProfile::Osquery,
                None,
            ),
            // Unrelated context: reputation event with NO indicator.
            evt(
                "evt-rep-1",
                nexus_sentinel_advanced::AdvancedSensorProfile::Crowdsec,
                None,
            ),
        ];
        let incident = triage
            .triage_events(
                &tenant(),
                IncidentCorrelationId::new("corr-aud032").unwrap(),
                &events,
            )
            .expect("triage correlates");
        // Only ONE plane corroborates the scanner indicator; the
        // unrelated endpoint/reputation context cannot inflate it.
        assert_eq!(
            incident.confidence,
            CorrelationConfidence::Medium,
            "unrelated context events must never inflate confidence"
        );
        // Context events still join the incident window (observed,
        // deduped) - they are evidence, just not corroboration.
        assert_eq!(incident.event_ids.len(), 3);
        assert!(incident.summary.contains("1 independent plane"));
    }

    #[test]
    fn aud032_unit_two_planes_same_indicator_high_confidence() {
        // Two INDEPENDENT planes corroborating the SAME indicator
        // still yield High - the fix must not weaken real
        // corroboration.
        let triage = SentinelTriageService;
        let events = vec![
            evt(
                "evt-net-1",
                nexus_sentinel_advanced::AdvancedSensorProfile::Zeek,
                Some("src=192.168.40.77"),
            ),
            evt(
                "evt-rep-1",
                nexus_sentinel_advanced::AdvancedSensorProfile::Crowdsec,
                Some("src=192.168.40.77"),
            ),
        ];
        let incident = triage
            .triage_events(
                &tenant(),
                IncidentCorrelationId::new("corr-aud032b").unwrap(),
                &events,
            )
            .expect("triage correlates");
        assert_eq!(incident.confidence, CorrelationConfidence::High);
    }

    #[test]
    fn aud032_unit_no_shared_indicator_refuses_correlation() {
        // No event carries a source indicator: correlation is refused
        // (fail closed) - nothing is invented.
        let triage = SentinelTriageService;
        let events = vec![
            evt(
                "evt-a",
                nexus_sentinel_advanced::AdvancedSensorProfile::Zeek,
                None,
            ),
            evt(
                "evt-b",
                nexus_sentinel_advanced::AdvancedSensorProfile::Wazuh,
                None,
            ),
        ];
        let err = triage
            .triage_events(
                &tenant(),
                IncidentCorrelationId::new("corr-aud032c").unwrap(),
                &events,
            )
            .unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Validation);
    }
}
