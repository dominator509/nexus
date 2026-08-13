//! EP-007 dependency-direction enforcement (SPEC-001 acceptance).
//!
//! The auth domain crate may import `nexus-domain`, `nexus-identity`,
//! and serde only. No infrastructure, database, network, or vendor crate
//! may appear in its resolved dependency tree. This test fails the build
//! on any violation.

use std::process::Command;

#[test]
fn ep007_unit_auth_crate_has_no_infrastructure_dependencies() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-auth", "--prefix", "none"])
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
        "keycloak",
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
        "nexus-auth imports forbidden infrastructure crates: {violations:?}"
    );
}
