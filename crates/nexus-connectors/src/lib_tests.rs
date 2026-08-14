//! EP-010 M2 unit tests: real registry behavior, typed dispatch
//! (no generic execute string), idempotency, and tenant isolation.

use std::sync::Arc;

use nexus_capabilities::changefeed::{ChangeCursor, ChangeFeedCapability};
use nexus_capabilities::command::{CommandCapability, CommandRequest, CommandResult};
use nexus_capabilities::context::InvocationContext;
use nexus_capabilities::descriptor::{CapabilityDescriptor, CapabilityVersion};
use nexus_capabilities::error::{CapabilityError, CapabilityErrorCode};
use nexus_capabilities::health::{HealthCapability, HealthReport};
use nexus_capabilities::query::{QueryCapability, QueryRequest, QueryResult};
use nexus_capabilities::registry::CapabilityRegistry;
use nexus_capabilities::vocabulary::{HealthState, SchemaRef};
use nexus_capabilities::workflow::{WorkflowCapability, WorkflowHandle, WorkflowRequest};
use nexus_domain::{
    ApprovalClass, Availability, CapabilityClass, CorrelationId, Idempotency, Locality, NexusId,
    PrincipalType, Privacy, Reversal, Risk, TenantId,
};

use crate::dispatcher::CapabilityDispatcher;
use crate::idempotency::{IdempotencyRecord, IdempotencyTracker};
use crate::registry::InMemoryCapabilityRegistry;

fn rid() -> NexusId {
    NexusId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn cid() -> CorrelationId {
    CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
}

fn tid_a() -> TenantId {
    TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap()
}

fn tid_b() -> TenantId {
    TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000009").unwrap()
}

fn ctx(tenant: TenantId, actor: &str) -> InvocationContext {
    InvocationContext::new(
        rid(),
        cid(),
        None,
        "test-client",
        actor,
        PrincipalType::Human,
        tenant,
        Some("web".to_string()),
        None,
        None,
        None,
    )
    .unwrap()
}

fn schema_input() -> SchemaRef {
    SchemaRef::new("schemas/invocation-context.schema.json").unwrap()
}

fn schema_output() -> SchemaRef {
    SchemaRef::new("schemas/capability-descriptor.schema.json").unwrap()
}

fn query_descriptor(id: &str, availability: Availability) -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        id,
        CapabilityVersion("1.0.0".to_string()),
        CapabilityClass::Query,
        "Query the state of home lights",
        schema_input(),
        schema_output(),
        vec!["home.lights.read".to_string()],
        Risk::R0,
        ApprovalClass::None,
        Reversal::None,
        Idempotency::NotApplicable,
        availability,
        Some(Locality::HomeEdge),
        vec![Privacy::Household],
        vec!["home.lights.changed".to_string()],
        Some("provider-test".to_string()),
    )
    .unwrap()
}

fn command_descriptor(id: &str) -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        id,
        CapabilityVersion("1.0.0".to_string()),
        CapabilityClass::Command,
        "Set the state of home lights",
        schema_input(),
        schema_output(),
        vec!["home.lights.write".to_string()],
        Risk::R2,
        ApprovalClass::None,
        Reversal::Compensating,
        Idempotency::Required,
        Availability::Available,
        Some(Locality::HomeEdge),
        vec![Privacy::Household],
        vec!["home.lights.changed".to_string()],
        Some("provider-test".to_string()),
    )
    .unwrap()
}

fn workflow_descriptor(id: &str) -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        id,
        CapabilityVersion("1.0.0".to_string()),
        CapabilityClass::Workflow,
        "Optimize home lighting schedule",
        schema_input(),
        schema_output(),
        vec!["home.lights.manage".to_string()],
        Risk::R3,
        ApprovalClass::Human,
        Reversal::Compensating,
        Idempotency::Required,
        Availability::Available,
        Some(Locality::ControlPlane),
        vec![Privacy::Household],
        vec!["home.lights.optimized".to_string()],
        Some("provider-test".to_string()),
    )
    .unwrap()
}

fn stream_descriptor(id: &str) -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        id,
        CapabilityVersion("1.0.0".to_string()),
        CapabilityClass::Stream,
        "Stream home lighting state changes",
        schema_input(),
        schema_output(),
        vec!["home.lights.read".to_string()],
        Risk::R0,
        ApprovalClass::None,
        Reversal::None,
        Idempotency::NotApplicable,
        Availability::Available,
        Some(Locality::HomeEdge),
        vec![Privacy::Household],
        vec!["home.lights.changed".to_string()],
        Some("provider-test".to_string()),
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// InMemoryCapabilityRegistry
// ---------------------------------------------------------------------------

