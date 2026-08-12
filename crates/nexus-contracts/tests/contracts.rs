//! EP-001 contract pipeline tests: generated bindings must match the schemas.

use std::path::PathBuf;
use std::process::Command;

/// The generated Rust bindings must stay in sync with the canonical schemas.
/// Regenerating must produce a byte-identical file; otherwise the pipeline is
/// stale and the commit is rejected.
#[test]
fn generated_contracts_match() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let status = Command::new("python3")
        .arg("packages/contracts/scripts/generate.py")
        .arg("--check")
        .current_dir(&root)
        .status()
        .expect("failed to run contract generator");
    assert!(
        status.success(),
        "generated.rs is out of date; run packages/contracts/scripts/generate.py and commit"
    );
}

/// The generated NexusControlObject serializes with the canonical snake_case
/// wire names exactly as the schema property names define (schema_version,
/// approval_required, executable_instruction, required_capabilities).
#[test]
fn control_object_serde_roundtrip() {
    let obj = nexus_contracts::NexusControlObject {
        schema_version: "1.0.0".into(),
        intent: "home.lights.set".into(),
        route: "DETERMINISTIC".into(),
        risk: "R0".into(),
        privacy: "HOUSEHOLD".into(),
        ambiguity: 0.0,
        approval_required: false,
        executable_instruction: true,
        confidence: 0.99,
        required_capabilities: vec!["home.lights.set".into()],
        entities: serde_json::json!({}),
        escalation_reason: None,
        workflow: None,
    };
    let json = serde_json::to_string(&obj).expect("serialize");
    assert!(json.contains("\"schema_version\":\"1.0.0\""));
    assert!(json.contains("\"approval_required\""));
    assert!(json.contains("\"executable_instruction\""));
    assert!(json.contains("\"required_capabilities\""));
    let back: nexus_contracts::NexusControlObject =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(obj, back);
}

/// ActionRequest carries the canonical SPEC-006 fields: idempotency key, risk,
/// approval class, and reversal must round-trip with snake_case wire names.
#[test]
fn action_request_roundtrip() {
    let req = nexus_contracts::ActionRequest {
        action_id: "act_123".into(),
        tenant_id: "tenant_1".into(),
        principal_id: "user_1".into(),
        capability_id: "cap.lock".into(),
        idempotency_key: "key_1".into(),
        risk: "R3".into(),
        approval_class: "HUMAN".into(),
        reversal: "COMPENSATING".into(),
        arguments: serde_json::json!({"door": "front"}),
        expected_state: serde_json::json!({"locked": true}),
        invocation: nexus_contracts::InvocationContext {
            request_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073".into(),
            correlation_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6074".into(),
            origin_system: "voice".into(),
            external_actor_id: "user_1".into(),
            external_actor_type: "PERSON".into(),
            channel: Some(Some("voice".into())),
            causation_id: None,
            approval_id: None,
            device_id: None,
            objective_id: None,
            room_id: None,
            task_id: None,
        },
    };
    let json = serde_json::to_string(&req).expect("serialize");
    assert!(json.contains("\"idempotency_key\""));
    assert!(json.contains("\"approval_class\""));
    assert!(json.contains("\"request_id\""));
    let back: nexus_contracts::ActionRequest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(req, back);
}
