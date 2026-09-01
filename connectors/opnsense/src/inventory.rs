//! EP-030 OPNsense network inventory (SPEC-013; AUD-028).
//!
//! Real production `NetworkInventory` implementation backed by the
//! DOCUMENTED OPNsense diagnostics API
//! (`GET /api/diagnostics/interface/getArp`). Devices are DISCOVERED
//! from the router's observed ARP table - never fabricated, never
//! assembled by hand. A device that the router demonstrably sees is a
//! device; a permanent entry (the firewall's own interface) is not a
//! neighbor.
//!
//! Permanent invariants (SPEC-013 / AUD-028):
//!
//! - INVENTORY IS OBSERVED DATA: list_devices enumerates the real ARP
//!   table. Permanent entries (the firewall's own interfaces) and
//!   expired entries are excluded - they are not discovered devices.
//! - FINGERPRINTS ARE OBSERVED DATA: a fingerprint binds the MAC, IP,
//!   and manufacturer the router actually observed. Unknown
//!   fingerprints fail closed (NotFound) - never guessed identity.
//! - BASELINES BEGIN LEARNING: behavior baselines start in LEARNING
//!   and only reach ESTABLISHED from real repeated observation.
//!   Expected destinations/protocols are never fabricated.
//! - UNBOUND PROVIDERS FAIL CLOSED (Reality rule): no session is
//!   fabricated and no capability is advertised.
//!
//! No test-mode branches exist in production code.

use std::collections::BTreeMap;
use std::sync::Mutex;

use nexus_domain::TenantId;
use nexus_sentinel::{
    BaselineId, BehaviorBaseline, DeviceFingerprint, DeviceFingerprintId, NetworkDevice,
    NetworkDeviceId, NetworkInventory, NetworkSegment, SentinelCapabilityKind,
    SentinelCapabilityMap, SentinelError, SentinelErrorCode, TrustClass,
};

use crate::observability::{SentinelAuditEntry, SentinelObservability};
use crate::transport::{OpnsenseArpEntry, OpnsenseTransport};

/// Real production OPNsense network inventory over a real OPNsense
/// transport.
///
/// `Send + Sync`: the transport trait object is required to be
/// shareable so observation state can be proven with real concurrent
/// callers.
pub struct OpnsenseNetworkInventory {
    transport: Box<dyn OpnsenseTransport + Send + Sync>,
    tenant_id: TenantId,
    /// Declared inventory scope: the segment this inventory covers.
    /// The operator declares the scope; the router's ARP table
    /// supplies the observed devices within it. This is never a claim
    /// about an individual device's trust - trust stays Unknown until
    /// classified from evidence.
    segment: NetworkSegment,
    /// device label -> first/last observation timestamps. This is
    /// OBSERVATION state only (never fabricated identity): a device is
    /// only present here after the router reported it in the ARP table.
    observations: Mutex<BTreeMap<String, (String, String)>>,
    observability: Mutex<SentinelObservability>,
}

impl OpnsenseNetworkInventory {
    pub fn new(
        transport: Box<dyn OpnsenseTransport + Send + Sync>,
        tenant_id: TenantId,
        segment: NetworkSegment,
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let api_secret = api_secret.into();
        Self {
            transport,
            tenant_id,
            segment,
            observations: Mutex::new(BTreeMap::new()),
            observability: Mutex::new(SentinelObservability::new(256, vec![api_key, api_secret])),
        }
    }

    fn correlation(&self) -> String {
        format!("opnsense-inventory-{}", std::process::id())
    }

    fn record(
        &self,
        operation: &str,
        outcome: &str,
        message: impl Into<String>,
        context: std::collections::BTreeMap<String, String>,
    ) {
        if let Ok(mut obs) = self.observability.lock() {
            obs.record(SentinelAuditEntry {
                correlation: self.correlation(),
                operation: operation.to_string(),
                outcome: outcome.to_string(),
                detail: message.into(),
                fields: context,
            });
        }
    }

