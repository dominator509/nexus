//! EP-017 dependency-direction guard for `nexus-agents` (SPEC-001):
//! the agent orchestrator contract crate must not drag infrastructure,
//! network, HTTP, vendor, or later-node crates into its production
//! tree. All harness behavior is injected through the `AgentAdapter`
//! port; provider adapters live in the M2 crate boundary.

use std::process::Command;

#[test]
fn ep017_unit_dependency_direction() {
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "nexus-agents",
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
        "postgres",
        "sqlx",
        "rusqlite",
        "nats",
        "tonic",
        "prost",
        "clap",
        "temporal",
        "openbao",
        "headscale",
        "openfga",
        "opa",
        "jsonschema",
        "bifrost",
        "transport",
        "nexus-reflex",
        "nexus-model-router",
        "nexus-policy",
        "nexus-action-gateway",
        "nexus-harness-adapters",
        "openai",
        "anthropic",
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
        "nexus-agents production tree violates dependency direction:\n{}",
        violations.join("\n")
    );
}
