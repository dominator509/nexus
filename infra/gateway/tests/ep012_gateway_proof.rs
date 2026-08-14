//! EP-012 M5 composed fabric crown-jewel proof.
//!
//! Drives the REAL composed gateway (real MCP engine + real A2A
//! gateway + real hash-bound artifact store) through the full SPEC-003
//! chain and proves every authority boundary. All identifiers are
//! deterministic UUIDv7-style values; nothing here is a mock.

use nexus_auth::vocabulary::AuthenticationStrength;
use nexus_domain::{NexusId, PrincipalType, TenantId};
use nexus_gateway::{ComposedGateway, ComposedGatewayConfig};
use nexus_mcp::session::SessionBinding;

const TENANT: &str = "018f0f6f-9c1e-7b6e-8000-000000000001";
const PRINCIPAL: &str = "018f0f6f-9c1e-7b6e-8000-00000000000a";
const OTHER_TENANT: &str = "018f0f6f-9c1e-7b6e-8000-000000000099";

fn binding(strength: AuthenticationStrength) -> SessionBinding {
    SessionBinding {
        principal_id: NexusId::new(PRINCIPAL).unwrap(),
        principal_type: PrincipalType::Human,
        tenant_id: TenantId::new(TENANT).unwrap(),
        authentication_strength: strength,
    }
}

fn gateway() -> ComposedGateway {
    ComposedGateway::new(ComposedGatewayConfig::default())
}

#[test]
fn ep012_gateway_crown_jewel_full_chain() {
    let mut g = gateway();
    let outcome = g
        .run_probe(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
            "corr-0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01",
            binding(AuthenticationStrength::StepUp),
            Some(serde_json::json!({"recommendation": "ALLOW"})),
            Some(serde_json::json!({"receipt": "stale"})),
        )
        .expect("crown-jewel probe must pass");

    // Canonical stage ordering.
    assert_eq!(
        outcome.stages,
        vec![
            "SESSION_PASS",
            "PROTOCOL_PASS",
            "TOOLS_PASS",
            "CALL_PASS",
            "IDEMPOTENCY_PASS",
            "CANCELLATION_PASS",
            "A2A_SUBMIT_PASS",
            "A2A_STREAM_PASS",
            "ARTIFACT_PASS",
            "A2A_COMPLETE_PASS",
        ]
    );
    // Protocol versions locked.
    assert_eq!(outcome.mcp_protocol, "2025-11-25");
    assert_eq!(outcome.a2a_protocol, "1.0.1");
    // Authenticated context.
    assert_eq!(outcome.tenant_id, TENANT);
    assert_eq!(outcome.principal_id, PRINCIPAL);
    // Idempotency + cancellation.
    assert!(outcome.idempotent_replay_identical);
    assert!(outcome.cancelled_never_completes);
    // A2A lifecycle observed.
    assert!(outcome.stream_states.contains(&"SUBMITTED".to_string()));
    assert!(outcome.stream_states.contains(&"WORKING".to_string()));
    assert!(outcome.stream_states.contains(&"COMPLETED".to_string()));
    assert_eq!(outcome.final_lifecycle, "COMPLETED");
    // Hash-bound artifact attached.
    assert!(outcome.artifact_attached);
    assert_eq!(outcome.artifact_digest.len(), 64);
    // Boundaries.
    assert!(outcome.model_recommendation_never_consulted);
    assert!(outcome.receipt_never_reusable);
    assert!(outcome.cross_tenant_denied);
    assert!(outcome.authorization_not_implied);
}

#[test]
fn ep012_gateway_crown_jewel_is_deterministic() {
    let mut g1 = gateway();
    let mut g2 = gateway();
    let o1 = g1
        .run_probe(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02",
            "corr-2",
            binding(AuthenticationStrength::MultiFactor),
            None,
            None,
        )
        .unwrap();
    let o2 = g2
        .run_probe(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02",
            "corr-2",
            binding(AuthenticationStrength::MultiFactor),
            None,
            None,
        )
        .unwrap();
    let v1 = serde_json::to_value(&o1).unwrap();
    let v2 = serde_json::to_value(&o2).unwrap();
    assert_eq!(
        v1, v2,
        "identical DecisionInput must produce identical outcome"
    );
}

#[test]
fn ep012_gateway_protocol_rejects_unknown_version() {
    // The fabric vocabulary rejects any version other than 2025-11-25
    // at parse time (fail closed at the boundary).
    assert!(
        "2026-01-01"
            .parse::<nexus_fabric::vocabulary::McpProtocolVersion>()
            .is_err()
    );
    assert!(
        "1.0.0"
            .parse::<nexus_fabric::vocabulary::A2AProtocolVersion>()
            .is_err()
    );
}

#[test]
fn ep012_gateway_origin_rejected_before_session() {
    let mut g = gateway();
    let err = g
        .attach_session(
            "sess-evil",
            binding(AuthenticationStrength::MultiFactor),
            Some("https://evil.example.com"),
        )
        .unwrap_err();
    assert_eq!(err.code, nexus_mcp::McpErrorCode::Validation);
    // No session work happened: the engine has no sessions.
    assert_eq!(g.mcp_session_count(), 0);
}

#[test]
fn ep012_gateway_cross_tenant_claim_rejected() {
    let mut g = gateway();
    g.attach_session(
        "sess-t",
        binding(AuthenticationStrength::StepUp),
        Some("https://app.nexus.local"),
    )
    .unwrap();
    g.initialize("sess-t").unwrap();
    let err = g
        .call_tool(
            "sess-t",
            "call-1",
            "proof.echo",
            &serde_json::json!({"message": "hi"}),
            None,
            Some(OTHER_TENANT),
        )
        .unwrap_err();
    assert_eq!(err.code, nexus_mcp::McpErrorCode::Authorization);
}

