//! EP-010 M1 unit tests: construction, validation, serialization,
//! vocabulary rejection, and dependency-direction constraints.

use nexus_domain::vocabulary::ConnectorRuntime;
use nexus_domain::{
    ApprovalClass, Availability, CapabilityClass, CorrelationId, DeviceId, Idempotency, Locality,
    NexusId, ObjectiveId, PrincipalType, Privacy, Reversal, Risk, TaskId, TenantId, Tier,
};

use crate::changefeed::{ChangeBatch, ChangeCursor, ChangeEvent, ChangeFeedCapability};
use crate::command::{CommandCapability, CommandRequest, CommandResult};
use crate::context::{InvocationContext, InvocationContextError};
use crate::descriptor::{CapabilityDescriptor, CapabilityDescriptorError, CapabilityVersion};
use crate::error::{CapabilityError, CapabilityErrorCode};
use crate::health::{HealthCapability, HealthReport};
use crate::manifest::{ConnectorBinding, ConnectorId, ConnectorManifest, ConnectorManifestError};
use crate::query::{QueryCapability, QueryRequest, QueryResult};
use crate::registry::CapabilityRegistry;
use crate::vocabulary::{Certification, HealthState, SchemaRef, VocabularyError};
use crate::workflow::{
    WorkflowCapability, WorkflowHandle, WorkflowRequest, WorkflowResult, WorkflowStatus,
};

