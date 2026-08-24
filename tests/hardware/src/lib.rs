//! nexus-hardware-certification: EP-040 M4 hardware certification
//! behavior (SPEC-008; node contract HardwareCertificationSuite).
//!
//! Permanent invariants proven by tests:
//! - DECLARED DEVICE != OBSERVED DEVICE
//! - OBSERVED DEVICE != EXERCISED DEVICE
//! - EXERCISED DEVICE != CERTIFIED DEVICE
//! - SIMULATOR PASS != HARDWARE PASS
//! - MISSING HARDWARE != HARDWARE GREEN
//! - FAKE DISPLAY-NAME-ONLY IDENTITY != OBSERVED DEVICE
//!
//! No real hardware is fabricated. If no real hardware is present, the
//! honest certification state is CAPABILITY_BLOCKED / NOT_ASSERTED, never
//! CERTIFIED from a simulator.

pub mod certifier;
pub mod device;

pub use certifier::{HardwareCertifier, HardwareVerdict};
pub use device::{DeviceIdentity, DeviceObservation, DeviceState, HardwareProvenance};