    /// Normalize one observed ARP entry into a discovered device. The
    /// device id binds the OBSERVED MAC; the label prefers the observed
    /// hostname, falling back to the IP. Vendor and hostname are
    /// observed fields, never guessed. The device carries the declared
    /// inventory scope segment; trust stays Unknown until classified.
    fn device_from_arp(
        entry: &OpnsenseArpEntry,
        tenant_id: &TenantId,
        segment: NetworkSegment,
    ) -> NetworkDevice {
        let device_id = NetworkDeviceId::new(format!("mac:{}", entry.mac)).unwrap_or_else(|_| {
            NetworkDeviceId::new(format!("ip:{}", entry.ip)).expect("fallback device id")
        });
        let label = if entry.hostname.is_empty() {
            entry.ip.clone()
        } else {
            entry.hostname.clone()
        };
        let mut device = NetworkDevice::new(
            device_id,
            tenant_id.clone(),
            segment,
            TrustClass::Unknown,
            label,
            "opnsense",
            String::new(),
            String::new(),
        );
        if !entry.manufacturer.is_empty() {
            device = device.with_vendor(entry.manufacturer.clone());
        }
        device
    }
}

impl NetworkInventory for OpnsenseNetworkInventory {
    fn capabilities(&self) -> SentinelCapabilityMap {
        let mut map = SentinelCapabilityMap::new();
        // Reality rule: the inventory advertises only when the
        // transport answers (the ARP table is the inventory source).
        if self.transport.arp_table().is_ok() {
            map.insert(SentinelCapabilityKind::Inventory);
            map.insert(SentinelCapabilityKind::Fingerprint);
            map.insert(SentinelCapabilityKind::Baselines);
        }
        map
    }

