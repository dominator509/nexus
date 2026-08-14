//! Typed capability dispatcher (EP-010 M2).
//!
//! The dispatcher is the composition path from a registry lookup to a
//! typed capability port. It is the only place that maps a
//! `CapabilityClass` to a port, and it makes a generic execute string
//! impossible: every entry point is a typed method
//! (`dispatch_query`, `dispatch_command`, `dispatch_workflow`,
//! `dispatch_health`, `dispatch_changefeed`) and each validates class
//! before touching a port. A `QUERY` capability can never be invoked
//! through the command path; a `COMMAND` can never be invoked through
//! the query path.
//!
//! The dispatcher is deterministic and pure: all inputs arrive as
//! parameters, no wall clock, no randomness, no network. Providers
//! (ports) are injected; a provider error is returned typed and never
//! coerced into an allow.

use nexus_capabilities::changefeed::{ChangeBatch, ChangeCursor, ChangeFeedCapability};
use nexus_capabilities::command::{CommandCapability, CommandRequest, CommandResult};
use nexus_capabilities::context::InvocationContext;
use nexus_capabilities::descriptor::CapabilityDescriptor;
use nexus_capabilities::error::{CapabilityError, CapabilityErrorCode};
use nexus_capabilities::health::{HealthCapability, HealthReport};
use nexus_capabilities::query::{QueryCapability, QueryRequest, QueryResult};
use nexus_capabilities::registry::CapabilityRegistry;
use nexus_capabilities::workflow::{WorkflowCapability, WorkflowHandle, WorkflowRequest};
use nexus_domain::{Availability, CapabilityClass, TenantId};

use crate::idempotency::{IdempotencyRecord, IdempotencyTracker};

/// Error produced by the capability dispatcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatcherError(pub CapabilityError);

impl std::fmt::Display for DispatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "capability dispatcher: {}", self.0)
    }
}

impl std::error::Error for DispatcherError {}

impl From<CapabilityError> for DispatcherError {
    fn from(e: CapabilityError) -> Self {
        Self(e)
    }
}

/// Deterministic capability dispatcher over a shared registry.
///
/// The registry is referenced by shared handle so multiple dispatchers
/// (or a dispatcher and the registry owner) observe the same
/// advertisement state.
pub struct CapabilityDispatcher {
    /// Shared registry used for resolution.
    registry: std::sync::Arc<dyn CapabilityRegistry + Send + Sync>,
}

impl std::fmt::Debug for CapabilityDispatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapabilityDispatcher")
            .field("registry", &"<shared>")
            .finish()
    }
}

impl Clone for CapabilityDispatcher {
    fn clone(&self) -> Self {
        Self {
            registry: self.registry.clone(),
        }
    }
}

impl CapabilityDispatcher {
    /// Construct a dispatcher over a shared registry port.
    pub fn new(registry: std::sync::Arc<dyn CapabilityRegistry + Send + Sync>) -> Self {
        Self { registry }
    }

    /// Resolve the descriptor for a capability in a tenant.
    fn resolve(
        &self,
        capability_id: &str,
        tenant_id: &TenantId,
        context: &InvocationContext,
    ) -> Result<CapabilityDescriptor, CapabilityError> {
        self.registry
            .resolve(capability_id, tenant_id, context.clone())
    }

    /// Dispatch a read-only query.
    pub fn dispatch_query<Q: QueryCapability>(
        &self,
        request: QueryRequest,
        port: &Q,
    ) -> Result<QueryResult, DispatcherError> {
        let descriptor = self.resolve(
            &request.capability_id,
            &request.context.tenant_id,
            &request.context,
        )?;
        if descriptor.class != CapabilityClass::Query {
            return Err(DispatcherError(CapabilityError::new(
                CapabilityErrorCode::Validation,
                "capability is not a QUERY class",
                Some(request.context.correlation_id.to_string()),
                Some(request.context.external_actor_id),
                Some(request.context.tenant_id.to_string()),
                Some(request.capability_id),
            )));
        }
        Ok(port.query(request)?)
    }

    /// Dispatch an idempotent command.
    pub fn dispatch_command<C: CommandCapability>(
        &self,
        request: CommandRequest,
        port: &C,
        tracker: &IdempotencyTracker,
    ) -> Result<CommandResult, DispatcherError> {
        let descriptor = self.resolve(
            &request.capability_id,
            &request.context.tenant_id,
            &request.context,
        )?;
        if descriptor.class != CapabilityClass::Command {
            return Err(DispatcherError(CapabilityError::new(
                CapabilityErrorCode::Validation,
                "capability is not a COMMAND class",
                Some(request.context.correlation_id.to_string()),
                Some(request.context.external_actor_id),
                Some(request.context.tenant_id.to_string()),
                Some(request.capability_id),
            )));
        }
        // Idempotency: replay the stored result for a repeated key.
        if let Some(key) = request.idempotency_key.clone()
            && let Ok(Some(record)) = tracker.get(&key)
        {
            return Ok(CommandResult {
                capability_id: request.capability_id,
                output: record.result.clone(),
            });
        }
        let result = port.command(request.clone())?;
        if let Some(key) = request.idempotency_key {
            let _ = tracker.record(IdempotencyRecord {
                key,
                capability_id: result.capability_id.clone(),
                result: result.output.clone(),
            });
        }
        Ok(result)
    }

    /// Start a durable workflow.
    pub fn dispatch_workflow<W: WorkflowCapability>(
        &self,
        request: WorkflowRequest,
        port: &W,
    ) -> Result<WorkflowHandle, DispatcherError> {
        let descriptor = self.resolve(
            &request.capability_id,
            &request.context.tenant_id,
            &request.context,
        )?;
        if descriptor.class != CapabilityClass::Workflow {
            return Err(DispatcherError(CapabilityError::new(
                CapabilityErrorCode::Validation,
                "capability is not a WORKFLOW class",
                Some(request.context.correlation_id.to_string()),
                Some(request.context.external_actor_id),
                Some(request.context.tenant_id.to_string()),
                Some(request.capability_id),
            )));
        }
        Ok(port.start(request)?)
    }

    /// Read health.
    pub fn dispatch_health<H: HealthCapability>(
        &self,
        capability_id: String,
        context: InvocationContext,
        port: &H,
    ) -> Result<HealthReport, DispatcherError> {
        let descriptor = self.resolve(&capability_id, &context.tenant_id, &context)?;
        if descriptor.availability != Availability::Available {
            return Err(DispatcherError(CapabilityError::new(
                CapabilityErrorCode::Unavailable,
                "capability is not available",
                Some(context.correlation_id.to_string()),
                Some(context.external_actor_id),
                Some(context.tenant_id.to_string()),
                Some(capability_id),
            )));
        }
        Ok(port.health(context)?)
    }

    /// Read change-feed events.
    pub fn dispatch_changefeed<F: ChangeFeedCapability>(
        &self,
        capability_id: String,
        cursor: Option<ChangeCursor>,
        context: InvocationContext,
        port: &F,
    ) -> Result<ChangeBatch, DispatcherError> {
        let descriptor = self.resolve(&capability_id, &context.tenant_id, &context)?;
        if descriptor.class != CapabilityClass::Stream && descriptor.class != CapabilityClass::Query
        {
            return Err(DispatcherError(CapabilityError::new(
                CapabilityErrorCode::Validation,
                "capability does not expose a change feed",
                Some(context.correlation_id.to_string()),
                Some(context.external_actor_id),
                Some(context.tenant_id.to_string()),
                Some(capability_id),
            )));
        }
        Ok(port.changes_since(capability_id, cursor, context)?)
    }
}
