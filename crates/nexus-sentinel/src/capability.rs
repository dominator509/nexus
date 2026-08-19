//! EP-030 provider-neutral sentinel capability advertisement
//! (SPEC-013; reality rule).
//!
//! A provider advertises only capabilities it actually holds.
//! Unbound/uncertified providers advertise nothing (fail closed), and
//! an unadvertised capability is UNAVAILABLE. Unknown provider
//! capability kinds are skipped at the infrastructure boundary and
//! never widen the contract vocabulary.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::vocabulary::SentinelVocabularyError;

/// Sentinel capability kind (SPEC-013 behaviors: firewall telemetry,
/// AdGuard DNS, inventory, expected destinations, flow baselines,
/// identity events, Nexus system events; containment).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SentinelCapabilityKind {
    /// Read firewall telemetry and rules.
    ReadFirewallTelemetry,
    /// Propose and apply preauthorized reversible containment.
    Containment,
    /// Read DNS security telemetry (AdGuard).
    ReadDnsTelemetry,
    /// Read DNS blocklist state (AdGuard).
    ReadDnsBlocklist,
    /// Enumerate the network inventory.
    Inventory,
    /// Fingerprint devices.
    Fingerprint,
    /// Build and read behavior baselines.
    Baselines,
    /// Read findings (anomalies, violations).
    ReadFindings,
    /// Propose quarantine (verified containment).
    ProposeQuarantine,
}

impl SentinelCapabilityKind {
    /// Canonical wire string for this class.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadFirewallTelemetry => "READ_FIREWALL_TELEMETRY",
            Self::Containment => "CONTAINMENT",
            Self::ReadDnsTelemetry => "READ_DNS_TELEMETRY",
            Self::ReadDnsBlocklist => "READ_DNS_BLOCKLIST",
            Self::Inventory => "INVENTORY",
            Self::Fingerprint => "FINGERPRINT",
            Self::Baselines => "BASELINES",
            Self::ReadFindings => "READ_FINDINGS",
            Self::ProposeQuarantine => "PROPOSE_QUARANTINE",
        }
    }
}

impl std::fmt::Display for SentinelCapabilityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SentinelCapabilityKind {
    type Err = SentinelVocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "READ_FIREWALL_TELEMETRY" => Ok(Self::ReadFirewallTelemetry),
            "CONTAINMENT" => Ok(Self::Containment),
            "READ_DNS_TELEMETRY" => Ok(Self::ReadDnsTelemetry),
            "READ_DNS_BLOCKLIST" => Ok(Self::ReadDnsBlocklist),
            "INVENTORY" => Ok(Self::Inventory),
            "FINGERPRINT" => Ok(Self::Fingerprint),
            "BASELINES" => Ok(Self::Baselines),
            "READ_FINDINGS" => Ok(Self::ReadFindings),
            "PROPOSE_QUARANTINE" => Ok(Self::ProposeQuarantine),
            other => Err(SentinelVocabularyError(other.to_string())),
        }
    }
}

/// Fail-closed capability map: empty by default, advertises nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SentinelCapabilityMap {
    kinds: BTreeSet<SentinelCapabilityKind>,
}

impl SentinelCapabilityMap {
    pub fn new() -> Self {
        Self {
            kinds: BTreeSet::new(),
        }
    }

    pub fn insert(&mut self, kind: SentinelCapabilityKind) {
        self.kinds.insert(kind);
    }

    pub fn contains(&self, kind: SentinelCapabilityKind) -> bool {
        self.kinds.contains(&kind)
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    pub fn kinds(&self) -> impl Iterator<Item = SentinelCapabilityKind> + '_ {
        self.kinds.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep030_unit_capability_map_fails_closed() {
        let map = SentinelCapabilityMap::new();
        assert!(map.is_empty());
        assert!(!map.contains(SentinelCapabilityKind::Containment));
        let mut map = map;
        map.insert(SentinelCapabilityKind::ReadDnsTelemetry);
        assert!(map.contains(SentinelCapabilityKind::ReadDnsTelemetry));
        assert!(!map.contains(SentinelCapabilityKind::Containment));
    }

    #[test]
    fn ep030_unit_capability_kind_wire_spelling_locked() {
        assert_eq!(
            SentinelCapabilityKind::ReadFirewallTelemetry.as_str(),
            "READ_FIREWALL_TELEMETRY"
        );
        assert_eq!(SentinelCapabilityKind::Containment.as_str(), "CONTAINMENT");
        assert_eq!(
            SentinelCapabilityKind::ProposeQuarantine.as_str(),
            "PROPOSE_QUARANTINE"
        );
        assert_eq!(
            "FABRICATED".parse::<SentinelCapabilityKind>(),
            Err(SentinelVocabularyError("FABRICATED".to_string()))
        );
    }

    #[test]
    fn ep030_unit_capability_map_serde_roundtrip() {
        let mut map = SentinelCapabilityMap::new();
        map.insert(SentinelCapabilityKind::Inventory);
        map.insert(SentinelCapabilityKind::Baselines);
        let json = serde_json::to_string(&map).unwrap();
        let back: SentinelCapabilityMap = serde_json::from_str(&json).unwrap();
        assert_eq!(back, map);
    }
}
