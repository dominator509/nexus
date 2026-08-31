//! EP-035 M2 OwnerBootstrap security-critical tests (SPEC-004/016).

use nexus_domain::{CorrelationId, PersonId};
use nexus_setup::{
    advance_owner_state, resolve_first_owner, FirstOwnerDecision, FirstOwnerRecord,
    OwnerBootstrapRequest, OwnerBootstrapState, RecoveryKitId, SetupErrorCode,
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
    let known = FirstOwnerRecord::new("bootstrap-1", principal(2));
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
    let known = FirstOwnerRecord::new("bootstrap-1", principal(2));
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

// ---------------------------------------------------------------------------
// AUD-044 regressions: OWNER_AUTHORIZED can only be persisted by
// traversing the full passkey/recovery ladder.
// ---------------------------------------------------------------------------

#[test]
fn ep035_unit_owner_record_starts_at_lowest_rung() {
    let record = FirstOwnerRecord::new("bootstrap-1", principal(2));
    assert_eq!(record.state, OwnerBootstrapState::DetailsProvided);
}

#[test]
fn ep035_unit_owner_cannot_jump_to_authorized() {
    // A record at DETAILS_PROVIDED can never jump to OWNER_AUTHORIZED:
    // the persistence layer cannot write the terminal state directly.
    let record = FirstOwnerRecord::new("bootstrap-1", principal(2));
    let err = advance_owner_state(&record, OwnerBootstrapState::OwnerAuthorized).unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Policy);
}

#[test]
fn ep035_unit_owner_ladder_traversal_is_enforced() {
    let record = FirstOwnerRecord::new("bootstrap-1", principal(2));
    // Rung 1: identity verified (passkey proof).
    let r1 = advance_owner_state(&record, OwnerBootstrapState::IdentityVerified).unwrap();
    assert_eq!(r1.state, OwnerBootstrapState::IdentityVerified);
    // Rung 2: principal created (recovery material proof).
    let r2 = advance_owner_state(&r1, OwnerBootstrapState::PrincipalCreated).unwrap();
    assert_eq!(r2.state, OwnerBootstrapState::PrincipalCreated);
    // Rung 3: authorized ONLY after both preceding transitions.
    let r3 = advance_owner_state(&r2, OwnerBootstrapState::OwnerAuthorized).unwrap();
    assert_eq!(r3.state, OwnerBootstrapState::OwnerAuthorized);
    // No skipped rungs possible from a fresh record.
    let err = advance_owner_state(
        &FirstOwnerRecord::new("bootstrap-1", principal(2)),
        OwnerBootstrapState::PrincipalCreated,
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Policy);
}

#[test]
fn ep035_unit_owner_idempotent_reassert_allowed() {
    let record = FirstOwnerRecord::new("bootstrap-1", principal(2));
    let same = advance_owner_state(&record, OwnerBootstrapState::DetailsProvided).unwrap();
    assert_eq!(same.state, OwnerBootstrapState::DetailsProvided);
}
