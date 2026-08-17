//! EP-024 provider ports (fail-closed defaults; SPEC-011 behaviors
//! 1-7).
//!
//! Home Assistant is the preferred provider for commodity devices;
//! direct providers exist only for capability or reliability gaps.
//! Unbound providers fail closed and never fabricate devices, states,
//! or events (Reality rule).

use crate::error::DevicesError;
use crate::vocabulary::{
    ApplianceCapability, ApplianceDeviceId, DeviceAvailability, IrrigationCapability,
    IrrigationZoneId, MediaCapability, MediaDeviceId, RobotCapability, RobotId, VacuumCapability,
    VacuumDeviceId,
};

/// Media provider port (Sonos, major TVs, media; SPEC-011 behavior 5).
pub trait MediaProvider {
    fn list_devices(&self) -> Result<Vec<MediaDeviceId>, DevicesError> {
        Err(DevicesError::unavailable(
            "media provider has no implementation bound",
        ))
    }

    fn capabilities(&self, device: &MediaDeviceId) -> Result<Vec<MediaCapability>, DevicesError> {
        let _ = device;
        Err(DevicesError::unavailable(
            "media provider has no implementation bound",
        ))
    }

    fn availability(&self, device: &MediaDeviceId) -> Result<DeviceAvailability, DevicesError> {
        let _ = device;
        Err(DevicesError::unavailable(
            "media provider has no implementation bound",
        ))
    }
}

/// Appliance provider port (appliances; SPEC-011 behavior 5).
pub trait ApplianceProvider {
    fn list_devices(&self) -> Result<Vec<ApplianceDeviceId>, DevicesError> {
        Err(DevicesError::unavailable(
            "appliance provider has no implementation bound",
        ))
    }

    fn capabilities(
        &self,
        device: &ApplianceDeviceId,
    ) -> Result<Vec<ApplianceCapability>, DevicesError> {
        let _ = device;
        Err(DevicesError::unavailable(
            "appliance provider has no implementation bound",
        ))
    }

    fn availability(&self, device: &ApplianceDeviceId) -> Result<DeviceAvailability, DevicesError> {
        let _ = device;
        Err(DevicesError::unavailable(
            "appliance provider has no implementation bound",
        ))
    }
}

/// Irrigation provider port (lawn/zone watering; SPEC-011 behavior 5).
pub trait IrrigationProvider {
    fn list_zones(&self) -> Result<Vec<IrrigationZoneId>, DevicesError> {
        Err(DevicesError::unavailable(
            "irrigation provider has no implementation bound",
        ))
    }

    fn capabilities(
        &self,
        zone: &IrrigationZoneId,
    ) -> Result<Vec<IrrigationCapability>, DevicesError> {
        let _ = zone;
        Err(DevicesError::unavailable(
            "irrigation provider has no implementation bound",
        ))
    }

    fn availability(&self, zone: &IrrigationZoneId) -> Result<DeviceAvailability, DevicesError> {
        let _ = zone;
        Err(DevicesError::unavailable(
            "irrigation provider has no implementation bound",
        ))
    }
}

/// Vacuum provider port (robotic vacuums; SPEC-011 behavior 5).
pub trait VacuumProvider {
    fn list_devices(&self) -> Result<Vec<VacuumDeviceId>, DevicesError> {
        Err(DevicesError::unavailable(
            "vacuum provider has no implementation bound",
        ))
    }

    fn capabilities(&self, device: &VacuumDeviceId) -> Result<Vec<VacuumCapability>, DevicesError> {
        let _ = device;
        Err(DevicesError::unavailable(
            "vacuum provider has no implementation bound",
        ))
    }

    fn availability(&self, device: &VacuumDeviceId) -> Result<DeviceAvailability, DevicesError> {
        let _ = device;
        Err(DevicesError::unavailable(
            "vacuum provider has no implementation bound",
        ))
    }
}

/// Robot provider port (future robots; SPEC-011 behavior 6).
///
/// A robot is activated only for declared capabilities. The provider
/// exposes `declared_capabilities` so callers can prove the robot never
/// receives broader authority than declared before any activation.
pub trait RobotProvider {
    fn list_robots(&self) -> Result<Vec<RobotId>, DevicesError> {
        Err(DevicesError::unavailable(
            "robot provider has no implementation bound",
        ))
    }

    fn declared_capabilities(&self, robot: &RobotId) -> Result<Vec<RobotCapability>, DevicesError> {
        let _ = robot;
        Err(DevicesError::unavailable(
            "robot provider has no implementation bound",
        ))
    }

    fn availability(&self, robot: &RobotId) -> Result<DeviceAvailability, DevicesError> {
        let _ = robot;
        Err(DevicesError::unavailable(
            "robot provider has no implementation bound",
        ))
    }
}
