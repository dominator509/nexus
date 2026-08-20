//! EP-030 AdGuard Home adapter core (SPEC-013; M4).
//!
//! Real production adapter behind the nexus-sentinel
//! `DnsSecurityProvider` port: capability advertisement only when the
//! transport answers, DNS security telemetry derived from the
//! documented query log (OBSERVED data, never fabricated), bounded
//! observability, and fail-closed behavior.
//!
//! Permanent invariants (SPEC-013):
//!
//! - AdGuard Home supplies DNS security and telemetry (acceptance
//!   obligation 2) through the DnsSecurityProvider port.
//! - TELEMETRY IS OBSERVED DATA: total/blocked queries are counted
//!   from the documented query log; an unreachable sidecar reports
//!   Unavailable, never a fabricated zero.
//! - BLOCKLIST STATE IS OBSERVED: entries are derived from observed
//!   FilteredBlackList reasons; unknown filtering reasons are
//!   normalized at the boundary and never widen the contract.
//! - UNBOUND PROVIDERS FAIL CLOSED (Reality rule): no session is
//!   fabricated and no capability is advertised.
//!
//! No test-mode branches exist in production code.

use std::sync::Mutex;

use nexus_domain::TenantId;
use nexus_sentinel::{
    DnsBlocklistEntry, DnsSecurityProvider, DnsTelemetry, SentinelCapabilityKind,
    SentinelCapabilityMap, SentinelError,
};

use crate::observability::{SentinelAuditEntry, SentinelObservability};
use crate::transport::{AdGuardTransport, QueryLogEntry};

/// Real production AdGuard Home adapter over a real AdGuard transport.
pub struct AdGuardDnsSecurityProvider {
    transport: Box<dyn AdGuardTransport + Send + Sync>,
    tenant_id: TenantId,
    observability: Mutex<SentinelObservability>,
}

impl AdGuardDnsSecurityProvider {
    pub fn new(
        transport: Box<dyn AdGuardTransport + Send + Sync>,
        tenant_id: TenantId,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        let username = username.into();
        let password = password.into();
        // Credentials are registered as redaction secrets so a
        // poisoned error can never leak them into the audit ring. The
        // transport holds the credentials for the Basic auth header.
        Self {
            transport,
            tenant_id,
            observability: Mutex::new(SentinelObservability::new(256, vec![username, password])),
        }
    }

    pub fn audit(&self) -> Vec<SentinelAuditEntry> {
        self.observability.lock().unwrap().audit()
    }

    fn record(
        &self,
        correlation: &str,
        operation: &str,
        outcome: &str,
        detail: String,
        fields: std::collections::BTreeMap<String, String>,
    ) {
        self.observability
            .lock()
            .unwrap()
            .record(SentinelAuditEntry {
                correlation: correlation.to_string(),
                operation: operation.to_string(),
                outcome: outcome.to_string(),
                detail,
                fields,
            });
    }

    fn correlation(&self) -> String {
        self.observability.lock().unwrap().next_correlation()
    }

    /// Count blocked entries: a query is blocked when its documented
    /// FilteringReason is one of the Filtered* classes (observed data).
    fn is_blocked(entry: &QueryLogEntry) -> bool {
        entry.reason.starts_with("Filtered")
    }
}

impl DnsSecurityProvider for AdGuardDnsSecurityProvider {
    fn capabilities(&self) -> SentinelCapabilityMap {
        // Advertise only when the transport answers (reality rule).
        // An unbound or failing transport advertises nothing.
        let mut map = SentinelCapabilityMap::new();
        if self.transport.status().is_ok() {
            map.insert(SentinelCapabilityKind::ReadDnsTelemetry);
            map.insert(SentinelCapabilityKind::ReadDnsBlocklist);
        }
        map
    }

