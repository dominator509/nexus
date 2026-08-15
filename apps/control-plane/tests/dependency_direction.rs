//! EP-044 dependency-direction guard for `nexus-control-plane`.
//!
//! The control-plane app may depend on the approved Nexus contract
//! crates plus the real HTTP server chain (axum/tokio/hyper/tower),
//! but it must never drag vendor infrastructure or provider adapters
//! into its production tree (SPEC-001 dependency direction; ARCHITECTURE
//! code import law 6). This mirrors the `nexus-gateway` guard.

use std::process::Command;

#[test]
fn ep044_unit_dependency_direction() {
    let out = Command::new(env!("CARGO"))
        .args([
            "tree",
            "-p",
            "nexus-control-plane",
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
        "keycloak",
        "temporal",
        "postgres",
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
        "nexus-control-plane production tree violates dependency direction:\n{}",
        violations.join("\n")
    );
}
