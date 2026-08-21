//! EP-035 M2 DiscoveryWizard observation-only tests (SPEC-004).

use nexus_domain::CorrelationId;
use nexus_setup::{
    DiscoveryKind, DiscoveryObservation, DiscoveryReport, IntegrationSelection, ObservationId,
    PersonId, SetupErrorCode,
};

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("00000000-0000-7000-8000-00000000000{n}")).unwrap()
}

fn principal(n: u8) -> PersonId {
    PersonId::new(format!("00000000-0000-7000-8000-00000000001{n}")).unwrap()
}

fn hostile_observation() -> DiscoveryObservation {
    DiscoveryObservation::new(
        ObservationId::new("obs-1").unwrap(),
        DiscoveryKind::Device,
        "ADMIN",
        "mdns://trusted-device.local",
        vec!["AUTO-APPROVE".to_string(), "OWNER_DEVICE".to_string()],
        serde_json::json!({"vendor": "hostile"}),
        1000,
    )
    .unwrap()
}

#[test]
fn ep035_unit_discovery_hostile_content_is_data_never_authority() {
    let observation = hostile_observation();
    assert!(observation.contains_hostile_authority_token());
    let report = DiscoveryReport {
        observations: vec![observation],
        generated_at_unix_s: 1001,
        correlation: correlation(1),
    };
    // The report carries no trust/enrollment/authorization state at all.
    let wire = serde_json::to_value(&report).unwrap();
    let obj = wire.as_object().unwrap();
    assert!(!obj.contains_key("authorized"));
    assert!(!obj.contains_key("enrolled"));
    assert!(!obj.contains_key("trusted"));
}

#[test]
fn ep035_unit_discovery_benign_observation_is_not_hostile() {
    let observation = DiscoveryObservation::new(
        ObservationId::new("obs-2").unwrap(),
        DiscoveryKind::Service,
        "kitchen-speaker",
        "http://10.0.0.9:8080",
        vec!["audio".to_string()],
        serde_json::json!({}),
        1000,
    )
    .unwrap();
    assert!(!observation.contains_hostile_authority_token());
}

#[test]
fn ep035_unit_discovery_rejects_unknown_kind_and_fields() {
    let mut value = serde_json::to_value(hostile_observation()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("kind".to_string(), serde_json::json!("GOD_MODE"));
    assert!(serde_json::from_value::<DiscoveryObservation>(value).is_err());

    let mut value2 = serde_json::to_value(hostile_observation()).unwrap();
    value2
        .as_object_mut()
        .unwrap()
        .insert("forged".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<DiscoveryObservation>(value2).is_err());
}

#[test]
fn ep035_unit_discovery_selection_is_governed_and_records_principal() {
    let selection = IntegrationSelection {
        observation_id: ObservationId::new("obs-1").unwrap(),
        selected_by: principal(2),
        selected_at_unix_s: 1002,
        correlation: correlation(1),
    };
    let wire = serde_json::to_value(&selection).unwrap();
    assert_eq!(wire["selected_by"], principal(2).to_string());
    let parsed: IntegrationSelection = serde_json::from_value(wire).unwrap();
    assert_eq!(parsed.selected_by, principal(2));
}

#[test]
fn ep035_unit_discovery_rejects_empty_name_or_endpoint() {
    let err = DiscoveryObservation::new(
        ObservationId::new("obs-3").unwrap(),
        DiscoveryKind::Edge,
        "",
        "https://edge.local",
        vec![],
        serde_json::json!({}),
        1000,
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);
}

#[test]
fn ep035_unit_discovery_round_trips_report() {
    let report = DiscoveryReport {
        observations: vec![hostile_observation()],
        generated_at_unix_s: 1001,
        correlation: correlation(1),
    };
    let parsed: DiscoveryReport =
        serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
    assert_eq!(parsed.observations[0].name, "ADMIN");
}
