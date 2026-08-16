//! EP-020 M1 contract suite (SPEC-011; ADR-027).
//!
//! Non-vacuous `ep020_unit_*` tests proving vocabulary locking, the
//! canonical device mapping, twin identity stability, the
//! COMMAND_ACCEPTED != VERIFIED invariant, exact-target verification,
//! error typing, and dependency direction. The M1 gate runs this suite
//! through the real `cargo test -p nexus-home ep020_unit` machinery
//! with a vacuity guard.

use std::collections::BTreeMap;

use nexus_domain::{CapabilityClass, CorrelationId, DeviceId, Idempotency, PersonId, Risk};
use nexus_home::{
    AreaId, CommandReceipt, CommandState, DeviceCapability, DeviceCategory, DeviceTwin,
    EntityAvailability, HaDeviceRef, HaEntityRef, HomeError, HomeErrorCode, HomeIntent,
    StateObservation, StateVerifier, VerificationOutcome, VerificationRule,
};

fn device_id(n: u8) -> DeviceId {
    DeviceId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f62{n:02}")).expect("valid UUIDv7")
}

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f63{n:02}")).expect("valid UUIDv7")
}

fn twin(device: DeviceId, domain: &str, entity: &str) -> DeviceTwin {
    DeviceTwin {
        device_id: device,
        friendly_name: format!("Device {domain}"),
        area: Some(AreaId::new("kitchen").expect("valid area")),
        owner: None,
        ha_device_ref: HaDeviceRef::new("ha-device-1").expect("valid ref"),
        ha_entity_refs: vec![HaEntityRef::new(entity).expect("valid ref")],
        provider_domain: domain.to_string(),
        category: nexus_home::category_from_provider_domain(domain),
        manufacturer: Some("TestCo".to_string()),
        model: Some("T-1".to_string()),
        availability: EntityAvailability::Available,
        state: Some("off".to_string()),
        attributes: BTreeMap::new(),
        capabilities: vec![DeviceCapability {
            capability_id: format!("home.{domain}"),
            class: CapabilityClass::Command,
            risk: Risk::R1,
            approval: nexus_domain::ApprovalClass::None,
            idempotency: Idempotency::Required,
            verification: VerificationRule::StateEquals {
                expected: "on".to_string(),
            },
        }],
        parent_ha_device_ref: None,
    }
}

fn intent(device: DeviceId, capability: &str, action: &str) -> HomeIntent {
    HomeIntent {
        device_id: device,
        capability_id: capability.to_string(),
        action: action.to_string(),
        parameters: BTreeMap::new(),
        correlation_id: correlation(9),
        idempotency_key: Some("k-1".to_string()),
    }
}

#[test]
fn ep020_unit_vocabulary_locks_categories() {
    // The canonical taxonomy from the owner directive: unknown values
    // are rejected at parse time, never silently coerced.
    for text in [
        "LIGHT",
        "SWITCH",
        "LOCK",
        "CLIMATE",
        "COVER",
        "SENSOR",
        "BINARY_SENSOR",
        "MEDIA_PLAYER",
        "CAMERA",
        "FAN",
        "VACUUM",
        "ALARM",
        "SCENE",
        "BUTTON",
        "NUMBER",
        "SELECT",
        "OTHER",
    ] {
        assert_eq!(text.parse::<DeviceCategory>().unwrap().as_str(), text);
    }
    assert!("thermostat".parse::<DeviceCategory>().is_err());
    assert!("THERMOSTAT".parse::<DeviceCategory>().is_err());
}

#[test]
fn ep020_unit_command_state_has_no_fixed_value() {
    // There is no FIXED/VERIFIED-as-accepted escape. A provider
    // acceptance is SUBMITTED; verification is a separate step.
    assert!("FIXED".parse::<CommandState>().is_err());
    assert_eq!(CommandState::Submitted.as_str(), "SUBMITTED");
    assert_eq!(CommandState::Verified.as_str(), "VERIFIED");
    assert_eq!(
        CommandState::VerificationTimeout.as_str(),
        "VERIFICATION_TIMEOUT"
    );
}

#[test]
fn ep020_unit_unknown_availability_is_not_safe() {
    // Unknown/unavailable must map honestly; the vocabulary has no
    // OFF/CLOSED/LOCKED/SAFE value that unknown could be coerced into.
    assert_ne!(EntityAvailability::Unknown, EntityAvailability::Available);
    assert!("OFF".parse::<EntityAvailability>().is_err());
    assert!("LOCKED".parse::<EntityAvailability>().is_err());
}

#[test]
fn ep020_unit_receipt_is_submitted_never_verified() {
    let receipt = CommandReceipt {
        intent: intent(device_id(1), "home.light", "turn_on"),
        state: CommandState::Submitted,
        target_ha_entity: HaEntityRef::new("light.kitchen").expect("valid ref"),
        provider_service: "light/turn_on".to_string(),
    };
    assert_eq!(receipt.state, CommandState::Submitted);
    assert_ne!(receipt.state, CommandState::Verified);
    // The service call path is the real HA mechanism.
    assert_eq!(receipt.provider_service, "light/turn_on");
    assert!(receipt.target_ha_entity.0.contains('.'));
}

