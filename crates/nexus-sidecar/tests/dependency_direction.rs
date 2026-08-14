//! EP-011 dependency-direction enforcement (SPEC-001 acceptance).
//!
//! The sidecar crate may import the SDK/capabilities/domain contract
//! crates plus the vetted transport stack (tokio/hyper/reqwest/...
//! for the real HTTP boundary). It must NOT import authorization,
//! secrets, capability semantics, event/workflow durability, or any
//! infrastructure vendor crate - those are owned by other nodes.
//! This test fails the build on any violation.

use std::process::Command;

#[test]
fn ep011_unit_sidecar_crate_has_no_forbidden_dependencies() {
    let output = Command::new("cargo")
        .args([
            "tree",
            "-p",
            "nexus-sidecar",
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
        // EP-008 authorization boundary must remain external.
        "nexus-policy",
        "nexus-action-gateway",
        "nexus-auth",
        // EP-009 secret authority must remain external.
        "nexus-trust",
        // EP-010 capability semantics must remain external.
        "nexus-connectors",
        // EP-005/EP-006 durability must remain external.
        "nexus-events",
        // Infrastructure vendors the sidecar must not embed.
        "openbao",
        "headscale",
        "openfga",
        "openpolicyagent",
        "keycloak",
        "temporal",
        "sqlx",
        "diesel",
        "sea-orm",
        "postgres",
        "redis",
        "nats",
        "async-nats",
        "tonic",
        "opentelemetry",
        // NOTE: `tracing` is intentionally NOT forbidden: reqwest/hyper
        // pull it transitively through the vetted HTTP stack. The
        // sidecar never emits tracing spans itself; the forbidden list
        // guards semantic boundaries (authz/secrets/capability/
        // durability/vendors), not the transport layer.
    ];
    let violations: Vec<&str> = forbidden
        .iter()
        .filter(|name| tree.lines().any(|line| line.contains(**name)))
        .copied()
        .collect();
    assert!(
        violations.is_empty(),
        "forbidden dependency edge(s) in nexus-sidecar production tree: {violations:?}\n{tree}"
    );
}
