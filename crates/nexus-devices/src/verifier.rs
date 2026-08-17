//! EP-024 device command verifier (SPEC-011 behaviors 2-3;
//! acceptance obligation 3: commands are target-scoped and verified).
//!
//! Verification binds to the exact target device and the requested
//! action's expected result. An observation for any other device is
//! `UnrelatedChange`, never `Verified`. A missing state is `Unknown`; a
//! non-matching value is `Mismatch`. No fabricated pass.

use serde::{Deserialize, Serialize};

use crate::vocabulary::{ApplianceDeviceId, IrrigationZoneId, MediaDeviceId, VacuumDeviceId};

/// Verification outcome (canonical, deterministic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerificationOutcome {
    Verified,
    Mismatch,
    Unknown,
    UnrelatedChange,
}

impl VerificationOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "VERIFIED",
            Self::Mismatch => "MISMATCH",
            Self::Unknown => "UNKNOWN",
            Self::UnrelatedChange => "UNRELATED_CHANGE",
        }
    }
}

/// A device state observation captured after a command.
///
/// `device` is the exact target identity; `state` is the observed
/// canonical state value when present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceStateObservation {
    pub device: String,
    pub state: Option<String>,
}

/// Device command verifier (SPEC-011 canonical term
/// StateVerification).
///
/// Verification succeeds only when the observation is for the exact
/// target device and the expected state is observed. An unrelated
/// device change is `UnrelatedChange`; a missing state is `Unknown`; a
/// non-matching value is `Mismatch`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeviceCommandVerifier;

impl DeviceCommandVerifier {
    pub fn verify(
        &self,
        target: &str,
        expected: &str,
        observation: &DeviceStateObservation,
    ) -> VerificationOutcome {
        if observation.device != target {
            return VerificationOutcome::UnrelatedChange;
        }
        match observation.state.as_deref() {
            Some(actual) if actual == expected => VerificationOutcome::Verified,
            Some(_) => VerificationOutcome::Mismatch,
            None => VerificationOutcome::Unknown,
        }
    }
}

// The verifier operates on canonical device identity strings, so
// callers may pass `MediaDeviceId::as_str()`, etc. These conveniences
// keep the verifier provider-neutral across the device classes.

/// Convenience: verify against a typed media device.
pub fn verify_media(
    verifier: &DeviceCommandVerifier,
    target: &MediaDeviceId,
    expected: &str,
    observation: &DeviceStateObservation,
) -> VerificationOutcome {
    verifier.verify(target.as_str(), expected, observation)
}

/// Convenience: verify against a typed appliance device.
pub fn verify_appliance(
    verifier: &DeviceCommandVerifier,
    target: &ApplianceDeviceId,
    expected: &str,
    observation: &DeviceStateObservation,
) -> VerificationOutcome {
    verifier.verify(target.as_str(), expected, observation)
}

/// Convenience: verify against a typed irrigation zone.
pub fn verify_irrigation(
    verifier: &DeviceCommandVerifier,
    target: &IrrigationZoneId,
    expected: &str,
    observation: &DeviceStateObservation,
) -> VerificationOutcome {
    verifier.verify(target.as_str(), expected, observation)
}

/// Convenience: verify against a typed vacuum device.
pub fn verify_vacuum(
    verifier: &DeviceCommandVerifier,
    target: &VacuumDeviceId,
    expected: &str,
    observation: &DeviceStateObservation,
) -> VerificationOutcome {
    verifier.verify(target.as_str(), expected, observation)
}
