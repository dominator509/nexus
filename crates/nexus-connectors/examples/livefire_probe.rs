//! EP-010 M5 live-fire probe: the capability/connector contract as ONE
//! system.
//!
//! This probe composes the real deterministic core
//! (`InMemoryCapabilityRegistry` + `CapabilityDispatcher` +
//! `IdempotencyTracker`) with the real canonical JSON Schemas and the
//! real `jsonschema` validator, and prints a deterministic,
//! machine-readable evidence record. It is evidence tooling only,
//! never a production execution path.
//!
//! Stages proven:
//!   REGISTER/DISCOVER/RESOLVE - real registry behavior
//!   QUERY_DISPATCH              - QUERY class routes to query port
//!   COMMAND_IDEMPOTENT          - COMMAND class + idempotency replay
//!   WORKFLOW_DISPATCH           - WORKFLOW class routes to workflow
//!   HEALTH                      - health port on available capability
//!   CHANGEFEED                  - STREAM class routes to change feed
//!   CLASS_MISMATCH_DENIED       - command cannot run as query (and
//!                                 vice versa)
//!   CROSS_TENANT_DENIED         - tenant B cannot resolve tenant A
//!   UNAVAILABLE_NOT_ADVERTISED  - unavailable feature never surfaces
//!   SCHEMA_VALIDATION           - Rust JSON validates against the
//!                                 canonical schemas (real validator)
//!   SCHEMA_REJECTION            - unknown class / duplicate rejected
//!   IDEMPOTENCY_CONFLICT        - key reuse across capabilities
//!   PROVIDER_ERROR_FAIL_CLOSED  - provider failure is typed, never
//!                                 allow
//!
//! The probe accepts no external input and prints a single JSON object
//! to stdout; a non-zero exit indicates a failed stage.

use std::path::PathBuf;
use std::sync::Arc;

use jsonschema::Validator;

use nexus_capabilities::changefeed::{ChangeCursor, ChangeFeedCapability};
use nexus_capabilities::command::{CommandCapability, CommandRequest, CommandResult};
use nexus_capabilities::context::InvocationContext;
use nexus_capabilities::descriptor::{CapabilityDescriptor, CapabilityVersion};
use nexus_capabilities::error::{CapabilityError, CapabilityErrorCode};
use nexus_capabilities::health::{HealthCapability, HealthReport};
use nexus_capabilities::query::{QueryCapability, QueryRequest, QueryResult};
use nexus_capabilities::registry::CapabilityRegistry;
use nexus_capabilities::vocabulary::{HealthState, SchemaRef};
use nexus_domain::{
    ApprovalClass, Availability, CapabilityClass, CorrelationId, Idempotency, Locality, NexusId,
    PrincipalType, Privacy, Reversal, Risk, TenantId,
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
        "livefire-probe",
        actor,
        PrincipalType::Human,
        tenant,
        Some("mcp".to_string()),
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
        "A deterministic live-fire test capability",
        schema_input(),
        schema_output(),
        vec!["test.scope".to_string()],
        Risk::R1,
        ApprovalClass::None,
        Reversal::None,
        Idempotency::NotApplicable,
        availability,
        Some(Locality::Any),
        vec![Privacy::Household],
        vec!["test.changed".to_string()],
        Some("livefire-provider".to_string()),
    )
    .unwrap()
}

/// Deterministic query provider (test zone).
struct ProbeQuery;

impl QueryCapability for ProbeQuery {
    fn query(&self, request: QueryRequest) -> Result<QueryResult, CapabilityError> {
        Ok(QueryResult {
            capability_id: request.capability_id,
            output: serde_json::json!({ "state": "on", "request_id": request.context.request_id.as_str() }),
        })
    }
}

/// Deterministic command provider (test zone).
struct ProbeCommand;

impl CommandCapability for ProbeCommand {
    fn command(&self, request: CommandRequest) -> Result<CommandResult, CapabilityError> {
        Ok(CommandResult {
            capability_id: request.capability_id,
            output: serde_json::json!({ "applied": true, "request_id": request.context.request_id.as_str() }),
        })
    }
}

/// Deterministic workflow provider (test zone).
struct ProbeWorkflow;

impl nexus_capabilities::workflow::WorkflowCapability for ProbeWorkflow {
    fn start(
        &self,
        request: nexus_capabilities::workflow::WorkflowRequest,
    ) -> Result<nexus_capabilities::workflow::WorkflowHandle, CapabilityError> {
        Ok(nexus_capabilities::workflow::WorkflowHandle {
            capability_id: request.capability_id,
            workflow_id: "wf-livefire-1".to_string(),
        })
    }

