//! EP-031 osquery endpoint telemetry adapter (M5).
//!
//! Implements the nexus-sentinel-advanced `EndpointTelemetryProvider`
//! port over the documented osquery TLS remote API (self-hosted
//! collector). Observed distributed query results are OBSERVED data;
//! normalization maps ONLY the documented owned rule: a wildcard
//! listening socket (address `0.0.0.0` or `::`) observed in the
//! documented osquery `listening_ports` table is a baseline
//! violation. Non-wildcard rows are observed telemetry but are never
//! fabricated into canonical findings. A non-zero query status is an
//! OBSERVED query execution failure and fails closed.
//!
//! Capabilities advertise ReadFindings ONLY when the transport is
//! bound (Reality rule). Unbound providers fail closed. Every public
//! operation records a bounded redacted audit entry with correlation.

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
use crate::transport::OsqueryTransport;

/// A normalized osquery telemetry observation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OsqueryQueryResult {
    /// Query id (documented distributed_write queries key).
    pub query_id: String,
    /// Row count observed.
    pub row_count: usize,
    /// Number of rows that mapped to a canonical finding.
    pub finding_count: usize,
    /// Query execution status (0 = success; non-0 = observed failure).
    pub status: i64,
}

/// osquery endpoint telemetry provider over a bound transport.
#[derive(Debug)]
pub struct OsqueryEndpointTelemetryProvider<T> {
    transport: Option<RefCell<T>>,
    observability: SentinelObservability,
}

impl<T> OsqueryEndpointTelemetryProvider<T> {
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

impl<T: OsqueryTransport> EndpointTelemetryProvider for OsqueryEndpointTelemetryProvider<T> {
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
                "osquery.read_telemetry",
                "denied",
                "no transport bound",
                tenant_id.as_str(),
            ));
            return Err(SentinelError::unavailable(
                "osquery endpoint telemetry provider has no transport bound",
            ));
        };
        let mut transport = transport.borrow_mut();
        let observed = transport.drain_observed();
        let mut events = Vec::new();
        let mut results = Vec::new();
        for result in observed {
            if result.status != 0 {
                // OBSERVED query execution failure: fail closed, never
                // fabricate rows for a failed query.
                self.observability.record(SentinelAuditEntry::new(
                    "osquery.read_telemetry",
                    "failed",
                    format!("query {} status {}", result.query_id, result.status),
                    tenant_id.as_str(),
                ));
                return Err(SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    format!(
                        "osquery distributed query {} failed (status {})",
                        result.query_id, result.status
                    ),
                    None,
                    None,
                    None,
                    None,
                ));
            }
            let mut finding_count = 0usize;
            for (i, row) in result.rows.iter().enumerate() {
                let Some(event) = normalize_row(tenant_id, &result.query_id, i, row)? else {
                    continue;
                };
                finding_count += 1;
                events.push(event);
            }
            results.push(OsqueryQueryResult {
                query_id: result.query_id.clone(),
                row_count: result.rows.len(),
                finding_count,
                status: result.status,
            });
        }
        self.observability.record(SentinelAuditEntry::new(
            "osquery.read_telemetry",
            "ok",
            format!(
                "{} rows observed across {} queries",
                results.iter().map(|r| r.row_count).sum::<usize>(),
                results.len()
            ),
            tenant_id.as_str(),
        ));
        Ok(events)
    }
}

