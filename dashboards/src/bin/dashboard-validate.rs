//! EP-038 M5 dashboard validator CLI (SPEC-007 dashboards).
//!
//! Validates every `dashboards/*.json` file against the canonical
//! metric catalog (M4 ops catalog + M1 canonical metrics + M1 alert/slo
//! catalogs) and the Grafana structural contract. Exits non-zero on any
//! finding. Never prints secret-shaped content.

use nexus_dashboards::validate_dashboard_dir;
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "dashboards".to_string());
    let results = validate_dashboard_dir(Path::new(&dir));
    let mut failed = 0usize;
    for (name, findings) in &results {
        if findings.is_empty() {
            println!("{name}: ok");
        } else {
            failed += 1;
            println!("{name}: FAIL");
            for f in findings {
                println!("  [{}] {}", f.code, f.detail);
            }
        }
    }
    if results.is_empty() {
        eprintln!("dashboard validate: FAIL - no dashboard JSON files found under {dir}");
        return ExitCode::FAILURE;
    }
    if failed == 0 {
        println!(
            "dashboard validate: ok ({} dashboards validated against the canonical catalog)",
            results.len()
        );
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "dashboard validate: FAIL - {failed}/{} dashboards failed validation",
            results.len()
        );
        ExitCode::FAILURE
    }
}
