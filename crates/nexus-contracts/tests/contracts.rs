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

/// The generated NexusControlObject serializes with camelCase field names
/// exactly as the canonical schema expects (schema_version, approval_required,
/// executable_instruction, required_capabilities).
#[test]
fn control_object_serde_roundtrip() {
    let obj = nexus_contracts::NexusControlObject {
        schema_version: serde_json::json!("1"),
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
    assert!(json.contains("\"schemaVersion\""));
    assert!(json.contains("\"approvalRequired\""));
    assert!(json.contains("\"executableInstruction\""));
    assert!(json.contains("\"requiredCapabilities\""));
    let back: nexus_contracts::NexusControlObject = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(obj, back);
}

/// ActionRequest carries the canonical SPEC-006 fields: idempotency key, risk,
/// approval class, and reversal must round-trip.
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
        invocation: serde_json::json!({"channel": "voice"}),
    };
    let json = serde_json::to_string(&req).expect("serialize");
    assert!(json.contains("\"idempotencyKey\""));
    assert!(json.contains("\"approvalClass\""));
    let back: nexus_contracts::ActionRequest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(req, back);
}
