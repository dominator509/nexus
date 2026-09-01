//! EP-031 Suricata network detection adapter (AUD-030).
//!
//! Implements the nexus-sentinel-advanced `NetworkDetectionProvider`
//! port over the documented Suricata EVE JSON surface. Alerts are
//! OBSERVED data; normalization maps documented alert signatures to
//! canonical sentinel findings without inventing root causes.
//!
//! Normalization map (documented Suricata alert signatures and
//! categories; unknown signatures are never fabricated into a
//! finding):
//! - signature/category containing `SCAN` -> FindingKind::ScanDetected
//! - signature/category containing `DNS` -> FindingKind::DnsAnomaly
//! - other documented alerts -> observed only (not classified)
//!
//! Severity follows the DOCUMENTED Suricata alert severity bound
//! 1..=4 (1 highest): 1 -> Critical, 2 -> High, 3 -> Medium, 4 ->
//! Low. A severity outside the documented bound fails closed
//! (External) - it is never guessed.
//!
//! Capabilities advertise ReadFindings ONLY when the transport is
//! bound and answers (Reality rule). Unbound providers fail closed.

use nexus_domain::TenantId;
use nexus_sentinel::{
    FindingKind, FindingSeverity, SentinelCapabilityKind, SentinelCapabilityMap, SentinelError,
    SentinelErrorCode,
};
use nexus_sentinel_advanced::{
    AdvancedSensorProfile, AlertState, NetworkDetectionProvider, SecurityEvent, SecurityEventId,
};

use crate::eve::SuricataAlertSeverity;
use crate::transport::{SuricataAlert, SuricataTransport};

/// Suricata network detection provider over a bound transport.
#[derive(Debug)]
pub struct SuricataNetworkDetectionProvider<T> {
    transport: Option<std::cell::RefCell<T>>,
}

impl<T> SuricataNetworkDetectionProvider<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport: Some(std::cell::RefCell::new(transport)),
        }
    }

    pub fn unbound() -> Self {
        Self { transport: None }
    }
}

impl<T: SuricataTransport> NetworkDetectionProvider for SuricataNetworkDetectionProvider<T> {
    fn capabilities(&self) -> SentinelCapabilityMap {
        let mut map = SentinelCapabilityMap::new();
        if self.transport.is_some() {
            map.insert(SentinelCapabilityKind::ReadFindings);
        }
        map
    }

    fn read_events(&self, tenant_id: &TenantId) -> Result<Vec<SecurityEvent>, SentinelError> {
        let Some(transport) = &self.transport else {
            return Err(SentinelError::unavailable(
                "suricata network detection provider has no transport bound",
            ));
        };
        let alerts = transport
            .borrow_mut()
            .read_alerts()
            .map_err(|_| SentinelError::unavailable("suricata transport read failed"))?;
        let mut events = Vec::new();
        for alert in alerts {
            let Some((kind, severity)) = classify(&alert) else {
                // Observed but not classified: never fabricate a
                // canonical finding for an unknown signature.
                continue;
            };
            // Severity follows the DOCUMENTED Suricata bound 1..=4;
            // a value outside the bound fails closed instead of being
            // guessed (reuses the vocabulary-validated type).
            let severity = match alert.severity {
                Some(raw) => match SuricataAlertSeverity::new(raw) {
                    Ok(_) => severity_from_suricata(raw),
                    Err(_) => {
                        return Err(SentinelError::new(
                            SentinelErrorCode::ExternalProvider,
                            "suricata alert severity outside documented bound 1..=4",
                            None,
                            None,
                            None,
                            None,
                        ))
                    }
                },
                None => severity,
            };
            let event_id = SecurityEventId::new(format!(
                "suricata-{}-{}",
                alert
                    .flow_id
                    .map(|f| f.to_string())
                    .unwrap_or_else(|| "na".to_string()),
                alert
                    .signature_id
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "na".to_string())
            ))
            .map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::Validation,
                    "suricata alert identifiers cannot form event id",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            let evidence = format!("suricata:{}:{}", alert.signature, alert.event_type);
            let mut event = SecurityEvent::new(
                event_id,
                tenant_id.clone(),
                AdvancedSensorProfile::Suricata,
                kind,
                severity,
                evidence,
                alert.observed_at.clone(),
            );
            if let Some(src) = &alert.src_ip {
                event = event.with_correlation(format!("src={src}"));
            }
            event = event.with_state(AlertState::Open);
            events.push(event);
        }
        Ok(events)
    }
}

/// Classify a documented Suricata alert signature/category into a
/// canonical sentinel finding. Unknown signatures return None
/// (observed but never fabricated).
fn classify(alert: &SuricataAlert) -> Option<(FindingKind, FindingSeverity)> {
    let hay = format!("{} {}", alert.signature, alert.category).to_uppercase();
    if hay.contains("SCAN") {
        Some((FindingKind::ScanDetected, FindingSeverity::Medium))
    } else if hay.contains("DNS") {
        Some((FindingKind::DnsAnomaly, FindingSeverity::Low))
    } else {
        None
    }
}

