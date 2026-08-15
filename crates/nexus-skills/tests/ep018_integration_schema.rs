//! EP-018 M3 integration tests: real cross-boundary schema parity.
//!
//! The canonical JSON Schemas under `schemas/skills/` are the single
//! cross-language contract source for Agent Skills (SPEC-010 behavior
//! 6; ADR-025). These tests serialize the REAL Rust contract types
//! from `nexus-skills` and validate the produced JSON against the REAL
//! schema documents using the REAL JSON Schema 2020-12 validator
//! (`jsonschema` crate, EP-010 M3 pattern). Any drift between the Rust
//! serde surface and the canonical schema fails the build.
//!
//! The tests also validate the REAL on-disk bundles under `skills/`
//! against the canonical manifest schema, proving the shipped skill
//! content conforms to the cross-language contract. Schemas are loaded
//! from the repository at test time; refs resolve locally only.

use std::path::PathBuf;

use jsonschema::Validator;

use nexus_skills::{
    JsonFileSkillRegistryStore, SkillBundleLoader, SkillManifest, SkillRegistryState,
    SkillRegistryStore,
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

/// Build a draft 2020-12 validator resolving `$ref` URIs against the
/// repository `schemas/` directory (canonical namespace
/// `https://schemas.nexus.local/...`), never the network.
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
        // Canonical namespace: https://schemas.nexus.local/<path>
        let relative = raw
            .strip_prefix("https://schemas.nexus.local/")
            .ok_or_else(|| format!("ref outside canonical schema namespace: {raw}"))?;
        let path = self.root.join("schemas").join(relative);
        let text = std::fs::read_to_string(&path)?;
        let value: serde_json::Value = serde_json::from_str(&text)?;
        Ok(value)
    }
}

fn skills_root() -> PathBuf {
    root().join("skills")
}

/// A real installed package: serialize -> schema -> validate.
fn assert_schema_ok(validator: &Validator, value: &serde_json::Value, label: &str) {
    let errors: Vec<String> = validator
        .iter_errors(value)
        .take(5)
        .map(|e| format!("{}: {}", e.instance_path(), e))
        .collect();
    if !errors.is_empty() {
        panic!("{label} fails canonical schema: {errors:?}");
    }
}

#[test]
fn ep018_integration_real_manifest_matches_canonical_schema() {
    let schema = load_schema("schemas/skills/skill-manifest.schema.json");
    let validator = validator_for(&schema);
    let loader = SkillBundleLoader::new(skills_root());
    for (name, version) in loader.list_available().expect("list bundles") {
        let bundle = loader.load(&name, &version).expect("load bundle");
        let json = serde_json::to_value(&bundle.package.manifest).expect("serialize manifest");
        assert_schema_ok(
            &validator,
            &json,
            &format!("skills/{name}/{version} manifest"),
        );
    }
}

#[test]
fn ep018_integration_real_package_matches_canonical_schema() {
    let schema = load_schema("schemas/skills/skill-package.schema.json");
    let validator = validator_for(&schema);
    let loader = SkillBundleLoader::new(skills_root());
    for (name, version) in loader.list_available().expect("list bundles") {
        let bundle = loader.load(&name, &version).expect("load bundle");
        let json = serde_json::to_value(&bundle.package).expect("serialize package");
        assert_schema_ok(
            &validator,
            &json,
            &format!("skills/{name}/{version} package"),
        );
    }
}

#[test]
fn ep018_integration_rust_manifest_type_matches_canonical_schema() {
    // Serialize a hand-built SkillManifest (the contract type) and
    // validate it: the Rust serde surface and the canonical schema must
    // not drift.
    let schema = load_schema("schemas/skills/skill-manifest.schema.json");
    let validator = validator_for(&schema);
    let loader = SkillBundleLoader::new(skills_root());
    let bundle = loader
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle");
    let mut manifest: SkillManifest = bundle.package.manifest;
    manifest.version = "2.3.4".into();
    let json = serde_json::to_value(&manifest).expect("serialize");
    assert_schema_ok(&validator, &json, "constructed SkillManifest");
}

#[test]
fn ep018_integration_registry_state_matches_canonical_schema() {
    let schema = load_schema("schemas/skills/skill-registry-state.schema.json");
    let validator = validator_for(&schema);
    let store = JsonFileSkillRegistryStore::new(
        std::env::temp_dir().join(format!("ep018-m3-state-{}.json", std::process::id())),
    );
    let _ = std::fs::remove_file(store.path());
    // Install the real summarize skill through the real store.
    let loader = SkillBundleLoader::new(skills_root());
    let bundle = loader
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle");
    let mut registry = nexus_skills::SkillRegistry::new();
    registry
        .install_bundle(bundle, nexus_skills::SkillTrustLevel::Sandboxed, 1, &store)
        .expect("install");
    registry
        .revoke("nexus/summarize", "1.0.0", &store)
        .expect("revoke");
    let state: SkillRegistryState = store.load().expect("store loads");
    let json = serde_json::to_value(&state).expect("serialize state");
    assert_schema_ok(&validator, &json, "SkillRegistryState");
    let _ = std::fs::remove_file(store.path());
}

#[test]
fn ep018_integration_schema_rejects_invalid_permissions_and_ids() {
    // The canonical schema is a real gate: malformed contract values
    // must fail validation (the schema is not decorative).
    let schema = load_schema("schemas/skills/skill-manifest.schema.json");
    let validator = validator_for(&schema);
    let loader = SkillBundleLoader::new(skills_root());
    let bundle = loader
        .load("nexus/summarize", "1.0.0")
        .expect("real bundle");
    let mut json = serde_json::to_value(&bundle.package.manifest).expect("serialize");

    // Unknown permission is rejected by the schema vocabulary.
    json["permissions"] = serde_json::json!(["ALL_THE_THINGS"]);
    assert!(
        validator.validate(&json).is_err(),
        "unknown permission passes schema"
    );

    // Non-canonical id is rejected.
    json["permissions"] = serde_json::json!([]);
    json["skill_id"] = serde_json::json!("not-a-uuid");
    assert!(
        validator.validate(&json).is_err(),
        "invalid skill_id passes schema"
    );

    // Invalid semver is rejected.
    json["skill_id"] = serde_json::json!(bundle.package.manifest.skill_id.as_str());
    json["version"] = serde_json::json!("1.0");
    assert!(
        validator.validate(&json).is_err(),
        "invalid version passes schema"
    );
}
