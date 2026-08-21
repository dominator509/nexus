//! EP-035 M2 OwnerBootstrap security-critical tests (SPEC-004/016).

use nexus_domain::{CorrelationId, PersonId};
use nexus_setup::{
    resolve_first_owner, FirstOwnerDecision, FirstOwnerRecord, OwnerBootstrapRequest,
    RecoveryKitId, SetupErrorCode,
};

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("00000000-0000-7000-8000-00000000000{n}")).unwrap()
}

fn principal(n: u8) -> PersonId {
    PersonId::new(format!("00000000-0000-7000-8000-00000000001{n}")).unwrap()
}

fn request(idempotency_key: &str) -> OwnerBootstrapRequest {
    OwnerBootstrapRequest::new(
        "Alice Owner",
        "alice@example.com",
        correlation(1),
        idempotency_key,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn ep035_unit_owner_parses_valid_request() {
    let req = request("bootstrap-1");
    assert_eq!(req.owner_email, "alice@example.com");
}

#[test]
fn ep035_unit_owner_client_isowner_flag_is_rejected() {
    let mut value = serde_json::to_value(request("bootstrap-1")).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("isOwner".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<OwnerBootstrapRequest>(value).is_err());
}

#[test]
fn ep035_unit_owner_rejects_missing_required_values() {
    let err = OwnerBootstrapRequest::new(
        "",
        "alice@example.com",
        correlation(1),
        "bootstrap-1",
        None,
        None,
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);
}

#[test]
fn ep035_unit_owner_first_initialize_is_initialized() {
    let decision = resolve_first_owner(None, &request("bootstrap-1"), principal(2));
    assert_eq!(
        decision,
        FirstOwnerDecision::Initialized {
            principal_id: principal(2)
        }
    );
}

#[test]
fn ep035_unit_owner_replay_is_idempotent() {
    let known = FirstOwnerRecord {
        idempotency_key: "bootstrap-1".to_string(),
        principal_id: principal(2),
    };
    let decision = resolve_first_owner(Some(&known), &request("bootstrap-1"), principal(2));
    assert_eq!(
        decision,
        FirstOwnerDecision::AlreadyInitialized {
            principal_id: principal(2)
        }
    );
}

#[test]
fn ep035_unit_owner_competing_request_is_conflict() {
    let known = FirstOwnerRecord {
        idempotency_key: "bootstrap-1".to_string(),
        principal_id: principal(2),
    };
    let decision = resolve_first_owner(Some(&known), &request("bootstrap-2"), principal(2));
    assert_eq!(decision, FirstOwnerDecision::Conflict);
}

#[test]
fn ep035_unit_owner_round_trips_request_serialization() {
    let req = request("bootstrap-1");
    let wire = serde_json::to_string(&req).unwrap();
    let parsed: OwnerBootstrapRequest = serde_json::from_str(&wire).unwrap();
    assert_eq!(parsed.idempotency_key, "bootstrap-1");
}

#[test]
fn ep035_unit_owner_optional_recovery_kit_round_trips() {
    let req = OwnerBootstrapRequest::new(
        "Alice Owner",
        "alice@example.com",
        correlation(1),
        "bootstrap-3",
        Some(RecoveryKitId::new("kit-1").unwrap()),
        Some("recovery-kit".to_string()),
    )
    .unwrap();
    let wire = serde_json::to_string(&req).unwrap();
    assert!(wire.contains("kit-1"));
    let parsed: OwnerBootstrapRequest = serde_json::from_str(&wire).unwrap();
    assert_eq!(parsed.verification_method.as_deref(), Some("recovery-kit"));
}
