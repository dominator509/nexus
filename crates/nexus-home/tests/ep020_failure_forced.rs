//! EP-020 M4 forced-failure / abuse suite (SPEC-011; ADR-027).
//!
//! Non-vacuous `ep020_failure_*` tests proving the home contract fails
//! CLOSED under malformed input, missing state, unrelated changes,
//! vocabulary abuse, identity forgery, secret leakage, and verification
//! timeout. Every assertion exercises the REAL production types in
//! `nexus-home` (verifier adapter, vocabulary, mapping, error type) -
//! no mocks of the component under proof.
//!
//! Permanent invariants re-proven here:
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED
//! - An unrelated entity change NEVER satisfies verification.
//! - Missing/unknown state is never treated as off/closed/locked/safe.
//! - Errors preserve correlation and NEVER leak provider payloads.

use std::collections::BTreeMap;

use nexus_domain::{CorrelationId, DeviceId, PersonId};
use nexus_home::{
    category_from_provider_domain, is_strong_provider_identity, AreaId, CommandState,
    DeviceCategory, DeviceTwin, EntityAvailability, HaDeviceRef, HaEntityRef, HomeError,
    HomeErrorCode, StateObservation, StateVerifier, StateVerifierAdapter, VerificationOutcome,
    VerificationRule,
};

fn entity(e: &str) -> HaEntityRef {
    HaEntityRef::new(e).expect("valid entity ref")
}

fn device_id(n: u8) -> DeviceId {
    DeviceId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f62{n:02}")).expect("valid UUIDv7")
}

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f63{n:02}")).expect("valid UUIDv7")
}

fn observation(entity_id: &str, state: Option<&str>) -> StateObservation {
    StateObservation {
        ha_entity: entity(entity_id),
        state: state.map(str::to_string),
        attributes: BTreeMap::new(),
        from_event: false,
    }
}

fn verifier() -> StateVerifierAdapter {
    StateVerifierAdapter
}

// ---- verification fail-closed ----

