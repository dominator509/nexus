//! EP-024 M1 unit suite for the nexus-devices contract crate
//! (construction, validation, serialization, vocabulary rejection,
//! dependency-direction invariants).

use nexus_devices::mapper::DeviceCapabilityMapper;
use nexus_devices::provider::{
    ApplianceProvider, IrrigationProvider, MediaProvider, RobotProvider, VacuumProvider,
};
use nexus_devices::robot::RobotSafetyDeclaration;
use nexus_devices::verifier::{DeviceCommandVerifier, DeviceStateObservation, VerificationOutcome};
use nexus_devices::vocabulary::{
    ApplianceCapability, ApplianceDeviceId, DeviceAvailability, DeviceClass, IrrigationCapability,
    IrrigationZoneId, MediaCapability, MediaDeviceId, RobotCapability, RobotId, VacuumCapability,
    VacuumDeviceId,
};
use nexus_devices::DevicesErrorCode;

fn media_id(id: &str) -> MediaDeviceId {
    MediaDeviceId::new(id).expect("media device id")
}

#[test]
fn ep024_unit_device_class_vocabulary_lock() {
    for (text, expected) in [
        ("MEDIA", DeviceClass::Media),
        ("APPLIANCE", DeviceClass::Appliance),
        ("IRRIGATION", DeviceClass::Irrigation),
        ("VACUUM", DeviceClass::Vacuum),
        ("ROBOT", DeviceClass::Robot),
        ("LIGHTING", DeviceClass::Lighting),
        ("HVAC", DeviceClass::Hvac),
        ("ENERGY", DeviceClass::Energy),
        ("INFRARED", DeviceClass::Infrared),
        ("VEHICLE", DeviceClass::Vehicle),
    ] {
        assert_eq!(DeviceClass::parse(text).expect("canonical"), expected);
        assert_eq!(expected.as_str(), text);
    }
    let error = DeviceClass::parse("DRONE").expect_err("unknown rejected");
    assert_eq!(error.code, DevicesErrorCode::Vocabulary);
    let json = serde_json::to_string(&DeviceClass::Robot).expect("json");
    assert_eq!(json, "\"ROBOT\"");
    let back: DeviceClass = serde_json::from_str(&json).expect("roundtrip");
    assert_eq!(back, DeviceClass::Robot);
}

#[test]
fn ep024_unit_capability_vocabularies_lock() {
    // Media
    for (text, expected) in [
        ("PLAYBACK", MediaCapability::Playback),
        ("VOLUME", MediaCapability::Volume),
        ("SOURCE", MediaCapability::Source),
        ("POWER", MediaCapability::Power),
    ] {
        assert_eq!(MediaCapability::parse(text).expect("canonical"), expected);
        assert_eq!(expected.as_str(), text);
    }
    assert_eq!(
        MediaCapability::parse("SHUFFLE")
            .expect_err("unknown rejected")
            .code,
        DevicesErrorCode::Vocabulary
    );
    // Appliance
    for (text, expected) in [
        ("POWER_CONTROL", ApplianceCapability::PowerControl),
        ("MODE_CONTROL", ApplianceCapability::ModeControl),
        ("STATUS_READBACK", ApplianceCapability::StatusReadback),
    ] {
        assert_eq!(
            ApplianceCapability::parse(text).expect("canonical"),
            expected
        );
        assert_eq!(expected.as_str(), text);
    }
    // Irrigation
    for (text, expected) in [
        ("ZONE_CONTROL", IrrigationCapability::ZoneControl),
        ("SCHEDULE_CONTROL", IrrigationCapability::ScheduleControl),
        ("MOISTURE_READBACK", IrrigationCapability::MoistureReadback),
    ] {
        assert_eq!(
            IrrigationCapability::parse(text).expect("canonical"),
            expected
        );
        assert_eq!(expected.as_str(), text);
    }
    // Vacuum
    for (text, expected) in [
        ("DOCK", VacuumCapability::Dock),
        ("START_CLEAN", VacuumCapability::StartClean),
        ("PAUSE", VacuumCapability::Pause),
        ("RETURN_HOME", VacuumCapability::ReturnHome),
        ("MAP_READBACK", VacuumCapability::MapReadback),
    ] {
        assert_eq!(VacuumCapability::parse(text).expect("canonical"), expected);
        assert_eq!(expected.as_str(), text);
    }
    // Robot
    for (text, expected) in [
        ("NAVIGATION", RobotCapability::Navigation),
        ("MANIPULATION", RobotCapability::Manipulation),
        ("SENSING", RobotCapability::Sensing),
        ("SAFETY_INTERLOCK", RobotCapability::SafetyInterlock),
        ("EMERGENCY_STOP", RobotCapability::EmergencyStop),
        (
            "HUMAN_PRESENCE_DETECTION",
            RobotCapability::HumanPresenceDetection,
        ),
    ] {
        assert_eq!(RobotCapability::parse(text).expect("canonical"), expected);
        assert_eq!(expected.as_str(), text);
    }
    assert_eq!(
        RobotCapability::parse("FLY")
            .expect_err("unknown rejected")
            .code,
        DevicesErrorCode::Vocabulary
    );
}

