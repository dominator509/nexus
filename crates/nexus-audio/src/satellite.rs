//! EP-022 voice satellite ports (SPEC-012 behavior 6).
//!
//! Ports fail closed: a provider that is not bound or not certified
//! returns a typed UNAVAILABLE error; it never fabricates audio or
//! claims a hardware class is operational (Reality rule).

use std::fmt;

use crate::error::AudioError;
use crate::vocabulary::HardwareClass;

/// Typed voice satellite id.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VoiceSatelliteId(String);

impl VoiceSatelliteId {
    pub fn new(value: impl Into<String>) -> Result<Self, AudioError> {
        let value = value.into();
        if value.is_empty() || value.len() > 128 {
            return Err(AudioError::new(
                crate::error::AudioErrorCode::Validation,
                "voice satellite id must be 1..=128 characters",
                None,
                None,
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VoiceSatelliteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A voice satellite: identity + hardware class + local capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiceSatellite {
    pub satellite_id: VoiceSatelliteId,
    pub hardware_class: HardwareClass,
    pub name: String,
}

impl VoiceSatellite {
    pub fn new(
        satellite_id: VoiceSatelliteId,
        hardware_class: HardwareClass,
        name: impl Into<String>,
    ) -> Self {
        Self {
            satellite_id,
            hardware_class,
            name: name.into(),
        }
    }
}

/// Assist satellite provider port (locally functional, room-local).
pub trait AssistSatelliteProvider {
    fn start_listening(&self, satellite_id: &VoiceSatelliteId) -> Result<(), AudioError> {
        let _ = satellite_id;
        Err(AudioError::unavailable(
            "assist satellite provider has no implementation bound",
        ))
    }

    fn stop_listening(&self, satellite_id: &VoiceSatelliteId) -> Result<(), AudioError> {
        let _ = satellite_id;
        Err(AudioError::unavailable(
            "assist satellite provider has no implementation bound",
        ))
    }
}

/// Wyoming provider port (Home Assistant satellite protocol).
pub trait WyomingProvider {
    fn connect(&self, uri: &str) -> Result<(), AudioError> {
        let _ = uri;
        Err(AudioError::unavailable(
            "wyoming provider has no implementation bound",
        ))
    }

    fn disconnect(&self, uri: &str) -> Result<(), AudioError> {
        let _ = uri;
        Err(AudioError::unavailable(
            "wyoming provider has no implementation bound",
        ))
    }
}
