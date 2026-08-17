//! Camera availability mapping (SPEC-021 behavior 1; owner directive
//! I/Q).
//!
//! A camera that exists in configuration is NOT automatically
//! reachable, and a reachable camera is NOT automatically streaming.
//! The adapter maps these states truthfully:
//!
//! - `Discovered` - camera present (and enabled) in Frigate config
//! - `Available` - provider health confirmed AND camera reachable
//! - `Streaming` - a go2rtc stream exists with an attached producer
//!   (media-level proof is owned by the M3/M5 live-fire; the adapter
//!   never upgrades this from configuration alone)
//! - `Degraded` - camera configured but stream missing or dead
//! - `Unavailable` - camera not configured / provider unreachable

use serde::{Deserialize, Serialize};

/// Camera availability states (canonical adapter vocabulary).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CameraAvailability {
    Discovered,
    Available,
    Streaming,
    Degraded,
    Unavailable,
}

impl CameraAvailability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "DISCOVERED",
            Self::Available => "AVAILABLE",
            Self::Streaming => "STREAMING",
            Self::Degraded => "DEGRADED",
            Self::Unavailable => "UNAVAILABLE",
        }
    }
}

/// Deterministic availability mapping.
///
/// Truth table (never collapses configured/reachable/streaming):
///
/// | configured | enabled | provider_healthy | stream_attached | result |
/// | --- | --- | --- | --- | --- |
/// | false | _ | _ | _ | Unavailable |
/// | true | false | _ | _ | Degraded |
/// | true | true | false | _ | Unavailable |
/// | true | true | true | false | Available |
/// | true | true | true | true | Streaming |
pub fn availability(
    configured: bool,
    enabled: bool,
    provider_healthy: bool,
    stream_attached: bool,
) -> CameraAvailability {
    if !configured || !enabled {
        return if configured && !enabled {
            CameraAvailability::Degraded
        } else {
            CameraAvailability::Unavailable
        };
    }
    if !provider_healthy {
        return CameraAvailability::Unavailable;
    }
    if stream_attached {
        CameraAvailability::Streaming
    } else {
        CameraAvailability::Available
    }
}

/// Whether a state may be advertised as operational (streaming media
/// or at least reachable). `Discovered` alone is never operational.
pub const fn is_operational(state: CameraAvailability) -> bool {
    matches!(
        state,
        CameraAvailability::Available | CameraAvailability::Streaming
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep023_unit_frigate_availability_not_configured_is_unavailable() {
        assert_eq!(
            availability(false, false, true, true),
            CameraAvailability::Unavailable
        );
    }

    #[test]
    fn ep023_unit_frigate_availability_disabled_is_degraded_never_online() {
        assert_eq!(
            availability(true, false, true, true),
            CameraAvailability::Degraded
        );
    }

    #[test]
    fn ep023_unit_frigate_availability_provider_down_is_unavailable() {
        assert_eq!(
            availability(true, true, false, true),
            CameraAvailability::Unavailable
        );
    }

    #[test]
    fn ep023_unit_frigate_availability_configured_reachable_no_stream_is_available() {
        assert_eq!(
            availability(true, true, true, false),
            CameraAvailability::Available
        );
    }

    #[test]
    fn ep023_unit_frigate_availability_stream_attached_is_streaming() {
        assert_eq!(
            availability(true, true, true, true),
            CameraAvailability::Streaming
        );
    }

    #[test]
    fn ep023_unit_frigate_availability_discovered_is_not_operational() {
        assert!(!is_operational(CameraAvailability::Discovered));
        assert!(is_operational(CameraAvailability::Available));
        assert!(is_operational(CameraAvailability::Streaming));
    }
}
