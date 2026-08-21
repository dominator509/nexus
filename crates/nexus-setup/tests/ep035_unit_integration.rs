//! EP-035 M2 IntegrationCard truthfulness tests (SPEC-004).

use nexus_domain::CorrelationId;
use nexus_setup::{
    is_valid_integration_transition, IntegrationCard, IntegrationId, IntegrationStatus,
    SetupErrorCode,
};

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("00000000-0000-7000-8000-00000000000{n}")).unwrap()
}

fn integration_id(n: u8) -> IntegrationId {
    IntegrationId::new(format!("integration-{n}")).unwrap()
}

fn unconfigured() -> IntegrationCard {
    IntegrationCard::new(
        integration_id(1),
        "Home Assistant",
        IntegrationStatus::Unconfigured,
        vec![],
        None,
        None,
        correlation(1),
    )
    .unwrap()
}

#[test]
fn ep035_unit_integration_unconfigured_carries_no_claims() {
    let card = unconfigured();
    assert_eq!(card.status, IntegrationStatus::Unconfigured);
    assert!(card.configured_at_unix_s.is_none());
    assert!(card.last_verified_at_unix_s.is_none());
}

#[test]
fn ep035_unit_integration_capabilities_never_derived_from_name() {
    let card = IntegrationCard::new(
        integration_id(1),
        "Home Assistant",
        IntegrationStatus::Configured,
        vec![],
        Some(1000),
        None,
        correlation(1),
    )
    .unwrap();
    assert!(card.advertised_capabilities.is_empty());
    assert!(!card.advertised_capabilities.contains(&"lights".to_string()));
    assert!(!card
        .advertised_capabilities
        .contains(&"cameras".to_string()));
}

#[test]
fn ep035_unit_integration_configured_requires_timestamp() {
    let err = IntegrationCard::new(
        integration_id(1),
        "Home Assistant",
        IntegrationStatus::Configured,
        vec![],
        None,
        None,
        correlation(1),
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);
}

#[test]
fn ep035_unit_integration_healthy_requires_verification_event() {
    let err = IntegrationCard::new(
        integration_id(1),
        "Home Assistant",
        IntegrationStatus::Healthy,
        vec![],
        Some(1000),
        None,
        correlation(1),
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Verification);
    let healthy = IntegrationCard::new(
        integration_id(1),
        "Home Assistant",
        IntegrationStatus::Healthy,
        vec![],
        Some(1000),
        Some(2000),
        correlation(1),
    )
    .unwrap();
    assert_eq!(healthy.status, IntegrationStatus::Healthy);
}

#[test]
fn ep035_unit_integration_credential_exists_never_mints_healthy() {
    let configured = IntegrationCard::new(
        integration_id(1),
        "Home Assistant",
        IntegrationStatus::Configured,
        vec![],
        Some(1000),
        None,
        correlation(1),
    )
    .unwrap();
    let err = configured
        .transition(IntegrationStatus::Healthy, 1001)
        .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Policy);
}

#[test]
fn ep035_unit_integration_status_ladder_is_ordered() {
    assert!(is_valid_integration_transition(
        IntegrationStatus::Unconfigured,
        IntegrationStatus::Configured
    ));
    assert!(!is_valid_integration_transition(
        IntegrationStatus::Unconfigured,
        IntegrationStatus::Healthy
    ));
    assert!(is_valid_integration_transition(
        IntegrationStatus::Authenticated,
        IntegrationStatus::Reachable
    ));
    assert!(!is_valid_integration_transition(
        IntegrationStatus::Healthy,
        IntegrationStatus::Reachable
    ));
}

#[test]
fn ep035_unit_integration_reachable_requires_verification_event() {
    let configured = IntegrationCard::new(
        integration_id(1),
        "Home Assistant",
        IntegrationStatus::Configured,
        vec![],
        Some(1000),
        None,
        correlation(1),
    )
    .unwrap();
    // CONFIGURED -> REACHABLE is not even a legal transition; the
    // truthful path is CONFIGURED -> AUTHENTICATED -> REACHABLE with a
    // verification event.
    assert!(configured
        .clone()
        .transition(IntegrationStatus::Reachable, 1001)
        .is_err());
    let authenticated = configured
        .transition(IntegrationStatus::Authenticated, 1002)
        .unwrap();
    let reachable = authenticated
        .transition(IntegrationStatus::Reachable, 1003)
        .unwrap();
    assert_eq!(reachable.status, IntegrationStatus::Reachable);
    assert!(reachable.last_verified_at_unix_s.is_some());
}

#[test]
fn ep035_unit_integration_round_trips_and_rejects_unknown() {
    let card = IntegrationCard::new(
        integration_id(1),
        "Home Assistant",
        IntegrationStatus::Degraded,
        vec![],
        Some(1000),
        Some(1500),
        correlation(1),
    )
    .unwrap();
    let parsed: IntegrationCard =
        serde_json::from_str(&serde_json::to_string(&card).unwrap()).unwrap();
    assert_eq!(parsed.status, IntegrationStatus::Degraded);
    let mut value = serde_json::to_value(&card).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("forged".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<IntegrationCard>(value).is_err());
}

#[test]
fn ep035_unit_integration_unconfigured_rejects_verification_timestamp() {
    let err = IntegrationCard::new(
        integration_id(1),
        "Home Assistant",
        IntegrationStatus::Unconfigured,
        vec![],
        None,
        Some(2000),
        correlation(1),
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);
}
