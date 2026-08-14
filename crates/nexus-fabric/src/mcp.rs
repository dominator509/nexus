//! MCP Streamable HTTP contract (SPEC-003 required behavior 2).
//!
//! Targets MCP specification 2025-11-25 with Streamable HTTP. The port
//! requires authentication BEFORE tenant resolution; a caller can never
//! select a tenant through untrusted metadata.

use crate::error::FabricError;
use crate::vocabulary::McpProtocolVersion;
use serde::{Deserialize, Serialize};

/// MCP structured content kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpContentKind {
    Text,
    Image,
    Audio,
    Resource,
    Embedded,
}

impl McpContentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::Image => "IMAGE",
            Self::Audio => "AUDIO",
            Self::Resource => "RESOURCE",
            Self::Embedded => "EMBEDDED",
        }
    }
}

impl std::str::FromStr for McpContentKind {
    type Err = crate::vocabulary::FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "TEXT" => Ok(Self::Text),
            "IMAGE" => Ok(Self::Image),
            "AUDIO" => Ok(Self::Audio),
            "RESOURCE" => Ok(Self::Resource),
            "EMBEDDED" => Ok(Self::Embedded),
            other => Err(crate::vocabulary::FabricVocabularyError::unknown(
                "McpContentKind",
                other,
            )),
        }
    }
}

/// A declared MCP tool (never an arbitrary-string executor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// An MCP tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolCall {
    pub tool: String,
    pub arguments: serde_json::Value,
}

/// Server-side MCP port.
pub trait McpServer {
    /// Initialize the server session; rejects unknown protocol versions.
    fn initialize(&mut self, protocol_version: McpProtocolVersion) -> Result<(), FabricError>;
    /// List declared tools.
    fn list_tools(&self) -> Result<Vec<McpToolDefinition>, FabricError>;
    /// Call a declared tool by exact name; unknown tools are NOT_FOUND.
    fn call_tool(&self, call: McpToolCall) -> Result<serde_json::Value, FabricError>;
    /// Cancel an in-flight tool call (SPEC-003 required behavior 2).
    fn cancel(&mut self, call_id: &str) -> Result<(), FabricError>;
}

/// Client-side MCP port.
pub trait McpClient {
    /// Connect and negotiate the protocol version.
    fn connect(&mut self, protocol_version: McpProtocolVersion) -> Result<(), FabricError>;
    /// Call a tool on the remote server.
    fn call_tool(&self, call: McpToolCall) -> Result<serde_json::Value, FabricError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_mcp_content_kind_round_trip() {
        for (wire, expected) in [
            ("TEXT", McpContentKind::Text),
            ("IMAGE", McpContentKind::Image),
            ("AUDIO", McpContentKind::Audio),
            ("RESOURCE", McpContentKind::Resource),
            ("EMBEDDED", McpContentKind::Embedded),
        ] {
            assert_eq!(wire.parse::<McpContentKind>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("VIDEO".parse::<McpContentKind>().is_err());
    }

    #[test]
    fn ep012_unit_mcp_tool_definition_round_trip() {
        let tool = McpToolDefinition {
            name: "contacts.query".into(),
            description: "query contacts".into(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_value(&tool).unwrap();
        let back: McpToolDefinition = serde_json::from_value(json).unwrap();
        assert_eq!(back.name, "contacts.query");
        assert_eq!(back.input_schema, serde_json::json!({"type": "object"}));
    }
}
