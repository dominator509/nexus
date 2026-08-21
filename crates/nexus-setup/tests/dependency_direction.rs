//! EP-035 M2 dependency-direction test (anti-drift).
//!
//! The setup behavior crate may depend only on canonical contract
//! crates (nexus-domain) plus serde/serde_json. It must never import
//! provider, infrastructure, transport, or framework crates.

use std::process::Command;

#[test]
fn ep035_unit_dependency_direction_tree_depth_one() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-setup", "--depth", "1"])
        .output()
        .expect("cargo tree must run");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        tree.contains("nexus-setup v0.1.0"),
        "cargo tree missing nexus-setup root: {tree}"
    );

    let forbidden = [
        "tokio",
        "axum",
        "hyper",
        "reqwest",
        "ureq",
        "openbao",
        "headscale",
        "openfga",
        "opa",
        "jsonschema",
        "rusqlite",
        "sqlx",
        "nats",
        "tonic",
        "prost",
        "clap",
        "bifrost",
        "transport",
        "tracing",
        "temporal",
    ];
    for name in forbidden {
        assert!(
            !tree.lines().any(|line| line.contains(name)),
            "nexus-setup must not depend on '{name}':\n{tree}"
        );
    }
}

#[test]
fn ep035_unit_dependency_direction_allowed_dependencies_only() {
    let manifest = std::fs::read_to_string("Cargo.toml").expect("manifest must exist");
    assert!(manifest.contains("nexus-domain"), "{manifest}");
    assert!(
        !manifest.contains("tokio") && !manifest.contains("reqwest"),
        "nexus-setup Cargo.toml must not reference provider/transport crates"
    );
}
