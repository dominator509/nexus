//! EP-012 M5 composed fabric gateway (SPEC-003).
//!
//! This crate is the COMPOSITION SURFACE for the EP-012 fabric: it owns
//! the REAL MCP Streamable HTTP engine (`nexus-mcp`), the REAL A2A
//! gateway (`nexus-a2a`), and a real hash-bound artifact store, and
//! exposes one authenticated entry point that drives the full chain:
//!
//! authenticated principal/tenant -> MCP session -> exact-name tool
//! discovery/call -> schema validation -> idempotent execution or
//! cancellation -> A2A task creation -> streamed task progress ->
//! hash-bound artifact -> completion/cancellation -> evidence.
//!
//! Authority boundaries are explicit and tested:
//!
//! - A valid MCP session/tool call means only that an authenticated
//!   protocol request is structurally valid. It is NOT final execution
//!   authorization (EP-008 owns authorization).
//! - A2A agent/task identity and tenant scope do not grant arbitrary
//!   capabilities.
//! - Artifact attachment is integrity/reference binding (hash-bound),
//!   never execution authority.
//! - Protocol acceptance is never execution permission.
//!
//! This crate never evaluates policy and never issues capability
//! grants. It is a transport/domain composition, not an oracle.

#![forbid(unsafe_code)]

pub mod artifact_store;
pub mod composed;

pub use artifact_store::MemoryArtifactStore;
pub use composed::{ComposedGateway, ComposedGatewayConfig, GatewayProbeOutcome, ProbeStage};
