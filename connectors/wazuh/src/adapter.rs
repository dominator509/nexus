//! EP-031 Wazuh endpoint telemetry adapter (M4).
//!
//! Implements the nexus-sentinel-advanced `EndpointTelemetryProvider`
//! port over the documented Wazuh server API. Alerts are OBSERVED
//! data; normalization maps documented Wazuh rule levels to canonical
//! sentinel findings without inventing root causes.
//!
//! Normalization: Wazuh rule level is a documented 0..=15 severity
//! scale (documentation.wazuh.com rule levels). Levels >= 12 are
//! HIGH, 7..=11 are MEDIUM, else LOW. A rule description is never
//! parsed for meaning; the level mapping is the only derived
//! classification.
//!
//! Capabilities advertise ReadFindings ONLY when the transport is
//! bound (Reality rule). Unbound providers fail closed. Every public
//! operation records a bounded redacted audit entry with correlation
//! (SentinelObservability).

use std::cell::RefCell;

use nexus_domain::TenantId;
use nexus_sentinel::{
    FindingKind, FindingSeverity, SentinelCapabilityKind, SentinelCapabilityMap, SentinelError,
    SentinelErrorCode,
};
use nexus_sentinel_advanced::{
    AdvancedSensorProfile, AlertState, EndpointTelemetryProvider, SecurityEvent, SecurityEventId,
};

use crate::observability::{SentinelAuditEntry, SentinelObservability};
use crate::transport::WazuhTransport;

/// Wazuh endpoint telemetry provider over a bound transport.
#[derive(Debug)]
pub struct WazuhEndpointTelemetryProvider<T> {
    transport: Option<RefCell<T>>,
    observability: SentinelObservability,
}

impl<T> WazuhEndpointTelemetryProvider<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport: Some(RefCell::new(transport)),
            observability: SentinelObservability::new(),
        }
    }

    pub fn unbound() -> Self {
        Self {
            transport: None,
            observability: SentinelObservability::new(),
        }
    }

    /// Bounded redacted audit entries (observability).
    pub fn audit_entries(&self) -> Vec<SentinelAuditEntry> {
        self.observability.entries()
    }
}

impl<T: WazuhTransport> EndpointTelemetryProvider for WazuhEndpointTelemetryProvider<T> {
    fn capabilities(&self) -> SentinelCapabilityMap {
        let mut map = SentinelCapabilityMap::new();
        if self.transport.is_some() {
            map.insert(SentinelCapabilityKind::ReadFindings);
        }
        map
    }

    fn read_telemetry(&self, tenant_id: &TenantId) -> Result<Vec<SecurityEvent>, SentinelError> {
        let Some(transport) = &self.transport else {
            self.observability.record(SentinelAuditEntry::new(
                "wazuh.read_telemetry",
                "denied",
                "no transport bound",
                tenant_id.as_str(),
            ));
            return Err(SentinelError::unavailable(
                "wazuh endpoint telemetry provider has no transport bound",
            ));
        };
        let outcome = transport
            .borrow_mut()
            .read_alerts(100)
            .map_err(|_| SentinelError::unavailable("wazuh endpoint telemetry read failed"));
        let alerts = match outcome {
            Ok(a) => a,
            Err(e) => {
                self.observability.record(SentinelAuditEntry::new(
                    "wazuh.read_telemetry",
                    "failed",
                    &e.message,
                    tenant_id.as_str(),
                ));
                return Err(e);
            }
        };
        let mut events = Vec::new();
        for alert in alerts {
            let (kind, severity) = classify(alert.rule_level);
            let event_id = SecurityEventId::new(format!("wazuh-{}", alert.id)).map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::Validation,
                    "wazuh alert id cannot form event id",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            let evidence = format!("wazuh:{}:{}", alert.rule_description, alert.id);
            let mut event = SecurityEvent::new(
                event_id,
                tenant_id.clone(),
                AdvancedSensorProfile::Wazuh,
                kind,
                severity,
                evidence,
                alert.timestamp.clone(),
            );
            if let Some(agent) = &alert.agent_name {
                event = event.with_correlation(format!("agent={agent}"));
            }
            event = event.with_state(AlertState::Open);
            events.push(event);
        }
        self.observability.record(SentinelAuditEntry::new(
            "wazuh.read_telemetry",
            "ok",
            format!("{} alerts observed", events.len()),
            tenant_id.as_str(),
        ));
        Ok(events)
    }
}

/// Classify a documented Wazuh rule level into a canonical sentinel
/// severity. The level scale is documented (0..=15); the mapping is
/// the ONLY derived classification - rule descriptions are never
/// parsed for meaning.
fn classify(level: u32) -> (FindingKind, FindingSeverity) {
    let severity = if level >= 12 {
        FindingSeverity::High
    } else if level >= 7 {
        FindingSeverity::Medium
    } else {
        FindingSeverity::Low
    };
    (FindingKind::BaselineViolation, severity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_sentinel_advanced::EndpointTelemetryProvider;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    #[test]
    fn ep031_unit_wazuh_unbound_fails_closed_with_audit() {
        let p = WazuhEndpointTelemetryProvider::<()>::unbound();
        assert!(p.capabilities().is_empty());
        let err = p.read_telemetry(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        assert!(!p.audit_entries().is_empty(), "denial recorded");
        assert_eq!(p.audit_entries()[0].outcome, "denied");
    }

    #[test]
    fn ep031_unit_wazuh_level_classification_bounded() {
        assert_eq!(
            classify(15),
            (FindingKind::BaselineViolation, FindingSeverity::High)
        );
        assert_eq!(
            classify(12),
            (FindingKind::BaselineViolation, FindingSeverity::High)
        );
        assert_eq!(
            classify(7),
            (FindingKind::BaselineViolation, FindingSeverity::Medium)
        );
        assert_eq!(
            classify(3),
            (FindingKind::BaselineViolation, FindingSeverity::Low)
        );
    }

    #[test]
    fn ep031_unit_wazuh_alerts_normalized_with_audit_ok() {
        let alerts = vec![
            crate::transport::WazuhAlert {
                id: "1".into(),
                rule_level: 12,
                rule_description: "PAM: Login session opened.".into(),
                agent_id: Some("001".into()),
                agent_name: Some("laptop".into()),
                agent_ip: Some("192.0.2.5".into()),
                timestamp: "2026-08-20T00:00:00Z".into(),
            },
            crate::transport::WazuhAlert {
                id: "2".into(),
                rule_level: 3,
                rule_description: "Audit: policy loaded.".into(),
                agent_id: None,
                agent_name: None,
                agent_ip: None,
                timestamp: "2026-08-20T00:00:01Z".into(),
            },
        ];
        let transport = crate::transport::StubWazuhTransport { alerts };
        let p = WazuhEndpointTelemetryProvider::new(transport);
        let caps = p.capabilities();
        assert!(caps.contains(SentinelCapabilityKind::ReadFindings));
        let events = p.read_telemetry(&tenant()).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].profile, AdvancedSensorProfile::Wazuh);
        assert_eq!(events[0].severity, FindingSeverity::High);
        assert_eq!(events[0].correlation.as_deref(), Some("agent=laptop"));
        assert_eq!(events[1].severity, FindingSeverity::Low);
        assert!(events[1].correlation.is_none());
        assert_eq!(p.audit_entries().last().unwrap().outcome, "ok");
    }
}
