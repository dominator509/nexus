//! EP-008 dependency-direction enforcement (SPEC-001 acceptance).
//!
//! The policy domain crate may import `nexus-domain`, `nexus-identity`,
//! `nexus-auth`, and serde only. No infrastructure, database, network,
//! or vendor crate may appear in its resolved dependency tree. This
//! test fails the build on any violation.

use std::process::Command;

#[test]
fn ep008_unit_policy_crate_has_no_infrastructure_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-policy", "--prefix", "none"])
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
        "openfga",
        "opa",
    ];
    let mut violations: Vec<&str> = Vec::new();
    for needle in forbidden {
        // Match crate names as full lines (cargo tree --prefix none
        // emits `name vX.Y.Z`), avoiding substring false positives.
        if tree.lines().any(|l| l.trim_start().starts_with(needle)) {
            violations.push(needle);
        }
    }
    assert!(
        violations.is_empty(),
        "nexus-policy dependency tree contains forbidden crates: {:?}\n{}",
        violations,
        tree
    );
}
