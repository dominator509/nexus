//! EP-004 M1 unit tests: construction, validation, serialization,
//! vocabulary rejection, and dependency-direction constraints.

use std::str::FromStr;

use nexus_data::{
    DataError, DataErrorCode, EmbeddingRef, MemoryQuery, MemoryRecord, MemoryStatus,
    RetentionPolicy, RetentionUnit, Sensitivity,
};
use nexus_domain::{MemoryType, NexusId, TenantId};

fn sample_record() -> MemoryRecord {
    MemoryRecord {
        memory_id: NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01").unwrap(),
        tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02").unwrap(),
        namespace: "household".to_string(),
        memory_type: MemoryType::Episodic,
        content: serde_json::json!({ "note": "groceries" }),
        content_hash: "a".repeat(64),
        source: "voice".to_string(),
        actor: "principal".to_string(),
        created_at: "2026-08-12T00:00:00Z".to_string(),
        observed_at: "2026-08-12T00:00:00Z".to_string(),
        confidence: 0.8,
        sensitivity: Sensitivity::Household,
        purpose: "remember".to_string(),
        retention: RetentionPolicy::for_duration(RetentionUnit::Days, 30),
        status: MemoryStatus::Proposed,
        derived_from: vec![],
        supersedes: None,
        embedding_ref: None,
    }
}

#[test]
fn ep004_unit_memory_record_validates_canonical_invariants() {
    let record = sample_record();
    assert!(record.validate().is_ok());
}

#[test]
fn ep004_unit_memory_record_rejects_confidence_out_of_range() {
    let mut record = sample_record();
    record.confidence = 1.5;
    let err = record.validate().unwrap_err();
    assert_eq!(err.code(), DataErrorCode::Validation);
    assert!(err.message().contains("confidence"));
}

#[test]
fn ep004_unit_memory_record_rejects_bad_content_hash() {
    let mut record = sample_record();
    record.content_hash = "not-a-hash".to_string();
    let err = record.validate().unwrap_err();
    assert_eq!(err.code(), DataErrorCode::Validation);
    assert!(err.message().contains("content_hash"));
}

#[test]
fn ep004_unit_memory_record_rejects_empty_namespace() {
    let mut record = sample_record();
    record.namespace = String::new();
    let err = record.validate().unwrap_err();
    assert_eq!(err.code(), DataErrorCode::Validation);
    assert!(err.message().contains("namespace"));
}

#[test]
fn ep004_unit_memory_record_round_trips_json_canonical_wire() {
    let record = sample_record();
    let json = serde_json::to_value(&record).unwrap();
    let back: MemoryRecord = serde_json::from_value(json).unwrap();
    assert_eq!(record, back);
}

#[test]
fn ep004_unit_memory_record_wire_uses_snake_case_fields() {
    let json = serde_json::to_value(sample_record()).unwrap();
    let obj = json.as_object().unwrap();
    for key in [
        "memory_id",
        "tenant_id",
        "namespace",
        "memory_type",
        "content",
        "content_hash",
        "source",
        "actor",
        "created_at",
        "observed_at",
        "confidence",
        "sensitivity",
        "purpose",
        "retention",
        "status",
        "derived_from",
        "supersedes",
        "embedding_ref",
    ] {
        assert!(obj.contains_key(key), "missing canonical field {key}");
    }
    // additionalProperties: false - no extra keys on the wire.
    assert_eq!(obj.len(), 18, "wire model must not carry extra fields");
}

#[test]
fn ep004_unit_sensitivity_parses_canonical_strings() {
    for (text, expected) in [
        ("PUBLIC", Sensitivity::Public),
        ("HOUSEHOLD", Sensitivity::Household),
        ("PERSONAL", Sensitivity::Personal),
        ("SENSITIVE", Sensitivity::Sensitive),
        ("BUSINESS_CONFIDENTIAL", Sensitivity::BusinessConfidential),
        ("SECURITY", Sensitivity::Security),
        ("SECRET", Sensitivity::Secret),
    ] {
        assert_eq!(Sensitivity::from_str(text).unwrap(), expected);
        assert_eq!(expected.as_str(), text);
    }
    assert!(Sensitivity::from_str("TOP_SECRET").is_err());
}

#[test]
fn ep004_unit_memory_status_parses_canonical_strings() {
    for (text, expected) in [
        ("PROPOSED", MemoryStatus::Proposed),
        ("ACTIVE", MemoryStatus::Active),
        ("SUPERSEDED", MemoryStatus::Superseded),
        ("REJECTED", MemoryStatus::Rejected),
        ("DELETED", MemoryStatus::Deleted),
    ] {
        assert_eq!(MemoryStatus::from_str(text).unwrap(), expected);
        assert_eq!(expected.as_str(), text);
    }
    assert!(MemoryStatus::from_str("DRAFT").is_err());
}

#[test]
fn ep004_unit_memory_query_defaults_are_bounded() {
    let query = MemoryQuery::default();
    assert_eq!(query.limit, 20);
    assert!(query.namespace.is_none());
    assert!(query.status.is_none());
}

#[test]
fn ep004_unit_retention_policy_round_trips_and_display() {
    let bounded = RetentionPolicy::for_duration(RetentionUnit::Days, 30);
    assert!(!bounded.is_indefinite());
    assert_eq!(bounded.to_string(), "Days 30");
    let indefinite = RetentionPolicy::indefinite();
    assert!(indefinite.is_indefinite());
    assert_eq!(indefinite.to_string(), "INDEFINITE");
}

#[test]
fn ep004_unit_embedding_ref_is_versioned() {
    let embedding = EmbeddingRef {
        model: "minilm".to_string(),
        dimensions: 384,
        version: "v1".to_string(),
    };
    let json = serde_json::to_value(&embedding).unwrap();
    let back: EmbeddingRef = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(embedding, back);
    assert_eq!(json["dimensions"], 384);
}

#[test]
fn ep004_unit_data_error_carries_stable_code_and_correlation() {
    let err =
        DataError::new(DataErrorCode::Conflict, "version conflict").with_correlation("corr-123");
    assert_eq!(err.code(), DataErrorCode::Conflict);
    assert_eq!(err.correlation_id(), Some("corr-123"));
    assert_eq!(err.code().as_str(), "CONFLICT");
}

#[test]
fn ep004_unit_memory_type_rejects_unknown_vocabulary() {
    // MemoryType is vocabulary-locked in nexus-domain; unknown values must
    // be rejected by FromStr at the contract boundary.
    assert!(MemoryType::from_str("EXPERIMENTAL").is_err());
    assert!(MemoryType::from_str("WORKING").is_ok());
}