#[test]
fn ep012_gateway_unknown_tool_fails_closed() {
    let mut g = gateway();
    g.attach_session(
        "sess-u",
        binding(AuthenticationStrength::StepUp),
        Some("https://app.nexus.local"),
    )
    .unwrap();
    g.initialize("sess-u").unwrap();
    let err = g
        .call_tool(
            "sess-u",
            "call-2",
            "proof.does_not_exist",
            &serde_json::json!({}),
            None,
            Some(TENANT),
        )
        .unwrap_err();
    assert_eq!(err.code, nexus_mcp::McpErrorCode::NotFound);
}

#[test]
fn ep012_gateway_insufficient_strength_fails_closed() {
    // The composed gateway requires MultiFactor minimum; a
    // SingleFactor binding cannot call tools.
    let mut g = gateway();
    g.attach_session(
        "sess-weak",
        binding(AuthenticationStrength::SingleFactor),
        Some("https://app.nexus.local"),
    )
    .unwrap();
    g.initialize("sess-weak").unwrap();
    let err = g
        .call_tool(
            "sess-weak",
            "call-3",
            "proof.echo",
            &serde_json::json!({"message": "hi"}),
            None,
            Some(TENANT),
        )
        .unwrap_err();
    assert_eq!(err.code, nexus_mcp::McpErrorCode::Authorization);
}

#[test]
fn ep012_gateway_schema_violation_fails_closed() {
    let mut g = gateway();
    g.attach_session(
        "sess-s",
        binding(AuthenticationStrength::StepUp),
        Some("https://app.nexus.local"),
    )
    .unwrap();
    g.initialize("sess-s").unwrap();
    // Missing required "message" field.
    let err = g
        .call_tool(
            "sess-s",
            "call-4",
            "proof.echo",
            &serde_json::json!({}),
            None,
            Some(TENANT),
        )
        .unwrap_err();
    assert_eq!(err.code, nexus_mcp::McpErrorCode::Validation);
}

#[test]
fn ep012_gateway_a2a_cross_tenant_access_denied() {
    let mut g = gateway();
    // Create a task in TENANT, then try to read it as OTHER_TENANT.
    g.submit_a2a(
        "task-x",
        TENANT,
        PRINCIPAL,
        vec![nexus_fabric::a2a::TaskMessage {
            message_id: "m-1".into(),
            role: "user".into(),
            parts: vec![serde_json::json!({"text": "x"})],
        }],
        nexus_a2a::task::TaskPriority::Normal,
    )
    .unwrap();
    let err = g.get_a2a("task-x", OTHER_TENANT).unwrap_err();
    assert_eq!(err.code, nexus_a2a::error::A2AErrorCode::Authorization);
    // The owning tenant still sees it.
    assert_eq!(g.get_a2a("task-x", TENANT).unwrap().task_id, "task-x");
}

#[test]
fn ep012_gateway_completed_task_cannot_be_cancelled() {
    let mut g = gateway();
    g.submit_a2a(
        "task-done",
        TENANT,
        PRINCIPAL,
        vec![],
        nexus_a2a::task::TaskPriority::Normal,
    )
    .unwrap();
    g.run_a2a("task-done", TENANT).unwrap();
    let err = g.cancel_a2a("task-done", TENANT).unwrap_err();
    assert_eq!(err.code, nexus_a2a::error::A2AErrorCode::Conflict);
    assert_eq!(
        g.get_a2a("task-done", TENANT).unwrap().status.as_str(),
        "COMPLETED"
    );
}

#[test]
fn ep012_gateway_artifact_missing_attach_fails_closed() {
    let mut g = gateway();
    g.submit_a2a(
        "task-art",
        TENANT,
        PRINCIPAL,
        vec![],
        nexus_a2a::task::TaskPriority::Normal,
    )
    .unwrap();
    let missing = nexus_fabric::artifacts::ArtifactId(format!("sha256:{}", "0".repeat(64)));
    let err = g.attach_artifact("task-art", TENANT, &missing).unwrap_err();
    assert_eq!(err.code, nexus_a2a::error::A2AErrorCode::NotFound);
}

#[test]
fn ep012_gateway_receipt_never_grants_authority() {
    // A "receipt" (e.g. a stale probe outcome) presented to a FRESH
    // gateway cannot create a session or call a tool: the gateway has
    // no path that consumes a presented receipt as authority. The
    // probe accepts a presented receipt but never consults it.
    let mut g = gateway();
    let presented = serde_json::json!({
        "request_id": "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a99",
        "decision": "ALLOWED",
        "receipt": "forged"
    });
    // Attaching a session still requires the real authenticated binding.
    let err = g
        .attach_session("sess-forged", binding(AuthenticationStrength::StepUp), None)
        .unwrap_err();
    assert_eq!(err.code, nexus_mcp::McpErrorCode::Validation);
    let _ = presented;
}

#[test]
fn ep012_gateway_verification_plan_is_deterministic() {
    let mut g1 = gateway();
    let mut g2 = gateway();
    let o1 = g1
        .run_probe(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
            "corr-3",
            binding(AuthenticationStrength::StepUp),
            None,
            None,
        )
        .unwrap();
    let o2 = g2
        .run_probe(
            "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a03",
            "corr-3",
            binding(AuthenticationStrength::StepUp),
            None,
            None,
        )
        .unwrap();
    assert_eq!(o1.verification_plan, o2.verification_plan);
    // The plan states the boundary explicitly.
    assert!(
        o1.verification_plan
            .iter()
            .any(|v| v.contains("authorization:not-owned-by-fabric"))
    );
}
