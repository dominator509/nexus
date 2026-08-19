//! EP-028 Hydra capability map (SPEC-015 required test: Hydra
//! capability and event contract). Capabilities are provider-neutral
//! and fail closed: an unknown or absent capability is never
//! advertised as available.

use std::collections::BTreeMap;

use nexus_domain::Availability;
use serde::{Deserialize, Serialize};

use crate::vocabulary::HydraCapabilityKind;

/// Provider-neutral capability map. The fallback is read-only context
/// and proposal generation until execution capabilities advertise
/// certified availability (node contract replacement/fallback).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydraCapabilityMap {
    capabilities: BTreeMap<HydraCapabilityKind, Availability>,
}

impl Default for HydraCapabilityMap {
    fn default() -> Self {
        Self::new()
    }
}

impl HydraCapabilityMap {
    /// A fresh map contains NO capabilities: every lookup fails closed
    /// as UNAVAILABLE until a real provider advertises it.
    pub fn new() -> Self {
        Self {
            capabilities: BTreeMap::new(),
        }
    }

    pub fn advertise(&mut self, kind: HydraCapabilityKind, availability: Availability) {
        self.capabilities.insert(kind, availability);
    }

    /// Lookup fails closed: an unadvertised capability is UNAVAILABLE,
    /// never silently Available.
    pub fn availability(&self, kind: HydraCapabilityKind) -> Availability {
        self.capabilities
            .get(&kind)
            .copied()
            .unwrap_or(Availability::Unavailable)
    }

    pub fn is_available(&self, kind: HydraCapabilityKind) -> bool {
        self.availability(kind) == Availability::Available
    }

    pub fn kinds(&self) -> impl Iterator<Item = HydraCapabilityKind> + '_ {
        self.capabilities.keys().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep028_unit_capability_map_fails_closed_when_empty() {
        let map = HydraCapabilityMap::new();
        assert_eq!(
            map.availability(HydraCapabilityKind::ExecuteUpdate),
            Availability::Unavailable
        );
        assert!(!map.is_available(HydraCapabilityKind::ReadContext));
        assert_eq!(map.kinds().count(), 0);
    }

    #[test]
    fn ep028_unit_capability_map_advertise_and_roundtrip() {
        let mut map = HydraCapabilityMap::new();
        map.advertise(HydraCapabilityKind::ReadContext, Availability::Available);
        map.advertise(
            HydraCapabilityKind::ExecuteUpdate,
            Availability::Uncertified,
        );
        assert!(map.is_available(HydraCapabilityKind::ReadContext));
        // Uncertified execution is NOT available (fallback: proposal
        // generation only).
        assert!(!map.is_available(HydraCapabilityKind::ExecuteUpdate));
        // Unknown kind never becomes available.
        assert!(!map.is_available(HydraCapabilityKind::SocialPublish));
        let json = serde_json::to_string(&map).unwrap();
        let back: HydraCapabilityMap = serde_json::from_str(&json).unwrap();
        assert_eq!(back, map);
    }
}
