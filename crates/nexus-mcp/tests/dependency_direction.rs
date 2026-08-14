//! EP-012 dependency-direction enforcement (SPEC-001 acceptance).
//!
//! The MCP engine crate may import `nexus-domain`, `nexus-auth`,
//! `nexus-fabric`, and serde only. No infrastructure, HTTP, or vendor
//! crate may appear in its resolved dependency tree.

use std::process::Command;

#[test]
fn ep012_unit_mcp_crate_has_no_infrastructure_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-mcp", "--prefix", "none"])
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
        "anyhow",
        "thiserror",
        "http",
        "hyper",
        "tower",
        "openbao",
        "sops",
        "headscale",
        "wireguard",
        "openssl",
        "rustls",
        "jsonschema",
    ];
    let mut violations: Vec<&str> = Vec::new();
    for needle in forbidden {
        if tree.lines().any(|l| l.trim_start().starts_with(needle)) {
            violations.push(needle);
        }
    }
    assert!(
        violations.is_empty(),
        "nexus-mcp dependency tree contains forbidden crates: {:?}\n{}",
        violations,
        tree
    );
}
