//! EP-013 M2 dependency-direction guard for `nexus-bifrost`.
//!
//! The Bifrost adapter imports the model gateway application ports
//! and `nexus-domain` vocabulary, but it must never drag
//! infrastructure, HTTP, network, database, or vendor crates into
//! its production tree (SPEC-001 dependency direction).

use std::process::Command;

#[test]
fn ep013_bifrost_dependency_direction() {
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "nexus-bifrost",
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
        "sha2",
    ];
    let mut violations: Vec<String> = Vec::new();
    for line in stdout.lines().skip(1) {
        for needle in forbidden {
            // Match the crate name at the start of the dependency line
            // with the box-drawing prefix characters rendered as unicode
            // escapes: \u{251c}\u{2500}\u{2500} and \u{2514}\u{2500}\u{2500}.
            let trimmed =
                line.trim_start_matches(['\u{251c}', '\u{2514}', '\u{2500}', '\u{2502}', ' ']);
            if trimmed.starts_with(needle) || trimmed.contains(&format!(" {needle} v")) {
                violations.push(format!("{needle}: {line}"));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "nexus-bifrost production tree violates dependency direction:\n{}",
        violations.join("\n")
    );
}
