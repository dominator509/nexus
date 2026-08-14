//! EP-011 dependency-direction enforcement (SPEC-001 acceptance).
//!
//! The connector SDK contract crate may import `nexus-domain`,
//! `nexus-identity`, `nexus-capabilities`, and serde only. No
//! infrastructure, database, network, or vendor crate may appear in
//! its resolved production dependency tree. This test fails the build
//! on any violation.
//!
//! NOTE: like EP-010, the dev tree may legitimately pull extra crates;
//! the invariant is the production edge set, so `--edges normal` is
//! used (EP-010 M3 Decision Log).

use std::process::Command;

#[test]
fn ep011_unit_connector_sdk_crate_has_no_infrastructure_dependencies() {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-p",
            "nexus-connector-sdk",
            "--edges",
            "normal",
            "--prefix",
            "none",
        ])
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("cargo tree must run");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout);
    let forbidden = [
        "tokio",
        "axum",
        "actix",
        "sqlx",
        "diesel",
        "sea-orm",
        "postgres",
        "redis",
        "nats",
        "reqwest",
        "tonic",
        "opentelemetry",
        "tracing",
        "uuid",
        "chrono",
        "clap",
        "openbao",
        "headscale",
        "jsonschema",
        "rcgen",
        "x509-parser",
        "openfga",
        "openpolicyagent",
        "keycloak",
        "temporal",
    ];
    let violations: Vec<&str> = forbidden
        .iter()
        .filter(|name| tree.lines().any(|line| line.contains(**name)))
        .copied()
        .collect();
    assert!(
        violations.is_empty(),
        "forbidden dependency edge(s) in nexus-connector-sdk production tree: {violations:?}\n{tree}"
    );
}
