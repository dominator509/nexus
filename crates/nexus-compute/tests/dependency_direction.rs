//! EP-036 M1 dependency-direction test (anti-drift).
//!
//! The compute fabric contract crate may depend only on canonical
//! contract crates (nexus-domain) plus serde/serde_json. It must never
//! import provider SDKs (AWS, DigitalOcean, Hetzner, Contabo), OpenTofu/
//! Terraform, cloud-init, Kubernetes, Docker, transport, or framework
//! crates. Later milestones implement provider adapters; M1 defines what
//! a provider must prove.

use std::process::Command;

#[test]
fn ep036_unit_dependency_direction_tree_depth_one() {
    let output = Command::new("cargo")
        .args(["tree", "-p", "nexus-compute", "--depth", "1"])
        .output()
        .expect("cargo tree must run");
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tree = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        tree.contains("nexus-compute v0.1.0"),
        "cargo tree missing nexus-compute root: {tree}"
    );

    let forbidden = [
        "tokio",
        "axum",
        "hyper",
        "reqwest",
        "ureq",
        "aws",
        "digitalocean",
        "hetzner",
        "contabo",
        "opentofu",
        "terraform",
        "cloud-init",
        "cloudinit",
        "kubernetes",
        "kube",
        "docker",
        "bollard",
        "ssh",
        "russh",
        "openbao",
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
            "nexus-compute must not depend on '{name}':\n{tree}"
        );
    }
}

#[test]
fn ep036_unit_dependency_direction_allowed_dependencies_only() {
    // Integration tests run with the package directory as CWD.
    let manifest = std::fs::read_to_string("Cargo.toml").expect("manifest");
    assert!(manifest.contains("nexus-domain"), "{manifest}");
    for forbidden in [
        "tokio",
        "reqwest",
        "aws",
        "digitalocean",
        "opentofu",
        "cloud-init",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "nexus-compute Cargo.toml must not reference provider/transport crates: {forbidden}"
        );
    }
}
