//! EP-031 CrowdSec reputation adapter (M3).
//!
//! Implements the nexus-sentinel-advanced `ThreatIntelProvider` port
//! over the documented CrowdSec LAPI decisions surface. Decisions are
//! OBSERVED data; a ban decision for an indicator is normalized into
//! a canonical SecurityEvent without inventing root causes.
//!
//! Normalization: an observed `ban` decision produces a SecurityEvent
//! with FindingKind::ScanDetected (reputation enforcement) only when
//! the decision action is documented as `ban`. Non-ban actions are
//! observed but never fabricated into a finding. An indicator with NO
//! active decisions returns None (clean reputation is absence of
//! evidence, never a fabricated verdict).
//!
//! Capabilities advertise ReadFindings ONLY when the transport is
//! bound (Reality rule). Unbound providers fail closed.

use std::cell::RefCell;

use nexus_domain::TenantId;
use nexus_sentinel::{
    FindingKind, FindingSeverity, SentinelCapabilityKind, SentinelCapabilityMap, SentinelError,
    SentinelErrorCode,
};
use nexus_sentinel_advanced::{
    AdvancedSensorProfile, AlertState, SecurityEvent, SecurityEventId, ThreatIntelProvider,
};

use crate::transport::CrowdSecTransport;

/// CrowdSec threat intelligence provider over a bound transport.
#[derive(Debug)]
pub struct CrowdSecThreatIntelProvider<T> {
    transport: Option<RefCell<T>>,
}

impl<T> CrowdSecThreatIntelProvider<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport: Some(RefCell::new(transport)),
        }
    }

    pub fn unbound() -> Self {
        Self { transport: None }
    }
}

impl<T: CrowdSecTransport> ThreatIntelProvider for CrowdSecThreatIntelProvider<T> {
    fn capabilities(&self) -> SentinelCapabilityMap {
        let mut map = SentinelCapabilityMap::new();
        if self.transport.is_some() {
            map.insert(SentinelCapabilityKind::ReadFindings);
        }
        map
    }

    fn lookup_reputation(
        &self,
        tenant_id: &TenantId,
        indicator: &str,
    ) -> Result<Option<SecurityEvent>, SentinelError> {
        if indicator.is_empty() {
            return Err(SentinelError::validation(
                "crowdsec reputation indicator must not be empty",
            ));
        }
        let Some(transport) = &self.transport else {
            return Err(SentinelError::unavailable(
                "crowdsec threat intel provider has no transport bound",
            ));
        };
        let decisions = transport
            .borrow_mut()
            .decisions_for(indicator)
            .map_err(|_| SentinelError::unavailable("crowdsec reputation lookup failed"))?;
        // A ban decision is OBSERVED reputation evidence. Only the
        // documented `ban` action maps to a canonical finding; other
        // decisions are observed but never fabricated.
        let ban = decisions.iter().find(|d| d.action == "ban").cloned();
        let Some(decision) = ban else {
            return Ok(None);
        };
        let event_id = SecurityEventId::new(format!("crowdsec-{}", decision.id)).map_err(|_| {
            SentinelError::new(
                SentinelErrorCode::Validation,
                "crowdsec decision id cannot form event id",
                None,
                None,
                None,
                None,
            )
        })?;
        let evidence = format!("crowdsec:{}:{}", decision.scenario, decision.value);
        let mut event = SecurityEvent::new(
            event_id,
            tenant_id.clone(),
            AdvancedSensorProfile::Crowdsec,
            FindingKind::ScanDetected,
            FindingSeverity::Medium,
            evidence,
            decision.created_at.clone(),
        );
        event = event
            .with_correlation(format!("origin={}", decision.origin))
            .with_state(AlertState::Open);
        Ok(Some(event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_sentinel_advanced::ThreatIntelProvider;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    #[test]
    fn ep031_unit_crowdsec_unbound_fails_closed() {
        let p = CrowdSecThreatIntelProvider::<()>::unbound();
        assert!(p.capabilities().is_empty());
        let err = p.lookup_reputation(&tenant(), "1.2.3.4").unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
    }

    #[test]
    fn ep031_unit_crowdsec_empty_indicator_rejected() {
        let p = CrowdSecThreatIntelProvider::<()>::unbound();
        let err = p.lookup_reputation(&tenant(), "").unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Validation);
    }

    #[test]
    fn ep031_unit_crowdsec_ban_decision_normalized_to_event() {
        let decisions = vec![crate::transport::CrowdSecDecision {
            id: 42,
            origin: "cscli".into(),
            action: "ban".into(),
            scope: "Ip".into(),
            value: "1.2.3.4".into(),
            duration: "4h0m0s".into(),
            scenario: "ssh-bf".into(),
            created_at: "2026-08-20T00:00:00Z".into(),
        }];
        let transport = crate::transport::StubCrowdSecTransport { decisions };
        let p = CrowdSecThreatIntelProvider::new(transport);
        let caps = p.capabilities();
        assert!(caps.contains(SentinelCapabilityKind::ReadFindings));
        let event = p
            .lookup_reputation(&tenant(), "1.2.3.4")
            .unwrap()
            .expect("ban decision produces event");
        assert_eq!(event.profile, AdvancedSensorProfile::Crowdsec);
        assert_eq!(event.kind, FindingKind::ScanDetected);
        assert_eq!(event.severity, FindingSeverity::Medium);
        assert_eq!(event.evidence_ref, "crowdsec:ssh-bf:1.2.3.4");
    }

    #[test]
    fn ep031_unit_crowdsec_no_ban_decision_returns_none() {
        let decisions = vec![crate::transport::CrowdSecDecision {
            id: 43,
            origin: "cscli".into(),
            action: "captcha".into(),
            scope: "Ip".into(),
            value: "1.2.3.5".into(),
            duration: "1h0m0s".into(),
            scenario: "http-shakedown".into(),
            created_at: "2026-08-20T00:00:00Z".into(),
        }];
        let transport = crate::transport::StubCrowdSecTransport { decisions };
        let p = CrowdSecThreatIntelProvider::new(transport);
        // Non-ban actions are observed but never fabricated into a
        // canonical finding.
        assert!(p.lookup_reputation(&tenant(), "1.2.3.5").unwrap().is_none());
    }
}
