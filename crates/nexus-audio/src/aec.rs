//! EP-022 echo cancellation profile (SPEC-012 capability AEC).

use serde::{Deserialize, Serialize};

use crate::error::{AudioError, AudioErrorCode, VocabularyError};

/// Canonical AEC profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AecProfile {
    None,
    EchoCancellation,
    NoiseSuppression,
    Full,
}

impl AecProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::EchoCancellation => "ECHO_CANCELLATION",
            Self::NoiseSuppression => "NOISE_SUPPRESSION",
            Self::Full => "FULL",
        }
    }

    pub fn parse(value: &str) -> Result<Self, VocabularyError> {
        match value {
            "NONE" => Ok(Self::None),
            "ECHO_CANCELLATION" => Ok(Self::EchoCancellation),
            "NOISE_SUPPRESSION" => Ok(Self::NoiseSuppression),
            "FULL" => Ok(Self::Full),
            other => Err(VocabularyError(format!("unknown aec profile: {other}"))),
        }
    }
}

/// Echo cancellation profile with validated parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EchoCancellationProfile {
    pub profile: AecProfile,
    /// Aggressiveness 0..=2 (0 = least aggressive).
    pub aggressiveness: u8,
    pub noise_suppression: bool,
}

impl EchoCancellationProfile {
    pub fn new(
        profile: AecProfile,
        aggressiveness: u8,
        noise_suppression: bool,
    ) -> Result<Self, AudioError> {
        if aggressiveness > 2 {
            return Err(AudioError::new(
                AudioErrorCode::Validation,
                "aec aggressiveness must be 0..=2",
                None,
                None,
            ));
        }
        Ok(Self {
            profile,
            aggressiveness,
            noise_suppression,
        })
    }
}
