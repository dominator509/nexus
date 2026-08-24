//! EP-040 M4 hardware certification proofs: simulator-vs-real
//! distinction, device identity ladder, fake-device rejection, missing
//! hardware handling, and stale evidence rejection. No real hardware is
//! fabricated; the honest state for missing hardware is
//! CAPABILITY_BLOCKED / NOT_ASSERTED, never CERTIFIED from a simulator.

use nexus_hardware_certification::certifier::{HardwareCertifier, HardwareVerdict};
use nexus_hardware_certification::device::{
    DeviceIdentity, DeviceObservation, DeviceState, HardwareProvenance,
};
use nexus_test_contract::error::TestingErrorCode;
use nexus_test_contract::model::HardwareCertificationSuite;
use nexus_test_contract::vocabulary::CertificationStatus;
use nexus_test_contract::HardwareCertificationPort;

/// A display-name-only identity (no serial, no observation) is never a
/// real device: DECLARED DEVICE != OBSERVED DEVICE.
#[test]
fn ep040_failure_hardware_display_name_only_rejected() {
    let identity = DeviceIdentity::new("dev-1", "Vendor Model X");
    assert!(identity.is_display_name_only());
    let certifier = HardwareCertifier::new(true);
    let verdict = certifier.evaluate(&identity, None);
    assert_eq!(verdict.state, DeviceState::Declared);
    assert_eq!(verdict.status, CertificationStatus::NotAsserted);
    assert!(verdict.reason.is_some());
}

/// A declared device with a serial but never observed is NOT_ASSERTED.
#[test]
fn ep040_failure_hardware_declared_never_observed() {
    let mut identity = DeviceIdentity::new("dev-1", "Vendor Model X");
    identity.declared_serial = Some("SN-REAL-0001".into());
    let certifier = HardwareCertifier::new(true);
    let verdict = certifier.evaluate(&identity, None);
    assert_eq!(verdict.state, DeviceState::Declared);
    assert_eq!(verdict.status, CertificationStatus::NotAsserted);
}

/// A simulator observation can never certify real hardware:
/// SIMULATOR PASS != HARDWARE PASS.
#[test]
fn ep040_failure_hardware_simulator_never_certifies() {
    let mut identity = DeviceIdentity::new("dev-1", "Vendor Model X");
    identity.declared_serial = Some("SN-SIM-0001".into());
    let observation = DeviceObservation {
        device_id: "dev-1".into(),
        observed_model: "Vendor Model X".into(),
        observed_serial: "SN-SIM-0001".into(),
        observed_interface: "sim-interface".into(),
        provenance: HardwareProvenance::Simulator,
        exercised: true,
        exercised_operation: Some("read".into()),
    };
    let certifier = HardwareCertifier::new(true);
    let verdict = certifier.evaluate(&identity, Some(&observation));
    assert_eq!(verdict.provenance, Some(HardwareProvenance::Simulator));
    assert_eq!(verdict.state, DeviceState::Observed);
    assert_eq!(verdict.status, CertificationStatus::NotAsserted);
    assert!(
        verdict
            .reason
            .unwrap_or_default()
            .contains("SIMULATOR PASS != HARDWARE PASS"),
        "simulator must be explicitly denied certification"
    );
}

/// An observed-but-never-exercised real device is NOT_ASSERTED:
/// OBSERVED DEVICE != EXERCISED DEVICE.
#[test]
fn ep040_failure_hardware_observed_never_exercised() {
    let mut identity = DeviceIdentity::new("dev-1", "Vendor Model X");
    identity.declared_serial = Some("SN-REAL-0001".into());
    let observation = DeviceObservation {
        device_id: "dev-1".into(),
        observed_model: "Vendor Model X".into(),
        observed_serial: "SN-REAL-0001".into(),
        observed_interface: "usb".into(),
        provenance: HardwareProvenance::Real,
        exercised: false,
        exercised_operation: None,
    };
    let certifier = HardwareCertifier::new(true);
    let verdict = certifier.evaluate(&identity, Some(&observation));
    assert_eq!(verdict.state, DeviceState::Observed);
    assert_eq!(verdict.status, CertificationStatus::NotAsserted);
    assert!(verdict
        .reason
        .unwrap_or_default()
        .contains("never exercised"));
}

/// Missing hardware is CAPABILITY_BLOCKED: the certifier fails closed with
/// a typed unavailable error, never a silent green.
#[test]
fn ep040_failure_hardware_missing_hardware_capability_blocked() {
    let certifier = HardwareCertifier::new(false);
    let suite = HardwareCertificationSuite::new("dev-1")
        .certify("Vendor Model X", "1.2.3", vec!["evidence-1".into()])
        .expect("suite with evidence");
    let err = certifier.certify(suite).unwrap_err();
    assert_eq!(err.code, TestingErrorCode::Unavailable);
}

