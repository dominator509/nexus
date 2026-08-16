//! EP-022 audio vocabulary (SPEC-012 canonical terms).
//!
//! Vocabulary-locked canonical values for the audio surface. The
//! hardware classes are the SPEC-012 top-ten satellite matrix plus the
//! software endpoints (VOICE_PREVIEW, WYOMING, ASSIST_SATELLITE).
//! Unknown values are rejected at parse time; wire values are
//! canonical SCREAMING_SNAKE strings so the Rust surface matches the
//! Python (nexus_voice) and TypeScript surfaces exactly.

use crate::error::VocabularyError;

/// Canonical endpoint roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EndpointRole {
    Input,
    Output,
    Bidirectional,
}

impl EndpointRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "INPUT",
            Self::Output => "OUTPUT",
            Self::Bidirectional => "BIDIRECTIONAL",
        }
    }
}

/// Canonical SPEC-012 hardware classes (top-ten satellite matrix +
/// software endpoints).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HardwareClass {
    VoicePreview,
    Esp32S3Box3,
    AtomEcho,
    Esp32S3I2s,
    Pi5,
    Pi4,
    PiZero2W,
    X86Linux,
    Android,
    Ios,
    Wyoming,
    AssistSatellite,
}

/// All canonical hardware classes in SPEC-012 order.
pub const HARDWARE_CLASSES: &[HardwareClass] = &[
    HardwareClass::VoicePreview,
    HardwareClass::Esp32S3Box3,
    HardwareClass::AtomEcho,
    HardwareClass::Esp32S3I2s,
    HardwareClass::Pi5,
    HardwareClass::Pi4,
    HardwareClass::PiZero2W,
    HardwareClass::X86Linux,
    HardwareClass::Android,
    HardwareClass::Ios,
    HardwareClass::Wyoming,
    HardwareClass::AssistSatellite,
];

impl HardwareClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VoicePreview => "VOICE_PREVIEW",
            Self::Esp32S3Box3 => "ESP32_S3_BOX_3",
            Self::AtomEcho => "ATOM_ECHO",
            Self::Esp32S3I2s => "ESP32_S3_I2S",
            Self::Pi5 => "PI_5",
            Self::Pi4 => "PI_4",
            Self::PiZero2W => "PI_ZERO_2_W",
            Self::X86Linux => "X86_LINUX",
            Self::Android => "ANDROID",
            Self::Ios => "IOS",
            Self::Wyoming => "WYOMING",
            Self::AssistSatellite => "ASSIST_SATELLITE",
        }
    }

    pub fn parse(value: &str) -> Result<Self, VocabularyError> {
        match value {
            "VOICE_PREVIEW" => Ok(Self::VoicePreview),
            "ESP32_S3_BOX_3" => Ok(Self::Esp32S3Box3),
            "ATOM_ECHO" => Ok(Self::AtomEcho),
            "ESP32_S3_I2S" => Ok(Self::Esp32S3I2s),
            "PI_5" => Ok(Self::Pi5),
            "PI_4" => Ok(Self::Pi4),
            "PI_ZERO_2_W" => Ok(Self::PiZero2W),
            "X86_LINUX" => Ok(Self::X86Linux),
            "ANDROID" => Ok(Self::Android),
            "IOS" => Ok(Self::Ios),
            "WYOMING" => Ok(Self::Wyoming),
            "ASSIST_SATELLITE" => Ok(Self::AssistSatellite),
            other => Err(VocabularyError(format!("unknown hardware class: {other}"))),
        }
    }
}

/// Require a canonical hardware class value; reject unknown values.
pub fn require_hardware_class(value: &str) -> Result<HardwareClass, VocabularyError> {
    HardwareClass::parse(value)
}

/// Require a canonical endpoint role; reject unknown values.
pub fn require_role(value: &str) -> Result<EndpointRole, VocabularyError> {
    match value {
        "INPUT" => Ok(EndpointRole::Input),
        "OUTPUT" => Ok(EndpointRole::Output),
        "BIDIRECTIONAL" => Ok(EndpointRole::Bidirectional),
        other => Err(VocabularyError(format!("unknown endpoint role: {other}"))),
    }
}
