//! EP-010 M4 forced-failure tests: unavailable, malformed, duplicate,
//! denied, and fail-closed behavior through the real deterministic
//! core and the real JSON Schema validator.
//!
//! The real failure mechanisms exercised here are the real ones: a
//! missing capability (not found), an unavailable capability
//! (fail closed), a duplicate idempotency key across capabilities
//! (conflict), a cross-tenant invocation (denied), a malformed
//! descriptor (validation), and schema-level rejections (duplicate
//! array items, unknown classes) against the canonical
//! `connector-manifest.schema.json` and
//! `capability-descriptor.schema.json` using the real `jsonschema`
//! validator. No component being proven is mocked.

use std::path::PathBuf;
use std::sync::Arc;

use nexus_capabilities::context::InvocationContext;
use nexus_capabilities::descriptor::{CapabilityDescriptor, CapabilityVersion};
use nexus_capabilities::error::{CapabilityError, CapabilityErrorCode};
use nexus_capabilities::health::{HealthCapability, HealthReport};
use nexus_capabilities::query::{QueryCapability, QueryRequest, QueryResult};
use nexus_capabilities::registry::CapabilityRegistry;
use nexus_capabilities::vocabulary::{HealthState, SchemaRef};
use nexus_domain::{
    ApprovalClass, Availability, CapabilityClass, CorrelationId, Idempotency, NexusId,
    PrincipalType, Reversal, Risk, TenantId,
};

use nexus_connectors::dispatcher::CapabilityDispatcher;
use nexus_connectors::idempotency::{IdempotencyRecord, IdempotencyTracker};
use nexus_connectors::registry::InMemoryCapabilityRegistry;

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

fn descriptor(
    id: &str,
    class: CapabilityClass,
    availability: Availability,
) -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        id,
        CapabilityVersion("1.0.0".to_string()),
        class,
        "A deterministic test capability",
        schema_input(),
        schema_output(),
        vec!["test.scope".to_string()],
        Risk::R1,
        ApprovalClass::None,
        Reversal::None,
        Idempotency::NotApplicable,
        availability,
        None,
        vec![],
        vec![],
        Some("provider-test".to_string()),
    )
    .unwrap()
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn load_schema(relative: &str) -> serde_json::Value {
    let path = root().join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read schema {relative}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("cannot parse schema {relative}: {e}"))
}

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

/// A provider that always fails unavailable. Test-double zone per
/// TESTING.md.
struct DownQuery;

impl QueryCapability for DownQuery {
    fn query(&self, request: QueryRequest) -> Result<QueryResult, CapabilityError> {
        Err(CapabilityError::new(
            CapabilityErrorCode::Unavailable,
            "provider unavailable",
            Some(request.context.correlation_id.to_string()),
            Some(request.context.external_actor_id),
            Some(request.context.tenant_id.to_string()),
            Some(request.capability_id),
        ))
    }
}

/// A health provider. Test-double zone per TESTING.md.
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

fn dispatcher_with(d: CapabilityDescriptor) -> (CapabilityDispatcher, TenantId) {
    let registry = InMemoryCapabilityRegistry::new();
    let tenant = tid_a();
    registry
        .register(d, ctx(tenant.clone(), "user:alice"))
        .unwrap();
    (CapabilityDispatcher::new(Arc::new(registry)), tenant)
}

// ---------------------------------------------------------------------------
// Unavailable and fail-closed
// ---------------------------------------------------------------------------

#[test]
fn ep010_failure_missing_capability_returns_typed_not_found() {
    let (dispatcher, tenant) = dispatcher_with(descriptor(
        "test.query",
        CapabilityClass::Query,
        Availability::Available,
    ));
    let err = dispatcher
        .dispatch_health(
            "test.missing".to_string(),
            ctx(tenant.clone(), "user:alice"),
            &TestHealth,
        )
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::NotFound);
    assert_eq!(err.0.resource.as_deref(), Some("test.missing"));
    assert_eq!(err.0.correlation.as_deref(), Some(cid().as_str()));
    assert_eq!(err.0.actor.as_deref(), Some("user:alice"));
}

