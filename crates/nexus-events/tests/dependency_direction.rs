//! EP-005 dependency-direction enforcement (SPEC-001 acceptance).
//!
//! The event contracts crate may import `nexus-domain` and `nexus-data`
//! only. No infrastructure, database, network, or vendor crate may appear
//! in its resolved dependency tree (NATS JetStream lives in
//! `infra/nats` and implements the ports; it must never be a dependency
//! of the contract crate). This test fails the build on any violation.

use std::process::Command;

#[test]
fn ep005_unit_events_crate_has_no_infrastructure_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-events", "--prefix", "none"])
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
        "async-nats",
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
        "nexus-events imports forbidden infrastructure crates: {violations:?}"
    );
}