    fn list_devices(&self, tenant_id: &TenantId) -> Result<Vec<NetworkDevice>, SentinelError> {
        let correlation = self.correlation();
        let entries = self.transport.arp_table().map_err(|e| {
            self.record(
                "LIST_DEVICES",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
        })?;
        let observed_at = now_rfc3339();
        let mut devices = Vec::new();
        {
            let mut observations = self.observations.lock().unwrap();
            for entry in entries.iter() {
                // Permanent entries are the firewall's own interfaces,
                // not neighbors; expired entries are hosts that stopped
                // answering.
                if entry.permanent || entry.expired {
                    continue;
                }
                let mut device = Self::device_from_arp(entry, tenant_id, self.segment);
                let key = entry.mac.clone();
                let first = observations
                    .get(&key)
                    .map(|(f, _)| f.clone())
                    .unwrap_or_else(|| observed_at.clone());
                observations.insert(key, (first.clone(), observed_at.clone()));
                device.first_seen_at = first;
                device.last_seen_at = observed_at.clone();
                devices.push(device);
            }
        }
        self.record(
            "LIST_DEVICES",
            "ok",
            format!("{} observed devices", devices.len()),
            std::collections::BTreeMap::new(),
        );
        Ok(devices)
    }

    fn fingerprint(&self, device: &NetworkDevice) -> Result<DeviceFingerprint, SentinelError> {
        let correlation = self.correlation();
        let entries = self.transport.arp_table().map_err(|e| {
            self.record(
                "FINGERPRINT",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
        })?;
        // The fingerprint binds the OBSERVED ARP entry matching this
        // device's id (MAC reference). A device the router does not
        // currently see fails closed - identity is never guessed.
        let entry = entries
            .iter()
            .find(|e| {
                !e.permanent && !e.expired && device.device_id.as_str() == format!("mac:{}", e.mac)
            })
            .ok_or_else(|| {
                self.record(
                    "FINGERPRINT",
                    "NOT_FOUND",
                    format!(
                        "device {} not observed in ARP table",
                        device.device_id.as_str()
                    ),
                    std::collections::BTreeMap::new(),
                );
                SentinelError::new(
                    SentinelErrorCode::NotFound,
                    "device not observed in ARP table",
                    Some(correlation.clone()),
                    None,
                    Some(self.tenant_id.to_string()),
                    Some(device.device_id.as_str().to_string()),
                )
            })?;
        let mut fingerprint = DeviceFingerprint::new(
            DeviceFingerprintId::new(format!("fp:{}", entry.mac)).unwrap_or_else(|_| {
                DeviceFingerprintId::new(format!("fp:{}:{}", entry.mac, entry.ip))
                    .expect("fallback fingerprint id")
            }),
            device.device_id.clone(),
            now_rfc3339(),
        )
        .with_mac(entry.mac.clone())
        .with_ip(entry.ip.clone());
        if !entry.manufacturer.is_empty() {
            fingerprint = fingerprint.with_vendor(entry.manufacturer.clone());
        }
        self.record(
            "FINGERPRINT",
            "ok",
            format!("fingerprint observed for {}", entry.ip),
            std::collections::BTreeMap::new(),
        );
        Ok(fingerprint)
    }

    fn baseline(&self, device: &NetworkDevice) -> Result<BehaviorBaseline, SentinelError> {
        let correlation = self.correlation();
        // A baseline exists only for an OBSERVED device. It begins
        // LEARNING; expected destinations/protocols are never
        // fabricated.
        let entries = self.transport.arp_table().map_err(|e| {
            self.record(
                "BASELINE",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
        })?;
        let observed = entries.iter().any(|e| {
            !e.permanent && !e.expired && device.device_id.as_str() == format!("mac:{}", e.mac)
        });
        if !observed {
            self.record(
                "BASELINE",
                "NOT_FOUND",
                format!(
                    "device {} not observed in ARP table",
                    device.device_id.as_str()
                ),
                std::collections::BTreeMap::new(),
            );
            return Err(SentinelError::new(
                SentinelErrorCode::NotFound,
                "device not observed in ARP table",
                Some(correlation.clone()),
                None,
                Some(self.tenant_id.to_string()),
                Some(device.device_id.as_str().to_string()),
            ));
        }
        let observations = self.observations.lock().unwrap();
        let updated_at = observations
            .get(
                device
                    .device_id
                    .as_str()
                    .strip_prefix("mac:")
                    .unwrap_or(device.device_id.as_str()),
            )
            .map(|(_, l)| l.clone())
            .unwrap_or_else(now_rfc3339);
        let baseline = BehaviorBaseline::new(
            BaselineId::new(format!("bl:{}", device.device_id.as_str()))
                .unwrap_or_else(|_| BaselineId::new("bl:fallback").expect("fallback baseline id")),
            device.device_id.clone(),
            self.tenant_id.clone(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            updated_at,
        );
        self.record(
            "BASELINE",
            "ok",
            format!("baseline learning for {}", device.device_id.as_str()),
            std::collections::BTreeMap::new(),
        );
        Ok(baseline)
    }
}

/// RFC3339 UTC observation timestamp (no external clock dependency;
/// deterministic in tests via fixed fixtures where required).
fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Seconds since epoch -> RFC3339 UTC. Deterministic formatting
    // without a date library: epoch 0 = 1970-01-01T00:00:00Z.
    format_epoch_utc(secs)
}

fn format_epoch_utc(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days as i64);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Convert days since 1970-01-01 to (year, month, day) using Howard
/// Hinnant's civil-from-days algorithm (public domain).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_sentinel::{NetworkSegment, SentinelErrorCode};
    use std::str::FromStr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex as StdMutex};

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    #[derive(Clone, Default)]
    struct CountingTransport {
        arp_calls: Arc<AtomicUsize>,
        entries: Arc<StdMutex<Vec<OpnsenseArpEntry>>>,
        fail_arp: bool,
    }

    impl CountingTransport {
        fn with_entries(entries: Vec<OpnsenseArpEntry>) -> Self {
            Self {
                entries: Arc::new(StdMutex::new(entries)),
                ..Default::default()
            }
        }
    }

    impl OpnsenseTransport for CountingTransport {
        fn arp_table(&self) -> Result<Vec<OpnsenseArpEntry>, SentinelError> {
            self.arp_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_arp {
                return Err(SentinelError::new(
                    SentinelErrorCode::Unavailable,
                    "fixture arp failed",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Ok(self.entries.lock().unwrap().clone())
        }
    }

    fn arp(
        mac: &str,
        ip: &str,
        permanent: bool,
        expired: bool,
        manufacturer: &str,
        hostname: &str,
    ) -> OpnsenseArpEntry {
        OpnsenseArpEntry {
            mac: mac.into(),
            ip: ip.into(),
            intf: "lan".into(),
            expired,
            permanent,
            r#type: "ethernet".into(),
            manufacturer: manufacturer.into(),
            hostname: hostname.into(),
        }
    }

    #[test]
    fn aud028_unit_capabilities_fail_closed_when_transport_unavailable() {
        let t = CountingTransport {
            fail_arp: true,
            ..Default::default()
        };
        let inventory = OpnsenseNetworkInventory::new(
            Box::new(t.clone()),
            tenant(),
            NetworkSegment::Iot,
            "key",
            "secret",
        );
        let caps = inventory.capabilities();
        assert!(!caps.contains(SentinelCapabilityKind::Inventory));
        assert!(!caps.contains(SentinelCapabilityKind::Fingerprint));
        assert!(!caps.contains(SentinelCapabilityKind::Baselines));

        // A transport that answers advertises the inventory surface.
        let t2 = CountingTransport::default();
        let inventory2 = OpnsenseNetworkInventory::new(
            Box::new(t2.clone()),
            tenant(),
            NetworkSegment::Iot,
            "key",
            "secret",
        );
        let caps2 = inventory2.capabilities();
        assert!(caps2.contains(SentinelCapabilityKind::Inventory));
        assert!(caps2.contains(SentinelCapabilityKind::Fingerprint));
        assert!(caps2.contains(SentinelCapabilityKind::Baselines));
    }

    #[test]
    fn aud028_unit_list_devices_is_observed_not_fabricated() {
        let t = CountingTransport::with_entries(vec![
            arp(
                "aa:bb:cc:00:00:01",
                "192.0.2.10",
                false,
                false,
                "ACME Devices",
                "thermostat",
            ),
            arp("aa:bb:cc:00:00:02", "192.0.2.11", false, false, "", ""),
            // Permanent entries are the firewall's OWN interfaces, not
            // neighbors; expired entries are hosts that stopped
            // answering. Neither is a discovered device.
            arp(
                "aa:bb:cc:00:00:03",
                "192.0.2.1",
                true,
                false,
                "",
                "firewall",
            ),
            arp(
                "aa:bb:cc:00:00:04",
                "192.0.2.99",
                false,
                true,
                "Old",
                "gone",
            ),
        ]);
        let inventory = OpnsenseNetworkInventory::new(
            Box::new(t.clone()),
            tenant(),
            NetworkSegment::Iot,
            "key",
            "secret",
        );
        let devices = inventory.list_devices(&tenant()).unwrap();
        assert_eq!(devices.len(), 2);
        let d1 = devices
            .iter()
            .find(|d| d.device_id.as_str() == "mac:aa:bb:cc:00:00:01")
            .expect("thermostat device");
        assert_eq!(d1.label, "thermostat");
        assert_eq!(d1.vendor.as_deref(), Some("ACME Devices"));
        assert_eq!(d1.provider, "opnsense");
        assert_eq!(d1.segment, NetworkSegment::Iot);
        assert_eq!(d1.trust_class, TrustClass::Unknown);
        // The hostname-less entry falls back to the observed IP.
        let d2 = devices
            .iter()
            .find(|d| d.device_id.as_str() == "mac:aa:bb:cc:00:00:02")
            .expect("ip-label device");
        assert_eq!(d2.label, "192.0.2.11");
        assert!(!devices
            .iter()
            .any(|d| d.device_id.as_str() == "mac:aa:bb:cc:00:00:03"));
        assert!(!devices
            .iter()
            .any(|d| d.device_id.as_str() == "mac:aa:bb:cc:00:00:04"));
    }

    #[test]
    fn aud028_unit_fingerprint_binds_observed_entry_and_fails_closed() {
        let t = CountingTransport::with_entries(vec![arp(
            "aa:bb:cc:00:00:01",
            "192.0.2.10",
            false,
            false,
            "ACME Devices",
            "thermostat",
        )]);
        let inventory = OpnsenseNetworkInventory::new(
            Box::new(t.clone()),
            tenant(),
            NetworkSegment::Iot,
            "key",
            "secret",
        );
        // Discover the device first (observed), then fingerprint it.
        let devices = inventory.list_devices(&tenant()).unwrap();
        let device = &devices[0];
        let fp = inventory.fingerprint(device).unwrap();
        assert_eq!(fp.mac_ref.as_deref(), Some("aa:bb:cc:00:00:01"));
        assert_eq!(fp.ip_ref.as_deref(), Some("192.0.2.10"));
        assert_eq!(fp.vendor.as_deref(), Some("ACME Devices"));

        // A device the router does not see fails closed (NotFound) -
        // identity is never guessed.
        let ghost = NetworkDevice::new(
            NetworkDeviceId::new("mac:de:ad:be:ef:00:99").unwrap(),
            tenant(),
            NetworkSegment::Iot,
            TrustClass::Unknown,
            "ghost",
            "opnsense",
            "2026-08-20T00:00:00Z",
            "2026-08-20T00:00:00Z",
        );
        let err = inventory.fingerprint(&ghost).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::NotFound);
    }

    #[test]
    fn aud028_unit_baseline_begins_learning_for_observed_device() {
        let t = CountingTransport::with_entries(vec![arp(
            "aa:bb:cc:00:00:01",
            "192.0.2.10",
            false,
            false,
            "ACME Devices",
            "thermostat",
        )]);
        let inventory = OpnsenseNetworkInventory::new(
            Box::new(t.clone()),
            tenant(),
            NetworkSegment::Iot,
            "key",
            "secret",
        );
        let devices = inventory.list_devices(&tenant()).unwrap();
        let baseline = inventory.baseline(&devices[0]).unwrap();
        assert_eq!(
            baseline.state,
            nexus_sentinel::BehaviorBaselineState::Learning
        );
        assert!(baseline.expected_destinations.is_empty());
        assert!(baseline.expected_protocols.is_empty());

        // An unobserved device has no baseline - fails closed.
        let ghost = NetworkDevice::new(
            NetworkDeviceId::new("mac:de:ad:be:ef:00:99").unwrap(),
            tenant(),
            NetworkSegment::Iot,
            TrustClass::Unknown,
            "ghost",
            "opnsense",
            "2026-08-20T00:00:00Z",
            "2026-08-20T00:00:00Z",
        );
        let err = inventory.baseline(&ghost).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::NotFound);
    }

    #[test]
    fn aud028_unit_observation_state_tracks_first_and_last_seen() {
        let t = CountingTransport::with_entries(vec![arp(
            "aa:bb:cc:00:00:01",
            "192.0.2.10",
            false,
            false,
            "ACME Devices",
            "thermostat",
        )]);
        let inventory = OpnsenseNetworkInventory::new(
            Box::new(t.clone()),
            tenant(),
            NetworkSegment::Iot,
            "key",
            "secret",
        );
        let first = inventory.list_devices(&tenant()).unwrap();
        assert_eq!(first[0].first_seen_at, first[0].last_seen_at);
        // Second observation preserves first_seen and advances
        // last_seen (observation state, never fabricated identity).
        let second = inventory.list_devices(&tenant()).unwrap();
        assert_eq!(second[0].first_seen_at, first[0].first_seen_at);
        assert!(second[0].last_seen_at >= first[0].last_seen_at);
    }
}