#[test]
fn ep010_unit_registry_registers_and_resolves() {
    let registry = InMemoryCapabilityRegistry::new();
    let tenant = tid_a();
    let d = query_descriptor("home.lights.query", Availability::Available);
    registry
        .register(d.clone(), ctx(tenant.clone(), "user:alice"))
        .unwrap();
    assert_eq!(registry.len(), 1);
    let resolved = registry
        .resolve(
            "home.lights.query",
            &tenant,
            ctx(tenant.clone(), "user:alice"),
        )
        .unwrap();
    assert_eq!(resolved.id, "home.lights.query");
}

#[test]
fn ep010_unit_registry_register_is_idempotent_for_same_id() {
    let registry = InMemoryCapabilityRegistry::new();
    let tenant = tid_a();
    let d1 = query_descriptor("home.lights.query", Availability::Available);
    let mut d2 = query_descriptor("home.lights.query", Availability::Available);
    d2.description = "Query the state of home lights (updated)".to_string();
    registry
        .register(d1, ctx(tenant.clone(), "user:alice"))
        .unwrap();
    registry
        .register(d2, ctx(tenant.clone(), "user:alice"))
        .unwrap();
    assert_eq!(registry.len(), 1);
}

#[test]
fn ep010_unit_registry_unregister_missing_returns_not_found() {
    let registry = InMemoryCapabilityRegistry::new();
    let tenant = tid_a();
    let err = registry
        .unregister("home.lights.missing", ctx(tenant.clone(), "user:alice"))
        .unwrap_err();
    assert_eq!(err.code, CapabilityErrorCode::NotFound);
    assert_eq!(err.resource.as_deref(), Some("home.lights.missing"));
}

#[test]
fn ep010_unit_registry_tenant_isolation() {
    let registry = InMemoryCapabilityRegistry::new();
    let tenant_a = tid_a();
    let tenant_b = tid_b();
    registry
        .register(
            query_descriptor("home.lights.query", Availability::Available),
            ctx(tenant_a.clone(), "user:alice"),
        )
        .unwrap();
    // Tenant B cannot resolve tenant A's capability.
    let err = registry
        .resolve(
            "home.lights.query",
            &tenant_b,
            ctx(tenant_b.clone(), "user:mallory"),
        )
        .unwrap_err();
    assert_eq!(err.code, CapabilityErrorCode::NotFound);
    // Tenant B's discovery is empty.
    let discovered = registry
        .discover(&tenant_b, ctx(tenant_b.clone(), "user:mallory"))
        .unwrap();
    assert!(discovered.is_empty());
}

#[test]
fn ep010_unit_registry_advertises_only_available() {
    let registry = InMemoryCapabilityRegistry::new();
    let tenant = tid_a();
    registry
        .register(
            query_descriptor("home.lights.query", Availability::Available),
            ctx(tenant.clone(), "user:alice"),
        )
        .unwrap();
    registry
        .register(
            query_descriptor("home.lights.admin", Availability::Unavailable),
            ctx(tenant.clone(), "user:alice"),
        )
        .unwrap();
    registry
        .register(
            query_descriptor("home.lights.lab", Availability::Uncertified),
            ctx(tenant.clone(), "user:alice"),
        )
        .unwrap();
    let discovered = registry
        .discover(&tenant, ctx(tenant.clone(), "user:alice"))
        .unwrap();
    let ids: Vec<&str> = discovered.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, vec!["home.lights.query"]);
}

// ---------------------------------------------------------------------------
// IdempotencyTracker
// ---------------------------------------------------------------------------

#[test]
fn ep010_unit_idempotency_replays_stored_result() {
    let tracker = IdempotencyTracker::new();
    tracker
        .record(IdempotencyRecord {
            key: "op-1".to_string(),
            capability_id: "home.lights.set".to_string(),
            result: serde_json::json!({ "applied": true }),
        })
        .unwrap();
    let record = tracker.get("op-1").unwrap().unwrap();
    assert_eq!(record.capability_id, "home.lights.set");
    assert_eq!(record.result["applied"], true);
    assert_eq!(tracker.len(), 1);
}

#[test]
fn ep010_unit_idempotency_rejects_cross_capability_reuse() {
    let tracker = IdempotencyTracker::new();
    tracker
        .record(IdempotencyRecord {
            key: "op-1".to_string(),
            capability_id: "home.lights.set".to_string(),
            result: serde_json::json!({ "applied": true }),
        })
        .unwrap();
    let err = tracker
        .record(IdempotencyRecord {
            key: "op-1".to_string(),
            capability_id: "home.locks.set".to_string(),
            result: serde_json::json!({ "applied": true }),
        })
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::Conflict);
}

