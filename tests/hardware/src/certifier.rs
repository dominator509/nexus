//! Hardware certifier: only a real observed, exercised device with
//! model, firmware, and evidence can be CERTIFIED. Simulator evidence is
//! simulated-certified at most; missing hardware is CAPABILITY_BLOCKED;
//! fake display-name-only identities are rejected.
use nexus_test_contract::error::{TestingError, TestingErrorCode, TestingResult};
use nexus_test_contract::model::HardwareCertificationSuite;
use nexus_test_contract::vocabulary::CertificationStatus;
use nexus_test_contract::HardwareCertificationPort;

use crate::device::{DeviceIdentity, DeviceObservation, DeviceState, HardwareProvenance};

/// The honest certification outcome for a hardware target.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardwareVerdict {
    /// Target id.
    pub target: String,
    /// Provenance of the evidence (simulator or real).
    pub provenance: Option<HardwareProvenance>,
    /// Final device state on the ladder.
    pub state: DeviceState,
    /// Certification status (NOT_ASSERTED / CERTIFIED / FAILED).
    pub status: CertificationStatus,
    /// Reason for a non-certified outcome (never a fabricated pass).
    pub reason: Option<String>,
}

/// Real hardware certifier. Certifying requires a real observed,
/// exercised device plus model/firmware/evidence; anything less fails
/// closed to NOT_ASSERTED or CAPABILITY_BLOCKED.
#[derive(Debug, Clone)]
pub struct HardwareCertifier {
    /// Whether real hardware is available in this environment.
    pub hardware_available: bool,
}

impl HardwareCertifier {
    pub fn new(hardware_available: bool) -> Self {
        Self { hardware_available }
    }

    /// Evaluate one device up the ladder. Returns the honest verdict.
    pub fn evaluate(
        &self,
        identity: &DeviceIdentity,
        observation: Option<&DeviceObservation>,
    ) -> HardwareVerdict {
        // A display-name-only identity (no serial, no observation) is
        // never a real device.
        if identity.is_display_name_only() {
            return HardwareVerdict {
                target: identity.id.clone(),
                provenance: observation.map(|o| o.provenance),
                state: DeviceState::Declared,
                status: CertificationStatus::NotAsserted,
                reason: Some(
                    "display-name-only identity: no observed serial, no observation".into(),
                ),
            };
        }
        let Some(obs) = observation else {
            // Declared with serial but never observed.
            return HardwareVerdict {
                target: identity.id.clone(),
                provenance: None,
                state: DeviceState::Declared,
                status: CertificationStatus::NotAsserted,
                reason: Some("device declared but never observed".into()),
            };
        };
        // Identity binding: the observation must be for this device.
        if obs.device_id != identity.id {
            return HardwareVerdict {
                target: identity.id.clone(),
                provenance: Some(obs.provenance),
                state: DeviceState::Declared,
                status: CertificationStatus::NotAsserted,
                reason: Some("observation device_id does not match declared identity".into()),
            };
        }
        if obs.validate().is_err() {
            return HardwareVerdict {
                target: identity.id.clone(),
                provenance: Some(obs.provenance),
                state: DeviceState::Observed,
                status: CertificationStatus::NotAsserted,
                reason: Some("observation missing model/serial/interface".into()),
            };
        }
        if obs.provenance == HardwareProvenance::Simulator {
            // SIMULATOR PASS != HARDWARE PASS: the simulator proves the
            // adapter, never real hardware.
            return HardwareVerdict {
                target: identity.id.clone(),
                provenance: Some(HardwareProvenance::Simulator),
                state: DeviceState::Observed,
                status: CertificationStatus::NotAsserted,
                reason: Some(
                    "simulator observation cannot certify real hardware (SIMULATOR PASS != HARDWARE PASS)"
                        .into(),
                ),
            };
        }
        if !obs.exercised {
            // OBSERVED != EXERCISED.
            return HardwareVerdict {
                target: identity.id.clone(),
                provenance: Some(HardwareProvenance::Real),
                state: DeviceState::Observed,
                status: CertificationStatus::NotAsserted,
                reason: Some("device observed but never exercised".into()),
            };
        }
        // Real observed + exercised device: certifiable if hardware is
        // actually available in this environment.
        if !self.hardware_available {
            return HardwareVerdict {
                target: identity.id.clone(),
                provenance: Some(HardwareProvenance::Real),
                state: DeviceState::Exercised,
                status: CertificationStatus::NotAsserted,
                reason: Some(
                    "real device observed and exercised, but environment reports no hardware availability (CAPABILITY_BLOCKED)"
                        .into(),
                ),
            };
        }
        HardwareVerdict {
            target: identity.id.clone(),
            provenance: Some(HardwareProvenance::Real),
            state: DeviceState::Exercised,
            status: CertificationStatus::NotAsserted,
            reason: Some("acceptance checks required before certification".into()),
        }
    }
}

impl HardwareCertificationPort for HardwareCertifier {
    fn certify(
        &self,
        suite: HardwareCertificationSuite,
    ) -> TestingResult<HardwareCertificationSuite> {
        if suite.model.trim().is_empty() || suite.firmware.trim().is_empty() {
            return Err(TestingError::missing_evidence(
                "hardware certification requires model and firmware",
            ));
        }
        if suite.evidence.is_empty() {
            return Err(TestingError::missing_evidence(
                "hardware certification requires real physical evidence",
            ));
        }
        if !self.hardware_available {
            return Err(TestingError::new(
                TestingErrorCode::Unavailable,
                "hardware certification capability blocked: no real hardware available",
            ));
        }
        suite.clone().certify(
            suite.model.clone(),
            suite.firmware.clone(),
            suite.evidence.clone(),
        )
    }
}
