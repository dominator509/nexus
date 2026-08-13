//! EP-007 dependency-direction enforcement (SPEC-001 acceptance).
//!
//! The Keycloak adapter crate may import `nexus-domain`, `nexus-auth`,
//! and serde only. It must NOT import any other infrastructure, database,
//! or vendor crate beyond the pinned Keycloak client surface. This test
//! fails the build on any violation.

use std::process::Command;

#[test]
fn ep007_unit_keycloak_adapter_has_no_unexpected_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-keycloak", "--prefix", "none"])
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
        "jsonwebtoken",
        "oauth2",
        "openidconnect",
        "webauthn",
    ];
    let mut violations: Vec<&str> = Vec::new();
    for dep in forbidden {
        if tree.lines().any(|line| line.trim() == dep) {
            violations.push(dep);
        }
    }
    assert!(
        violations.is_empty(),
        "nexus-keycloak imports forbidden infrastructure crates: {violations:?}"
    );
}