#[test]
fn ep010_failure_unavailable_capability_fails_closed() {
    let (dispatcher, tenant) = dispatcher_with(descriptor(
        "test.admin",
        CapabilityClass::Query,
        Availability::Unavailable,
    ));
    let err = dispatcher
        .dispatch_health(
            "test.admin".to_string(),
            ctx(tenant.clone(), "user:alice"),
            &TestHealth,
        )
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::Unavailable);
}

#[test]
fn ep010_failure_provider_error_is_typed_and_never_allows() {
    let (dispatcher, tenant) = dispatcher_with(descriptor(
        "test.query",
        CapabilityClass::Query,
        Availability::Available,
    ));
    let err = dispatcher
        .dispatch_query(
            QueryRequest {
                capability_id: "test.query".to_string(),
                context: ctx(tenant.clone(), "user:alice"),
                input: serde_json::json!({}),
            },
            &DownQuery,
        )
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::Unavailable);
    assert!(err.0.message.contains("provider unavailable"));
}

// ---------------------------------------------------------------------------
// Malformed input and duplicate requests
// ---------------------------------------------------------------------------

#[test]
fn ep010_failure_malformed_descriptor_rejected_at_construction() {
    let err = CapabilityDescriptor::new(
        "UPPER.Case!",
        CapabilityVersion("1.0.0".to_string()),
        CapabilityClass::Query,
        "A deterministic test capability",
        schema_input(),
        schema_output(),
        vec!["test.scope".to_string()],
        Risk::R1,
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
fn ep010_failure_idempotency_duplicate_key_across_capabilities_conflicts() {
    let tracker = IdempotencyTracker::new();
    tracker
        .record(IdempotencyRecord {
            key: "op-1".to_string(),
            capability_id: "test.set".to_string(),
            result: serde_json::json!({ "applied": true }),
        })
        .unwrap();
    let err = tracker
        .record(IdempotencyRecord {
            key: "op-1".to_string(),
            capability_id: "test.other".to_string(),
            result: serde_json::json!({ "applied": true }),
        })
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::Conflict);
}

// ---------------------------------------------------------------------------
// Denied permission
// ---------------------------------------------------------------------------

#[test]
fn ep010_failure_cross_tenant_invocation_is_denied() {
    let registry = InMemoryCapabilityRegistry::new();
    let tenant_a = tid_a();
    let tenant_b = tid_b();
    registry
        .register(
            descriptor(
                "test.query",
                CapabilityClass::Query,
                Availability::Available,
            ),
            ctx(tenant_a.clone(), "user:alice"),
        )
        .unwrap();
    let dispatcher = CapabilityDispatcher::new(Arc::new(registry));
    // Tenant B cannot even resolve tenant A's capability.
    let err = dispatcher
        .dispatch_query(
            QueryRequest {
                capability_id: "test.query".to_string(),
                context: ctx(tenant_b.clone(), "user:mallory"),
                input: serde_json::json!({}),
            },
            &TestQuery,
        )
        .unwrap_err();
    assert_eq!(err.0.code, CapabilityErrorCode::NotFound);
}

#[test]
fn ep010_failure_unregister_missing_returns_typed_not_found() {
    let registry = InMemoryCapabilityRegistry::new();
    let tenant = tid_a();
    let err = registry
        .unregister("test.missing", ctx(tenant.clone(), "user:alice"))
        .unwrap_err();
    assert_eq!(err.code, CapabilityErrorCode::NotFound);
    assert_eq!(err.resource.as_deref(), Some("test.missing"));
}

// ---------------------------------------------------------------------------
// Schema authority: the canonical schema rejects abuse
// ---------------------------------------------------------------------------

fn validator_for(schema: &serde_json::Value) -> jsonschema::Validator {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_retriever(LocalSchemasRetriever { root: root() })
        .build(schema)
        .expect("canonical schema must compile")
}

#[derive(Clone)]
struct LocalSchemasRetriever {
    root: PathBuf,
}

impl jsonschema::Retrieve for LocalSchemasRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let raw = uri.as_str();
        let name = raw
            .strip_prefix("https://schemas.nexus.local/")
            .ok_or_else(|| format!("ref outside canonical schema namespace: {raw}"))?;
        let file = name.split('/').next_back().unwrap_or(name);
        let path = self.root.join("schemas").join(file);
        let text = std::fs::read_to_string(&path)?;
        let value: serde_json::Value = serde_json::from_str(&text)?;
        Ok(value)
    }
}