/// Map the DOCUMENTED Suricata alert severity (1..=4, 1 highest) to a
/// canonical sentinel severity. The caller has already validated the
/// bound.
fn severity_from_suricata(raw: u8) -> FindingSeverity {
    match raw {
        1 => FindingSeverity::Critical,
        2 => FindingSeverity::High,
        3 => FindingSeverity::Medium,
        _ => FindingSeverity::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::TenantId;
    use nexus_sentinel_advanced::NetworkDetectionProvider;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    fn alert(
        signature: &str,
        category: &str,
        severity: Option<u8>,
        src_ip: &str,
        flow_id: u64,
        signature_id: u64,
    ) -> SuricataAlert {
        SuricataAlert {
            event_type: "alert".into(),
            observed_at: "2026-08-20T00:00:01Z".into(),
            flow_id: Some(flow_id),
            src_ip: Some(src_ip.into()),
            src_port: Some(40000),
            dest_ip: Some("192.168.40.1".into()),
            dest_port: Some(22),
            proto: Some("TCP".into()),
            signature_id: Some(signature_id),
            signature: signature.into(),
            category: category.into(),
            severity,
        }
    }

    #[test]
    fn aud030_unit_suricata_unbound_fails_closed() {
        let p = SuricataNetworkDetectionProvider::<()>::unbound();
        assert!(p.capabilities().is_empty());
        let err = p.read_events(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
    }

    #[test]
    fn aud030_unit_suricata_classifies_documented_signatures_only() {
        assert_eq!(
            classify(&alert(
                "ET SCAN Potential SSH Scan",
                "Attempted Information Leak",
                None,
                "x",
                1,
                1
            )),
            Some((FindingKind::ScanDetected, FindingSeverity::Medium))
        );
        assert_eq!(
            classify(&alert(
                "ET DNS Suspicious Query",
                "Potentially Bad Traffic",
                None,
                "x",
                2,
                2
            )),
            Some((FindingKind::DnsAnomaly, FindingSeverity::Low))
        );
        assert_eq!(
            classify(&alert(
                "ET TROJAN Generic",
                "A Network Trojan was detected",
                None,
                "x",
                3,
                3
            )),
            None
        );
    }

    #[test]
    fn aud030_unit_suricata_normalizes_alerts_to_events() {
        let json = r#"{"timestamp":"2026-08-20T00:00:01.000000+0000","flow_id":1,"event_type":"alert","src_ip":"192.168.40.77","src_port":40000,"dest_ip":"192.168.40.1","dest_port":22,"proto":"TCP","alert":{"signature_id":2018358,"signature":"ET SCAN Potential SSH Scan","category":"Attempted Information Leak","severity":2}}
{"timestamp":"2026-08-20T00:00:02.000000+0000","flow_id":2,"event_type":"alert","src_ip":"192.168.40.99","src_port":50000,"dest_ip":"192.168.40.1","dest_port":443,"proto":"TCP","alert":{"signature_id":9999999,"signature":"ET TROJAN Generic","category":"A Network Trojan was detected","severity":1}}"#;
        let transport = crate::transport::JsonLinesSuricataTransport::new(json.as_bytes());
        let p = SuricataNetworkDetectionProvider::new(transport);
        let caps = p.capabilities();
        assert!(caps.contains(SentinelCapabilityKind::ReadFindings));
        let events = p.read_events(&tenant()).unwrap();
        // Only the documented SCAN signature is classified; the
        // Trojan signature is observed but never fabricated.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].profile, AdvancedSensorProfile::Suricata);
        assert_eq!(events[0].kind, FindingKind::ScanDetected);
        assert_eq!(events[0].severity, FindingSeverity::High);
        assert_eq!(
            events[0].evidence_ref,
            "suricata:ET SCAN Potential SSH Scan:alert"
        );
        assert_eq!(events[0].correlation.as_deref(), Some("src=192.168.40.77"));
    }

    #[test]
    fn aud030_unit_suricata_severity_outside_bound_fails_closed() {
        let json = r#"{"timestamp":"2026-08-20T00:00:01Z","flow_id":1,"event_type":"alert","src_ip":"192.168.40.77","dest_ip":"192.168.40.1","proto":"TCP","alert":{"signature_id":1,"signature":"ET SCAN x","category":"Scan","severity":9}}"#;
        let transport = crate::transport::JsonLinesSuricataTransport::new(json.as_bytes());
        let p = SuricataNetworkDetectionProvider::new(transport);
        let err = p.read_events(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
    }
}
