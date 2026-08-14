//! Invocation context (SPEC-003 canonical term; schema
//! `invocation-context`).
//!
//! Every capability and connector request carries principal, tenant,
//! request ID, correlation ID, causation ID, and schema version. The
//! context is constructed from authenticated identity and can never be
//! selected by untrusted request metadata (SPEC-003 behavior 7:
//! connector tenant and account bindings resolve from authenticated
//! identity).

use serde::{Deserialize, Serialize};

use nexus_domain::{
    CorrelationId, DeviceId, NexusId, ObjectiveId, PrincipalType, TaskId, TenantId,
};

/// Invocation context attached to every capability and connector
/// request (SPEC-003 behavior 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvocationContext {
    /// Request identifier (canonical NexusId).
    pub request_id: NexusId,
    /// Correlation identifier spanning the causal chain.
    pub correlation_id: CorrelationId,
    /// Causation identifier when the request continues prior work.
    pub causation_id: Option<NexusId>,
    /// Origin system or client that initiated the request.
    pub origin_system: String,
    /// External actor identifier (principal reference, never a secret).
    pub external_actor_id: String,
    /// External actor class.
    pub external_actor_type: PrincipalType,
    /// Tenant boundary resolved from authenticated identity.
    pub tenant_id: TenantId,
    /// Optional channel (e.g. `telegram`, `web`, `mcp`).
    pub channel: Option<String>,
    /// Optional device identifier.
    pub device_id: Option<DeviceId>,
    /// Optional objective identifier.
    pub objective_id: Option<ObjectiveId>,
    /// Optional task identifier.
    pub task_id: Option<TaskId>,
}

/// Error produced when constructing an invocation context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationContextError(pub String);

impl std::fmt::Display for InvocationContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid invocation context: {}", self.0)
    }
}

impl std::error::Error for InvocationContextError {}

impl InvocationContext {
    /// Construct a validated invocation context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_id: NexusId,
        correlation_id: CorrelationId,
        causation_id: Option<NexusId>,
        origin_system: impl Into<String>,
        external_actor_id: impl Into<String>,
        external_actor_type: PrincipalType,
        tenant_id: TenantId,
        channel: Option<String>,
        device_id: Option<DeviceId>,
        objective_id: Option<ObjectiveId>,
        task_id: Option<TaskId>,
    ) -> Result<Self, InvocationContextError> {
        let origin_system = origin_system.into();
        if origin_system.trim().is_empty() {
            return Err(InvocationContextError(
                "origin_system must not be empty".to_string(),
            ));
        }
        let external_actor_id = external_actor_id.into();
        if external_actor_id.trim().is_empty() {
            return Err(InvocationContextError(
                "external_actor_id must not be empty".to_string(),
            ));
        }
        Ok(Self {
            request_id,
            correlation_id,
            causation_id,
            origin_system,
            external_actor_id,
            external_actor_type,
            tenant_id,
            channel,
            device_id,
            objective_id,
            task_id,
        })
    }
}
