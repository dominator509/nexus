//! EP-023 provider ports (fail-closed defaults; SPEC-021 behavior 2).
//!
//! A provider advertises only capabilities proven through supported or
//! observed authenticated paths. Unbound providers fail closed and
//! never fabricate cameras, events, or streams (Reality rule).

use crate::error::VisionError;
use crate::event::CameraEvent;
use crate::stream::StreamRef;
use crate::vocabulary::{CameraCapability, CameraId, RokuCapabilityTier};

/// Camera provider port (provider-neutral).
pub trait CameraProvider {
    fn list_cameras(&self) -> Result<Vec<CameraId>, VisionError> {
        Err(VisionError::unavailable(
            "camera provider has no implementation bound",
        ))
    }

    fn capabilities(&self, camera: &CameraId) -> Result<Vec<CameraCapability>, VisionError> {
        let _ = camera;
        Err(VisionError::unavailable(
            "camera provider has no implementation bound",
        ))
    }

    fn stream(&self, camera: &CameraId) -> Result<StreamRef, VisionError> {
        let _ = camera;
        Err(VisionError::unavailable(
            "camera provider has no implementation bound",
        ))
    }
}

/// Frigate provider port (primary local NVR / object-event source;
/// SPEC-021 behavior 1).
pub trait FrigateProvider {
    fn events(&self, camera: &CameraId, since_ms: u64) -> Result<Vec<CameraEvent>, VisionError> {
        let _ = (camera, since_ms);
        Err(VisionError::unavailable(
            "frigate provider has no implementation bound",
        ))
    }

    fn health(&self) -> Result<(), VisionError> {
        Err(VisionError::unavailable(
            "frigate provider has no implementation bound",
        ))
    }
}

/// Roku home provider port. Inventory advertises only proven
/// capabilities (SPEC-021 behavior 2); the fallback ladder selects the
/// best verified tier.
pub trait RokuHomeProvider {
    fn inventory(&self) -> Result<Vec<CameraId>, VisionError> {
        Err(VisionError::unavailable(
            "roku home provider has no implementation bound",
        ))
    }

    fn tier(&self, device: &CameraId) -> Result<RokuCapabilityTier, VisionError> {
        let _ = device;
        Err(VisionError::unavailable(
            "roku home provider has no implementation bound",
        ))
    }
}