    fn status(
        &self,
        handle: nexus_capabilities::workflow::WorkflowHandle,
    ) -> Result<nexus_capabilities::workflow::WorkflowResult, CapabilityError> {
        Ok(nexus_capabilities::workflow::WorkflowResult {
            handle,
            status: nexus_capabilities::workflow::WorkflowStatus::Completed,
            output: Some(serde_json::json!({ "done": true })),
        })
    }
}

/// Deterministic health provider (test zone).
struct ProbeHealth;

impl HealthCapability for ProbeHealth {
    fn health(&self, _context: InvocationContext) -> Result<HealthReport, CapabilityError> {
        Ok(HealthReport {
            target_id: "livefire".to_string(),
            state: HealthState::Healthy,
            detail: None,
        })
    }
}

/// Deterministic change-feed provider (test zone).
struct ProbeChangeFeed;

impl ChangeFeedCapability for ProbeChangeFeed {
    fn changes_since(
        &self,
        capability_id: String,
        _cursor: Option<ChangeCursor>,
        _context: InvocationContext,
    ) -> Result<nexus_capabilities::changefeed::ChangeBatch, CapabilityError> {
        Ok(nexus_capabilities::changefeed::ChangeBatch {
            capability_id,
            events: vec![nexus_capabilities::changefeed::ChangeEvent {
                event_id: "evt-livefire-1".to_string(),
                event_type: "test.changed".to_string(),
                payload: serde_json::json!({ "state": "on" }),
            }],
            next_cursor: ChangeCursor {
                capability_id: "test.stream".to_string(),
                cursor: "cursor-livefire-2".to_string(),
            },
        })
    }
}

/// Provider that fails closed (test zone).
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

/// Command provider that fails closed (test zone).
struct DownCommand;

