//! EP-003 dependency-direction enforcement for the presence crate.
//!
//! `nexus-presence` may import `nexus-domain`, `nexus-identity`, and serde
//! only. No infrastructure, database, network, or vendor crate may appear
//! in its resolved dependency tree.

use std::process::Command;

#[test]
fn ep003_unit_presence_crate_has_no_infrastructure_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-presence", "--prefix", "none"])
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
        "nexus-presence imports forbidden infrastructure crates: {violations:?}"
    );
}