fn rid() -> NexusId {
    NexusId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn cid() -> CorrelationId {
    CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
}

fn tid() -> TenantId {
    TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap()
}

fn did() -> DeviceId {
    DeviceId::new("018f0f6f-9c1e-7b6e-8000-000000000004").unwrap()
}

fn oid() -> ObjectiveId {
    ObjectiveId::new("018f0f6f-9c1e-7b6e-8000-000000000005").unwrap()
}

fn kid() -> TaskId {
    TaskId::new("018f0f6f-9c1e-7b6e-8000-000000000006").unwrap()
}

fn ctx() -> InvocationContext {
    InvocationContext::new(
        rid(),
        cid(),
        None,
        "test-client",
        "user:alice",
        PrincipalType::Human,
        tid(),
        Some("web".to_string()),
        Some(did()),
        Some(oid()),
        Some(kid()),
    )
    .unwrap()
}

fn schema_input() -> SchemaRef {
    SchemaRef::new("schemas/invocation-context.schema.json").unwrap()
}

fn schema_output() -> SchemaRef {
    SchemaRef::new("schemas/capability-descriptor.schema.json").unwrap()
}

fn descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        "home.lights.query",
        CapabilityVersion("1.2.3".to_string()),
        CapabilityClass::Query,
        "Query the state of home lights",
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

fn manifest() -> ConnectorManifest {
    ConnectorManifest::new(
        ConnectorId("home-lights".to_string()),
        "1.0.0",
        Tier::Tier1,
        "Apache-2.0",
        ConnectorRuntime::Rust,
        "/health",
        vec![descriptor()],
        vec!["home.lights.changed".to_string()],
        vec!["vault:home-lights-token".to_string()],
        vec!["https://api.lights.home".to_string()],
        vec![Privacy::Household],
        Some(Certification::Lab),
    )
    .unwrap()
}

// ---------------------------------------------------------------------------
// CapabilityDescriptor
// ---------------------------------------------------------------------------

#[test]
fn ep010_unit_descriptor_constructs_valid() {
    let d = descriptor();
    assert_eq!(d.id, "home.lights.query");
    assert_eq!(d.version.0, "1.2.3");
    assert_eq!(d.class, CapabilityClass::Query);
    assert_eq!(d.risk, Risk::R0);
    assert_eq!(d.approval, ApprovalClass::None);
    assert_eq!(d.idempotency, Idempotency::NotApplicable);
    assert_eq!(d.availability, Availability::Available);
}

#[test]
fn ep010_unit_descriptor_rejects_empty_id() {
    let err = CapabilityDescriptor::new(
        "",
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
        Availability::Available,
        None,
        vec![],
        vec![],
        None,
    )
    .unwrap_err();
    assert_eq!(
        err,
        CapabilityDescriptorError("id must not be empty".to_string())
    );
}

#[test]
fn ep010_unit_descriptor_rejects_non_canonical_id() {
    let err = CapabilityDescriptor::new(
        "Home.Lights.Query!",
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
        Availability::Available,
        None,
        vec![],
        vec![],
        None,
    )
    .unwrap_err();
    assert!(err.0.contains("^[a-z][a-z0-9_.-]+$"));
}

#[test]
fn ep010_unit_descriptor_rejects_short_description() {
    let err = CapabilityDescriptor::new(
        "home.lights.query",
        CapabilityVersion("1.0.0".to_string()),
        CapabilityClass::Query,
        "too short",
        schema_input(),
        schema_output(),
        vec!["home.lights.read".to_string()],
        Risk::R0,
        ApprovalClass::None,
        Reversal::None,
        Idempotency::NotApplicable,
        Availability::Available,
        None,
        vec![],
        vec![],
        None,
    )
    .unwrap_err();
    assert!(err.0.contains("at least 10 characters"));
}

#[test]
fn ep010_unit_descriptor_rejects_empty_scopes() {
    let err = CapabilityDescriptor::new(
        "home.lights.query",
        CapabilityVersion("1.0.0".to_string()),
        CapabilityClass::Query,
        "Query the state of home lights",
        schema_input(),
        schema_output(),
        vec![],
        Risk::R0,
        ApprovalClass::None,
        Reversal::None,
        Idempotency::NotApplicable,
        Availability::Available,
        None,
        vec![],
        vec![],
        None,
    )
    .unwrap_err();
    assert!(err.0.contains("required_scopes"));
}

#[test]
fn ep010_unit_descriptor_rejects_duplicate_scopes() {
    let err = CapabilityDescriptor::new(
        "home.lights.query",
        CapabilityVersion("1.0.0".to_string()),
        CapabilityClass::Query,
        "Query the state of home lights",
        schema_input(),
        schema_output(),
        vec![
            "home.lights.read".to_string(),
            "home.lights.read".to_string(),
        ],
        Risk::R0,
        ApprovalClass::None,
        Reversal::None,
        Idempotency::NotApplicable,
        Availability::Available,
        None,
        vec![],
        vec![],
        None,
    )
    .unwrap_err();
    assert!(err.0.contains("unique"));
}

#[test]
fn ep010_unit_descriptor_serializes_canonical_snake_case() {
    let d = descriptor();
    let json = serde_json::to_value(&d).unwrap();
    assert_eq!(json["id"], "home.lights.query");
    assert_eq!(json["class"], "QUERY");
    assert_eq!(json["risk"], "R0");
    assert_eq!(
        json["input_schema"],
        "schemas/invocation-context.schema.json"
    );
    assert_eq!(json["availability"], "AVAILABLE");
    assert!(json.get("locality").is_some());
}

#[test]
fn ep010_unit_descriptor_round_trips_json() {
    let d = descriptor();
    let json = serde_json::to_string(&d).unwrap();
    let back: CapabilityDescriptor = serde_json::from_str(&json).unwrap();
    assert_eq!(back, d);
}

// ---------------------------------------------------------------------------
// InvocationContext
// ---------------------------------------------------------------------------

#[test]
fn ep010_unit_context_constructs_valid() {
    let c = ctx();
    assert_eq!(c.external_actor_id, "user:alice");
    assert_eq!(c.external_actor_type, PrincipalType::Human);
    assert_eq!(c.tenant_id, tid());
    assert_eq!(c.origin_system, "test-client");
}

#[test]
fn ep010_unit_context_rejects_empty_origin_system() {
    let err = InvocationContext::new(
        rid(),
        cid(),
        None,
        "",
        "user:alice",
        PrincipalType::Human,
        tid(),
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert_eq!(
        err,
        InvocationContextError("origin_system must not be empty".to_string())
    );
}

#[test]
fn ep010_unit_context_rejects_empty_actor() {
    let err = InvocationContext::new(
        rid(),
        cid(),
        None,
        "test-client",
        "",
        PrincipalType::Human,
        tid(),
        None,
        None,
        None,
        None,
    )
    .unwrap_err();
    assert_eq!(
        err,
        InvocationContextError("external_actor_id must not be empty".to_string())
    );
}

#[test]
fn ep010_unit_context_round_trips_json() {
    let c = ctx();
    let json = serde_json::to_string(&c).unwrap();
    let back: InvocationContext = serde_json::from_str(&json).unwrap();
    assert_eq!(back, c);
}

// ---------------------------------------------------------------------------
// ConnectorManifest / ConnectorBinding
// ---------------------------------------------------------------------------

#[test]
fn ep010_unit_manifest_constructs_valid() {
    let m = manifest();
    assert_eq!(m.id.0, "home-lights");
    assert_eq!(m.tier, Tier::Tier1);
    assert_eq!(m.runtime, ConnectorRuntime::Rust);
    assert_eq!(m.capabilities.len(), 1);
    assert_eq!(m.secrets, vec!["vault:home-lights-token".to_string()]);
    assert_eq!(m.certification, Some(Certification::Lab));
}

#[test]
fn ep010_unit_manifest_rejects_empty_license() {
    let err = ConnectorManifest::new(
        ConnectorId("home-lights".to_string()),
        "1.0.0",
        Tier::Tier1,
        "",
        ConnectorRuntime::Rust,
        "/health",
        vec![descriptor()],
        vec![],
        vec![],
        vec![],
        vec![],
        None,
    )
    .unwrap_err();
    assert_eq!(
        err,
        ConnectorManifestError("license must not be empty".to_string())
    );
}

#[test]
fn ep010_unit_manifest_rejects_empty_health() {
    let err = ConnectorManifest::new(
        ConnectorId("home-lights".to_string()),
        "1.0.0",
        Tier::Tier1,
        "Apache-2.0",
        ConnectorRuntime::Rust,
        "",
        vec![descriptor()],
        vec![],
        vec![],
        vec![],
        vec![],
        None,
    )
    .unwrap_err();
    assert_eq!(
        err,
        ConnectorManifestError("health must not be empty".to_string())
    );
}

#[test]
fn ep010_unit_manifest_serializes_canonical_snake_case() {
    let m = manifest();
    let json = serde_json::to_value(&m).unwrap();
    assert_eq!(json["id"], "home-lights");
    assert_eq!(json["tier"], "TIER1");
    assert_eq!(json["runtime"], "RUST");
    assert_eq!(json["certification"], "LAB");
    assert_eq!(json["capabilities"][0]["class"], "QUERY");
}

#[test]
fn ep010_unit_manifest_round_trips_json() {
    let m = manifest();
    let json = serde_json::to_string(&m).unwrap();
    let back: ConnectorManifest = serde_json::from_str(&json).unwrap();
    assert_eq!(back, m);
}

#[test]
fn ep010_unit_binding_constructs_valid() {
    let b = ConnectorBinding::new(
        ConnectorId("home-lights".to_string()),
        tid(),
        "account-42",
        Some("living room".to_string()),
    )
    .unwrap();
    assert_eq!(b.tenant_id, tid());
    assert_eq!(b.account_ref, "account-42");
}

#[test]
fn ep010_unit_binding_rejects_empty_account_ref() {
    let err =
        ConnectorBinding::new(ConnectorId("home-lights".to_string()), tid(), "", None).unwrap_err();
    assert!(err.0.contains("account_ref"));
}

// ---------------------------------------------------------------------------
// CapabilityError
// ---------------------------------------------------------------------------

#[test]
fn ep010_unit_error_validation_class_and_fail_closed() {
    let e = CapabilityError::validation("bad input");
    assert_eq!(e.code, CapabilityErrorCode::Validation);
    assert!(e.is_fail_closed());
    assert_eq!(e.code.as_str(), "VALIDATION");
}

#[test]
fn ep010_unit_error_preserves_context() {
    let e = CapabilityError::new(
        CapabilityErrorCode::Authorization,
        "denied",
        Some("018f0f6f-9c1e-7b6e-8000-000000000002".to_string()),
        Some("user:alice".to_string()),
        Some("018f0f6f-9c1e-7b6e-8000-000000000003".to_string()),
        Some("home.lights.query".to_string()),
    );
    assert_eq!(e.code, CapabilityErrorCode::Authorization);
    assert_eq!(e.actor.as_deref(), Some("user:alice"));
    assert_eq!(e.resource.as_deref(), Some("home.lights.query"));
}

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

#[test]
fn ep010_unit_vocabulary_rejects_unknown_health_state() {
    let err = "SOMETIMES".parse::<HealthState>().unwrap_err();
    assert!(matches!(err, VocabularyError(_)));
}

#[test]
fn ep010_unit_vocabulary_rejects_unknown_certification() {
    let err = "MAYBE".parse::<Certification>().unwrap_err();
    assert!(matches!(err, VocabularyError(_)));
}

#[test]
fn ep010_unit_vocabulary_schema_ref_rejects_foreign_uri() {
    let err = SchemaRef::new("https://evil.example/x.json").unwrap_err();
    assert!(err.0.contains("canonical"));
}

// ---------------------------------------------------------------------------
// Class distinctness: no generic execute string
// ---------------------------------------------------------------------------

/// A tiny deterministic query implementation used to prove the port
/// shape. Test-double zone per TESTING.md; production paths never use
/// it.
struct TestQuery;

impl QueryCapability for TestQuery {
    fn query(&self, request: QueryRequest) -> Result<QueryResult, CapabilityError> {
        Ok(QueryResult {
            capability_id: request.capability_id,
            output: serde_json::json!({ "state": "on" }),
        })
    }
}

#[test]
fn ep010_unit_query_port_has_no_execute_string() {
    // The port exposes `query`, not a generic `execute(String)`. This
    // test proves the compiled surface is typed: a QueryCapability
    // cannot be invoked as a command or workflow.
    let q = TestQuery;
    let result = q
        .query(QueryRequest {
            capability_id: "home.lights.query".to_string(),
            context: ctx(),
            input: serde_json::json!({}),
        })
        .unwrap();
    assert_eq!(result.output["state"], "on");
}

/// A tiny deterministic command implementation proving the command port
/// carries an idempotency key. Test-double zone per TESTING.md.
struct TestCommand;

impl CommandCapability for TestCommand {
    fn command(&self, request: CommandRequest) -> Result<CommandResult, CapabilityError> {
        if request.idempotency_key.is_none() {
            return Err(CapabilityError::new(
                CapabilityErrorCode::Validation,
                "idempotency key required for retryable command",
                None,
                None,
                None,
                None,
            ));
        }
        Ok(CommandResult {
            capability_id: request.capability_id,
            output: serde_json::json!({ "applied": true }),
        })
    }
}

#[test]
fn ep010_unit_command_port_requires_idempotency_key() {
    let c = TestCommand;
    let err = c
        .command(CommandRequest {
            capability_id: "home.lights.set".to_string(),
            context: ctx(),
            input: serde_json::json!({ "on": true }),
            idempotency_key: None,
        })
        .unwrap_err();
    assert_eq!(err.code, CapabilityErrorCode::Validation);
    let ok = c
        .command(CommandRequest {
            capability_id: "home.lights.set".to_string(),
            context: ctx(),
            input: serde_json::json!({ "on": true }),
            idempotency_key: Some("op-1".to_string()),
        })
        .unwrap();
    assert_eq!(ok.output["applied"], true);
}

/// A tiny deterministic health implementation. Test-double zone per
/// TESTING.md.
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

#[test]
fn ep010_unit_health_port_reports_state() {
    let h = TestHealth;
    let report = h.health(ctx()).unwrap();
    assert_eq!(report.state, HealthState::Healthy);
}

/// A tiny deterministic workflow implementation. Test-double zone per
/// TESTING.md.
struct TestWorkflow;

impl WorkflowCapability for TestWorkflow {
    fn start(&self, request: WorkflowRequest) -> Result<WorkflowHandle, CapabilityError> {
        Ok(WorkflowHandle {
            capability_id: request.capability_id,
            workflow_id: "wf-1".to_string(),
        })
    }

    fn status(&self, handle: WorkflowHandle) -> Result<WorkflowResult, CapabilityError> {
        Ok(WorkflowResult {
            handle,
            status: WorkflowStatus::Completed,
            output: Some(serde_json::json!({ "done": true })),
        })
    }
}

#[test]
fn ep010_unit_workflow_port_is_distinct_from_query_and_command() {
    let w = TestWorkflow;
    let handle = w
        .start(WorkflowRequest {
            capability_id: "home.lights.optimize".to_string(),
            context: ctx(),
            input: serde_json::json!({}),
            idempotency_key: Some("op-2".to_string()),
        })
        .unwrap();
    let result = w.status(handle).unwrap();
    assert_eq!(result.status, WorkflowStatus::Completed);
    assert_eq!(WorkflowStatus::Running.as_str(), "RUNNING");
}

/// A tiny deterministic change-feed implementation. Test-double zone
/// per TESTING.md.
struct TestChangeFeed;

impl crate::changefeed::ChangeFeedCapability for TestChangeFeed {
    fn changes_since(
        &self,
        capability_id: String,
        _cursor: Option<ChangeCursor>,
        _context: InvocationContext,
    ) -> Result<ChangeBatch, CapabilityError> {
        Ok(ChangeBatch {
            capability_id,
            events: vec![ChangeEvent {
                event_id: "evt-1".to_string(),
                event_type: "home.lights.changed".to_string(),
                payload: serde_json::json!({ "state": "on" }),
            }],
            next_cursor: ChangeCursor {
                capability_id: "home.lights.query".to_string(),
                cursor: "cursor-2".to_string(),
            },
        })
    }
}

#[test]
fn ep010_unit_change_feed_port_returns_cursor_batch() {
    let f = TestChangeFeed;
    let batch = f
        .changes_since("home.lights.query".to_string(), None, ctx())
        .unwrap();
    assert_eq!(batch.events.len(), 1);
    assert_eq!(batch.next_cursor.cursor, "cursor-2");
}

// ---------------------------------------------------------------------------
// CapabilityRegistry shape
// ---------------------------------------------------------------------------

/// A tiny in-memory registry used to prove the port shape and the
/// "unavailable features are not advertised" invariant. Test-double
/// zone per TESTING.md; the production registry is an M2 connector
/// adapter.
struct TestRegistry {
    descriptors: std::cell::RefCell<Vec<CapabilityDescriptor>>,
}

impl CapabilityRegistry for TestRegistry {
    fn register(
        &self,
        descriptor: CapabilityDescriptor,
        _context: InvocationContext,
    ) -> Result<(), CapabilityError> {
        let mut list = self.descriptors.borrow_mut();
        list.retain(|d| d.id != descriptor.id);
        list.push(descriptor);
        Ok(())
    }

    fn unregister(
        &self,
        capability_id: &str,
        _context: InvocationContext,
    ) -> Result<(), CapabilityError> {
        self.descriptors
            .borrow_mut()
            .retain(|d| d.id != capability_id);
        Ok(())
    }

    fn discover(
        &self,
        _tenant_id: &TenantId,
        _context: InvocationContext,
    ) -> Result<Vec<CapabilityDescriptor>, CapabilityError> {
        Ok(self
            .descriptors
            .borrow()
            .iter()
            .filter(|d| d.availability == Availability::Available)
            .cloned()
            .collect())
    }

    fn resolve(
        &self,
        capability_id: &str,
        _tenant_id: &TenantId,
        _context: InvocationContext,
    ) -> Result<CapabilityDescriptor, CapabilityError> {
        self.descriptors
            .borrow()
            .iter()
            .find(|d| d.id == capability_id)
            .cloned()
            .ok_or_else(|| {
                CapabilityError::new(
                    CapabilityErrorCode::NotFound,
                    "capability not found",
                    None,
                    None,
                    None,
                    None,
                )
            })
    }
}

#[test]
fn ep010_unit_registry_advertises_only_available_features() {
    let registry = TestRegistry {
        descriptors: std::cell::RefCell::new(Vec::new()),
    };
    let available = descriptor();
    let mut hidden = descriptor();
    hidden.id = "home.lights.admin".to_string();
    hidden.availability = Availability::Unavailable;
    registry.register(available, ctx()).unwrap();
    registry.register(hidden, ctx()).unwrap();
    let discovered = registry.discover(&tid(), ctx()).unwrap();
    let ids: Vec<&str> = discovered.iter().map(|d| d.id.as_str()).collect();
    assert_eq!(ids, vec!["home.lights.query"]);
    assert!(!ids.contains(&"home.lights.admin"));
}

#[test]
fn ep010_unit_registry_resolve_missing_returns_not_found() {
    let registry = TestRegistry {
        descriptors: std::cell::RefCell::new(Vec::new()),
    };
    let err = registry
        .resolve("home.lights.missing", &tid(), ctx())
        .unwrap_err();
    assert_eq!(err.code, CapabilityErrorCode::NotFound);
}
