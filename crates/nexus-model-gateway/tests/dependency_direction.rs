//! EP-013 dependency-direction guard for `nexus-model-gateway`
//! (SPEC-001): the contract crate must not drag infrastructure,
//! network, HTTP, or vendor crates into its production tree.

use std::process::Command;

#[test]
fn ep013_unit_dependency_direction() {
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "nexus-model-gateway",
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
        "deepseek",
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
        "nexus-model-gateway production tree violates dependency direction:\n{}",
        violations.join("\n")
    );
}
