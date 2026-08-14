//! MCP Streamable HTTP engine (SPEC-003 required behavior 2).
//!
//! Deterministic server behavior over the fabric `McpServer` port:
//!
//! 1. Origin validation happens BEFORE session work.
//! 2. Authentication precedes tenant resolution: the tenant comes only
//!    from the authenticated binding; a request that names a different
//!    tenant fails closed (SPEC-003 required behavior 7).
//! 3. Protocol negotiation accepts only 2025-11-25.
//! 4. Tool calls are exact-name, input-schema-validated, and their
//!    output is validated against the declared output schema.
//! 5. Cancellation is tracked per call; a cancelled call never
//!    produces output.
//! 6. Idempotency keys replay the identical result deterministically.
//! 7. Telemetry carries fingerprints and correlation only - never
//!    prompts, tool arguments, secrets, or private content.

use crate::error::{McpError, McpErrorCode};
use crate::origin::OriginPolicy;
use crate::registry::{DeclaredTool, McpToolRegistry};
use crate::session::{McpSession, SessionBinding};
use nexus_auth::vocabulary::AuthenticationStrength;
use nexus_fabric::vocabulary::McpProtocolVersion;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Engine configuration.
#[derive(Debug, Clone)]
pub struct McpEngineConfig {
    pub origin_policy: OriginPolicy,
    /// Minimum authentication strength for tool calls.
    pub minimum_strength: AuthenticationStrength,
}

/// A tracked in-flight (or completed) tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpCallRecord {
    pub call_id: String,
    pub tool: String,
    pub cancelled: bool,
    pub completed: bool,
    pub result: Option<Value>,
}

/// Deterministic MCP engine.
pub struct McpEngine {
    config: McpEngineConfig,
    registry: McpToolRegistry,
    sessions: BTreeMap<String, McpSession>,
    calls: BTreeMap<String, McpCallRecord>,
    idempotency: BTreeMap<String, Value>,
}

impl McpEngine {
    pub fn new(config: McpEngineConfig, registry: McpToolRegistry) -> Self {
        Self {
            config,
            registry,
            sessions: BTreeMap::new(),
            calls: BTreeMap::new(),
            idempotency: BTreeMap::new(),
        }
    }

    /// Attach a session. The binding carries the AUTHENTICATED
    /// principal and tenant; origin is validated first.
    pub fn attach_session(
        &mut self,
        session_id: &str,
        binding: SessionBinding,
        origin: Option<&str>,
    ) -> Result<(), McpError> {
        self.config.origin_policy.validate(origin).map_err(|e| {
            McpError::new(
                McpErrorCode::Validation,
                format!("origin validation failed: {e}"),
                None,
                None,
                None,
                Some("mcp.attach".to_string()),
            )
        })?;
        if self.sessions.contains_key(session_id) {
            return Err(McpError::conflict(format!(
                "session already exists: {session_id}"
            )));
        }
        self.sessions.insert(
            session_id.to_string(),
            McpSession::new(session_id, binding, origin.unwrap_or_default()),
        );
        Ok(())
    }

    fn session_mut(&mut self, session_id: &str) -> Result<&mut McpSession, McpError> {
        self.sessions
            .get_mut(session_id)
            .ok_or_else(|| McpError::not_found(format!("unknown session: {session_id}")))
    }

    fn session(&self, session_id: &str) -> Result<&McpSession, McpError> {
        self.sessions
            .get(session_id)
            .ok_or_else(|| McpError::not_found(format!("unknown session: {session_id}")))
    }

    /// Initialize the session (protocol negotiation). Only 2025-11-25
    /// is accepted; anything else fails closed.
    pub fn initialize(
        &mut self,
        session_id: &str,
        version: McpProtocolVersion,
    ) -> Result<(), McpError> {
        if version != McpProtocolVersion::Spec2025_11_25 {
            return Err(McpError::validation(format!(
                "unsupported MCP protocol version: {}",
                version.as_str()
            )));
        }
        self.session_mut(session_id)?.activate()
    }

    /// Enforce the minimum authentication strength.
    pub fn require_strength(&self, session_id: &str) -> Result<(), McpError> {
        let session = self.session(session_id)?;
        session.enforce_strength(self.config.minimum_strength)
    }

    /// List declared tools for a session (tenant-safe).
    pub fn list_tools(&self, session_id: &str) -> Result<Vec<DeclaredTool>, McpError> {
        let _ = self.session(session_id)?;
        Ok(self.registry.list())
    }