#[test]
fn ep010_unit_idempotency_get_missing_returns_none() {
    let tracker = IdempotencyTracker::new();
    assert!(tracker.get("never-used").unwrap().is_none());
}

// ---------------------------------------------------------------------------
// CapabilityDispatcher: class distinctness, no generic execute
// ---------------------------------------------------------------------------

/// A deterministic query provider. Test-double zone per TESTING.md.
struct TestQuery;

impl QueryCapability for TestQuery {
    fn query(&self, request: QueryRequest) -> Result<QueryResult, CapabilityError> {
        Ok(QueryResult {
            capability_id: request.capability_id,
            output: serde_json::json!({ "state": "on" }),
        })
    }
}

/// A deterministic command provider. Test-double zone per TESTING.md.
struct TestCommand;

impl CommandCapability for TestCommand {
    fn command(&self, request: CommandRequest) -> Result<CommandResult, CapabilityError> {
        Ok(CommandResult {
            capability_id: request.capability_id,
            output: serde_json::json!({ "applied": true }),
        })
    }
}

/// A deterministic workflow provider. Test-double zone per TESTING.md.
struct TestWorkflow;

impl WorkflowCapability for TestWorkflow {
    fn start(&self, request: WorkflowRequest) -> Result<WorkflowHandle, CapabilityError> {
        Ok(WorkflowHandle {
            capability_id: request.capability_id,
            workflow_id: "wf-1".to_string(),
        })
    }

    fn status(
        &self,
        handle: WorkflowHandle,
    ) -> Result<nexus_capabilities::workflow::WorkflowResult, CapabilityError> {
        Ok(nexus_capabilities::workflow::WorkflowResult {
            handle,
            status: nexus_capabilities::workflow::WorkflowStatus::Completed,
            output: Some(serde_json::json!({ "done": true })),
        })
    }
}

/// A deterministic health provider. Test-double zone per TESTING.md.
struct TestHealth;

impl HealthCapability for TestHealth {
    fn health(&self, _context: InvocationContext) -> Result<HealthReport, CapabilityError> {
        Ok(HealthReport {
            target_id: "home-lights".to_string(),
            state: HealthState::Healthy,
            detail: None,
        })
    }
}

/// A deterministic change-feed provider. Test-double zone per
/// TESTING.md.
struct TestChangeFeed;

impl ChangeFeedCapability for TestChangeFeed {
    fn changes_since(
        &self,
        capability_id: String,
        _cursor: Option<ChangeCursor>,
        _context: InvocationContext,
    ) -> Result<nexus_capabilities::changefeed::ChangeBatch, CapabilityError> {
        Ok(nexus_capabilities::changefeed::ChangeBatch {
            capability_id,
            events: vec![],
            next_cursor: ChangeCursor {
                capability_id: "home.lights.query".to_string(),
                cursor: "cursor-2".to_string(),
            },
        })
    }
}

fn dispatcher_with(descriptor: CapabilityDescriptor) -> (CapabilityDispatcher, TenantId) {
    let registry = InMemoryCapabilityRegistry::new();
    let tenant = tid_a();
    registry
        .register(descriptor, ctx(tenant.clone(), "user:alice"))
        .unwrap();
    let dispatcher = CapabilityDispatcher::new(Arc::new(registry));
    (dispatcher, tenant)
}

#[test]
fn ep010_unit_dispatcher_routes_query_to_query_port() {
    let (dispatcher, tenant) = dispatcher_with(query_descriptor(
        "home.lights.query",
        Availability::Available,
    ));
    let result = dispatcher
        .dispatch_query(
            QueryRequest {
                capability_id: "home.lights.query".to_string(),
                context: ctx(tenant.clone(), "user:alice"),
                input: serde_json::json!({}),
            },
            &TestQuery,
        )
        .unwrap();
    assert_eq!(result.output["state"], "on");
}

#[test]
fn ep010_unit_dispatcher_denies_command_via_query_path() {
    // A COMMAND capability cannot be invoked through the query path.
    let (dispatcher, tenant) = dispatcher_with(command_descriptor("home.lights.set"));
    let err = dispatcher
        .dispatch_query(
            QueryRequest {
                capability_id: "home.lights.set".to_string(),
                context: ctx(tenant.clone(), "user:alice"),
                input: serde_json::json!({ "on": true }),
            },
            &TestQuery,
        )
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::Validation);
    assert!(err.0.message.contains("QUERY"));
}

