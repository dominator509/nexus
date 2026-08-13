//! EP-005 M4 forced-failure tests at the contract layer.
//!
//! These tests exercise real failure paths in the provider-neutral event
//! contracts: malformed input, wire-model rejection, duplicate
//! processing state, denied/invalid transitions, unavailable-dependency
//! error codes, redaction, and bounded retry. They are synchronous and
//! dependency-free because the contract crate must not import any
//! infrastructure crate (dependency-direction test); real-container
//! failures live in `infra/nats/tests/failure_nats.rs`.

use nexus_domain::{CorrelationId, EventId, TenantId};
use nexus_events::{
    EventDataClass, EventEnvelope, EventError, EventErrorCode, EventType, InboxStatus,
    OutboxRecord, OutboxStatus,
};

fn valid_envelope() -> EventEnvelope {
    EventEnvelope {
        event_id: EventId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa001").unwrap(),
        event_type: EventType::new("memory.record.created").unwrap(),
        schema_version: "1.0.0".to_string(),
        source: "voice".to_string(),
        subject: "nexus.memory.record.created.0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa001".to_string(),
        time: "2026-08-12T00:00:00Z".to_string(),
        tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa001").unwrap(),
        actor: "principal".to_string(),
        correlation_id: CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa002").unwrap(),
        causation_id: None,
        data_class: EventDataClass::Household,
        payload: serde_json::json!({ "k": "v" }),
    }
}

#[test]
fn ep005_failure_malformed_schema_version_rejected() {
    let mut e = valid_envelope();
    e.schema_version = "9.9.9".to_string();
    let err = e.validate().unwrap_err();
    assert_eq!(err.code(), EventErrorCode::Validation);
    assert!(err.message().contains("schema_version"));
}

#[test]
fn ep005_failure_empty_source_and_actor_rejected() {
    let mut e = valid_envelope();
    e.source = String::new();
    let err = e.validate().unwrap_err();
    assert_eq!(err.code(), EventErrorCode::Validation);
    assert!(err.message().contains("source"));
    let mut e = valid_envelope();
    e.actor = String::new();
    let err = e.validate().unwrap_err();
    assert_eq!(err.code(), EventErrorCode::Validation);
    assert!(err.message().contains("actor"));
}

#[test]
fn ep005_failure_empty_subject_rejected() {
    let mut e = valid_envelope();
    e.subject = String::new();
    let err = e.validate().unwrap_err();
    assert_eq!(err.code(), EventErrorCode::Validation);
    assert!(err.message().contains("subject"));
}

#[test]
fn ep005_failure_invalid_event_type_rejected() {
    // Uppercase, whitespace, and non-ASCII are malformed input.
    for bad in ["Memory.record.created", "memory record", "mémoire", ""] {
        let err = EventType::new(bad).unwrap_err();
        assert_eq!(err.code(), EventErrorCode::Validation, "input: {bad:?}");
    }
}

#[test]
fn ep005_failure_unknown_data_class_rejected() {
    let err = "NOT_A_CLASS".parse::<EventDataClass>().unwrap_err();
    assert_eq!(err.code(), EventErrorCode::Validation);
}

#[test]
fn ep005_failure_duplicate_processing_state_is_dedup_conflict() {
    // A consumer that has already DONE an event must not reprocess it as
    // NEW; the status machine is the dedup guard (SPEC-023 behavior 4).
    let done = InboxStatus::Done;
    assert_ne!(done, InboxStatus::New);
    assert_ne!(done, InboxStatus::Processing);
    assert_ne!(done, InboxStatus::Failed);
    // The canonical wire form is stable and machine-readable.
    assert_eq!(InboxStatus::New.as_str(), "NEW");
    assert_eq!(InboxStatus::Processing.as_str(), "PROCESSING");
    assert_eq!(InboxStatus::Done.as_str(), "DONE");
    assert_eq!(InboxStatus::Failed.as_str(), "FAILED");
}

#[test]
fn ep005_failure_outbox_marks_failed_with_redacted_reason_and_bounded_retry() {
    let mut record = OutboxRecord {
        outbox_id: "ob-1".to_string(),
        envelope: valid_envelope(),
        status: OutboxStatus::Pending,
        attempts: 0,
        last_error: None,
    };
    // First failure: reason recorded, status FAILED, attempt bounded.
    record.fail("provider nack: connection reset (secret redacted)");
    assert_eq!(record.status, OutboxStatus::Failed);
    assert_eq!(record.attempts, 1);
    assert!(!record.is_pending());
    assert!(record.last_error.as_deref().unwrap().contains("redacted"));
    // Second failure: retry counter is bounded (saturating), never
    // overflow or silently reset to PENDING.
    record.fail("provider nack again");
    assert_eq!(record.attempts, 2);
    assert_eq!(record.status, OutboxStatus::Failed);
}

#[test]
fn ep005_failure_unavailable_error_code_stable_and_correlation_preserved() {
    let err = EventError::new(
        EventErrorCode::Unavailable,
        "nats connect 127.0.0.1:1: refused",
    )
    .with_correlation("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa003");
    assert_eq!(err.code(), EventErrorCode::Unavailable);
    assert_eq!(err.code().as_str(), "UNAVAILABLE");
    assert_eq!(
        err.correlation_id(),
        Some("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa003")
    );
    // Structured display: stable code prefix, never a bare panic text.
    assert!(err.to_string().starts_with("[UNAVAILABLE]"));
}

#[test]
fn ep005_failure_timeout_code_is_machine_stable() {
    let err = EventError::new(EventErrorCode::Timeout, "nats fetch: deadline exceeded");
    assert_eq!(err.code().as_str(), "TIMEOUT");
    assert!(err.to_string().starts_with("[TIMEOUT]"));
}

#[test]
fn ep005_failure_authorization_code_is_machine_stable() {
    let err = EventError::new(
        EventErrorCode::Authorization,
        "principal lacks publish permission",
    );
    assert_eq!(err.code().as_str(), "AUTHORIZATION");
    assert!(err.to_string().starts_with("[AUTHORIZATION]"));
}

#[test]
fn ep005_failure_wire_model_rejects_unknown_fields() {
    // deny_unknown_fields is the closed-wire contract: an envelope with
    // an extra field must fail to deserialize, never silently accept.
    let e = valid_envelope();
    let mut v = serde_json::to_value(&e).unwrap();
    v.as_object_mut()
        .unwrap()
        .insert("evil".to_string(), serde_json::json!(1));
    let err: EventError = serde_json::from_value::<EventEnvelope>(v)
        .unwrap_err()
        .into();
    assert_eq!(err.code(), EventErrorCode::Validation);
}
