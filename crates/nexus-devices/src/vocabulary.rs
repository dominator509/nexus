//! EP-024 canonical device vocabulary (SPEC-011 terms are vocabulary
//! locked; a new synonym requires an ADR and schema update).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{DevicesError, DevicesErrorCode};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, DevicesError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(DevicesError::new(
                        DevicesErrorCode::Validation,
                        concat!(stringify!($name), " must be 1..=128 characters"),
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

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(MediaDeviceId);
typed_id!(ApplianceDeviceId);
typed_id!(IrrigationZoneId);
typed_id!(VacuumDeviceId);
typed_id!(RobotId);

/// Canonical device classes (SPEC-011 behavior 5: Sonos, major TVs,
/// lighting, HVAC, vacuum, irrigation, appliances, energy, IR, and
/// future robots expose provider-neutral capabilities).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceClass {
    Media,
    Appliance,
    Irrigation,
    Vacuum,
    Robot,
    Lighting,
    Hvac,
    Energy,
    Infrared,
    Vehicle,
}

impl DeviceClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Media => "MEDIA",
            Self::Appliance => "APPLIANCE",
            Self::Irrigation => "IRRIGATION",
            Self::Vacuum => "VACUUM",
            Self::Robot => "ROBOT",
            Self::Lighting => "LIGHTING",
            Self::Hvac => "HVAC",
            Self::Energy => "ENERGY",
            Self::Infrared => "INFRARED",
            Self::Vehicle => "VEHICLE",
        }
    }

    pub fn parse(text: &str) -> Result<Self, DevicesError> {
        match text {
            "MEDIA" => Ok(Self::Media),
            "APPLIANCE" => Ok(Self::Appliance),
            "IRRIGATION" => Ok(Self::Irrigation),
            "VACUUM" => Ok(Self::Vacuum),
            "ROBOT" => Ok(Self::Robot),
            "LIGHTING" => Ok(Self::Lighting),
            "HVAC" => Ok(Self::Hvac),
            "ENERGY" => Ok(Self::Energy),
            "INFRARED" => Ok(Self::Infrared),
            "VEHICLE" => Ok(Self::Vehicle),
            _ => Err(DevicesError::new(
                DevicesErrorCode::Vocabulary,
                format!("unknown device class {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Canonical media capabilities (Sonos, major TVs, media).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaCapability {
    Playback,
    Volume,
    Source,
    Power,
}

impl MediaCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Playback => "PLAYBACK",
            Self::Volume => "VOLUME",
            Self::Source => "SOURCE",
            Self::Power => "POWER",
        }
    }

    pub fn parse(text: &str) -> Result<Self, DevicesError> {
        match text {
            "PLAYBACK" => Ok(Self::Playback),
            "VOLUME" => Ok(Self::Volume),
            "SOURCE" => Ok(Self::Source),
            "POWER" => Ok(Self::Power),
            _ => Err(DevicesError::new(
                DevicesErrorCode::Vocabulary,
                format!("unknown media capability {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Canonical appliance capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApplianceCapability {
    PowerControl,
    ModeControl,
    StatusReadback,
}

impl ApplianceCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PowerControl => "POWER_CONTROL",
            Self::ModeControl => "MODE_CONTROL",
            Self::StatusReadback => "STATUS_READBACK",
        }
    }

    pub fn parse(text: &str) -> Result<Self, DevicesError> {
        match text {
            "POWER_CONTROL" => Ok(Self::PowerControl),
            "MODE_CONTROL" => Ok(Self::ModeControl),
            "STATUS_READBACK" => Ok(Self::StatusReadback),
            _ => Err(DevicesError::new(
                DevicesErrorCode::Vocabulary,
                format!("unknown appliance capability {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Canonical irrigation capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IrrigationCapability {
    ZoneControl,
    ScheduleControl,
    MoistureReadback,
}

impl IrrigationCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ZoneControl => "ZONE_CONTROL",
            Self::ScheduleControl => "SCHEDULE_CONTROL",
            Self::MoistureReadback => "MOISTURE_READBACK",
        }
    }

    pub fn parse(text: &str) -> Result<Self, DevicesError> {
        match text {
            "ZONE_CONTROL" => Ok(Self::ZoneControl),
            "SCHEDULE_CONTROL" => Ok(Self::ScheduleControl),
            "MOISTURE_READBACK" => Ok(Self::MoistureReadback),
            _ => Err(DevicesError::new(
                DevicesErrorCode::Vocabulary,
                format!("unknown irrigation capability {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Canonical vacuum capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VacuumCapability {
    Dock,
    StartClean,
    Pause,
    ReturnHome,
    MapReadback,
}

impl VacuumCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dock => "DOCK",
            Self::StartClean => "START_CLEAN",
            Self::Pause => "PAUSE",
            Self::ReturnHome => "RETURN_HOME",
            Self::MapReadback => "MAP_READBACK",
        }
    }

    pub fn parse(text: &str) -> Result<Self, DevicesError> {
        match text {
            "DOCK" => Ok(Self::Dock),
            "START_CLEAN" => Ok(Self::StartClean),
            "PAUSE" => Ok(Self::Pause),
            "RETURN_HOME" => Ok(Self::ReturnHome),
            "MAP_READBACK" => Ok(Self::MapReadback),
            _ => Err(DevicesError::new(
                DevicesErrorCode::Vocabulary,
                format!("unknown vacuum capability {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Canonical robot capabilities (SPEC-011 behavior 6). A robot may be
/// activated only for declared capabilities; it never receives broader
/// authority than declared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RobotCapability {
    Navigation,
    Manipulation,
    Sensing,
    SafetyInterlock,
    EmergencyStop,
    HumanPresenceDetection,
}

impl RobotCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Navigation => "NAVIGATION",
            Self::Manipulation => "MANIPULATION",
            Self::Sensing => "SENSING",
            Self::SafetyInterlock => "SAFETY_INTERLOCK",
            Self::EmergencyStop => "EMERGENCY_STOP",
            Self::HumanPresenceDetection => "HUMAN_PRESENCE_DETECTION",
        }
    }

    pub fn parse(text: &str) -> Result<Self, DevicesError> {
        match text {
            "NAVIGATION" => Ok(Self::Navigation),
            "MANIPULATION" => Ok(Self::Manipulation),
            "SENSING" => Ok(Self::Sensing),
            "SAFETY_INTERLOCK" => Ok(Self::SafetyInterlock),
            "EMERGENCY_STOP" => Ok(Self::EmergencyStop),
            "HUMAN_PRESENCE_DETECTION" => Ok(Self::HumanPresenceDetection),
            _ => Err(DevicesError::new(
                DevicesErrorCode::Vocabulary,
                format!("unknown robot capability {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Device availability truth table: configured != reachable !=
/// streaming. Unknown/unavailable stays unknown; a provider never
/// fabricates an online state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceAvailability {
    Discovered,
    Available,
    Streaming,
    Degraded,
    Unavailable,
}

impl DeviceAvailability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "DISCOVERED",
            Self::Available => "AVAILABLE",
            Self::Streaming => "STREAMING",
            Self::Degraded => "DEGRADED",
            Self::Unavailable => "UNAVAILABLE",
        }
    }

    pub fn parse(text: &str) -> Result<Self, DevicesError> {
        match text {
            "DISCOVERED" => Ok(Self::Discovered),
            "AVAILABLE" => Ok(Self::Available),
            "STREAMING" => Ok(Self::Streaming),
            "DEGRADED" => Ok(Self::Degraded),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            _ => Err(DevicesError::new(
                DevicesErrorCode::Vocabulary,
                format!("unknown device availability {text:?}"),
                None,
                None,
            )),
        }
    }
}
