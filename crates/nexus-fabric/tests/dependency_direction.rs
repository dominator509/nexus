//! EP-012 dependency-direction enforcement (SPEC-001 acceptance).
//!
//! The fabric contract crate may import `nexus-domain`, `nexus-identity`,
//! `nexus-auth`, and serde only. No infrastructure, network, HTTP, or
//! vendor crate may appear in its resolved dependency tree. This test
//! fails the build on any violation.

use std::process::Command;

#[test]
fn ep012_unit_fabric_crate_has_no_infrastructure_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-fabric", "--prefix", "none"])
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
        "mcp",
        "a2a",
    ];
    let mut violations: Vec<&str> = Vec::new();
    for needle in forbidden {
        if tree.lines().any(|l| l.trim_start().starts_with(needle)) {
            violations.push(needle);
        }
    }
    assert!(
        violations.is_empty(),
        "nexus-fabric dependency tree contains forbidden crates: {:?}\n{}",
        violations,
        tree
    );
}
