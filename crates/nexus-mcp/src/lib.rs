//! EP-012 MCP Streamable HTTP engine (SPEC-003 required behavior 2).
//!
//! Real, deterministic MCP server behavior targeting specification
//! 2025-11-25: origin validation, authentication BEFORE tenant
//! resolution, protocol negotiation, cancellation, structured content,
//! and declared output schemas. The engine is provider-neutral: all I/O
//! happens behind the fabric `McpServer`/`McpClient` ports.

#![forbid(unsafe_code)]

pub mod engine;
pub mod error;
pub mod origin;
pub mod registry;
pub mod schema;
pub mod session;

pub use engine::{McpCallRecord, McpEngine, McpEngineConfig};
pub use error::{McpError, McpErrorCode};
pub use origin::{OriginPolicy, OriginPolicyError};
pub use registry::{McpToolHandler, McpToolRegistry};
pub use schema::{SchemaCheck, SchemaValidator};
pub use session::{McpSession, McpSessionState, SessionBinding};
