//! EP-011 M3 cross-language golden wire parity (directive D/E).
//!
//! The golden fixtures in `tests/connectors/golden/` are generated
//! from the canonical Rust types (example `generate_golden`). This
//! suite proves the Rust binding serializes to the SAME semantic
//! structures and deserializes the SAME files as the TypeScript and
//! Python bindings. Comparison is semantic (serde_json::Value), never
//! raw JSON strings, so map ordering is irrelevant.

use std::fs;
use std::path::PathBuf;

use nexus_capabilities::changefeed::{ChangeBatch, ChangeCursor};
use nexus_capabilities::command::{CommandRequest, CommandResult};
use nexus_capabilities::context::InvocationContext;
use nexus_capabilities::descriptor::CapabilityDescriptor;
use nexus_capabilities::health::HealthReport;
use nexus_capabilities::manifest::ConnectorManifest;
use nexus_capabilities::query::{QueryRequest, QueryResult};
use nexus_capabilities::workflow::{WorkflowRequest, WorkflowResult};
use nexus_connector_sdk::credential::CredentialReference;
use nexus_connector_sdk::error::SdkError;
use nexus_connector_sdk::sidecar::{SidecarRequest, SidecarResponse};
use nexus_connector_sdk::vocabulary::WebhookEvent;
use nexus_connector_sdk::webhook::{NormalizedWebhook, RawWebhook};

fn golden_dir() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("../../tests/connectors/golden")
}

fn load(name: &str) -> serde_json::Value {
    let path = golden_dir().join(format!("{name}.json"));
    let text = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("golden fixture {name} missing: {e} (run the generate_golden example first)")
    });
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("golden fixture {name} invalid: {e}"))
}

/// Assert that the canonical type serializes to the same semantic
/// structure as the golden fixture, and that the golden fixture
/// deserializes back into the canonical type.
fn assert_semantic_parity<T>(name: &str, value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let golden = load(name);
    let serialized = serde_json::to_value(value).expect("serialize canonical type");
    assert_eq!(
        serialized, golden,
        "{name}: Rust serialization diverges from the canonical golden fixture"
    );
    let round: T = serde_json::from_value(golden.clone()).expect("deserialize golden fixture");
    let re_serialized = serde_json::to_value(&round).expect("re-serialize round trip");
    assert_eq!(
        re_serialized, golden,
        "{name}: golden fixture does not round-trip through the canonical type"
    );
}

#[test]
fn ep011_integration_golden_invocation_context() {
    let ctx: InvocationContext = serde_json::from_value(load("invocation_context")).unwrap();
    assert_eq!(
        ctx.tenant_id.to_string(),
        "018f0f6f-9c1e-7b6e-8000-000000000003"
    );
    assert_semantic_parity("invocation_context", &ctx);
}

#[test]
fn ep011_integration_golden_capability_descriptor() {
    let d: CapabilityDescriptor = serde_json::from_value(load("capability_descriptor")).unwrap();
    assert_eq!(d.id, "fixture.contacts.query");
    assert_semantic_parity("capability_descriptor", &d);
}

#[test]
fn ep011_integration_golden_connector_manifest() {
    let m: ConnectorManifest = serde_json::from_value(load("connector_manifest")).unwrap();
    assert_eq!(m.id.0, "fixture-connector");
    assert_semantic_parity("connector_manifest", &m);
}

#[test]
fn ep011_integration_golden_query_and_command() {
    let q: QueryRequest = serde_json::from_value(load("query_request")).unwrap();
    assert_semantic_parity("query_request", &q);
    let qr: QueryResult = serde_json::from_value(load("query_result")).unwrap();
    assert_semantic_parity("query_result", &qr);
    let c: CommandRequest = serde_json::from_value(load("command_request")).unwrap();
    assert_semantic_parity("command_request", &c);
    let cr: CommandResult = serde_json::from_value(load("command_result")).unwrap();
    assert_semantic_parity("command_result", &cr);
}

#[test]
fn ep011_integration_golden_workflow() {
    let w: WorkflowRequest = serde_json::from_value(load("workflow_request")).unwrap();
    assert_semantic_parity("workflow_request", &w);
    let wr: WorkflowResult = serde_json::from_value(load("workflow_result")).unwrap();
    assert_semantic_parity("workflow_result", &wr);
}

#[test]
fn ep011_integration_golden_health_and_changefeed() {
    let h: HealthReport = serde_json::from_value(load("health_report")).unwrap();
    assert_semantic_parity("health_report", &h);
    let c: ChangeCursor = serde_json::from_value(load("change_cursor")).unwrap();
    assert_semantic_parity("change_cursor", &c);
    let b: ChangeBatch = serde_json::from_value(load("change_batch")).unwrap();
    assert_semantic_parity("change_batch", &b);
}

#[test]
fn ep011_integration_golden_webhook() {
    let e: WebhookEvent = serde_json::from_value(load("webhook_event")).unwrap();
    assert_semantic_parity("webhook_event", &e);
    let raw: RawWebhook = serde_json::from_value(load("raw_webhook")).unwrap();
    assert_semantic_parity("raw_webhook", &raw);
    let n: NormalizedWebhook = serde_json::from_value(load("normalized_webhook")).unwrap();
    assert_semantic_parity("normalized_webhook", &n);
}

#[test]
fn ep011_integration_golden_sidecar() {
    let r: SidecarRequest = serde_json::from_value(load("sidecar_request")).unwrap();
    assert_semantic_parity("sidecar_request", &r);
    let s: SidecarResponse = serde_json::from_value(load("sidecar_response")).unwrap();
    assert_semantic_parity("sidecar_response", &s);
}

#[test]
fn ep011_integration_golden_credential_and_error() {
    let c: CredentialReference = serde_json::from_value(load("credential_reference")).unwrap();
    assert_semantic_parity("credential_reference", &c);
    let e: SdkError = serde_json::from_value(load("error_envelope")).unwrap();
    assert_semantic_parity("error_envelope", &e);
    // Canonical wire code (directive P): NOT_FOUND must be the exact
    // canonical string shared by Rust/TypeScript/Python.
    assert_eq!(e.code.as_str(), "NOT_FOUND");
}

#[test]
fn ep011_integration_golden_fixture_count_stable() {
    // The parity corpus is a fixed set; accidental fixture drift (new
    // files or deleted files) fails loudly instead of silently
    // changing the cross-language contract.
    let mut names: Vec<String> = fs::read_dir(golden_dir())
        .expect("golden dir")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    let expected = [
        "capability_descriptor.json",
        "change_batch.json",
        "change_cursor.json",
        "command_request.json",
        "command_result.json",
        "connector_manifest.json",
        "credential_reference.json",
        "error_envelope.json",
        "health_report.json",
        "invocation_context.json",
        "normalized_webhook.json",
        "query_request.json",
        "query_result.json",
        "raw_webhook.json",
        "sidecar_request.json",
        "sidecar_response.json",
        "webhook_event.json",
        "workflow_request.json",
        "workflow_result.json",
    ];
    assert_eq!(names, expected, "golden fixture set drifted");
}
