//! EP-012 fabric contracts (SPEC-003).
//!
//! Provider-neutral ports for the Nexus control surface: versioned REST,
//! WebSocket sessions, MCP Streamable HTTP (2025-11-25), A2A (1.0.1),
//! agent card registry, artifact exchange (immutable by hash), and
//! scoped context capsules. Every port is transport-agnostic, carries
//! authenticated tenant and principal context, fails closed with typed
//! SPEC-006 errors, and never grants authority itself.

#![forbid(unsafe_code)]

pub mod a2a;
pub mod agents;
pub mod artifacts;
pub mod capsules;
pub mod error;
pub mod mcp;
pub mod rest;
pub mod vocabulary;
pub mod websocket;

pub use a2a::{A2AGateway, A2ATask, A2ATaskId, A2ATaskState, A2ATaskStatus, TaskMessage};
pub use agents::{AgentCard, AgentCardId, AgentCardRegistry, AgentCardState};
pub use artifacts::{
    ArtifactExchange, ArtifactHandle, ArtifactId, ArtifactManifest, ArtifactState,
};
pub use capsules::{
    CapsuleId, CapsuleReference, CapsuleState, ContextCapsule, ContextCapsuleService,
};
pub use error::{FabricError, FabricErrorCode};
pub use mcp::{McpClient, McpContentKind, McpServer, McpToolCall, McpToolDefinition};
pub use rest::{RestApi, RestEndpoint, RestRequest, RestResponse};
pub use vocabulary::{A2AProtocolVersion, ApiTransport, McpProtocolVersion, StreamState};
pub use websocket::{WebSocketEvent, WebSocketSession, WebSocketState};