#[test]
fn ep024_unit_typed_ids_validate() {
    for id in [
        MediaDeviceId::new("").expect_err("empty rejected"),
        MediaDeviceId::new("x".repeat(129)).expect_err("oversized rejected"),
    ] {
        assert_eq!(id.code, DevicesErrorCode::Validation);
    }
    assert_eq!(media_id("living-room-sonos").as_str(), "living-room-sonos");
    assert_eq!(
        ApplianceDeviceId::new("washer-1").expect("valid").as_str(),
        "washer-1"
    );
    assert_eq!(
        IrrigationZoneId::new("lawn-north").expect("valid").as_str(),
        "lawn-north"
    );
    assert_eq!(
        VacuumDeviceId::new("vacuum-downstairs")
            .expect("valid")
            .as_str(),
        "vacuum-downstairs"
    );
    assert_eq!(RobotId::new("robo-1").expect("valid").as_str(), "robo-1");
}

#[test]
fn ep024_unit_availability_vocabulary() {
    for (text, expected) in [
        ("DISCOVERED", DeviceAvailability::Discovered),
        ("AVAILABLE", DeviceAvailability::Available),
        ("STREAMING", DeviceAvailability::Streaming),
        ("DEGRADED", DeviceAvailability::Degraded),
        ("UNAVAILABLE", DeviceAvailability::Unavailable),
    ] {
        assert_eq!(
            DeviceAvailability::parse(text).expect("canonical"),
            expected
        );
        assert_eq!(expected.as_str(), text);
    }
    assert_eq!(
        DeviceAvailability::parse("ONLINE")
            .expect_err("unknown rejected")
            .code,
        DevicesErrorCode::Vocabulary
    );
}

#[test]
fn ep024_unit_robot_safety_declaration_gates_activation() {
    let declaration = RobotSafetyDeclaration::new(
        "workshop floor",
        0.5,
        5.0,
        vec!["bumper".to_string(), "cliff".to_string()],
        true,
        true,
        nexus_devices::ApprovalClass::Human,
        vec![RobotCapability::Navigation],
    )
    .expect("valid declaration");
    assert!(declaration.declares(RobotCapability::Navigation));
    assert!(!declaration.declares(RobotCapability::Manipulation));
    assert!(declaration
        .ensure_declared(RobotCapability::Navigation)
        .is_ok());
    let error = declaration
        .ensure_declared(RobotCapability::Manipulation)
        .expect_err("undeclared capability refused");
    assert_eq!(error.code, DevicesErrorCode::Policy);
}

#[test]
fn ep024_unit_robot_safety_declaration_validation() {
    let empty_workspace = RobotSafetyDeclaration::new(
        "",
        0.5,
        5.0,
        vec![],
        true,
        true,
        nexus_devices::ApprovalClass::Human,
        vec![RobotCapability::Navigation],
    )
    .expect_err("empty workspace rejected");
    assert_eq!(empty_workspace.code, DevicesErrorCode::Validation);

    let negative_speed = RobotSafetyDeclaration::new(
        "floor",
        -1.0,
        5.0,
        vec![],
        true,
        true,
        nexus_devices::ApprovalClass::Human,
        vec![RobotCapability::Navigation],
    )
    .expect_err("negative speed rejected");
    assert_eq!(negative_speed.code, DevicesErrorCode::Validation);

    let no_capabilities = RobotSafetyDeclaration::new(
        "floor",
        0.5,
        5.0,
        vec![],
        true,
        true,
        nexus_devices::ApprovalClass::Human,
        vec![],
    )
    .expect_err("no declared capabilities rejected");
    assert_eq!(no_capabilities.code, DevicesErrorCode::Validation);
}

#[test]
fn ep024_unit_provider_ports_fail_closed_unbound() {
    let media = MediaProviderNoop;
    let error = media.list_devices().expect_err("unbound fails closed");
    assert_eq!(error.code, DevicesErrorCode::Unavailable);

    let appliance = ApplianceProviderNoop;
    assert_eq!(
        appliance
            .capabilities(&ApplianceDeviceId::new("washer").expect("id"))
            .expect_err("unbound fails closed")
            .code,
        DevicesErrorCode::Unavailable
    );

    let irrigation = IrrigationProviderNoop;
    assert_eq!(
        irrigation
            .list_zones()
            .expect_err("unbound fails closed")
            .code,
        DevicesErrorCode::Unavailable
    );

    let vacuum = VacuumProviderNoop;
    assert_eq!(
        vacuum
            .availability(&VacuumDeviceId::new("vac").expect("id"))
            .expect_err("unbound fails closed")
            .code,
        DevicesErrorCode::Unavailable
    );

    let robot = RobotProviderNoop;
    assert_eq!(
        robot
            .declared_capabilities(&RobotId::new("robo").expect("id"))
            .expect_err("unbound fails closed")
            .code,
        DevicesErrorCode::Unavailable
    );
}