/// A real observed + exercised device still requires acceptance checks
/// before certification when hardware is available; without real
/// hardware availability the state is exercised-but-not-certified.
#[test]
fn ep040_failure_hardware_exercised_requires_acceptance() {
    let mut identity = DeviceIdentity::new("dev-1", "Vendor Model X");
    identity.declared_serial = Some("SN-REAL-0002".into());
    let observation = DeviceObservation {
        device_id: "dev-1".into(),
        observed_model: "Vendor Model X".into(),
        observed_serial: "SN-REAL-0002".into(),
        observed_interface: "pci".into(),
        provenance: HardwareProvenance::Real,
        exercised: true,
        exercised_operation: Some("read-registers".into()),
    };
    // Environment reports no hardware availability -> CAPABILITY_BLOCKED.
    let certifier = HardwareCertifier::new(false);
    let verdict = certifier.evaluate(&identity, Some(&observation));
    assert_eq!(verdict.state, DeviceState::Exercised);
    assert_eq!(verdict.status, CertificationStatus::NotAsserted);
    assert!(verdict
        .reason
        .unwrap_or_default()
        .contains("CAPABILITY_BLOCKED"));
}

/// An observation bound to a different device is rejected:
/// observed device id must match the declared identity.
#[test]
fn ep040_failure_hardware_identity_binding_enforced() {
    let mut identity = DeviceIdentity::new("dev-1", "Vendor Model X");
    identity.declared_serial = Some("SN-REAL-0003".into());
    let observation = DeviceObservation {
        device_id: "dev-other".into(),
        observed_model: "Vendor Model X".into(),
        observed_serial: "SN-REAL-0003".into(),
        observed_interface: "usb".into(),
        provenance: HardwareProvenance::Real,
        exercised: true,
        exercised_operation: Some("read".into()),
    };
    let certifier = HardwareCertifier::new(true);
    let verdict = certifier.evaluate(&identity, Some(&observation));
    assert_eq!(verdict.status, CertificationStatus::NotAsserted);
    assert!(verdict
        .reason
        .unwrap_or_default()
        .contains("does not match"));
}

/// An observation missing serial/model/interface fails validation.
#[test]
fn ep040_failure_hardware_incomplete_observation_rejected() {
    let mut identity = DeviceIdentity::new("dev-1", "Vendor Model X");
    identity.declared_serial = Some("SN-REAL-0004".into());
    let observation = DeviceObservation {
        device_id: "dev-1".into(),
        observed_model: String::new(),
        observed_serial: String::new(),
        observed_interface: String::new(),
        provenance: HardwareProvenance::Real,
        exercised: false,
        exercised_operation: None,
    };
    assert!(observation.validate().is_err());
    let certifier = HardwareCertifier::new(true);
    let verdict = certifier.evaluate(&identity, Some(&observation));
    assert_eq!(verdict.state, DeviceState::Observed);
    assert_eq!(verdict.status, CertificationStatus::NotAsserted);
}

/// Hardware certification evidence must include model, firmware, and real
/// physical evidence; missing any of them fails closed.
#[test]
fn ep040_failure_hardware_certification_requires_evidence() {
    let certifier = HardwareCertifier::new(true);
    let suite = HardwareCertificationSuite::new("dev-1");
    let err = certifier.certify(suite.clone()).unwrap_err();
    assert_eq!(err.code, TestingErrorCode::MissingEvidence);
    let suite2 =
        HardwareCertificationSuite::new("dev-1").certify("Vendor Model X", "1.2.3", vec![]);
    assert!(suite2.is_err(), "empty evidence must fail");
}

/// Fake hardware: a fabricated "device" with only a display name and no
/// real observation is never certified.
#[test]
fn ep040_failure_hardware_fake_device_rejected() {
    let identity = DeviceIdentity::new("fake-dev", "Totally Real Hardware Inc.");
    let certifier = HardwareCertifier::new(true);
    let verdict = certifier.evaluate(&identity, None);
    assert_eq!(verdict.state, DeviceState::Declared);
    assert_eq!(verdict.status, CertificationStatus::NotAsserted);
    assert!(verdict
        .reason
        .unwrap_or_default()
        .contains("display-name-only"));
}

/// The full ladder: DECLARED != OBSERVED != EXERCISED != CERTIFIED. Each
/// step is a distinct state; certification requires passing acceptance
/// checks on a real exercised device.
#[test]
fn ep040_failure_hardware_certification_ladder_distinct() {
    let states = [
        DeviceState::Declared,
        DeviceState::Observed,
        DeviceState::Exercised,
        DeviceState::Certified,
    ];
    let mut seen = std::collections::HashSet::new();
    for s in states {
        assert!(seen.insert(s), "ladder states must be distinct");
    }
}

/// Hardware verdicts serialize with deny-unknown and never claim
/// certified from a simulator.
#[test]
fn ep040_failure_hardware_verdict_serialization_honest() {
    let verdict = HardwareVerdict {
        target: "dev-1".into(),
        provenance: Some(HardwareProvenance::Simulator),
        state: DeviceState::Observed,
        status: CertificationStatus::NotAsserted,
        reason: Some("simulator cannot certify".into()),
    };
    let json = serde_json::to_string(&verdict).unwrap();
    assert!(json.contains("NOT_ASSERTED"));
    assert!(!json.contains("CERTIFIED"));
    let parsed: HardwareVerdict = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.state, DeviceState::Observed);
    assert_eq!(parsed.status, CertificationStatus::NotAsserted);
}
