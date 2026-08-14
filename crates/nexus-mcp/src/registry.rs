//! MCP tool registry (SPEC-003: declared tools, exact-name dispatch,
//! never an arbitrary-string executor).

use crate::error::{McpError, McpErrorCode};
use crate::schema::{SchemaCheck, SchemaValidator};
use crate::session::McpSession;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool handler: a pure domain function bound to a session.
pub type McpToolHandler = fn(&McpSession, &Value) -> Result<Value, McpError>;

/// A declared MCP tool with input and output schemas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
}

/// Registered tool entry (handler is not serialized).
pub struct ToolEntry {
    pub tool: DeclaredTool,
    pub handler: McpToolHandler,
}

/// Deterministic tool registry with exact-name dispatch.
#[derive(Default)]
pub struct McpToolRegistry {
    tools: std::collections::BTreeMap<String, ToolEntry>,
}

impl McpToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool; duplicate name is a conflict.
    pub fn register(
        &mut self,
        tool: DeclaredTool,
        handler: McpToolHandler,
    ) -> Result<(), McpError> {
        if self.tools.contains_key(&tool.name) {
            return Err(McpError::conflict(format!(
                "tool already registered: {}",
                tool.name
            )));
        }
        if tool.name.trim().is_empty() {
            return Err(McpError::validation("tool name must not be empty"));
        }
        self.tools
            .insert(tool.name.clone(), ToolEntry { tool, handler });
        Ok(())
    }

    pub fn list(&self) -> Vec<DeclaredTool> {
        self.tools.values().map(|e| e.tool.clone()).collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Call a tool by EXACT name. Unknown tools are typed NOT_FOUND.
    /// Arguments are validated against the declared input schema; the
    /// output is validated against the declared output schema before
    /// it can be returned (declared output schemas, SPEC-003).
    pub fn call(
        &self,
        session: &McpSession,
        name: &str,
        arguments: &Value,
    ) -> Result<Value, McpError> {
        let Some(entry) = self.tools.get(name) else {
            return Err(McpError::not_found(format!("unknown tool: {name}")));
        };
        if let SchemaCheck::Mismatch(msg) =
            SchemaValidator::validate(&entry.tool.input_schema, arguments)
        {
            return Err(McpError::new(
                McpErrorCode::Validation,
                format!("tool {name} input invalid: {msg}"),
                None,
                None,
                None,
                Some(name.to_string()),
            ));
        }
        let output = (entry.handler)(session, arguments)?;
        if let SchemaCheck::Mismatch(msg) =
            SchemaValidator::validate(&entry.tool.output_schema, &output)
        {
            return Err(McpError::new(
                McpErrorCode::MalformedProviderResponse,
                format!("tool {name} output violates declared schema: {msg}"),
                None,
                None,
                None,
                Some(name.to_string()),
            ));
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{McpSession, SessionBinding};
    use nexus_auth::vocabulary::AuthenticationStrength;
    use nexus_domain::PrincipalType;

    fn test_session() -> McpSession {
        McpSession::new(
            "s1",
            SessionBinding {
                principal_id: "018f0f6f-9c1e-7b6e-8000-00000000000a".parse().unwrap(),
                principal_type: PrincipalType::Human,
                tenant_id: "018f0f6f-9c1e-7b6e-8000-000000000003".parse().unwrap(),
                authentication_strength: AuthenticationStrength::MultiFactor,
            },
            "https://app.nexus.local",
        )
    }

    fn echo_handler(_session: &McpSession, args: &Value) -> Result<Value, McpError> {
        Ok(serde_json::json!({"echo": args}))
    }

    fn tool(name: &str) -> DeclaredTool {
        DeclaredTool {
            name: name.to_string(),
            description: "test".into(),
            input_schema: serde_json::json!({"type": "object"}),
            output_schema: serde_json::json!({
                "type": "object",
                "required": ["echo"],
                "properties": {"echo": {"type": "object"}}
            }),
        }
    }

    #[test]
    fn ep012_unit_mcp_registry_exact_name_dispatch() {
        let mut registry = McpToolRegistry::new();
        registry
            .register(tool("contacts.query"), echo_handler)
            .unwrap();
        assert!(registry.contains("contacts.query"));
        let session = test_session();
        let out = registry
            .call(&session, "contacts.query", &serde_json::json!({"q": "a"}))
            .unwrap();
        assert_eq!(out["echo"]["q"], "a");
        // Unknown tool is NOT_FOUND.
        let err = registry
            .call(&session, "contacts.delete", &serde_json::json!({}))
            .unwrap_err();
        assert_eq!(err.code, McpErrorCode::NotFound);
    }

    #[test]
    fn ep012_unit_mcp_registry_duplicate_is_conflict() {
        let mut registry = McpToolRegistry::new();
        registry.register(tool("t1"), echo_handler).unwrap();
        assert!(registry.register(tool("t1"), echo_handler).is_err());
        assert!(
            registry
                .register(
                    DeclaredTool {
                        name: "".into(),
                        description: "".into(),
                        input_schema: serde_json::json!({}),
                        output_schema: serde_json::json!({}),
                    },
                    echo_handler
                )
                .is_err()
        );
    }

    #[test]
    fn ep012_unit_mcp_registry_validates_output_schema() {
        fn bad_handler(_s: &McpSession, _a: &Value) -> Result<Value, McpError> {
            // Returns a string, but the declared output schema is object.
            Ok(serde_json::json!("not-an-object"))
        }
        let mut registry = McpToolRegistry::new();
        registry.register(tool("t2"), bad_handler).unwrap();
        let session = test_session();
        let err = registry
            .call(&session, "t2", &serde_json::json!({}))
            .unwrap_err();
        assert_eq!(err.code, McpErrorCode::MalformedProviderResponse);
    }
}
