//! Device identity ladder: DECLARED != OBSERVED != EXERCISED != CERTIFIED.
//! A declared display-name-only identity is never an observed device; a
//! simulator observation is never a real exercised device.

use nexus_test_contract::error::{TestingError, TestingResult};

/// A declared device identity (what a manifest or config claims).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceIdentity {
    /// Canonical device id.
    pub id: String,
    /// Declared physical model.
    pub declared_model: String,
    /// Declared interface (e.g. USB, PCI, network).
    pub declared_interface: String,
    /// Optional declared serial. A display-name-only identity has no
    /// serial and no observation - it can never be certified.
    pub declared_serial: Option<String>,
}

impl DeviceIdentity {
    pub fn new(id: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            declared_model: model.into(),
            declared_interface: String::new(),
            declared_serial: None,
        }
    }

    /// A declared identity without a serial and without an observation is
    /// display-name-only: it cannot be the basis of any certification.
    pub fn is_display_name_only(&self) -> bool {
        self.declared_serial.is_none()
    }
}

/// An observation of a device. OBSERVED DEVICE != EXERCISED DEVICE: an
/// observation proves the device answered; it does not prove an
/// operation was exercised against it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceObservation {
    /// Device id observed.
    pub device_id: String,
    /// Physical model observed from the real device.
    pub observed_model: String,
    /// Observed serial (required for a real observation).
    pub observed_serial: String,
    /// Observed interface.
    pub observed_interface: String,
    /// Whether the observation came from a real device or a simulator.
    pub provenance: HardwareProvenance,
    /// Whether an operation was actually exercised against the device.
    pub exercised: bool,
    /// Operation exercised (only meaningful when exercised).
    pub exercised_operation: Option<String>,
}

impl DeviceObservation {
    /// A real observation must have model, serial, and interface. A
    /// simulator observation is explicitly classified as simulator.
    pub fn validate(&self) -> TestingResult<()> {
        if self.observed_model.trim().is_empty() {
            return Err(TestingError::validation(
                "device observation requires an observed model",
            ));
        }
        if self.observed_serial.trim().is_empty() {
            return Err(TestingError::validation(
                "device observation requires an observed serial",
            ));
        }
        if self.observed_interface.trim().is_empty() {
            return Err(TestingError::validation(
                "device observation requires an observed interface",
            ));
        }
        if self.exercised && self.exercised_operation.is_none() {
            return Err(TestingError::validation(
                "exercised device requires an exercised operation",
            ));
        }
        Ok(())
    }

    /// A simulator observation can never be treated as a real exercised
    /// device.
    pub fn is_real(&self) -> bool {
        self.provenance == HardwareProvenance::Real
    }
}

/// State of a device on the certification ladder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviceState {
    /// Only a declared identity exists.
    Declared,
    /// The device was observed (real or simulator).
    Observed,
    /// An operation was exercised against the device.
    Exercised,
    /// The device passed acceptance and is certified.
    Certified,
    /// Hardware is missing / capability is blocked.
    CapabilityBlocked,
}

/// Provenance of a hardware observation. SIMULATOR PASS != HARDWARE PASS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HardwareProvenance {
    /// Observation from a simulator.
    Simulator,
    /// Observation from real hardware.
    Real,
}