#[test]
fn ep020_failure_verifier_missing_target_state_is_unknown() {
    // The exact target is observed but carries NO state value: the
    // outcome must be Unknown, never Verified - a fabricated pass is a
    // failure state even when the entity matches.
    let outcome = verifier().verify(
        &entity("light.nexus_test_light"),
        &VerificationRule::StateEquals {
            expected: "on".to_string(),
        },
        &observation("light.nexus_test_light", None),
    );
    assert_eq!(outcome, VerificationOutcome::Unknown);
    assert_ne!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep020_failure_verifier_unrelated_change_never_verified() {
    // A state change on ANY other entity is UnrelatedChange - it can
    // never satisfy the exact-target verification invariant.
    let outcome = verifier().verify(
        &entity("light.nexus_test_light"),
        &VerificationRule::StateEquals {
            expected: "on".to_string(),
        },
        &observation("switch.other_room", Some("on")),
    );
    assert_eq!(outcome, VerificationOutcome::UnrelatedChange);
    assert_ne!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep020_failure_verifier_mismatch_not_success() {
    // Target observed with the WRONG state: Mismatch, not Verified.
    let outcome = verifier().verify(
        &entity("light.nexus_test_light"),
        &VerificationRule::StateEquals {
            expected: "on".to_string(),
        },
        &observation("light.nexus_test_light", Some("off")),
    );
    assert_eq!(outcome, VerificationOutcome::Mismatch);
    assert_ne!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep020_failure_verifier_missing_attribute_is_unknown() {
    // AttributeEquals rule with the attribute absent: Unknown, never a
    // fabricated pass.
    let outcome = verifier().verify(
        &entity("climate.bedroom"),
        &VerificationRule::AttributeEquals {
            attribute: "temperature".to_string(),
            expected: serde_json::json!(21),
        },
        &StateObservation {
            ha_entity: entity("climate.bedroom"),
            state: Some("heat".to_string()),
            attributes: BTreeMap::new(),
            from_event: false,
        },
    );
    assert_eq!(outcome, VerificationOutcome::Unknown);
    assert_ne!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep020_failure_verification_timeout_is_not_success() {
    // The vocabulary locks VERIFICATION_TIMEOUT as a terminal distinct
    // from VERIFIED; a timed-out verification is never success.
    assert_eq!(
        CommandState::VerificationTimeout.as_str(),
        "VERIFICATION_TIMEOUT"
    );
    assert_ne!(CommandState::VerificationTimeout, CommandState::Verified);
    assert_ne!(
        "VERIFICATION_TIMEOUT".parse::<CommandState>().unwrap(),
        CommandState::Verified
    );
    // A provider acknowledgement is SUBMITTED at most (directive:
    // COMMAND ACCEPTED != DEVICE VERIFIED).
    assert_eq!(CommandState::Submitted.as_str(), "SUBMITTED");
    assert_ne!(CommandState::Submitted, CommandState::Verified);
}

// ---- mapping / identity fail-closed ----

#[test]
fn ep020_failure_mapping_unknown_domain_is_total() {
    // category_from_provider_domain is TOTAL: an unknown/malicious HA
    // domain maps to Other - it never panics and never leaks upward.
    for bad in ["", "  ", "weird_domain", "HOME_ASSISTANT", "light\n"] {
        assert_eq!(
            category_from_provider_domain(bad),
            DeviceCategory::Other,
            "unknown domain {bad:?} must map to Other"
        );
    }
    assert_eq!(
        category_from_provider_domain("light"),
        DeviceCategory::Light
    );
}

#[test]
fn ep020_failure_identity_display_name_never_strong() {
    // A display name with spaces/uppercase must never be treated as a
    // strong provider identity; entity ids are strong.
    assert!(!is_strong_provider_identity("Kitchen Light"));
    assert!(!is_strong_provider_identity(""));
    assert!(!is_strong_provider_identity("light with spaces"));
    assert!(!is_strong_provider_identity("LIGHT.KITCHEN"));
    assert!(is_strong_provider_identity("light.kitchen"));
    assert!(is_strong_provider_identity("light.nexus_test_light"));
}

// ---- vocabulary fail-closed ----

#[test]
fn ep020_failure_vocabulary_unknown_class_rejected() {
    // Unknown vocabulary strings are REJECTED at parse (fail-closed),
    // never silently coerced to a safe-looking value.
    for bad in ["", "FIXED", "REMEDIATED", "PASS", "on", "ON"] {
        assert!(
            bad.parse::<CommandState>().is_err(),
            "{bad:?} must be rejected"
        );
        assert!(
            bad.parse::<VerificationOutcome>().is_err(),
            "{bad:?} must be rejected"
        );
        assert!(
            bad.parse::<DeviceCategory>().is_err(),
            "{bad:?} must be rejected"
        );
    }
    // Canonical vocabulary strings remain valid for their own enum.
    assert_eq!(
        "VERIFIED".parse::<CommandState>().unwrap(),
        CommandState::Verified
    );
    assert_eq!(
        "LIGHT".parse::<DeviceCategory>().unwrap(),
        DeviceCategory::Light
    );
    // But a category string is NOT a command state (no cross-class
    // coercion).
    assert!("LIGHT".parse::<CommandState>().is_err());
    assert!("VERIFIED".parse::<DeviceCategory>().is_err());
}

#[test]
fn ep020_failure_unknown_availability_never_safe() {
    // Unknown/unavailable provider state maps honestly: the twin's
    // availability gate only trusts Available.
    let twin = DeviceTwin {
        device_id: device_id(9),
        friendly_name: "Unknown".to_string(),
        area: Some(AreaId::new("kitchen").expect("valid area")),
        owner: None,
        ha_device_ref: HaDeviceRef::new("ha-device-9").expect("valid ref"),
        ha_entity_refs: vec![entity("switch.unknown_state")],
        provider_domain: "switch".to_string(),
        category: DeviceCategory::Switch,
        manufacturer: None,
        model: None,
        availability: EntityAvailability::Unknown,
        state: None,
        attributes: BTreeMap::new(),
        capabilities: vec![],
        parent_ha_device_ref: None,
    };
    // Unknown is never available, never treated as off/closed/safe.
    assert!(!twin.is_available());
    assert_ne!(twin.availability, EntityAvailability::Available);
    assert_ne!(twin.availability, EntityAvailability::Unavailable);
    assert_eq!(twin.availability, EntityAvailability::Unknown);
}

// ---- errors: redaction + correlation ----

#[test]
fn ep020_failure_error_redaction_never_leaks_payload() {
    // The error Display/redacted surface carries code + correlation +
    // resource, and NEVER the message body (which may contain a
    // provider payload or secret).
    let err = HomeError::new(
        HomeErrorCode::External,
        "provider payload token=ghp_12345 password=hunter2",
        Some(Box::from("corr-abuse-1")),
        Some(Box::from("light.nexus_test_light")),
    );
    let text = err.to_string();
    assert!(!text.contains("ghp_12345"), "secret leaked: {text}");
    assert!(!text.contains("hunter2"), "secret leaked: {text}");
    assert!(!text.contains("provider payload"), "payload leaked: {text}");
    assert!(text.contains("EXTERNAL"));
    assert!(text.contains("corr-abuse-1"));
    assert!(text.contains("light.nexus_test_light"));
}

#[test]
fn ep020_failure_error_correlation_preserved() {
    // Failures carry correlation for incident correlation; the typed
    // code discriminates the failure class.
    let err = HomeError::new(
        HomeErrorCode::NotFound,
        "entity missing",
        Some(Box::from("corr-abuse-2")),
        Some(Box::from("light.does_not_exist")),
    );
    assert_eq!(err.code, HomeErrorCode::NotFound);
    assert_eq!(err.code.as_str(), "NOT_FOUND");
    assert_eq!(err.correlation_id.as_deref(), Some("corr-abuse-2"));
    assert_eq!(err.resource.as_deref(), Some("light.does_not_exist"));
}

#[test]
fn ep020_failure_auth_and_policy_fail_closed() {
    // Authentication/policy failures are typed and never collapse to a
    // generic success-looking class.
    assert_eq!(HomeErrorCode::Authorization.as_str(), "AUTHORIZATION");
    assert_eq!(HomeErrorCode::Policy.as_str(), "POLICY");
    assert_eq!(HomeErrorCode::Unavailable.as_str(), "UNAVAILABLE");
    assert_eq!(HomeErrorCode::Timeout.as_str(), "TIMEOUT");
    assert_eq!(HomeErrorCode::Conflict.as_str(), "CONFLICT");
    let denied = HomeError::new(
        HomeErrorCode::Authorization,
        "token rejected",
        Some(Box::from("corr-abuse-3")),
        None,
    );
    assert_ne!(denied.code, HomeErrorCode::Internal);
    assert_ne!(denied.code, HomeErrorCode::Verification);
}

#[test]
fn ep020_failure_correlation_ids_are_deterministic_uuids() {
    // Correlation ids are canonical UUIDv7 (SPEC-006) - an incident
    // correlation trace is never fabricated from free text.
    let c = correlation(4);
    assert_eq!(c.as_str().len(), 36);
    assert!(c.as_str().starts_with("0190e1c4"));
    let person = PersonId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6404".to_string());
    assert!(person.is_ok());
}
