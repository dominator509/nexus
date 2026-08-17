//! EP-023 Roku home provider connector (SPEC-021 behavior 2/3).
//!
//! The Roku fallback ladder is: verified local stream, authenticated
//! vendor interface, Google Home bridge, browser automation, then
//! unavailable. This connector implements the ladder with REAL
//! fail-closed semantics: no Roku hardware or vendor credentials are
//! bound to this host, so inventory is empty and tier selection is
//! UNAVAILABLE. It NEVER fabricates a device, a capability, or a
//! higher ladder tier (Reality rule; owner directive: Roku stays a
//! layered provider ladder with no hardware).
//!
//! Certification boundary: Roku HARDWARE_CERTIFICATION is DEFERRED
//! (no physical device). This crate exists so the provider port is
//! bound to a real, honest implementation instead of an unbound
//! default, and so a future hardware milestone can fill in the
//! verified tiers without changing the contract.

use nexus_vision::provider::RokuHomeProvider;
use nexus_vision::vocabulary::{CameraId, RokuCapabilityTier};
use nexus_vision::VisionError;

/// Deterministic Roku ladder selection (SPEC-021 behavior 3): the
/// first available tier wins; nothing available fails closed to
/// UNAVAILABLE. Reuses the canonical ladder order from nexus-vision.
pub fn select_tier(available: &[RokuCapabilityTier]) -> RokuCapabilityTier {
    for tier in RokuCapabilityTier::ladder() {
        if available.contains(&tier) {
            return tier;
        }
    }
    RokuCapabilityTier::Unavailable
}

/// The real Roku home provider bound to this host.
///
/// Inventory is empty and tier is UNAVAILABLE because no verified
/// local/authenticated path exists yet - the honest state, never a
/// fabricated capability.
#[derive(Debug, Clone, Copy, Default)]
pub struct RokuHomeProviderHost;

impl RokuHomeProvider for RokuHomeProviderHost {
    fn inventory(&self) -> Result<Vec<CameraId>, VisionError> {
        // No Roku hardware or authenticated session is present on this
        // host; an empty inventory is the truthful result.
        Ok(Vec::new())
    }

    fn tier(&self, device: &CameraId) -> Result<RokuCapabilityTier, VisionError> {
        let _ = device;
        // The fallback ladder fails closed: no verified local stream,
        // no authenticated vendor interface, no Google Home bridge, no
        // certified browser-automation path => UNAVAILABLE.
        Ok(RokuCapabilityTier::Unavailable)
    }
}

/// A controlled-fixture provider that reports a caller-supplied
/// available-tier set. Used ONLY in test zones to prove ladder
/// selection order; production always uses `RokuHomeProviderHost`.
#[cfg(test)]
pub(crate) struct FixtureProvider {
    available: Vec<RokuCapabilityTier>,
}

#[cfg(test)]
impl FixtureProvider {
    pub(crate) fn new(available: Vec<RokuCapabilityTier>) -> Self {
        Self { available }
    }
}

#[cfg(test)]
impl RokuHomeProvider for FixtureProvider {
    fn inventory(&self) -> Result<Vec<CameraId>, VisionError> {
        Ok(Vec::new())
    }

    fn tier(&self, device: &CameraId) -> Result<RokuCapabilityTier, VisionError> {
        let _ = device;
        Ok(select_tier(&self.available))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_vision::VisionErrorCode;

    #[test]
    fn ep023_unit_roku_host_inventory_empty_and_tier_unavailable() {
        let provider = RokuHomeProviderHost;
        let inventory = provider.inventory().expect("inventory is readable");
        assert!(
            inventory.is_empty(),
            "no Roku hardware bound => empty inventory, got {inventory:?}"
        );
        let camera = CameraId::new("roku-living-room").expect("id");
        let tier = provider.tier(&camera).expect("tier is readable");
        assert_eq!(
            tier,
            RokuCapabilityTier::Unavailable,
            "no verified tier => UNAVAILABLE, never a fabricated capability"
        );
    }

    #[test]
    fn ep023_unit_roku_ladder_selects_highest_available() {
        // Best-to-worst: verified local wins over vendor auth; vendor
        // auth wins over Google Home; Google Home wins over browser
        // automation; nothing => unavailable.
        assert_eq!(
            select_tier(&[RokuCapabilityTier::VendorAuthenticated]),
            RokuCapabilityTier::VendorAuthenticated
        );
        assert_eq!(
            select_tier(&[
                RokuCapabilityTier::BrowserAutomation,
                RokuCapabilityTier::GoogleHomeBridge,
                RokuCapabilityTier::LocalVerified,
            ]),
            RokuCapabilityTier::LocalVerified
        );
        assert_eq!(
            select_tier(&[RokuCapabilityTier::GoogleHomeBridge]),
            RokuCapabilityTier::GoogleHomeBridge
        );
        assert_eq!(
            select_tier(&[RokuCapabilityTier::BrowserAutomation]),
            RokuCapabilityTier::BrowserAutomation
        );
        assert_eq!(select_tier(&[]), RokuCapabilityTier::Unavailable);
    }

    #[test]
    fn ep023_unit_roku_host_never_fabricates_higher_tier() {
        // The host reports UNAVAILABLE even when a caller asks for a
        // device that does not exist: there is no path to a fabricated
        // capability tier.
        let provider = RokuHomeProviderHost;
        let unknown = CameraId::new("roku-not-on-network").expect("id");
        assert_eq!(
            provider.tier(&unknown).expect("tier"),
            RokuCapabilityTier::Unavailable
        );
    }

    #[test]
    fn ep023_unit_roku_fixture_provider_reports_available_tier() {
        // The controlled fixture exercises the provider port itself:
        // a caller-supplied available set is honored through
        // select_tier, proving the ladder is what RokuHomeProviderHost
        // fails closed from.
        let provider = FixtureProvider::new(vec![RokuCapabilityTier::GoogleHomeBridge]);
        assert!(provider.inventory().expect("inventory").is_empty());
        let camera = CameraId::new("roku-fixture").expect("id");
        assert_eq!(
            provider.tier(&camera).expect("tier"),
            RokuCapabilityTier::GoogleHomeBridge
        );
        let empty = FixtureProvider::new(vec![]);
        assert_eq!(
            empty.tier(&camera).expect("tier"),
            RokuCapabilityTier::Unavailable
        );
    }

    #[test]
    fn ep023_unit_roku_unavailable_is_canonical_vocabulary() {
        // The vocabulary round-trips and the ladder order is fixed.
        let parsed: RokuCapabilityTier =
            serde_json::from_str("\"UNAVAILABLE\"").expect("canonical parse");
        assert_eq!(parsed, RokuCapabilityTier::Unavailable);
        assert_eq!(parsed.as_str(), "UNAVAILABLE");
        let err = VisionError::new(VisionErrorCode::Unavailable, "roku unavailable", None, None);
        assert_eq!(err.code.as_str(), "UNAVAILABLE");
    }
}