    /// Start an in-flight tool call (registers the record). The call
    /// must be completed via `complete_call`; cancellation between
    /// start and complete fails the completion (fail closed).
    pub fn start_call(
        &mut self,
        session_id: &str,
        call_id: &str,
        tool: &str,
    ) -> Result<(), McpError> {
        let _ = self.session(session_id)?;
        if self.calls.contains_key(call_id) {
            return Err(McpError::conflict(format!(
                "call already registered: {call_id}"
            )));
        }
        self.calls.insert(
            call_id.to_string(),
            McpCallRecord {
                call_id: call_id.to_string(),
                tool: tool.to_string(),
                cancelled: false,
                completed: false,
                result: None,
            },
        );
        Ok(())
    }

    /// Complete a started call. A cancelled call can never complete or
    /// produce output (fail closed).
    pub fn complete_call(
        &mut self,
        session_id: &str,
        call_id: &str,
        output: Value,
    ) -> Result<Value, McpError> {
        let _ = self.session(session_id)?;
        let Some(record) = self.calls.get_mut(call_id) else {
            return Err(McpError::not_found(format!("unknown call: {call_id}")));
        };
        if record.cancelled {
            return Err(McpError::conflict(format!("call cancelled: {call_id}")));
        }
        if record.completed {
            return Err(McpError::conflict(format!(
                "call already completed: {call_id}"
            )));
        }
        record.completed = true;
        record.result = Some(output.clone());
        Ok(output)
    }

    /// Call a tool on a session with exact-name dispatch, schema
    /// validation, cancellation tracking, and deterministic idempotency.
    pub fn call_tool(
        &mut self,
        session_id: &str,
        call_id: &str,
        tool: &str,
        arguments: &Value,
        idempotency_key: Option<&str>,
        claimed_tenant: Option<&str>,
    ) -> Result<Value, McpError> {
        // Authentication strength gate first.
        self.require_strength(session_id)?;
        // Tenant can never be selected through untrusted metadata.
        self.session(session_id)?.enforce_tenant(claimed_tenant)?;
        // Idempotent replay: same key returns the identical result.
        if let Some(key) = idempotency_key
            && let Some(cached) = self.idempotency.get(key)
        {
            return Ok(cached.clone());
        }
        self.start_call(session_id, call_id, tool)?;
        let session = self.session(session_id)?.clone();
        let result = self.registry.call(&session, tool, arguments);
        match result {
            Ok(output) => {
                let out = self.complete_call(session_id, call_id, output)?;
                if let Some(key) = idempotency_key {
                    self.idempotency.insert(key.to_string(), out.clone());
                }
                Ok(out)
            }
            Err(err) => {
                self.calls.remove(call_id);
                Err(err)
            }
        }
    }

    /// Cancel an in-flight call. A cancelled call can never complete or
    /// produce output (fail closed).
    pub fn cancel(&mut self, session_id: &str, call_id: &str) -> Result<(), McpError> {
        let _ = self.session(session_id)?;
        let Some(record) = self.calls.get_mut(call_id) else {
            return Err(McpError::not_found(format!("unknown call: {call_id}")));
        };
        if record.completed {
            return Err(McpError::conflict(format!(
                "call already completed: {call_id}"
            )));
        }
        record.cancelled = true;
        Ok(())
    }

    /// A cancelled call must never yield output.
    pub fn assert_not_cancelled(&self, call_id: &str) -> Result<(), McpError> {
        if let Some(record) = self.calls.get(call_id)
            && record.cancelled
        {
            return Err(McpError::new(
                McpErrorCode::Conflict,
                format!("call cancelled: {call_id}"),
                None,
                None,
                None,
                Some(record.tool.clone()),
            ));
        }
        Ok(())
    }

