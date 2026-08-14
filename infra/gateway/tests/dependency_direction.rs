//! EP-012 M5 dependency-direction guard for `nexus-gateway`.
//!
//! The composed gateway may depend on the fabric contract crates and
//! the two real engines, but it must never drag infrastructure, HTTP,
//! network, or vendor crates into its production tree beyond the
//! declared set. This mirrors the `nexus-policy`/`nexus-fabric`
//! guards (SPEC-001 dependency direction).

use std::process::Command;

#[test]
fn ep012_gateway_dependency_direction() {
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "nexus-gateway",
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
    ];
    let mut violations: Vec<String> = Vec::new();
    for line in stdout.lines().skip(1) {
        for needle in forbidden {
            // Match the crate name at the start of the dependency line
            // (e.g. "|-- tokio v1.53" or "`-- axum v0.8" with the
            // box-drawing prefix characters rendered as unicode
            // escapes: \u{251c}\u{2500}\u{2500} and \u{2514}\u{2500}\u{2500}).
            let trimmed =
                line.trim_start_matches(['\u{251c}', '\u{2514}', '\u{2500}', '\u{2502}', ' ']);
            if trimmed.starts_with(needle) || trimmed.contains(&format!(" {needle} v")) {
                violations.push(format!("{needle}: {line}"));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "nexus-gateway production tree violates dependency direction:\n{}",
        violations.join("\n")
    );
}
