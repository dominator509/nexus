//! EP-004 dependency-direction enforcement (SPEC-001 acceptance).
//!
//! The data/memory contract crate may import `nexus-domain` and serde only.
//! No infrastructure, database, network, or vendor crate may appear in its
//! resolved dependency tree. This test fails the build on any violation.

use std::process::Command;

#[test]
fn ep004_unit_data_crate_has_no_infrastructure_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-data", "--prefix", "none"])
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
    ];
    let mut violations: Vec<&str> = Vec::new();
    for dep in forbidden {
        if tree.lines().any(|line| line.trim() == dep) {
            violations.push(dep);
        }
    }
    assert!(
        violations.is_empty(),
        "nexus-data imports forbidden infrastructure crates: {violations:?}"
    );
}