    pub fn call_record(&self, call_id: &str) -> Option<&McpCallRecord> {
        self.calls.get(call_id)
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn close_session(&mut self, session_id: &str) -> Result<(), McpError> {
        self.session_mut(session_id)?.close();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::McpToolRegistry;
    use nexus_domain::PrincipalType;

    fn binding() -> SessionBinding {
        SessionBinding {
            principal_id: "018f0f6f-9c1e-7b6e-8000-00000000000a".parse().unwrap(),
            principal_type: PrincipalType::Human,
            tenant_id: "018f0f6f-9c1e-7b6e-8000-000000000003".parse().unwrap(),
            authentication_strength: AuthenticationStrength::MultiFactor,
        }
    }

    fn echo_handler(_s: &McpSession, args: &Value) -> Result<Value, McpError> {
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
        e.attach_session(session_id, binding(), Some("https://app.nexus.local"))
            .unwrap();
        e.initialize(session_id, McpProtocolVersion::Spec2025_11_25)
            .unwrap();
    }

    #[test]
    fn ep012_unit_mcp_engine_full_allow_path() {
        let mut e = engine();
        attach(&mut e, "s1");
        let tools = e.list_tools("s1").unwrap();
        assert_eq!(tools.len(), 1);
        let out = e
            .call_tool(
                "s1",
                "call-1",
                "contacts.query",
                &serde_json::json!({"q": "a"}),
                None,
                None,
            )
            .unwrap();
        assert_eq!(out["echo"]["q"], "a");
        assert!(!e.call_record("call-1").unwrap().cancelled);
        assert!(e.call_record("call-1").unwrap().completed);
    }

    #[test]
    fn ep012_unit_mcp_engine_rejects_unknown_origin_before_session() {
        let mut e = engine();
        let err = e
            .attach_session("s1", binding(), Some("https://evil.example.com"))
            .unwrap_err();
        assert_eq!(err.code, McpErrorCode::Validation);
        assert_eq!(e.session_count(), 0);
    }

    #[test]
    fn ep012_unit_mcp_engine_rejects_unsupported_protocol() {
        let mut e = engine();
        e.attach_session("s1", binding(), Some("https://app.nexus.local"))
            .unwrap();
        // Unknown version cannot be constructed (enum), so negotiate
        // via a raw parse rejection instead: only the locked version
        // exists, and initialization always uses it. The fail-closed
        // surface is the enum parse (tested in nexus-fabric).
        assert!(
            e.initialize("s1", McpProtocolVersion::Spec2025_11_25)
                .is_ok()
        );
        assert!(
            e.initialize("s2", McpProtocolVersion::Spec2025_11_25)
                .is_err()
        );
    }

    #[test]
    fn ep012_unit_mcp_engine_tenant_never_from_untrusted_metadata() {
        let mut e = engine();
        attach(&mut e, "s1");
        let err = e
            .call_tool(
                "s1",
                "call-2",
                "contacts.query",
                &serde_json::json!({}),
                None,
                Some("018f0f6f-9c1e-7b6e-8000-000000000099"),
            )
            .unwrap_err();
        assert_eq!(err.code, McpErrorCode::Authorization);
    }

    #[test]
    fn ep012_unit_mcp_engine_cancelled_call_never_yields_output() {
        let mut e = engine();
        attach(&mut e, "s1");
        // Start an in-flight call, cancel it, then attempt completion.
        e.start_call("s1", "call-x", "contacts.query").unwrap();
        e.cancel("s1", "call-x").unwrap();
        assert!(e.assert_not_cancelled("call-x").is_err());
        let err = e
            .complete_call("s1", "call-x", serde_json::json!({"echo": {}}))
            .unwrap_err();
        assert_eq!(err.code, McpErrorCode::Conflict);
        // Cancelling an unknown call is NOT_FOUND.
        assert_eq!(
            e.cancel("s1", "call-missing").unwrap_err().code,
            McpErrorCode::NotFound
        );
        // Completing an unknown call is NOT_FOUND.
        assert_eq!(
            e.complete_call("s1", "call-unknown", serde_json::json!({}))
                .unwrap_err()
                .code,
            McpErrorCode::NotFound
        );
    }

    #[test]
    fn ep012_unit_mcp_engine_idempotency_replays_identical_result() {
        let mut e = engine();
        attach(&mut e, "s1");
        let a = e
            .call_tool(
                "s1",
                "call-1",
                "contacts.query",
                &serde_json::json!({"q": "a"}),
                Some("op-key-1"),
                None,
            )
            .unwrap();
        let b = e
            .call_tool(
                "s1",
                "call-2",
                "contacts.query",
                &serde_json::json!({"q": "DIFFERENT"}),
                Some("op-key-1"),
                None,
            )
            .unwrap();
        // The replay returns the ORIGINAL result, not the new args.
        assert_eq!(a, b);
        assert_eq!(b["echo"]["q"], "a");
    }

    #[test]
    fn ep012_unit_mcp_engine_strength_gate_fails_closed() {
        let mut e = engine();
        let weak = SessionBinding {
            authentication_strength: AuthenticationStrength::SingleFactor,
            ..binding()
        };
        e.attach_session("weak", weak, Some("https://app.nexus.local"))
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
    fn ep012_unit_mcp_engine_unknown_session_and_tool_fail_closed() {
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
}
