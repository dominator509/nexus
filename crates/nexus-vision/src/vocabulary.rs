//! EP-023 canonical vocabulary (SPEC-021 terms are vocabulary locked;
//! a new synonym requires an ADR and schema update).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{VisionError, VisionErrorCode};

/// Typed camera id (bounded, non-empty).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CameraId(String);

impl CameraId {
    pub fn new(value: impl Into<String>) -> Result<Self, VisionError> {
        let value = value.into();
        if value.is_empty() || value.len() > 128 {
            return Err(VisionError::new(
                VisionErrorCode::Validation,
                "camera id must be 1..=128 characters",
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

impl fmt::Display for CameraId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Canonical camera capabilities (SPEC-021 canonical term
/// CameraCapability).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CameraCapability {
    ObjectDetection,
    Recording,
    LiveStream,
    TwoWayAudio,
    VisitorEvents,
    RokuControl,
}

impl CameraCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ObjectDetection => "OBJECT_DETECTION",
            Self::Recording => "RECORDING",
            Self::LiveStream => "LIVE_STREAM",
            Self::TwoWayAudio => "TWO_WAY_AUDIO",
            Self::VisitorEvents => "VISITOR_EVENTS",
            Self::RokuControl => "ROKU_CONTROL",
        }
    }

    /// Parse a canonical value; unknown values are rejected
    /// (vocabulary lock).
    pub fn parse(value: &str) -> Result<Self, VisionError> {
        match value {
            "OBJECT_DETECTION" => Ok(Self::ObjectDetection),
            "RECORDING" => Ok(Self::Recording),
            "LIVE_STREAM" => Ok(Self::LiveStream),
            "TWO_WAY_AUDIO" => Ok(Self::TwoWayAudio),
            "VISITOR_EVENTS" => Ok(Self::VisitorEvents),
            "ROKU_CONTROL" => Ok(Self::RokuControl),
            other => Err(VisionError::new(
                VisionErrorCode::Vocabulary,
                format!("unknown camera capability: {other}"),
                None,
                None,
            )),
        }
    }
}

/// Camera privacy class (SPEC-021 behavior 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrivacyClass {
    Private,
    Shared,
}

impl PrivacyClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Private => "PRIVATE",
            Self::Shared => "SHARED",
        }
    }
}

/// Roku capability tier; the canonical fallback ladder order is fixed
/// (SPEC-021 behavior 3): verified local, authenticated vendor
/// interface, Google Home bridge, browser automation, then
/// unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RokuCapabilityTier {
    LocalVerified,
    VendorAuthenticated,
    GoogleHomeBridge,
    BrowserAutomation,
    Unavailable,
}

impl RokuCapabilityTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalVerified => "LOCAL_VERIFIED",
            Self::VendorAuthenticated => "VENDOR_AUTHENTICATED",
            Self::GoogleHomeBridge => "GOOGLE_HOME_BRIDGE",
            Self::BrowserAutomation => "BROWSER_AUTOMATION",
            Self::Unavailable => "UNAVAILABLE",
        }
    }

    /// The canonical ladder, best to worst.
    pub const fn ladder() -> [RokuCapabilityTier; 5] {
        [
            Self::LocalVerified,
            Self::VendorAuthenticated,
            Self::GoogleHomeBridge,
            Self::BrowserAutomation,
            Self::Unavailable,
        ]
    }
}
