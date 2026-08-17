//! EP-023 camera fallback plan (SPEC-021 behavior 3) and browser
//! automation policy (behavior 4).

use serde::{Deserialize, Serialize};

use crate::vocabulary::RokuCapabilityTier;

/// Browser automation policy (SPEC-021 behavior 4): isolated,
/// monitored, rate-limited, and never a stable API without
/// certification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserAutomationPolicy {
    pub isolated: bool,
    pub monitored: bool,
    pub rate_limited: bool,
    pub never_stable_api: bool,
}

impl Default for BrowserAutomationPolicy {
    fn default() -> Self {
        Self {
            isolated: true,
            monitored: true,
            rate_limited: true,
            never_stable_api: true,
        }
    }
}

/// The Roku fallback ladder (SPEC-021 behavior 3): verified local,
/// authenticated vendor interface, Google Home bridge, browser
/// automation, then unavailable. Selection is deterministic: the first
/// ladder tier that is available wins; when nothing is available the
/// plan fails closed to Unavailable - it never fabricates a higher
/// tier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CameraFallbackPlan {
    pub ladder: [RokuCapabilityTier; 5],
}

impl Default for CameraFallbackPlan {
    fn default() -> Self {
        Self {
            ladder: RokuCapabilityTier::ladder(),
        }
    }
}

impl CameraFallbackPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select(&self, available: &[RokuCapabilityTier]) -> RokuCapabilityTier {
        for tier in self.ladder {
            if available.contains(&tier) {
                return tier;
            }
        }
        RokuCapabilityTier::Unavailable
    }
}