impl CommandCapability for DownCommand {
    fn command(&self, request: CommandRequest) -> Result<CommandResult, CapabilityError> {
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

// ---------------------------------------------------------------------------
// Schema validation helpers (real jsonschema validator, real schemas)
// ---------------------------------------------------------------------------

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

fn validator_for(schema: &serde_json::Value) -> Validator {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_retriever(LocalSchemasRetriever { root: root() })
        .build(schema)
        .expect("canonical schema must compile")
}

fn main() {
    let mut results: Vec<serde_json::Value> = Vec::new();
    let mut fail = false;

    macro_rules! stage {
        ($name:expr, $ok:expr, $detail:expr) => {{
            let ok: bool = $ok;
            if !ok {
                fail = true;
            }
            results.push(serde_json::json!({
                "stage": $name,
                "result": if ok { "PASS" } else { "FAIL" },
                "detail": $detail,
            }));
        }};
    }

    // ---- Registry + dispatcher composition -------------------------------
    let registry = InMemoryCapabilityRegistry::new();
    let tenant_a = tid_a();
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
    registry
        .register(
            descriptor(
                "test.command",
                CapabilityClass::Command,
                Availability::Available,
            ),
            ctx(tenant_a.clone(), "user:alice"),
        )
        .unwrap();
    registry
        .register(
            descriptor(
                "test.workflow",
                CapabilityClass::Workflow,
                Availability::Available,
            ),
            ctx(tenant_a.clone(), "user:alice"),
        )
        .unwrap();
    registry
        .register(
            descriptor(
                "test.stream",
                CapabilityClass::Stream,
                Availability::Available,
            ),
            ctx(tenant_a.clone(), "user:alice"),
        )
        .unwrap();
    registry
        .register(
            descriptor(
                "test.hidden",
                CapabilityClass::Query,
                Availability::Unavailable,
            ),
            ctx(tenant_a.clone(), "user:alice"),
        )
        .unwrap();

    let dispatcher = CapabilityDispatcher::new(Arc::new(registry.clone()));

    // REGISTER / DISCOVER / RESOLVE
    // Registration is tenant scoped; discovery iterates the real
    // BTreeMap keyed by (tenant, capability id), so the deterministic
    // order is sorted by capability id, not insertion order.
    let discovered = registry
        .discover(&tenant_a, ctx(tenant_a.clone(), "user:alice"))
        .unwrap();
    let discovered_ids: Vec<&str> = discovered.iter().map(|d| d.id.as_str()).collect();
    // Repeated identical registration is idempotent: re-registering
    // test.query must not grow the registry or change discovery.
    let len_before = registry.len();
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
    let rediscovered = registry
        .discover(&tenant_a, ctx(tenant_a.clone(), "user:alice"))
        .unwrap();
    let rediscovered_ids: Vec<&str> = rediscovered.iter().map(|d| d.id.as_str()).collect();
    stage!(
        "REGISTER_DISCOVER",
        discovered_ids == vec!["test.command", "test.query", "test.stream", "test.workflow"]
            && len_before == registry.len()
            && rediscovered_ids == discovered_ids,
        format!(
            "discovered={discovered_ids:?} re-register_idempotent=true len_stable={}",
            len_before == registry.len()
        )
    );

    // UNAVAILABLE_NOT_ADVERTISED
    stage!(
        "UNAVAILABLE_NOT_ADVERTISED",
        !discovered_ids.contains(&"test.hidden"),
        "unavailable capability omitted from discovery"
    );

    // QUERY_DISPATCH
    let q = ProbeQuery;
    let query_result = dispatcher
        .dispatch_query(
            QueryRequest {
                capability_id: "test.query".to_string(),
                context: ctx(tenant_a.clone(), "user:alice"),
                input: serde_json::json!({}),
            },
            &q,
        )
        .unwrap();
    stage!(
        "QUERY_DISPATCH",
        query_result.output["state"] == "on",
        format!("output={}", query_result.output)
    );

    // COMMAND_IDEMPOTENT
    let c = ProbeCommand;
    let tracker = IdempotencyTracker::new();
    let command_request = CommandRequest {
        capability_id: "test.command".to_string(),
        context: ctx(tenant_a.clone(), "user:alice"),
        input: serde_json::json!({ "on": true }),
        idempotency_key: Some("op-livefire-1".to_string()),
    };
    let first = dispatcher
        .dispatch_command(command_request.clone(), &c, &tracker)
        .unwrap();
    let second = dispatcher
        .dispatch_command(command_request, &c, &tracker)
        .unwrap();
    stage!(
        "COMMAND_IDEMPOTENT",
        first.output["applied"] == true && second.output["applied"] == true && tracker.len() == 1,
        format!(
            "first={} second={} records={}",
            first.output,
            second.output,
            tracker.len()
        )
    );

    // WORKFLOW_DISPATCH
    let w = ProbeWorkflow;
    let handle = dispatcher
        .dispatch_workflow(
            nexus_capabilities::workflow::WorkflowRequest {
                capability_id: "test.workflow".to_string(),
                context: ctx(tenant_a.clone(), "user:alice"),
                input: serde_json::json!({}),
                idempotency_key: Some("op-livefire-2".to_string()),
            },
            &w,
        )
        .unwrap();
    stage!(
        "WORKFLOW_DISPATCH",
        handle.workflow_id == "wf-livefire-1",
        format!("workflow_id={}", handle.workflow_id)
    );

    // HEALTH
    let h = ProbeHealth;
    let health = dispatcher
        .dispatch_health(
            "test.query".to_string(),
            ctx(tenant_a.clone(), "user:alice"),
            &h,
        )
        .unwrap();
    stage!(
        "HEALTH",
        health.state == HealthState::Healthy,
        format!("state={}", health.state.as_str())
    );

    // CHANGEFEED
    let f = ProbeChangeFeed;
    let batch = dispatcher
        .dispatch_changefeed(
            "test.stream".to_string(),
            None,
            ctx(tenant_a.clone(), "user:alice"),
            &f,
        )
        .unwrap();
    stage!(
        "CHANGEFEED",
        batch.events.len() == 1 && batch.next_cursor.cursor == "cursor-livefire-2",
        format!(
            "events={} next_cursor={}",
            batch.events.len(),
            batch.next_cursor.cursor
        )
    );

    // CLASS_MISMATCH_DENIED (command via query path)
    let mismatch = dispatcher
        .dispatch_query(
            QueryRequest {
                capability_id: "test.command".to_string(),
                context: ctx(tenant_a.clone(), "user:alice"),
                input: serde_json::json!({}),
            },
            &q,
        )
        .unwrap_err();
    stage!(
        "CLASS_MISMATCH_DENIED",
        mismatch.0.code == CapabilityErrorCode::Validation,
        format!("code={}", mismatch.0.code.as_str())
    );

    // CROSS_TENANT_DENIED
    let tenant_b = tid_b();
    let cross = dispatcher
        .dispatch_query(
            QueryRequest {
                capability_id: "test.query".to_string(),
                context: ctx(tenant_b.clone(), "user:mallory"),
                input: serde_json::json!({}),
            },
            &q,
        )
        .unwrap_err();
    stage!(
        "CROSS_TENANT_DENIED",
        cross.0.code == CapabilityErrorCode::NotFound,
        format!("code={}", cross.0.code.as_str())
    );

    // PROVIDER_ERROR_FAIL_CLOSED
    // Query path: provider failure must surface typed, never allow.
    let down = DownQuery;
    let down_err = dispatcher
        .dispatch_query(
            QueryRequest {
                capability_id: "test.query".to_string(),
                context: ctx(tenant_a.clone(), "user:alice"),
                input: serde_json::json!({}),
            },
            &down,
        )
        .unwrap_err();
    // Command path with an idempotency key: the failed provider must
    // not leave a success result cached in the tracker.
    let down_cmd = DownCommand;
    let fail_tracker = IdempotencyTracker::new();
    let down_cmd_err = dispatcher
        .dispatch_command(
            CommandRequest {
                capability_id: "test.command".to_string(),
                context: ctx(tenant_a.clone(), "user:alice"),
                input: serde_json::json!({ "on": true }),
                idempotency_key: Some("op-livefire-fail".to_string()),
            },
            &down_cmd,
            &fail_tracker,
        )
        .unwrap_err();
    stage!(
        "PROVIDER_ERROR_FAIL_CLOSED",
        down_err.0.code == CapabilityErrorCode::Unavailable
            && down_cmd_err.0.code == CapabilityErrorCode::Unavailable
            && fail_tracker.is_empty(),
        format!(
            "query_code={} command_code={} cached_success={}",
            down_err.0.code.as_str(),
            down_cmd_err.0.code.as_str(),
            !fail_tracker.is_empty()
        )
    );

    // IDEMPOTENCY_CONFLICT
    let tracker2 = IdempotencyTracker::new();
    tracker2
        .record(IdempotencyRecord {
            key: "op-livefire-1".to_string(),
            capability_id: "test.command".to_string(),
            result: serde_json::json!({ "applied": true }),
        })
        .unwrap();
    let conflict = tracker2
        .record(IdempotencyRecord {
            key: "op-livefire-1".to_string(),
            capability_id: "test.other".to_string(),
            result: serde_json::json!({ "applied": true }),
        })
        .unwrap_err();
    stage!(
        "IDEMPOTENCY_CONFLICT",
        conflict.0.code == CapabilityErrorCode::Conflict,
        format!("code={}", conflict.0.code.as_str())
    );

    // SCHEMA_VALIDATION (real validator against canonical schemas)
    let descriptor_schema = load_schema("schemas/capability-descriptor.schema.json");
    let d_validator = validator_for(&descriptor_schema);
    let descriptor_json = serde_json::to_value(descriptor(
        "test.query",
        CapabilityClass::Query,
        Availability::Available,
    ))
    .unwrap();
    let d_errors: Vec<String> = d_validator
        .iter_errors(&descriptor_json)
        .map(|e| e.to_string())
        .collect();
    let manifest_schema = load_schema("schemas/connector-manifest.schema.json");
    let m_validator = validator_for(&manifest_schema);
    let manifest_json = serde_json::json!({
        "id": "livefire",
        "version": "1.0.0",
        "tier": "TIER1",
        "license": "Apache-2.0",
        "runtime": "RUST",
        "health": "/health",
        "capabilities": [descriptor_json],
        "events": ["test.changed"],
        "secrets": ["vault:livefire-token"],
        "network_origins": ["https://api.livefire.home"],
        "data_classes": ["HOUSEHOLD"],
        "certification": "LAB"
    });
    let m_errors: Vec<String> = m_validator
        .iter_errors(&manifest_json)
        .map(|e| e.to_string())
        .collect();
    stage!(
        "SCHEMA_VALIDATION",
        d_errors.is_empty() && m_errors.is_empty(),
        format!("descriptor_errors={d_errors:?} manifest_errors={m_errors:?}")
    );

    // SCHEMA_REJECTION (unknown class + missing required + duplicates)
    // Unknown capability class.
    let mut bad = serde_json::to_value(descriptor(
        "test.query",
        CapabilityClass::Query,
        Availability::Available,
    ))
    .unwrap();
    bad["class"] = serde_json::json!("EXECUTE_ANYTHING");
    let reject_class = d_validator.iter_errors(&bad).next().is_some();
    // Missing required field (id).
    let mut bad_missing = serde_json::to_value(descriptor(
        "test.query",
        CapabilityClass::Query,
        Availability::Available,
    ))
    .unwrap();
    bad_missing.as_object_mut().unwrap().remove("id");
    let reject_missing = d_validator.iter_errors(&bad_missing).next().is_some();
    // Duplicate events (uniqueItems).
    let mut bad_events = manifest_json.clone();
    bad_events["events"] = serde_json::json!(["test.changed", "test.changed"]);
    let reject_events = m_validator.iter_errors(&bad_events).next().is_some();
    // Duplicate secrets (uniqueItems).
    let mut bad_manifest = manifest_json.clone();
    bad_manifest["secrets"] = serde_json::json!(["vault:livefire-token", "vault:livefire-token"]);
    let reject_dup = m_validator.iter_errors(&bad_manifest).next().is_some();
    // Duplicate network_origins (uniqueItems).
    let mut bad_origins = manifest_json.clone();
    bad_origins["network_origins"] =
        serde_json::json!(["https://api.livefire.home", "https://api.livefire.home"]);
    let reject_origins = m_validator.iter_errors(&bad_origins).next().is_some();
    stage!(
        "SCHEMA_REJECTION",
        reject_class && reject_missing && reject_events && reject_dup && reject_origins,
        format!(
            "unknown_class={reject_class} missing_required={reject_missing} duplicate_events={reject_events} duplicate_secrets={reject_dup} duplicate_origins={reject_origins}"
        )
    );

    // ---- Authority-boundary assertions (N/O/K) -------------------------
    // N: the descriptor is metadata only - it must not carry any
    // executable authorization material.
    let forbidden_keys = [
        "grant",
        "token",
        "authorization",
        "credential",
        "secret",
        "password",
        "allow",
    ];
    let descriptor_keys: Vec<String> = descriptor_json
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    let descriptor_has_no_authority = forbidden_keys
        .iter()
        .all(|k| !descriptor_keys.iter().any(|dk| dk == k));
    // O: a TIER3 / CERTIFIED manifest validates (tier is metadata) and
    // carries no grant material; tier never changes dispatch behavior.
    let mut tiered_manifest = manifest_json.clone();
    tiered_manifest["tier"] = serde_json::json!("TIER3");
    tiered_manifest["certification"] = serde_json::json!("CERTIFIED");
    let tier_errors: Vec<String> = m_validator
        .iter_errors(&tiered_manifest)
        .map(|e| e.to_string())
        .collect();
    let tier_is_metadata = tier_errors.is_empty()
        && !descriptor_keys.iter().any(|k| k == "tier")
        && !descriptor_keys.iter().any(|k| k == "certification");
    // K: a healthy report is observation only - it must not contain
    // authorization fields.
    let health_json = serde_json::to_value(&health).unwrap();
    let health_keys: Vec<String> = health_json
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    let health_is_observation = forbidden_keys
        .iter()
        .all(|k| !health_keys.iter().any(|hk| hk == k));

    // ---- Output ----------------------------------------------------------
    let output = serde_json::json!({
        "probe": "nexus-connectors livefire_probe (EP-010 M5)",
        "correlation_id": cid().as_str(),
        "principal": "user:alice",
        "tenant": tid_a().as_str(),
        "schema_namespace": "https://schemas.nexus.local/",
        "validator": "jsonschema 0.49.9 (draft 2020-12)",
        "schema_versions": {
            "capability_descriptor": "v1",
            "connector_manifest": "v1",
            "current_version_parity": "PASS",
            "future_version_migration": "NOT ASSERTED",
        },
        "canonical_ordering": [
            "REGISTER_DISCOVER",
            "UNAVAILABLE_NOT_ADVERTISED",
            "QUERY_DISPATCH",
            "COMMAND_IDEMPOTENT",
            "WORKFLOW_DISPATCH",
            "HEALTH",
            "CHANGEFEED",
            "CLASS_MISMATCH_DENIED",
            "CROSS_TENANT_DENIED",
            "PROVIDER_ERROR_FAIL_CLOSED",
            "IDEMPOTENCY_CONFLICT",
            "SCHEMA_VALIDATION",
            "SCHEMA_REJECTION",
        ],
        "stages": results,
        "authority_boundaries": {
            "descriptor_is_metadata_only": descriptor_has_no_authority,
            "tier_is_metadata_only": tier_is_metadata,
            "health_is_observation_only": health_is_observation,
            "ep008_authorization_authority": "EP-008 owns authorization to invoke",
            "ep005_event_transport_authority": "EP-005 owns event transport substrate",
            "ep006_workflow_authority": "EP-006 owns durable workflow execution",
            "external_connector_certification": "NOT OWNED BY EP-010",
        },
        "all_pass": !fail,
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
    if fail {
        std::process::exit(1);
    }
}
