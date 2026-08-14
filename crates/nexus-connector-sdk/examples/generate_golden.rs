//! Generate canonical golden wire fixtures for the EP-011 M3
//! cross-language parity suite (directive E).
//!
//! The golden fixtures are generated from the authoritative Rust
//! canonical types so every language binding (Rust, TypeScript,
//! Python) proves serialization/deserialization against the SAME
//! canonical JSON. Run once and commit the output:
//!
//! ```sh
//! cargo run -p nexus-connector-sdk --example generate_golden
//! ```
//!
//! Output lands in `tests/connectors/golden/`. The parity tests in
//! each language read these files and compare semantic structures
//! (never raw JSON strings), so map ordering is irrelevant.

use std::fs;
use std::path::Path;

use nexus_capabilities::changefeed::{ChangeBatch, ChangeCursor, ChangeEvent};
use nexus_capabilities::command::{CommandRequest, CommandResult};
use nexus_capabilities::context::InvocationContext;
use nexus_capabilities::descriptor::CapabilityDescriptor;
use nexus_capabilities::health::HealthReport;
use nexus_capabilities::manifest::{ConnectorId, ConnectorManifest};
use nexus_capabilities::query::{QueryRequest, QueryResult};
use nexus_capabilities::vocabulary::{Certification, SchemaRef};
use nexus_capabilities::workflow::{WorkflowHandle, WorkflowRequest, WorkflowResult};
use nexus_connector_sdk::credential::CredentialReference;
use nexus_connector_sdk::error::SdkError;
use nexus_connector_sdk::sidecar::{SidecarRequest, SidecarResponse};
use nexus_connector_sdk::vocabulary::{SidecarTransport, WebhookEvent};
use nexus_connector_sdk::webhook::{NormalizedWebhook, RawWebhook};
use nexus_domain::{
    ApprovalClass, Availability, CapabilityClass, Idempotency, Locality, NexusId, PrincipalType,
    Privacy, Reversal, Risk, TenantId, Tier,
};

fn write(name: &str, value: &impl serde::Serialize) {
    let dir = Path::new("tests/connectors/golden");
    fs::create_dir_all(dir).expect("create golden dir");
    let path = dir.join(format!("{name}.json"));
    let json = serde_json::to_string_pretty(value).expect("serialize golden");
    fs::write(&path, format!("{json}\n")).expect("write golden");
    println!("wrote {}", path.display());
}

fn ctx() -> InvocationContext {
    InvocationContext::new(
        NexusId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
        nexus_domain::CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        None,
        "test",
        "user:alice",
        PrincipalType::Human,
        TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap(),
        Some("mcp".to_string()),
        None,
        None,
        None,
    )
    .unwrap()
}

fn descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        "fixture.contacts.query",
        nexus_capabilities::descriptor::CapabilityVersion("1.0.0".to_string()),
        CapabilityClass::Query,
        "query contacts from the fixture connector",
        SchemaRef("https://schemas.nexus.local/query/v1".to_string()),
        SchemaRef("https://schemas.nexus.local/query-result/v1".to_string()),
        vec!["fixture.contacts:read".to_string()],
        Risk::R1,
        ApprovalClass::None,
        Reversal::None,
        Idempotency::NotApplicable,
        Availability::Available,
        Some(Locality::Any),
        vec![Privacy::Public],
        vec![],
        Some("fixture-provider".to_string()),
    )
    .unwrap()
}

