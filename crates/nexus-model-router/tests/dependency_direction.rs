//! EP-015 dependency-direction guard for `nexus-model-router`
//! (SPEC-001): the model router crate must not drag infrastructure,
//! network, HTTP, vendor, or later-node crates into its production
//! tree. Learned adapters and the Microbrain are injected behind ports;
//! routing logic stays vendor-neutral and policy-only.

use std::process::Command;

#[test]
fn ep015_unit_dependency_direction() {
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "nexus-model-router",
            "--depth",
            "1",
            "--edges",
            "normal",
        ])
        .output()
        .expect("cargo tree must run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&out.stderr)
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
        "nexus-policy",
        "nexus-action-gateway",
        "nexus-model-transport",
    ];
    let mut violations: Vec<String> = Vec::new();
    for line in stdout.lines().skip(1) {
        for needle in forbidden {
            let trimmed =
                line.trim_start_matches(['\u{251c}', '\u{2514}', '\u{2500}', '\u{2502}', ' ']);
            if trimmed.starts_with(needle) || trimmed.contains(&format!(" {needle} v")) {
                violations.push(format!("{needle}: {line}"));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "nexus-model-router production tree violates dependency direction:\n{}",
        violations.join("\n")
    );
}
