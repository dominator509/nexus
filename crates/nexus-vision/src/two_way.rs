//! EP-023 two-way audio capability (SPEC-021 behavior 7, acceptance
//! obligation 4).
//!
//! Two-way audio is enabled only after live certification. The
//! capability requires a verified speaker path, user or policy
//! approval, disclosure rules, and echo handling. `certify()` refuses
//! unless every gate is met.

use serde::{Deserialize, Serialize};

use crate::error::{VisionError, VisionErrorCode};

/// Lifecycle state of two-way audio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TwoWayAudioState {
    NotCertified,
    Certified,
}

/// Two-way audio capability. Certification gates (SPEC-021 behavior 7):
/// verified speaker path, approval required, disclosure required, echo
/// handling required.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TwoWayAudioCapability {
    pub state: TwoWayAudioState,
    pub verified_speaker_path: bool,
    pub approval_required: bool,
    pub disclosure_required: bool,
    pub echo_handling_required: bool,
}

impl Default for TwoWayAudioCapability {
    fn default() -> Self {
        Self {
            state: TwoWayAudioState::NotCertified,
            verified_speaker_path: false,
            approval_required: true,
            disclosure_required: true,
            echo_handling_required: true,
        }
    }
}

impl TwoWayAudioCapability {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_verified_speaker_path(mut self, verified: bool) -> Self {
        self.verified_speaker_path = verified;
        self
    }

    /// Certify only when every gate is met; otherwise fail closed
    /// (acceptance obligation 4: two-way audio is enabled only after
    /// live certification).
    pub fn certify(mut self) -> Result<Self, VisionError> {
        if !self.verified_speaker_path {
            return Err(VisionError::new(
                VisionErrorCode::Verification,
                "two-way audio requires a verified speaker path",
                None,
                None,
            ));
        }
        if !self.approval_required {
            return Err(VisionError::new(
                VisionErrorCode::Policy,
                "two-way audio requires user or policy approval",
                None,
                None,
            ));
        }
        if !self.disclosure_required {
            return Err(VisionError::new(
                VisionErrorCode::Policy,
                "two-way audio requires disclosure rules",
                None,
                None,
            ));
        }
        if !self.echo_handling_required {
            return Err(VisionError::new(
                VisionErrorCode::Policy,
                "two-way audio requires echo handling",
                None,
                None,
            ));
        }
        self.state = TwoWayAudioState::Certified;
        Ok(self)
    }
}