fn manifest_json() -> serde_json::Value {
    serde_json::json!({
        "id": "home-lights",
        "version": "1.0.0",
        "tier": "TIER1",
        "license": "Apache-2.0",
        "runtime": "RUST",
        "health": "/health",
        "capabilities": [],
        "events": ["home.lights.changed"],
        "secrets": ["vault:home-lights-token"],
        "network_origins": ["https://api.lights.home"],
        "data_classes": ["HOUSEHOLD"],
        "certification": "LAB"
    })
}

#[test]
fn ep010_failure_schema_rejects_duplicate_secrets() {
    let schema = load_schema("schemas/connector-manifest.schema.json");
    let validator = validator_for(&schema);
    let mut instance = manifest_json();
    instance["secrets"] = serde_json::json!(["vault:home-lights-token", "vault:home-lights-token"]);
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(
        !errors.is_empty(),
        "duplicate secret references must fail canonical schema validation"
    );
}

#[test]
fn ep010_failure_schema_rejects_duplicate_events() {
    let schema = load_schema("schemas/connector-manifest.schema.json");
    let validator = validator_for(&schema);
    let mut instance = manifest_json();
    instance["events"] = serde_json::json!(["home.lights.changed", "home.lights.changed"]);
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(
        !errors.is_empty(),
        "duplicate event types must fail canonical schema validation"
    );
}

#[test]
fn ep010_failure_schema_rejects_duplicate_network_origins() {
    let schema = load_schema("schemas/connector-manifest.schema.json");
    let validator = validator_for(&schema);
    let mut instance = manifest_json();
    instance["network_origins"] =
        serde_json::json!(["https://api.lights.home", "https://api.lights.home"]);
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(
        !errors.is_empty(),
        "duplicate network origins must fail canonical schema validation"
    );
}

#[test]
fn ep010_failure_schema_rejects_unknown_class() {
    let schema = load_schema("schemas/capability-descriptor.schema.json");
    let validator = validator_for(&schema);
    let mut instance = serde_json::json!({
        "id": "test.query",
        "version": "1.0.0",
        "class": "EXECUTE_ANYTHING",
        "description": "A deterministic test capability",
        "input_schema": "schemas/invocation-context.schema.json",
        "output_schema": "schemas/capability-descriptor.schema.json",
        "required_scopes": ["test.scope"],
        "risk": "R1",
        "approval": "NONE",
        "reversal": "NONE",
        "idempotency": "NOT_APPLICABLE",
        "availability": "AVAILABLE"
    });
    instance["class"] = serde_json::json!("EXECUTE_ANYTHING");
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(
        !errors.is_empty(),
        "an unknown capability class must fail canonical schema validation"
    );
}

#[test]
fn ep010_failure_schema_rejects_missing_required() {
    let schema = load_schema("schemas/connector-manifest.schema.json");
    let validator = validator_for(&schema);
    let mut instance = manifest_json();
    instance.as_object_mut().unwrap().remove("secrets");
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(
        !errors.is_empty(),
        "a manifest missing a required field must fail canonical schema validation"
    );
}