#[test]
fn ep010_unit_dispatcher_denies_query_via_command_path() {
    let (dispatcher, tenant) = dispatcher_with(query_descriptor(
        "home.lights.query",
        Availability::Available,
    ));
    let tracker = IdempotencyTracker::new();
    let err = dispatcher
        .dispatch_command(
            CommandRequest {
                capability_id: "home.lights.query".to_string(),
                context: ctx(tenant.clone(), "user:alice"),
                input: serde_json::json!({}),
                idempotency_key: Some("op-1".to_string()),
            },
            &TestCommand,
            &tracker,
        )
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::Validation);
    assert!(err.0.message.contains("COMMAND"));
}

#[test]
fn ep010_unit_dispatcher_routes_workflow_to_workflow_port() {
    let (dispatcher, tenant) = dispatcher_with(workflow_descriptor("home.lights.optimize"));
    let handle = dispatcher
        .dispatch_workflow(
            WorkflowRequest {
                capability_id: "home.lights.optimize".to_string(),
                context: ctx(tenant.clone(), "user:alice"),
                input: serde_json::json!({}),
                idempotency_key: Some("op-2".to_string()),
            },
            &TestWorkflow,
        )
        .unwrap();
    assert_eq!(handle.workflow_id, "wf-1");
}

#[test]
fn ep010_unit_dispatcher_denies_missing_capability() {
    let (dispatcher, tenant) = dispatcher_with(query_descriptor(
        "home.lights.query",
        Availability::Available,
    ));
    let err = dispatcher
        .dispatch_health(
            "home.lights.missing".to_string(),
            ctx(tenant.clone(), "user:alice"),
            &TestHealth,
        )
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::NotFound);
}

#[test]
fn ep010_unit_dispatcher_denies_unavailable_capability() {
    let (dispatcher, tenant) = dispatcher_with(query_descriptor(
        "home.lights.admin",
        Availability::Unavailable,
    ));
    let err = dispatcher
        .dispatch_health(
            "home.lights.admin".to_string(),
            ctx(tenant.clone(), "user:alice"),
            &TestHealth,
        )
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::Unavailable);
}

#[test]
fn ep010_unit_dispatcher_routes_changefeed_for_stream() {
    let (dispatcher, tenant) = dispatcher_with(stream_descriptor("home.lights.stream"));
    let batch = dispatcher
        .dispatch_changefeed(
            "home.lights.stream".to_string(),
            None,
            ctx(tenant.clone(), "user:alice"),
            &TestChangeFeed,
        )
        .unwrap();
    assert_eq!(batch.next_cursor.cursor, "cursor-2");
}

#[test]
fn ep010_unit_dispatcher_denies_changefeed_for_command_class() {
    let (dispatcher, tenant) = dispatcher_with(command_descriptor("home.lights.set"));
    let err = dispatcher
        .dispatch_changefeed(
            "home.lights.set".to_string(),
            None,
            ctx(tenant.clone(), "user:alice"),
            &TestChangeFeed,
        )
        .unwrap_err();
    // STREAM or QUERY class is required; COMMAND is rejected.
    assert_eq!(err.0.code, CapabilityErrorCode::Validation);
}

#[test]
fn ep010_unit_dispatcher_command_is_idempotent_via_tracker() {
    let (dispatcher, tenant) = dispatcher_with(command_descriptor("home.lights.set"));
    let tracker = IdempotencyTracker::new();
    let request = CommandRequest {
        capability_id: "home.lights.set".to_string(),
        context: ctx(tenant.clone(), "user:alice"),
        input: serde_json::json!({ "on": true }),
        idempotency_key: Some("op-1".to_string()),
    };
    let first = dispatcher
        .dispatch_command(request.clone(), &TestCommand, &tracker)
        .unwrap();
    assert_eq!(first.output["applied"], true);
    assert_eq!(tracker.len(), 1);
    // Replaying the same key returns the stored result without
    // invoking the port again; the provider cannot observe the replay.
    let second = dispatcher
        .dispatch_command(request, &TestCommand, &tracker)
        .unwrap();
    assert_eq!(second.output["applied"], true);
    assert_eq!(tracker.len(), 1);
}

#[test]
fn ep010_unit_dispatcher_denies_cross_tenant_invocation() {
    // Tenant B cannot invoke tenant A's capability: the registry
    // resolve fails closed with NotFound before any port is touched.
    let registry = InMemoryCapabilityRegistry::new();
    let tenant_a = tid_a();
    let tenant_b = tid_b();
    registry
        .register(
            query_descriptor("home.lights.query", Availability::Available),
            ctx(tenant_a.clone(), "user:alice"),
        )
        .unwrap();
    let dispatcher = CapabilityDispatcher::new(Arc::new(registry));
    let err = dispatcher
        .dispatch_query(
            QueryRequest {
                capability_id: "home.lights.query".to_string(),
                context: ctx(tenant_b.clone(), "user:mallory"),
                input: serde_json::json!({}),
            },
            &TestQuery,
        )
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::NotFound);
}