/// Normalize ONE observed osquery row into a canonical finding, or
/// None when the row is observed telemetry that does not match an
/// owned rule (never fabricated).
fn normalize_row(
    tenant_id: &TenantId,
    query_id: &str,
    index: usize,
    row: &serde_json::Value,
) -> Result<Option<SecurityEvent>, SentinelError> {
    // The connector owns ONE documented rule: a wildcard listening
    // socket (address 0.0.0.0 / ::) observed in the documented
    // osquery listening_ports table is a baseline violation.
    if query_id != "listening_ports" {
        return Ok(None);
    }
    let address = row
        .get("address")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if address != "0.0.0.0" && address != "::" {
        return Ok(None);
    }
    let port = row.get("port").and_then(|v| v.as_str()).unwrap_or("?");
    let protocol = row
        .get("protocol")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let event_id = SecurityEventId::new(format!("osquery-{query_id}-{index}")).map_err(|_| {
        SentinelError::new(
            SentinelErrorCode::Validation,
            "osquery row cannot form event id",
            None,
            None,
            None,
            None,
        )
    })?;
    let event = SecurityEvent::new(
        event_id,
        tenant_id.clone(),
        AdvancedSensorProfile::Osquery,
        FindingKind::BaselineViolation,
        FindingSeverity::Low,
        format!("osquery:{query_id}:{port}:{protocol}"),
        "2026-08-20T00:00:00Z",
    )
    .with_correlation(format!("address={address}:port={port}"))
    .with_state(AlertState::Open);
    Ok(Some(event))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::TenantId;
    use std::collections::HashMap;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    #[test]
    fn ep031_unit_osquery_unbound_fails_closed_with_audit() {
        let p = OsqueryEndpointTelemetryProvider::<()>::unbound();
        assert!(p.capabilities().is_empty());
        let err = p.read_telemetry(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
        let entries = p.audit_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].outcome, "denied");
    }

    #[test]
    fn ep031_unit_osquery_wildcard_listener_normalized_with_audit() {
        use crate::transport::{DistributedQuery, HttpOsqueryEndpoint};
        let ep = HttpOsqueryEndpoint::new(
            "ep031-secret",
            vec![DistributedQuery {
                id: "listening_ports".to_string(),
                query: "SELECT * FROM listening_ports;".to_string(),
            }],
        );
        let mut ep2 = ep.clone();
        let key = ep2.enroll("ep031-secret", "host-1").unwrap();
        let mut queries = HashMap::new();
        queries.insert(
            "listening_ports".to_string(),
            vec![
                serde_json::json!({"address": "0.0.0.0", "port": "8443", "protocol": "tcp"}),
                serde_json::json!({"address": "192.168.1.5", "port": "22", "protocol": "tcp"}),
            ],
        );
        let mut statuses = HashMap::new();
        statuses.insert("listening_ports".to_string(), 0);
        ep2.distributed_write(&key, &queries, &statuses).unwrap();

        let p = OsqueryEndpointTelemetryProvider::new(ep2);
        let events = p.read_telemetry(&tenant()).unwrap();
        // ONLY the wildcard row becomes a finding; the private address
        // row is observed but never fabricated.
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].profile, AdvancedSensorProfile::Osquery);
        assert_eq!(events[0].kind, FindingKind::BaselineViolation);
        assert_eq!(events[0].severity, FindingSeverity::Low);
        assert!(events[0].evidence_ref.contains("8443"));
        assert_eq!(events[0].state, AlertState::Open);
        let entries = p.audit_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].outcome, "ok");
    }

    #[test]
    fn ep031_unit_osquery_clean_telemetry_is_observed_not_fabricated() {
        use crate::transport::{DistributedQuery, HttpOsqueryEndpoint};
        let ep = HttpOsqueryEndpoint::new(
            "ep031-secret",
            vec![DistributedQuery {
                id: "listening_ports".to_string(),
                query: "SELECT * FROM listening_ports;".to_string(),
            }],
        );
        let mut ep2 = ep.clone();
        let key = ep2.enroll("ep031-secret", "host-1").unwrap();
        let mut queries = HashMap::new();
        queries.insert(
            "listening_ports".to_string(),
            vec![serde_json::json!({"address": "127.0.0.1", "port": "22", "protocol": "tcp"})],
        );
        let mut statuses = HashMap::new();
        statuses.insert("listening_ports".to_string(), 0);
        ep2.distributed_write(&key, &queries, &statuses).unwrap();

        let p = OsqueryEndpointTelemetryProvider::new(ep2);
        let events = p.read_telemetry(&tenant()).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn ep031_unit_osquery_query_failure_fails_closed() {
        use crate::transport::{DistributedQuery, HttpOsqueryEndpoint};
        let ep = HttpOsqueryEndpoint::new(
            "ep031-secret",
            vec![DistributedQuery {
                id: "listening_ports".to_string(),
                query: "SELECT * FROM listening_ports;".to_string(),
            }],
        );
        let mut ep2 = ep.clone();
        let key = ep2.enroll("ep031-secret", "host-1").unwrap();
        let mut queries = HashMap::new();
        queries.insert("listening_ports".to_string(), Vec::new());
        let mut statuses = HashMap::new();
        statuses.insert("listening_ports".to_string(), 2); // documented SQLite error code
        ep2.distributed_write(&key, &queries, &statuses).unwrap();

        let p = OsqueryEndpointTelemetryProvider::new(ep2);
        let err = p.read_telemetry(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
    }
}
