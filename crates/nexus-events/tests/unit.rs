//! EP-005 M1 unit tests: event contracts, validation, and vocabulary
//! rejection (SPEC-023).

use nexus_domain::{CorrelationId, EventId, TenantId};
use nexus_events::{
    ConsumerCheckpoint, ConsumerConfig, EventDataClass, EventEnvelope, EventError, EventErrorCode,
    EventType, InboxRecord, InboxStatus, OutboxRecord, OutboxStatus, StreamConfig,
};

fn env() -> EventEnvelope {
    EventEnvelope {
        event_id: EventId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa001").unwrap(),
        event_type: EventType::new("memory.record.created").unwrap(),
        schema_version: "1.0.0".to_string(),
        source: "voice".to_string(),
        subject: "nexus.memory.record".to_string(),
        time: "2026-08-12T00:00:00Z".to_string(),
        tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa002").unwrap(),
        actor: "principal".to_string(),
        correlation_id: CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa003").unwrap(),
        causation_id: None,
        data_class: EventDataClass::Household,
        payload: serde_json::json!({ "note": "hello" }),
    }
}

#[test]
fn ep005_unit_event_type_accepts_dotted_slug() {
    let t = EventType::new("memory.record.created").unwrap();
    assert_eq!(t.as_str(), "memory.record.created");
    assert_eq!(t.to_string(), "memory.record.created");
}

#[test]
fn ep005_unit_event_type_rejects_uppercase_and_space() {
    for bad in [
        "Memory.Record",
        "memory record",
        "memory/record",
        "",
        "UPPER",
    ] {
        let err = EventType::new(bad).unwrap_err();
        assert_eq!(err.code(), EventErrorCode::Validation, "bad type {bad:?}");
    }
}

#[test]
fn ep005_unit_event_data_class_round_trips_canonical_values() {
    for (s, expected) in [
        ("PUBLIC", EventDataClass::Public),
        ("HOUSEHOLD", EventDataClass::Household),
        ("PERSONAL", EventDataClass::Personal),
        ("SENSITIVE", EventDataClass::Sensitive),
        (
            "BUSINESS_CONFIDENTIAL",
            EventDataClass::BusinessConfidential,
        ),
        ("SECURITY", EventDataClass::Security),
        ("SECRET", EventDataClass::Secret),
    ] {
        let parsed: EventDataClass = s.parse().expect("valid class");
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), s);
    }
    let err = "UNKNOWN_CLASS".parse::<EventDataClass>().unwrap_err();
    assert_eq!(err.code(), EventErrorCode::Validation);
}

#[test]
fn ep005_unit_envelope_serializes_snake_case_and_closed() {
    let e = env();
    let json = serde_json::to_value(&e).unwrap();
    assert_eq!(json["event_id"], "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa001");
    assert_eq!(json["event_type"], "memory.record.created");
    assert_eq!(json["data_class"], "HOUSEHOLD");
    assert_eq!(json["schema_version"], "1.0.0");
    // Round trip.
    let back: EventEnvelope = serde_json::from_value(json).unwrap();
    assert_eq!(back, e);
}

#[test]
fn ep005_unit_envelope_rejects_unknown_fields() {
    let mut v = serde_json::to_value(env()).unwrap();
    v.as_object_mut()
        .unwrap()
        .insert("sneaky".into(), serde_json::json!(1));
    let err = serde_json::from_value::<EventEnvelope>(v).unwrap_err();
    assert!(err.is_data(), "unknown field must be rejected");
}

#[test]
fn ep005_unit_envelope_validation_rejects_bad_schema_version() {
    let mut e = env();
    e.schema_version = "0.9.0".to_string();
    let err = e.validate().unwrap_err();
    assert_eq!(err.code(), EventErrorCode::Validation);
}

#[test]
fn ep005_unit_envelope_validation_rejects_empty_source() {
    let mut e = env();
    e.source = "".to_string();
    assert!(e.validate().is_err());
}

#[test]
fn ep005_unit_outbox_status_machine() {
    assert_eq!(OutboxStatus::Pending.as_str(), "PENDING");
    assert_eq!(OutboxStatus::Publishing.as_str(), "PUBLISHING");
    assert_eq!(OutboxStatus::Published.as_str(), "PUBLISHED");
    assert_eq!(OutboxStatus::Failed.as_str(), "FAILED");
    let mut rec = OutboxRecord {
        outbox_id: "outbox-1".to_string(),
        envelope: env(),
        status: OutboxStatus::Pending,
        attempts: 0,
        last_error: None,
    };
    assert!(rec.is_pending());
    rec.fail("nats timeout");
    assert_eq!(rec.status, OutboxStatus::Failed);
    assert_eq!(rec.attempts, 1);
    assert_eq!(rec.last_error.as_deref(), Some("nats timeout"));
}

#[test]
fn ep005_unit_inbox_status_machine() {
    assert_eq!(InboxStatus::New.as_str(), "NEW");
    assert_eq!(InboxStatus::Processing.as_str(), "PROCESSING");
    assert_eq!(InboxStatus::Done.as_str(), "DONE");
    assert_eq!(InboxStatus::Failed.as_str(), "FAILED");
    let rec = InboxRecord {
        consumer: "memory-indexer".to_string(),
        event_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5fa001".to_string(),
        status: InboxStatus::New,
        attempts: 0,
    };
    assert_eq!(rec.consumer, "memory-indexer");
}

#[test]
fn ep005_unit_consumer_checkpoint_round_trips() {
    let cp = ConsumerCheckpoint {
        consumer: "memory-indexer".to_string(),
        stream: "nexus".to_string(),
        subject: "nexus.memory.>".to_string(),
        last_sequence: 42,
    };
    let json = serde_json::to_value(&cp).unwrap();
    let back: ConsumerCheckpoint = serde_json::from_value(json).unwrap();
    assert_eq!(back, cp);
    assert_eq!(back.last_sequence, 42);
}

#[test]
fn ep005_unit_consumer_config_and_stream_config_validate_construction() {
    let cc = ConsumerConfig {
        consumer: "memory-indexer".to_string(),
        stream: "nexus".to_string(),
        subject: "nexus.memory.>".to_string(),
        batch_size: 10,
    };
    assert_eq!(cc.batch_size, 10);
    let sc = StreamConfig {
        stream: "nexus".to_string(),
        subjects: vec!["nexus.memory.>".to_string()],
        max_messages: 100_000,
        max_age_seconds: 86_400,
    };
    assert_eq!(sc.stream, "nexus");
    assert_eq!(sc.subjects.len(), 1);
}

#[test]
fn ep005_unit_event_error_carries_stable_code_and_correlation() {
    let err =
        EventError::new(EventErrorCode::Conflict, "duplicate event").with_correlation("corr-9");
    assert_eq!(err.code(), EventErrorCode::Conflict);
    assert_eq!(err.correlation_id(), Some("corr-9"));
    assert_eq!(err.code().as_str(), "CONFLICT");
    let text = err.to_string();
    assert!(text.contains("CONFLICT"));
    assert!(!text.contains("password"));
}
