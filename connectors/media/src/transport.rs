//! EP-024 media transport port (SPEC-011 behaviors 1-3, 5).
//!
//! The transport boundary is provider-neutral: a media device exposes
//! play/pause/stop/seek/volume/source/power surfaces through a
//! documented transport. The adapter core (M2) is real and
//! deterministic; a concrete transport may be Home Assistant (primary,
//! behavior 1) or a direct Sonos/TV transport for capability or
//! reliability gaps (behavior 5, acceptance obligation 2). Unbound
//! transports fail closed and never fabricate devices or states
//! (Reality rule).

use serde::{Deserialize, Serialize};

use crate::error::{MediaError, MediaErrorCode};

/// Canonical media command (provider-neutral).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaCommand {
    Play,
    Pause,
    Stop,
    Seek,
    SetVolume,
    SetSource,
    PowerOn,
    PowerOff,
}

impl MediaCommand {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Play => "PLAY",
            Self::Pause => "PAUSE",
            Self::Stop => "STOP",
            Self::Seek => "SEEK",
            Self::SetVolume => "SET_VOLUME",
            Self::SetSource => "SET_SOURCE",
            Self::PowerOn => "POWER_ON",
            Self::PowerOff => "POWER_OFF",
        }
    }

    pub fn parse(text: &str) -> Result<Self, MediaError> {
        match text {
            "PLAY" => Ok(Self::Play),
            "PAUSE" => Ok(Self::Pause),
            "STOP" => Ok(Self::Stop),
            "SEEK" => Ok(Self::Seek),
            "SET_VOLUME" => Ok(Self::SetVolume),
            "SET_SOURCE" => Ok(Self::SetSource),
            "POWER_ON" => Ok(Self::PowerOn),
            "POWER_OFF" => Ok(Self::PowerOff),
            _ => Err(MediaError::new(
                MediaErrorCode::Vocabulary,
                format!("unknown media command {text:?}"),
                None,
                None,
            )),
        }
    }
}

/// Canonical media state observed after a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaState {
    /// Exact target device identity (canonical string form).
    pub device: String,
    /// Playback state when present: PLAYING, PAUSED, STOPPED, IDLE.
    pub playback: Option<String>,
    /// Volume 0..=100 when present.
    pub volume: Option<u8>,
    /// Source/input name when present.
    pub source: Option<String>,
    /// Power state when present: ON, OFF.
    pub power: Option<String>,
}

/// Media transport port.
///
/// The default implementations fail closed: an unbound transport is
/// UNAVAILABLE and never fabricates devices, states, or command
/// acceptance (Reality rule).
pub trait MediaTransport {
    fn list_devices(&self) -> Result<Vec<String>, MediaError> {
        Err(MediaError::unavailable(
            "media transport has no implementation bound",
        ))
    }

    fn state(&self, device: &str) -> Result<MediaState, MediaError> {
        let _ = device;
        Err(MediaError::unavailable(
            "media transport has no implementation bound",
        ))
    }

    fn send_command(&self, device: &str, command: MediaCommand) -> Result<(), MediaError> {
        let _ = (device, command);
        Err(MediaError::unavailable(
            "media transport has no implementation bound",
        ))
    }
}

/// Command receipt: SUBMITTED at most, never VERIFIED.
///
/// COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED. The adapter
/// returns SUBMITTED after transport acceptance; verification is a
/// separate exact-target readback.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaCommandReceipt {
    pub device: String,
    pub command: MediaCommand,
    pub state: MediaCommandState,
}

/// Media command state (canonical, deterministic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MediaCommandState {
    Authorized,
    Submitted,
    Verified,
    VerificationTimeout,
    Unknown,
}

impl MediaCommandState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authorized => "AUTHORIZED",
            Self::Submitted => "SUBMITTED",
            Self::Verified => "VERIFIED",
            Self::VerificationTimeout => "VERIFICATION_TIMEOUT",
            Self::Unknown => "UNKNOWN",
        }
    }
}
