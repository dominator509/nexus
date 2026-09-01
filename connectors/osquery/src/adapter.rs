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
    /// Durable endpoint identity of the reporting node (AUD-035).
    pub host_identifier: String,
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
            // AUD-035: telemetry without a durable endpoint identity is
            // unattributable - fail closed rather than normalize a
            // finding that cannot be traced to an endpoint.
            if result.host_identifier.trim().is_empty() {
                self.observability.record(SentinelAuditEntry::new(
                    "osquery.read_telemetry",
                    "failed",
                    format!(
                        "observed telemetry for query {} without durable endpoint identity",
                        result.query_id
                    ),
                    tenant_id.as_str(),
                ));
                return Err(SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    format!(
                        "osquery observed telemetry for query {} has no durable endpoint identity",
                        result.query_id
                    ),
                    None,
                    None,
                    None,
                    None,
                ));
            }
            let mut finding_count = 0usize;
            for (i, row) in result.rows.iter().enumerate() {
                let event = match normalize_row(
                    tenant_id,
                    &result.host_identifier,
                    &result.query_id,
                    result.observed_at,
                    result.batch_seq,
                    i,
                    row,
                ) {
                    Ok(Some(event)) => event,
                    Ok(None) => continue,
                    Err(err) => {
                        // Fail-closed normalization is AUDITED like
                        // every other denial (never silent).
                        self.observability.record(SentinelAuditEntry::new(
                            "osquery.read_telemetry",
                            "failed",
                            format!("row normalization failed: {}", err.message),
                            tenant_id.as_str(),
                        ));
                        return Err(err);
                    }
                };
                finding_count += 1;
                events.push(event);
            }
            results.push(OsqueryQueryResult {
                host_identifier: result.host_identifier.clone(),
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
/// owned rule (never fabricated). `host_identifier` is the durable
/// endpoint identity of the reporting node (AUD-035): every normalized
/// finding is attributable to a specific endpoint. `observed_at` is
/// the REAL observation time stamped by the collector at write
/// receipt (AUD-037) - a finding NEVER carries a fabricated time, and
/// telemetry without a stamped observation time fails closed. The
/// event id binds host + query + batch sequence + row index so it can
/// never collide across batches or endpoints (AUD-037).
fn normalize_row(
    tenant_id: &TenantId,
    host_identifier: &str,
    query_id: &str,
    observed_at: i64,
    batch_seq: u64,
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
    // AUD-037: a finding must carry the REAL observation time. If the
    // transport never stamped one (0), normalization fails closed -
    // fabricating a timestamp would repeat the audited lie.
    if observed_at <= 0 {
        return Err(SentinelError::new(
            SentinelErrorCode::ExternalProvider,
            format!(
                "osquery observed telemetry for query {query_id} has no stamped observation time"
            ),
            None,
            None,
            None,
            None,
        ));
    }
    let port = row.get("port").and_then(|v| v.as_str()).unwrap_or("?");
    let protocol = row
        .get("protocol")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    // AUD-037: collision-proof event id. The previous osquery-{query}-
    // {index} id collided across batches (same query, same row index)
    // and across endpoints (same query, same index, different host).
    // Host + query + batch sequence + row index is unique per
    // observation.
    let event_id = SecurityEventId::new(format!(
        "osquery-{host_identifier}-{query_id}-{batch_seq}-{index}"
    ))
    .map_err(|_| {
        SentinelError::new(
            SentinelErrorCode::Validation,
            "osquery row cannot form event id",
            None,
            None,
            None,
            None,
        )
    })?;
    // AUD-037: the observed_at field is the collector's REAL stamped
    // observation time rendered to RFC3339 UTC - never a fabricated
    // constant.
    let observed_at_rfc3339 = format!("{}Z", epoch_seconds_to_rfc3339(observed_at));
    // AUD-035: the evidence reference and correlation BOTH carry the
    // durable endpoint identity so a finding can always be traced to
    // the endpoint that reported it.
    let event = SecurityEvent::new(
        event_id,
        tenant_id.clone(),
        AdvancedSensorProfile::Osquery,
        FindingKind::BaselineViolation,
        FindingSeverity::Low,
        format!("osquery:{host_identifier}:{query_id}:{port}:{protocol}"),
        observed_at_rfc3339,
    )
    .with_correlation(format!(
        "host={host_identifier}:address={address}:port={port}"
    ))
    .with_state(AlertState::Open);
    Ok(Some(event))
}

/// Minimal RFC3339 UTC rendering of epoch seconds (AUD-037). The
/// month MUST NOT shadow the minute variable (AUD-034 reference): the
/// month binds to `mo`, the minute stays `m`, so the RFC3339 minute
/// field always carries the true minute.
fn epoch_seconds_to_rfc3339(secs: i64) -> String {
    // Days-from-epoch civil calendar (Howard Hinnant's algorithm).
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::ObservedOsqueryResult;
    use nexus_domain::TenantId;
    use std::collections::HashMap;
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    /// Stub transport that returns crafted observed results, used to
    /// prove the adapter fails closed on telemetry WITHOUT a durable
    /// endpoint identity (AUD-035).
    #[derive(Debug, Default)]
    struct StubObservedTransport {
        observed: Vec<ObservedOsqueryResult>,
    }

    impl OsqueryTransport for StubObservedTransport {
        fn drain_observed(&mut self) -> Vec<ObservedOsqueryResult> {
            std::mem::take(&mut self.observed)
        }
    }

    #[test]
    fn ep031_unit_osquery_unattributable_telemetry_fails_closed() {
        // AUD-035: a result with NO durable endpoint identity is
        // unattributable - normalization must fail closed (External),
        // never mint a finding that cannot be traced to an endpoint.
        let transport = StubObservedTransport {
            observed: vec![ObservedOsqueryResult {
                host_identifier: String::new(),
                query_id: "listening_ports".to_string(),
                rows: vec![serde_json::json!({
                    "address": "0.0.0.0", "port": "8443", "protocol": "tcp"
                })],
                status: 0,
                observed_at: 1_720_000_000,
                batch_seq: 1,
            }],
        };
        let p = OsqueryEndpointTelemetryProvider::new(transport);
        let err = p.read_telemetry(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
        assert!(
            p.audit_entries().iter().any(|e| e.outcome == "failed"),
            "denial recorded"
        );
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
        // AUD-035: the finding is attributable to the durable endpoint
        // identity bound at enrollment.
        assert!(events[0].evidence_ref.contains("host-1"));
        assert_eq!(
            events[0].correlation.as_deref(),
            Some("host=host-1:address=0.0.0.0:port=8443")
        );
        assert_eq!(events[0].state, AlertState::Open);
        // AUD-037: the finding's observed_at is the collector's REAL
        // stamped observation time - never the fabricated constant
        // "2026-08-20T00:00:00Z" - and the event id is collision-proof.
        assert_ne!(events[0].observed_at, "2026-08-20T00:00:00Z");
        assert!(
            events[0].observed_at.ends_with('Z'),
            "observed_at is RFC3339 UTC"
        );
        assert!(
            events[0].event_id.as_str().contains("host-1"),
            "event id binds the durable endpoint identity"
        );
        assert!(
            events[0].event_id.as_str().contains("listening_ports"),
            "event id binds the query id"
        );
        let entries = p.audit_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].outcome, "ok");
    }

    #[test]
    fn ep031_unit_osquery_observed_at_is_real_stamped_time() {
        // AUD-037: the RFC3339 observed_at must equal the collector's
        // REAL stamped seconds rendered to UTC - never a fabricated
        // constant. A fixed stamp is used so the assertion is exact.
        let transport = StubObservedTransport {
            observed: vec![ObservedOsqueryResult {
                host_identifier: "host-1".to_string(),
                query_id: "listening_ports".to_string(),
                rows: vec![serde_json::json!({
                    "address": "0.0.0.0", "port": "8443", "protocol": "tcp"
                })],
                status: 0,
                observed_at: 1_720_000_000,
                batch_seq: 7,
            }],
        };
        let p = OsqueryEndpointTelemetryProvider::new(transport);
        let events = p.read_telemetry(&tenant()).unwrap();
        assert_eq!(events.len(), 1);
        // 1720000000 = 2024-07-03T09:46:40Z (Python-confirmed).
        assert_eq!(events[0].observed_at, "2024-07-03T09:46:40Z");
        assert_eq!(
            events[0].event_id.as_str(),
            "osquery-host-1-listening_ports-7-0"
        );
    }

    #[test]
    fn ep031_unit_osquery_two_batches_same_host_no_id_collision() {
        // AUD-037: two distributed_write batches from the SAME host
        // with the SAME query and the SAME row shape must mint
        // DISTINCT event ids - the previous osquery-{query}-{index}
        // id collided because the row index repeats every batch.
        let transport = StubObservedTransport {
            observed: vec![
                ObservedOsqueryResult {
                    host_identifier: "host-1".to_string(),
                    query_id: "listening_ports".to_string(),
                    rows: vec![serde_json::json!({
                        "address": "0.0.0.0", "port": "8443", "protocol": "tcp"
                    })],
                    status: 0,
                    observed_at: 1_720_000_000,
                    batch_seq: 1,
                },
                ObservedOsqueryResult {
                    host_identifier: "host-1".to_string(),
                    query_id: "listening_ports".to_string(),
                    rows: vec![serde_json::json!({
                        "address": "0.0.0.0", "port": "8443", "protocol": "tcp"
                    })],
                    status: 0,
                    observed_at: 1_720_000_100,
                    batch_seq: 2,
                },
            ],
        };
        let p = OsqueryEndpointTelemetryProvider::new(transport);
        let events = p.read_telemetry(&tenant()).unwrap();
        assert_eq!(events.len(), 2);
        assert_ne!(events[0].event_id, events[1].event_id);
        assert!(
            events[0].event_id.as_str().contains("-1-0"),
            "first batch sequence bound"
        );
        assert!(
            events[1].event_id.as_str().contains("-2-0"),
            "second batch sequence bound"
        );
    }

    #[test]
    fn ep031_unit_osquery_two_hosts_no_id_collision() {
        // AUD-037: the SAME query and row index from DIFFERENT hosts
        // must mint DISTINCT event ids - the previous id had no host
        // component, so two endpoints produced identical ids.
        let transport = StubObservedTransport {
            observed: vec![
                ObservedOsqueryResult {
                    host_identifier: "host-1".to_string(),
                    query_id: "listening_ports".to_string(),
                    rows: vec![serde_json::json!({
                        "address": "0.0.0.0", "port": "8443", "protocol": "tcp"
                    })],
                    status: 0,
                    observed_at: 1_720_000_000,
                    batch_seq: 1,
                },
                ObservedOsqueryResult {
                    host_identifier: "host-2".to_string(),
                    query_id: "listening_ports".to_string(),
                    rows: vec![serde_json::json!({
                        "address": "0.0.0.0", "port": "8443", "protocol": "tcp"
                    })],
                    status: 0,
                    observed_at: 1_720_000_000,
                    batch_seq: 1,
                },
            ],
        };
        let p = OsqueryEndpointTelemetryProvider::new(transport);
        let events = p.read_telemetry(&tenant()).unwrap();
        assert_eq!(events.len(), 2);
        assert_ne!(events[0].event_id, events[1].event_id);
        assert!(events[0].event_id.as_str().contains("host-1"));
        assert!(events[1].event_id.as_str().contains("host-2"));
    }

    #[test]
    fn ep031_unit_osquery_unstamped_observation_time_fails_closed() {
        // AUD-037: telemetry WITHOUT a stamped observation time must
        // fail closed (ExternalProvider) - the adapter never fabricates
        // a timestamp to replace the missing observation time.
        let transport = StubObservedTransport {
            observed: vec![ObservedOsqueryResult {
                host_identifier: "host-1".to_string(),
                query_id: "listening_ports".to_string(),
                rows: vec![serde_json::json!({
                    "address": "0.0.0.0", "port": "8443", "protocol": "tcp"
                })],
                status: 0,
                observed_at: 0,
                batch_seq: 1,
            }],
        };
        let p = OsqueryEndpointTelemetryProvider::new(transport);
        let err = p.read_telemetry(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
        assert!(
            p.audit_entries().iter().any(|e| e.outcome == "failed"),
            "denial recorded"
        );
    }

    #[test]
    fn ep031_unit_epoch_seconds_to_rfc3339_never_shadows_minute() {
        // AUD-037 + AUD-034 reference: the RFC3339 minute field must
        // carry the TRUE minute. 1755650000 = 2025-08-20T00:33:20Z
        // (August) - a month/minute shadow would render 00:08:20.
        assert_eq!(
            epoch_seconds_to_rfc3339(1_755_650_000),
            "2025-08-20T00:33:20"
        );
        assert_eq!(
            epoch_seconds_to_rfc3339(1_755_650_099),
            "2025-08-20T00:34:59"
        );
        // Late-year sample: 2025-12-31T23:59:59Z.
        assert_eq!(
            epoch_seconds_to_rfc3339(1_767_225_599),
            "2025-12-31T23:59:59"
        );
        // Epoch 0 reference.
        assert_eq!(epoch_seconds_to_rfc3339(0), "1970-01-01T00:00:00");
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
