//! EP-017 M2 dependency-direction guard for `nexus-harness-adapters`
//! (SPEC-001): the harness adapter implementations must not drag
//! infrastructure, network, HTTP, or vendor SDK crates into their
//! production tree. All process I/O is behind the injected
//! `HarnessCommandRunner` port; the adapter never shells out directly.

use std::process::Command;

#[test]
fn ep017_unit_dependency_direction() {
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "nexus-harness-adapters",
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
        "openai",
        "anthropic",
        "nexus-context",
        "nexus-memory-workers",
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
        "nexus-harness-adapters production tree violates dependency direction:\n{}",
        violations.join("\n")
    );
}