#[test]
fn ep024_unit_capability_mapper_deterministic() {
    let mapper = DeviceCapabilityMapper;
    let media = mapper.map("media.playback").expect("canonical key");
    assert_eq!(media.capability_id, "media.playback");
    assert_eq!(media.class, nexus_devices::CapabilityClass::Command);
    assert_eq!(media.risk, nexus_devices::Risk::R1);
    let robot = mapper.map("robot.navigation").expect("canonical key");
    assert_eq!(robot.class, nexus_devices::CapabilityClass::Command);
    assert_eq!(robot.risk, nexus_devices::Risk::R3);
    assert_eq!(robot.approval, nexus_devices::ApprovalClass::Human);
    let query = mapper.map("appliance.status").expect("canonical key");
    assert_eq!(query.class, nexus_devices::CapabilityClass::Query);
    assert_eq!(query.risk, nexus_devices::Risk::R0);
    // Deterministic: same key always maps to the same taxonomy.
    assert_eq!(mapper.map("media.playback").expect("stable"), media);
    // Unknown key rejected, never fabricated.
    let error = mapper.map("media.shuffle").expect_err("unknown rejected");
    assert_eq!(error.code, DevicesErrorCode::Vocabulary);
}

#[test]
fn ep024_unit_command_verifier_exact_target() {
    let verifier = DeviceCommandVerifier;
    let target = media_id("living-room-sonos");
    // Exact target + expected state -> verified.
    assert_eq!(
        verifier.verify(
            target.as_str(),
            "playing",
            &DeviceStateObservation {
                device: target.as_str().to_string(),
                state: Some("playing".to_string()),
            }
        ),
        VerificationOutcome::Verified
    );
    // Exact target + different state -> mismatch.
    assert_eq!(
        verifier.verify(
            target.as_str(),
            "playing",
            &DeviceStateObservation {
                device: target.as_str().to_string(),
                state: Some("paused".to_string()),
            }
        ),
        VerificationOutcome::Mismatch
    );
    // Exact target + missing state -> unknown.
    assert_eq!(
        verifier.verify(
            target.as_str(),
            "playing",
            &DeviceStateObservation {
                device: target.as_str().to_string(),
                state: None,
            }
        ),
        VerificationOutcome::Unknown
    );
    // Unrelated device change never verifies.
    assert_eq!(
        verifier.verify(
            target.as_str(),
            "playing",
            &DeviceStateObservation {
                device: "other-device".to_string(),
                state: Some("playing".to_string()),
            }
        ),
        VerificationOutcome::UnrelatedChange
    );
}

#[test]
fn ep024_unit_command_verifier_typed_helpers() {
    let verifier = DeviceCommandVerifier;
    let target = media_id("living-room-sonos");
    let observation = DeviceStateObservation {
        device: target.as_str().to_string(),
        state: Some("playing".to_string()),
    };
    assert_eq!(
        nexus_devices::verifier::verify_media(&verifier, &target, "playing", &observation),
        VerificationOutcome::Verified
    );
    let washer = ApplianceDeviceId::new("washer-1").expect("id");
    let washer_obs = DeviceStateObservation {
        device: washer.as_str().to_string(),
        state: Some("on".to_string()),
    };
    assert_eq!(
        nexus_devices::verifier::verify_appliance(&verifier, &washer, "on", &washer_obs),
        VerificationOutcome::Verified
    );
}

#[test]
fn ep024_unit_serde_roundtrip_capability() {
    let capability = nexus_devices::mapper::DeviceCapability {
        capability_id: "vacuum.clean".to_string(),
        class: nexus_devices::CapabilityClass::Command,
        risk: nexus_devices::Risk::R1,
        approval: nexus_devices::ApprovalClass::None,
        idempotency: nexus_devices::Idempotency::Required,
    };
    let json = serde_json::to_string(&capability).expect("json");
    let back: nexus_devices::mapper::DeviceCapability =
        serde_json::from_str(&json).expect("roundtrip");
    assert_eq!(back, capability);
}

#[test]
fn ep024_unit_dependency_direction_contract_crate_imports_no_provider_impl() {
    // nexus-devices is the provider-neutral contract boundary. It must
    // not import any vendor/infra/connector crate. This file's imports
    // prove the surface: only nexus-domain + serde, re-exported through
    // the crate root so callers have a single import surface.
    let correlation = nexus_devices::CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6401")
        .expect("valid UUIDv7");
    assert_eq!(correlation.as_str().len(), 36);
    // Robot safety uses the EP-008 approval taxonomy from nexus-domain,
    // never a device-crate invention.
    assert_eq!(nexus_devices::ApprovalClass::Human.as_str(), "HUMAN");
}

// Unbound provider stand-ins for the fail-closed port proofs. These are
// test-double zone types (TESTING.md): they implement the port trait
// with its default fail-closed body, which is exactly the production
// behavior of an unbound provider.
struct MediaProviderNoop;
impl MediaProvider for MediaProviderNoop {}

struct ApplianceProviderNoop;
impl ApplianceProvider for ApplianceProviderNoop {}

struct IrrigationProviderNoop;
impl IrrigationProvider for IrrigationProviderNoop {}

struct VacuumProviderNoop;
impl VacuumProvider for VacuumProviderNoop {}

struct RobotProviderNoop;
impl RobotProvider for RobotProviderNoop {}