    fn read_telemetry(&self, tenant_id: &TenantId) -> Result<DnsTelemetry, SentinelError> {
        let correlation = self.correlation();
        // Documented GET /control/querylog?limit=100 (page bound).
        // Telemetry is counted from OBSERVED entries; an empty log is
        // an empty window, never a fabricated baseline.
        let entries = self.transport.query_log(100, "").map_err(|e| {
            self.record(
                &correlation,
                "READ_TELEMETRY",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
        })?;
        let total = entries.len() as u64;
        let blocked = entries.iter().filter(|e| Self::is_blocked(e)).count() as u64;
        let window_start = entries.first().map(|e| e.time.clone()).unwrap_or_default();
        let window_end = entries.last().map(|e| e.time.clone()).unwrap_or_default();
        self.record(
            &correlation,
            "READ_TELEMETRY",
            "ok",
            format!("{total} queries, {blocked} blocked"),
            std::collections::BTreeMap::new(),
        );
        Ok(DnsTelemetry::new(
            tenant_id.clone(),
            total,
            blocked,
            window_start,
            window_end,
        ))
    }

    fn read_blocklist(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<DnsBlocklistEntry>, SentinelError> {
        let correlation = self.correlation();
        // Blocklist state is OBSERVED from the query log: domains that
        // were actually blocked (FilteredBlackList reason) are active
        // blocklist entries. This never fabricates a blocklist; it
        // reports what the sidecar demonstrably blocked.
        let entries = self.transport.query_log(100, "").map_err(|e| {
            self.record(
                &correlation,
                "READ_BLOCKLIST",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
        })?;
        let mut seen = std::collections::BTreeMap::<String, String>::new();
        for e in entries.iter() {
            if e.reason == "FilteredBlackList" && !e.question.is_empty() {
                // Keep the latest observed time per domain.
                seen.insert(e.question.clone(), e.time.clone());
            }
        }
        let mut out = Vec::new();
        for (domain, updated) in seen {
            out.push(DnsBlocklistEntry::new(
                tenant_id.clone(),
                domain,
                true,
                updated,
            ));
        }
        self.record(
            &correlation,
            "READ_BLOCKLIST",
            "ok",
            format!("{} observed blocked domains", out.len()),
            std::collections::BTreeMap::new(),
        );
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{AdGuardStatus, QueryLogEntry};
    use nexus_sentinel::SentinelErrorCode;
    use std::str::FromStr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    #[derive(Clone, Default)]
    struct CountingTransport {
        status_calls: Arc<AtomicUsize>,
        query_calls: Arc<AtomicUsize>,
        entries: Arc<Mutex<Vec<QueryLogEntry>>>,
        fail_status: bool,
    }

    impl CountingTransport {
        fn with_entries(entries: Vec<QueryLogEntry>) -> Self {
            Self {
                entries: Arc::new(Mutex::new(entries)),
                ..Default::default()
            }
        }
    }

    impl AdGuardTransport for CountingTransport {
        fn status(&self) -> Result<AdGuardStatus, SentinelError> {
            self.status_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_status {
                return Err(SentinelError::new(
                    SentinelErrorCode::Unavailable,
                    "fixture status failed",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Ok(AdGuardStatus {
                dns_addresses: vec!["127.0.0.1".into()],
                dns_port: 53,
                http_port: 80,
                protection_enabled: true,
                running: true,
                version: "v0.108.0".into(),
            })
        }

        fn query_log(
            &self,
            _limit: usize,
            _search: &str,
        ) -> Result<Vec<QueryLogEntry>, SentinelError> {
            self.query_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.entries.lock().unwrap().clone())
        }
    }

    fn entry(time: &str, question: &str, reason: &str) -> QueryLogEntry {
        QueryLogEntry {
            time: time.into(),
            question: question.into(),
            client: "192.0.2.10".into(),
            reason: reason.into(),
        }
    }

    #[test]
    fn ep030_unit_capabilities_fail_closed_when_transport_unavailable() {
        let t = CountingTransport::default();
        let provider =
            AdGuardDnsSecurityProvider::new(Box::new(t.clone()), tenant(), "admin", "pass");
        let caps = provider.capabilities();
        assert!(caps.contains(SentinelCapabilityKind::ReadDnsTelemetry));

        let failing = CountingTransport {
            fail_status: true,
            ..Default::default()
        };
        let provider =
            AdGuardDnsSecurityProvider::new(Box::new(failing.clone()), tenant(), "admin", "bad");
        assert!(provider.capabilities().is_empty());
    }

    #[test]
    fn ep030_unit_telemetry_counts_observed_queries() {
        let t = CountingTransport::with_entries(vec![
            entry(
                "2026-08-20T00:00:00Z",
                "good.example.com",
                "NotFilteredNotFound",
            ),
            entry(
                "2026-08-20T00:00:01Z",
                "ads.example.com",
                "FilteredBlackList",
            ),
            entry(
                "2026-08-20T00:00:02Z",
                "tracker.example.net",
                "FilteredBlackList",
            ),
            entry(
                "2026-08-20T00:00:03Z",
                "safe.example.org",
                "NotFilteredWhiteList",
            ),
        ]);
        let provider =
            AdGuardDnsSecurityProvider::new(Box::new(t.clone()), tenant(), "admin", "pass");
        let telemetry = provider.read_telemetry(&tenant()).unwrap();
        assert_eq!(telemetry.total_queries, 4);
        assert_eq!(telemetry.blocked_queries, 2);
        assert_eq!(telemetry.blocked_ratio(), 0.5);
        assert_eq!(telemetry.window_start, "2026-08-20T00:00:00Z");
        assert_eq!(telemetry.window_end, "2026-08-20T00:00:03Z");
    }

    #[test]
    fn ep030_unit_telemetry_empty_log_is_empty_window() {
        let t = CountingTransport::with_entries(vec![]);
        let provider =
            AdGuardDnsSecurityProvider::new(Box::new(t.clone()), tenant(), "admin", "pass");
        let telemetry = provider.read_telemetry(&tenant()).unwrap();
        assert_eq!(telemetry.total_queries, 0);
        assert_eq!(telemetry.blocked_queries, 0);
        assert_eq!(telemetry.blocked_ratio(), 0.0);
    }

    #[test]
    fn ep030_unit_blocklist_is_observed_not_fabricated() {
        let t = CountingTransport::with_entries(vec![
            entry(
                "2026-08-20T00:00:00Z",
                "ads.example.com",
                "FilteredBlackList",
            ),
            entry(
                "2026-08-20T00:00:01Z",
                "tracker.example.net",
                "FilteredBlackList",
            ),
            // A rewrite or whitelist entry is NOT a blocklist entry.
            entry("2026-08-20T00:00:02Z", "rewrite.example.org", "Rewrite"),
            entry(
                "2026-08-20T00:00:03Z",
                "good.example.com",
                "NotFilteredNotFound",
            ),
        ]);
        let provider =
            AdGuardDnsSecurityProvider::new(Box::new(t.clone()), tenant(), "admin", "pass");
        let blocklist = provider.read_blocklist(&tenant()).unwrap();
        assert_eq!(blocklist.len(), 2);
        assert!(blocklist.iter().all(|e| e.active));
        assert!(blocklist.iter().any(|e| e.domain_ref == "ads.example.com"));
        assert!(!blocklist
            .iter()
            .any(|e| e.domain_ref == "rewrite.example.org"));
    }

    #[test]
    fn ep030_unit_redaction_canary_zero_leakage() {
        let t = CountingTransport::default();
        let provider = AdGuardDnsSecurityProvider::new(
            Box::new(t.clone()),
            tenant(),
            "sekret-user",
            "sekret-pass",
        );
        let _ = provider.capabilities();
        provider.record(
            &provider.correlation(),
            "POISON",
            "ok",
            "user=sekret-user pass=sekret-pass".into(),
            std::collections::BTreeMap::from([
                ("u".into(), "sekret-user".into()),
                ("p".into(), "sekret-pass".into()),
            ]),
        );
        let audit = provider.audit();
        let joined = serde_json::to_string(&audit).unwrap();
        assert!(!joined.contains("sekret-user"));
        assert!(!joined.contains("sekret-pass"));
    }

    #[test]
    fn ep030_unit_telemetry_failure_is_audited_with_correlation() {
        struct FailingTransport;
        impl AdGuardTransport for FailingTransport {
            fn status(&self) -> Result<AdGuardStatus, SentinelError> {
                Err(SentinelError::unavailable("fixture down"))
            }
            fn query_log(&self, _l: usize, _s: &str) -> Result<Vec<QueryLogEntry>, SentinelError> {
                Err(SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "fixture query log failed",
                    None,
                    None,
                    None,
                    None,
                ))
            }
        }
        let provider =
            AdGuardDnsSecurityProvider::new(Box::new(FailingTransport), tenant(), "admin", "pass");
        let err = provider.read_telemetry(&tenant()).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
        assert!(provider
            .audit()
            .iter()
            .any(|e| e.operation == "READ_TELEMETRY" && e.outcome == "EXTERNAL_PROVIDER"));
    }
}