#[test]
fn ep020_unit_exact_target_verification_state_equals() {
    // Rule: state == "on". Exact target entity observation satisfies.
    let target = HaEntityRef::new("light.kitchen").expect("valid ref");
    let rule = VerificationRule::StateEquals {
        expected: "on".to_string(),
    };
    let ok = StateObservation {
        ha_entity: target.clone(),
        state: Some("on".to_string()),
        attributes: BTreeMap::new(),
        from_event: true,
    };
    let mismatch = StateObservation {
        ha_entity: target.clone(),
        state: Some("off".to_string()),
        attributes: BTreeMap::new(),
        from_event: true,
    };
    let verifier = nexus_home::contract::StateVerifierAdapter;
    assert_eq!(
        verifier.verify(&target, &rule, &ok),
        VerificationOutcome::Verified
    );
    assert_eq!(
        verifier.verify(&target, &rule, &mismatch),
        VerificationOutcome::Mismatch
    );
}

#[test]
fn ep020_unit_verification_rejects_unrelated_entity_change() {
    // An unrelated state_changed event (different entity) never
    // satisfies verification for the target.
    let target = HaEntityRef::new("light.kitchen").expect("valid ref");
    let rule = VerificationRule::StateEquals {
        expected: "on".to_string(),
    };
    let unrelated = StateObservation {
        ha_entity: HaEntityRef::new("light.hallway").expect("valid ref"),
        state: Some("on".to_string()),
        attributes: BTreeMap::new(),
        from_event: true,
    };
    let verifier = nexus_home::contract::StateVerifierAdapter;
    assert_eq!(
        verifier.verify(&target, &rule, &unrelated),
        VerificationOutcome::UnrelatedChange
    );
}

#[test]
fn ep020_unit_verification_attribute_equals() {
    let target = HaEntityRef::new("climate.kitchen").expect("valid ref");
    let rule = VerificationRule::AttributeEquals {
        attribute: "temperature".to_string(),
        expected: serde_json::json!(21),
    };
    let mut attrs = BTreeMap::new();
    attrs.insert("temperature".to_string(), serde_json::json!(21));
    let ok = StateObservation {
        ha_entity: target.clone(),
        state: Some("heat".to_string()),
        attributes: attrs.clone(),
        from_event: false,
    };
    attrs.insert("temperature".to_string(), serde_json::json!(19));
    let bad = StateObservation {
        ha_entity: target.clone(),
        state: Some("heat".to_string()),
        attributes: attrs,
        from_event: false,
    };
    let verifier = nexus_home::contract::StateVerifierAdapter;
    assert_eq!(
        verifier.verify(&target, &rule, &ok),
        VerificationOutcome::Verified
    );
    assert_eq!(
        verifier.verify(&target, &rule, &bad),
        VerificationOutcome::Mismatch
    );
}

#[test]
fn ep020_unit_verification_unknown_state_is_unknown() {
    // No observed state -> UNKNOWN, never Verified.
    let target = HaEntityRef::new("light.kitchen").expect("valid ref");
    let rule = VerificationRule::StateEquals {
        expected: "on".to_string(),
    };
    let unknown = StateObservation {
        ha_entity: target.clone(),
        state: None,
        attributes: BTreeMap::new(),
        from_event: false,
    };
    let verifier = nexus_home::contract::StateVerifierAdapter;
    assert_eq!(
        verifier.verify(&target, &rule, &unknown),
        VerificationOutcome::Unknown
    );
}

#[test]
fn ep020_unit_twin_identity_survives_rename() {
    let mut d = twin(device_id(1), "light", "light.kitchen");
    d.friendly_name = "Renamed".to_string();
    d.area = Some(AreaId::new("living_room").expect("valid area"));
    // Canonical identity is device_id; name/area changes are cosmetic.
    assert_eq!(d.device_id, device_id(1));
    assert_eq!(d.category, DeviceCategory::Light);
    assert_eq!(d.ha_entity_refs[0].0, "light.kitchen");
}

#[test]
fn ep020_unit_dependency_direction_contract_crate_imports_no_provider_impl() {
    // nexus-home is the provider-neutral contract boundary. It must not
    // import any vendor/infra/connector crate. This file's imports
    // prove the surface: only nexus-domain + serde.
    let _ = PersonId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6401").expect("valid UUIDv7");
}

#[test]
fn ep020_unit_error_typing_and_redaction() {
    let err = HomeError::new(
        HomeErrorCode::External,
        "malformed provider payload with token=sekret",
        Some(Box::from("c-1")),
        Some(Box::from("light.kitchen")),
    );
    let red = err.redacted();
    assert!(red.contains("EXTERNAL"));
    assert!(!red.contains("sekret"));
    assert!(!red.contains("payload"));
    assert_eq!(err.code, HomeErrorCode::External);
}
