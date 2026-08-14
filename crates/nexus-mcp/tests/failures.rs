//! EP-012 M4 MCP failure/abuse tests (SPEC-003). Real failure
//! mechanisms on the real engine: malformed input, denied permission,
//! cancelled work, duplicate requests, unavailable dependencies. The
//! engine under test is never mocked.

use nexus_auth::vocabulary::AuthenticationStrength;
use nexus_domain::{PrincipalType, TenantId};
use nexus_fabric::vocabulary::McpProtocolVersion;
use nexus_mcp::engine::{McpEngine, McpEngineConfig};
use nexus_mcp::error::McpErrorCode;
use nexus_mcp::origin::OriginPolicy;
use nexus_mcp::registry::{DeclaredTool, McpToolRegistry};
use nexus_mcp::session::SessionBinding;

fn binding(strength: AuthenticationStrength) -> SessionBinding {
    SessionBinding {
        principal_id: "018f0f6f-9c1e-7b6e-8000-00000000000a".parse().unwrap(),
        principal_type: PrincipalType::Human,
        tenant_id: "018f0f6f-9c1e-7b6e-8000-000000000003".parse().unwrap(),
        authentication_strength: strength,
    }
}

fn echo_handler(
    _s: &nexus_mcp::session::McpSession,
    args: &serde_json::Value,
) -> Result<serde_json::Value, nexus_mcp::error::McpError> {
    Ok(serde_json::json!({"echo": args}))
}

fn engine() -> McpEngine {
    let mut registry = McpToolRegistry::new();
    registry
        .register(
            DeclaredTool {
                name: "contacts.query".into(),
                description: "query contacts".into(),
                input_schema: serde_json::json!({"type": "object"}),
                output_schema: serde_json::json!({
                    "type": "object",
                    "required": ["echo"],
                    "properties": {"echo": {"type": "object"}}
                }),
            },
            echo_handler,
        )
        .unwrap();
    McpEngine::new(
        McpEngineConfig {
            origin_policy: OriginPolicy::new(["https://app.nexus.local"]),
            minimum_strength: AuthenticationStrength::MultiFactor,
        },
        registry,
    )
}

fn attach(e: &mut McpEngine, session_id: &str) {
    e.attach_session(
        session_id,
        binding(AuthenticationStrength::MultiFactor),
        Some("https://app.nexus.local"),
    )
    .unwrap();
    e.initialize(session_id, McpProtocolVersion::Spec2025_11_25)
        .unwrap();
}

#[test]
fn ep012_failure_mcp_malformed_arguments_fail_closed() {
    let mut e = engine();
    attach(&mut e, "s1");
    // The tool declares input type object; a string argument fails.
    let err = e
        .call_tool(
            "s1",
            "call-1",
            "contacts.query",
            &serde_json::json!("not-an-object"),
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.code, McpErrorCode::Validation);
}

#[test]
fn ep012_failure_mcp_denied_origin_before_anything() {
    let mut e = engine();
    let err = e
        .attach_session(
            "s1",
            binding(AuthenticationStrength::MultiFactor),
            Some("https://evil.example"),
        )
        .unwrap_err();
    assert_eq!(err.code, McpErrorCode::Validation);
    assert_eq!(e.session_count(), 0);
}

#[test]
fn ep012_failure_mcp_cross_tenant_claim_rejected() {
    let mut e = engine();
    attach(&mut e, "s1");
    let err = e
        .call_tool(
            "s1",
            "call-1",
            "contacts.query",
            &serde_json::json!({}),
            None,
            Some("018f0f6f-9c1e-7b6e-8000-000000000099"),
        )
        .unwrap_err();
    assert_eq!(err.code, McpErrorCode::Authorization);
}

#[test]
fn ep012_failure_mcp_insufficient_strength_denied() {
    let mut e = engine();
    e.attach_session(
        "weak",
        binding(AuthenticationStrength::SingleFactor),
        Some("https://app.nexus.local"),
    )
    .unwrap();
    e.initialize("weak", McpProtocolVersion::Spec2025_11_25)
        .unwrap();
    let err = e
        .call_tool(
            "weak",
            "call-1",
            "contacts.query",
            &serde_json::json!({}),
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.code, McpErrorCode::Authorization);
}

#[test]
fn ep012_failure_mcp_cancelled_work_never_completes() {
    let mut e = engine();
    attach(&mut e, "s1");
    e.start_call("s1", "call-1", "contacts.query").unwrap();
    e.cancel("s1", "call-1").unwrap();
    let err = e
        .complete_call("s1", "call-1", serde_json::json!({"echo": {}}))
        .unwrap_err();
    assert_eq!(err.code, McpErrorCode::Conflict);
    assert!(e.assert_not_cancelled("call-1").is_err());
}

#[test]
fn ep012_failure_mcp_duplicate_session_and_call_conflict() {
    let mut e = engine();
    attach(&mut e, "s1");
    // Duplicate session is a conflict.
    let err = e
        .attach_session(
            "s1",
            binding(AuthenticationStrength::MultiFactor),
            Some("https://app.nexus.local"),
        )
        .unwrap_err();
    assert_eq!(err.code, McpErrorCode::Conflict);
    // Duplicate call registration is a conflict.
    e.start_call("s1", "call-1", "contacts.query").unwrap();
    let err = e.start_call("s1", "call-1", "contacts.query").unwrap_err();
    assert_eq!(err.code, McpErrorCode::Conflict);
}

#[test]
fn ep012_failure_mcp_unknown_session_and_tool_not_found() {
    let mut e = engine();
    assert_eq!(
        e.list_tools("missing").unwrap_err().code,
        McpErrorCode::NotFound
    );
    attach(&mut e, "s1");
    let err = e
        .call_tool(
            "s1",
            "call-1",
            "nope.tool",
            &serde_json::json!({}),
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(err.code, McpErrorCode::NotFound);
}

#[test]
fn ep012_failure_mcp_unknown_tenant_id_shape_rejected() {
    // TenantId parse failure must fail closed at the boundary: a
    // malformed tenant string cannot become a session binding.
    let bad: Result<TenantId, _> = "not-a-uuid".parse();
    assert!(bad.is_err());
}
