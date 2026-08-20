//! EP-031 Zeek network detection adapter (M2).
//!
//! Implements the nexus-sentinel-advanced `NetworkDetectionProvider`
//! port over the documented Zeek JSON log surface. Notices are
//! OBSERVED data; normalization maps documented Zeek notice classes
//! to canonical sentinel findings without inventing root causes.
//!
//! Normalization map (documented Zeek notice identifiers; unknown
//! note classes are never fabricated into a finding):
//! - `Scan::*` -> FindingKind::ScanDetected, severity Medium
//! - `Weird::*` -> FindingKind::BaselineViolation, severity Low
//! - `DNS::*` -> FindingKind::DnsAnomaly, severity Low
//! - other documented notices -> not classified (observed only)
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

use crate::transport::ZeekTransport;

/// Zeek network detection provider over a bound transport.
#[derive(Debug)]
pub struct ZeekNetworkDetectionProvider<T> {
    transport: Option<std::cell::RefCell<T>>,
}

impl<T> ZeekNetworkDetectionProvider<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport: Some(std::cell::RefCell::new(transport)),
        }
    }

    pub fn unbound() -> Self {
        Self { transport: None }
    }
}

impl<T: ZeekTransport> NetworkDetectionProvider for ZeekNetworkDetectionProvider<T> {
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
                "zeek network detection provider has no transport bound",
            ));
        };
        let notices = transport
            .borrow_mut()
            .read_notices()
            .map_err(|_| SentinelError::unavailable("zeek transport read failed"))?;
        let mut events = Vec::new();
        for notice in notices {
            let Some((kind, severity)) = classify(&notice.note) else {
                // Observed but not classified: never fabricate a
                // canonical finding for an unknown note class.
                continue;
            };
            let event_id = SecurityEventId::new(format!("zeek-{}", notice.uid)).map_err(|_| {
                SentinelError::new(
                    SentinelErrorCode::Validation,
                    "zeek notice uid cannot form event id",
                    None,
                    None,
                    None,
                    None,
                )
            })?;
            let evidence = format!("zeek:{}:{}", notice.note, notice.uid);
            let mut event = SecurityEvent::new(
                event_id,
                tenant_id.clone(),
                AdvancedSensorProfile::Zeek,
                kind,
                severity,
                evidence,
                notice.observed_at.clone(),
            );
            if let Some(orig) = &notice.orig_h {
                event = event.with_correlation(format!("src={orig}"));
            }
            event = event.with_state(AlertState::Open);
            events.push(event);
        }
        Ok(events)
    }
}

/// Classify a documented Zeek notice identifier into a canonical
/// sentinel finding. Unknown note classes return None (observed but
/// never fabricated).
fn classify(note: &str) -> Option<(FindingKind, FindingSeverity)> {
    if note.starts_with("Scan::") {
        Some((FindingKind::ScanDetected, FindingSeverity::Medium))
    } else if note.starts_with("Weird::") {
        Some((FindingKind::BaselineViolation, FindingSeverity::Low))
    } else if note.starts_with("DNS::") {
        Some((FindingKind::DnsAnomaly, FindingSeverity::Low))
    } else {
        None
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

    #[test]
    fn ep031_unit_zeek_unbound_fails_closed() {
        let p = ZeekNetworkDetectionProvider::<()>::unbound();
        assert!(p.capabilities().is_empty());
        let err = p.read_events(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
    }

    #[test]
    fn ep031_unit_zeek_classifies_documented_notices_only() {
        // Scan/Weird/DNS classes map to canonical findings; an
        // unknown note is observed but never fabricated.
        assert_eq!(
            classify("Scan::Port_Scan"),
            Some((FindingKind::ScanDetected, FindingSeverity::Medium))
        );
        assert_eq!(
            classify("Weird::TCP_No_Data"),
            Some((FindingKind::BaselineViolation, FindingSeverity::Low))
        );
        assert_eq!(
            classify("DNS::Suspicious_Query"),
            Some((FindingKind::DnsAnomaly, FindingSeverity::Low))
        );
        assert_eq!(classify("SSL::Invalid_Server_Cert"), None);
    }

    #[test]
    fn ep031_unit_zeek_normalizes_notices_to_events() {
        let json = r#"{"ts":1755650000.5,"uid":"C1","id.orig_h":"192.0.2.10","id.orig_p":54321,"id.resp_h":"198.51.100.7","id.resp_p":80,"proto":"tcp","note":"Scan::Port_Scan","msg":"Port scan detected","src":"192.0.2.10","dst":"198.51.100.7","p":80,"n":42,"actions":["Notice::ACTION_LOG"],"dropped":false}
{"ts":1755650001.0,"uid":"C2","note":"SSL::Invalid_Server_Cert","msg":"unclassified observed"}"#;
        let transport = crate::transport::JsonLinesZeekTransport::new(json.as_bytes());
        let p = ZeekNetworkDetectionProvider::new(transport);
        let caps = p.capabilities();
        assert!(caps.contains(SentinelCapabilityKind::ReadFindings));
        let events = p.read_events(&tenant()).unwrap();
        // Only the documented Scan class is classified; the SSL note
        // is observed but not fabricated into a finding.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].profile, AdvancedSensorProfile::Zeek);
        assert_eq!(events[0].kind, FindingKind::ScanDetected);
        assert_eq!(events[0].severity, FindingSeverity::Medium);
        assert_eq!(events[0].evidence_ref, "zeek:Scan::Port_Scan:C1");
        assert_eq!(events[0].correlation.as_deref(), Some("src=192.0.2.10"));
    }
}