fn main() {
    let context = ctx();

    write(
        "invocation_context",
        &serde_json::to_value(&context).unwrap(),
    );

    write(
        "capability_descriptor",
        &serde_json::to_value(descriptor()).unwrap(),
    );

    let manifest = ConnectorManifest::new(
        ConnectorId("fixture-connector".to_string()),
        "1.0.0",
        Tier::Tier2,
        "Apache-2.0",
        nexus_domain::vocabulary::ConnectorRuntime::Python,
        "/v1/health",
        vec![descriptor()],
        vec!["fixture.contact.updated".to_string()],
        vec!["vault:fixture-token".to_string()],
        vec!["https://fixture.nexus.local".to_string()],
        vec![Privacy::Public],
        Some(Certification::Lab),
    )
    .unwrap();
    write(
        "connector_manifest",
        &serde_json::to_value(&manifest).unwrap(),
    );

    let query_req = QueryRequest {
        capability_id: "fixture.contacts.query".to_string(),
        context: context.clone(),
        input: serde_json::json!({ "limit": 10 }),
    };
    write("query_request", &serde_json::to_value(&query_req).unwrap());
    let query_res = QueryResult {
        capability_id: "fixture.contacts.query".to_string(),
        output: serde_json::json!({ "contacts": [ { "id": "c1", "name": "Alice" } ] }),
    };
    write("query_result", &serde_json::to_value(&query_res).unwrap());

    let command_req = CommandRequest {
        capability_id: "fixture.contacts.command".to_string(),
        context: context.clone(),
        input: serde_json::json!({ "name": "Bob" }),
        idempotency_key: Some("op-1".to_string()),
    };
    write(
        "command_request",
        &serde_json::to_value(&command_req).unwrap(),
    );
    let command_res = CommandResult {
        capability_id: "fixture.contacts.command".to_string(),
        output: serde_json::json!({ "id": "c2" }),
    };
    write(
        "command_result",
        &serde_json::to_value(&command_res).unwrap(),
    );

    let workflow_req = WorkflowRequest {
        capability_id: "fixture.reconcile.workflow".to_string(),
        context: context.clone(),
        input: serde_json::json!({ "scope": "daily" }),
        idempotency_key: None,
    };
    write(
        "workflow_request",
        &serde_json::to_value(&workflow_req).unwrap(),
    );
    let workflow_res = WorkflowResult {
        handle: WorkflowHandle {
            capability_id: "fixture.reconcile.workflow".to_string(),
            workflow_id: "wf-1".to_string(),
        },
        status: nexus_capabilities::workflow::WorkflowStatus::Running,
        output: None,
    };
    write(
        "workflow_result",
        &serde_json::to_value(&workflow_res).unwrap(),
    );

    let health = HealthReport {
        target_id: "fixture-connector".to_string(),
        state: nexus_capabilities::vocabulary::HealthState::Healthy,
        detail: Some("ready".to_string()),
    };
    write("health_report", &serde_json::to_value(&health).unwrap());

    let cursor = ChangeCursor {
        capability_id: "fixture.audit.changefeed".to_string(),
        cursor: "seq-3".to_string(),
    };
    write("change_cursor", &serde_json::to_value(&cursor).unwrap());
    let batch = ChangeBatch {
        capability_id: "fixture.audit.changefeed".to_string(),
        events: vec![ChangeEvent {
            event_id: "evt-1".to_string(),
            event_type: "fixture.contact.updated".to_string(),
            payload: serde_json::json!({ "id": "c1" }),
        }],
        next_cursor: ChangeCursor {
            capability_id: "fixture.audit.changefeed".to_string(),
            cursor: "seq-4".to_string(),
        },
    };
    write("change_batch", &serde_json::to_value(&batch).unwrap());

    let webhook_event = WebhookEvent {
        event_id: "prov-1".to_string(),
        event_type: "invoice.paid".to_string(),
        version: "1".to_string(),
        correlation_id: context.correlation_id.to_string(),
        payload: serde_json::json!({ "amount": 100 }),
    };
    write(
        "webhook_event",
        &serde_json::to_value(&webhook_event).unwrap(),
    );

    let raw_webhook = RawWebhook {
        raw_payload: serde_json::json!({ "amount": 100 }),
        signature: Some("sha256=fp-test:abc".to_string()),
        provider_event_id: Some("prov-1".to_string()),
        provider_event_type: Some("invoice.paid".to_string()),
    };
    write("raw_webhook", &serde_json::to_value(&raw_webhook).unwrap());
    let normalized = NormalizedWebhook {
        event: Some(webhook_event.clone()),
        verification: nexus_connector_sdk::vocabulary::WebhookVerification::Valid,
    };
    write(
        "normalized_webhook",
        &serde_json::to_value(&normalized).unwrap(),
    );

    let sidecar_req = SidecarRequest {
        capability_id: "legacy.erp".to_string(),
        transport: SidecarTransport::Soap,
        action: "read.invoice".to_string(),
        input: serde_json::json!({ "invoice_id": "INV-1" }),
        idempotency_key: Some("op-1".to_string()),
        context: context.clone(),
    };
    write(
        "sidecar_request",
        &serde_json::to_value(&sidecar_req).unwrap(),
    );
    let sidecar_res = SidecarResponse {
        capability_id: "legacy.erp".to_string(),
        output: serde_json::json!({ "total": 100 }),
        cursor: None,
    };
    write(
        "sidecar_response",
        &serde_json::to_value(&sidecar_res).unwrap(),
    );

    let credential = CredentialReference::new("vault:fixture-token", "3", "fp-abc").unwrap();
    write(
        "credential_reference",
        &serde_json::to_value(&credential).unwrap(),
    );

    let error = SdkError::new(
        nexus_connector_sdk::error::SdkErrorCode::NotFound,
        "capability not found",
        Some(context.correlation_id.to_string()),
        Some(context.external_actor_id.clone()),
        Some(context.tenant_id.to_string()),
        Some("fixture.missing".to_string()),
    );
    write("error_envelope", &serde_json::to_value(&error).unwrap());

    println!("golden fixtures: ok");
}
