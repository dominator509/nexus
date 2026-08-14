//! EP-010 M3 integration tests: real cross-language schema parity.
//!
//! The canonical JSON Schemas under `schemas/` are the single
//! cross-language contract source (SPEC-003 behavior 1, SPEC-022
//! behavior 4). These tests serialize the real Rust contract types
//! from `nexus-capabilities` and validate the produced JSON against
//! the real schema documents using a real JSON Schema 2020-12
//! validator (`jsonschema` crate). Any drift between the Rust type
//! surface and the canonical schema fails the build.
//!
//! The schemas are loaded from the repository at test time; the
//! validator is the real `jsonschema` crate, never a hand-written
//! conformance stub.

use std::path::PathBuf;

use jsonschema::Validator;

use nexus_capabilities::descriptor::{CapabilityDescriptor, CapabilityVersion};
use nexus_capabilities::manifest::{ConnectorBinding, ConnectorId, ConnectorManifest};
use nexus_capabilities::vocabulary::{Certification, SchemaRef};
use nexus_domain::vocabulary::ConnectorRuntime;
use nexus_domain::{
    ApprovalClass, Availability, CapabilityClass, Idempotency, Locality, Privacy, Reversal, Risk,
    TenantId, Tier,
};

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

/// Build a draft 2020-12 validator over a canonical schema, resolving
/// `$ref` URIs against the repository `schemas/` directory instead of
/// the network. `https://schemas.nexus.local/...` is the canonical
/// schema namespace; relative refs inside a schema resolve to the
/// same namespace (e.g. `connector-manifest` referencing
/// `capability-descriptor.schema.json`).
fn validator_for(schema: &serde_json::Value) -> Validator {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .with_retriever(LocalSchemasRetriever { root: root() })
        .build(schema)
        .expect("canonical schema must compile")
}

/// Retrieves canonical schema refs from the local `schemas/` tree.
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
        // Canonical namespace: https://schemas.nexus.local/<name>
        let name = raw
            .strip_prefix("https://schemas.nexus.local/")
            .ok_or_else(|| format!("ref outside canonical schema namespace: {raw}"))?;
        // The final path segment is the schema file name.
        let file = name.split('/').next_back().unwrap_or(name);
        let path = self.root.join("schemas").join(file);
        let text = std::fs::read_to_string(&path)?;
        let value: serde_json::Value = serde_json::from_str(&text)?;
        Ok(value)
    }
}

fn descriptor() -> CapabilityDescriptor {
    CapabilityDescriptor::new(
        "home.lights.query",
        CapabilityVersion("1.2.3".to_string()),
        CapabilityClass::Query,
        "Query the state of home lights",
        SchemaRef::new("schemas/invocation-context.schema.json").unwrap(),
        SchemaRef::new("schemas/capability-descriptor.schema.json").unwrap(),
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

fn tenant() -> TenantId {
    TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap()
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

#[test]
fn ep010_integration_descriptor_validates_against_canonical_schema() {
    let schema = load_schema("schemas/capability-descriptor.schema.json");
    let validator = validator_for(&schema);
    let instance = serde_json::to_value(descriptor()).expect("descriptor serializes");
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "Rust CapabilityDescriptor JSON must validate against the canonical schema: {errors:?}"
    );
}

#[test]
fn ep010_integration_descriptor_with_all_classes_validates() {
    let schema = load_schema("schemas/capability-descriptor.schema.json");
    let validator = validator_for(&schema);
    for class in [
        CapabilityClass::Query,
        CapabilityClass::Command,
        CapabilityClass::Workflow,
        CapabilityClass::Stream,
        CapabilityClass::Administrative,
    ] {
        let mut d = descriptor();
        d.class = class;
        let instance = serde_json::to_value(&d).unwrap();
        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| e.to_string())
            .collect();
        assert!(
            errors.is_empty(),
            "descriptor class {} must validate: {errors:?}",
            class.as_str(),
        );
    }
}

#[test]
fn ep010_integration_manifest_validates_against_canonical_schema() {
    let schema = load_schema("schemas/connector-manifest.schema.json");
    let validator = validator_for(&schema);
    let instance = serde_json::to_value(manifest()).expect("manifest serializes");
    let errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "Rust ConnectorManifest JSON must validate against the canonical schema: {errors:?}"
    );
}

#[test]
fn ep010_integration_binding_validates_shape() {
    let binding = ConnectorBinding::new(
        ConnectorId("home-lights".to_string()),
        tenant(),
        "account-42",
        Some("living room".to_string()),
    )
    .unwrap();
    let json = serde_json::to_value(&binding).unwrap();
    // The binding is an interface record; assert the canonical shape
    // (connector_id, tenant_id, account_ref, label) survives
    // serialization.
    assert_eq!(json["connector_id"], "home-lights");
    assert_eq!(json["tenant_id"], "018f0f6f-9c1e-7b6e-8000-000000000003");
    assert_eq!(json["account_ref"], "account-42");
    assert_eq!(json["label"], "living room");
}

#[test]
fn ep010_integration_descriptor_rejects_unknown_class_against_schema() {
    // Cross-language contract: a document with an unknown class must be
    // rejected by the canonical schema, proving the schema is the
    // authority for enum values.
    let schema = load_schema("schemas/capability-descriptor.schema.json");
    let validator = validator_for(&schema);
    let mut instance = serde_json::to_value(descriptor()).unwrap();
    instance["class"] = serde_json::json!("EXECUTE_ANYTHING");
    assert!(
        validator.iter_errors(&instance).next().is_some(),
        "unknown class must fail schema validation"
    );
}

#[test]
fn ep010_integration_manifest_rejects_missing_required_fields_against_schema() {
    let schema = load_schema("schemas/connector-manifest.schema.json");
    let validator = validator_for(&schema);
    let mut instance = serde_json::to_value(manifest()).unwrap();
    instance.as_object_mut().unwrap().remove("license");
    assert!(
        validator.iter_errors(&instance).next().is_some(),
        "missing required field must fail validation"
    );
}
