//! EP-024 M5 vacuum operations diagnostic + bounded recovery
//! (SPEC-011; M5 directive V).
//!
//! Real production diagnostic over the REAL vacuum adapter:
//!   - status: auth check, vacuum discovery, per-device availability
//!     and capabilities, current canonical activity state, counters +
//!     redacted audit tail;
//!   - recover: bounded recovery (clears stuck in-flight entries),
//!     then re-checks availability.
//!
//! It NEVER starts/stops/returns a vacuum and NEVER fabricates
//! AVAILABLE/VERIFIED state.
//!
//! Output is redacted (the provider token never appears; secrets are
//! redacted by the observability ring).
//!
//! Env: NEXUS_HA_BASE + NEXUS_HA_TOKEN (fixture bootstrap) or
//! VACUUM_BASE_URL / VACUUM_TOKEN.

use std::env;
use std::process::ExitCode;

use nexus_devices::vocabulary::DeviceAvailability;
use nexus_devices::VacuumProvider;
use nexus_vacuum::{
    vacuum_device_id, vacuum_state_value, HaVacuumTransport, VacuumAdapter, VacuumDeviceSelector,
};

const VACUUM_A: &str = "vacuum.nexus_vacuum_a";
const VACUUM_B: &str = "vacuum.nexus_vacuum_b";

fn main() -> ExitCode {
    let base = env::var("VACUUM_BASE_URL")
        .or_else(|_| env::var("NEXUS_HA_BASE"))
        .unwrap_or_else(|_| "http://127.0.0.1:8126".to_string());
    let token = env::var("VACUUM_TOKEN")
        .or_else(|_| env::var("NEXUS_HA_TOKEN"))
        .unwrap_or_default();
    if token.is_empty() {
        eprintln!("vacuum-diag: FAIL - no token (set VACUUM_TOKEN or NEXUS_HA_TOKEN)");
        return ExitCode::from(2);
    }

    let transport = HaVacuumTransport::new(base, token.clone());
    let adapter = VacuumAdapter::new(
        transport,
        VacuumDeviceSelector::entities([VACUUM_A.to_string(), VACUUM_B.to_string()]),
    )
    .with_observability_secrets(vec![token]);

    let mode = env::args().nth(1).unwrap_or_else(|| "status".to_string());
    match mode.as_str() {
        "status" => status(&adapter),
        "recover" => recover(&adapter),
        other => {
            eprintln!("vacuum-diag: unknown mode {other:?} (status|recover)");
            ExitCode::from(2)
        }
    }
}

fn status(adapter: &VacuumAdapter<HaVacuumTransport>) -> ExitCode {
    // Auth check (real provider boundary).
    let base = env::var("VACUUM_BASE_URL")
        .or_else(|_| env::var("NEXUS_HA_BASE"))
        .unwrap_or_else(|_| "http://127.0.0.1:8126".to_string());
    let token = env::var("VACUUM_TOKEN")
        .or_else(|_| env::var("NEXUS_HA_TOKEN"))
        .unwrap_or_default();
    let probe = HaVacuumTransport::new(base, token);
    match probe.auth_check() {
        Ok(true) => println!("auth: ok"),
        Ok(false) => {
            println!("auth: FAIL (invalid credential)");
            return ExitCode::from(1);
        }
        Err(e) => {
            println!("auth: FAIL ({})", e.code.as_str());
            return ExitCode::from(1);
        }
    }

    let mut failed = false;
    for entity_id in [VACUUM_A, VACUUM_B] {
        let device = match vacuum_device_id(entity_id) {
            Ok(device) => device,
            Err(e) => {
                println!("vacuum {entity_id}: id error ({e})");
                failed = true;
                continue;
            }
        };
        match adapter.availability(&device) {
            Ok(avail) => {
                // Only AVAILABLE is healthy: a vacuum the provider does
                // not actually report as present + usable is DEGRADED.
                if avail != DeviceAvailability::Available {
                    failed = true;
                }
                let caps = match adapter.capabilities(&device) {
                    Ok(c) => c
                        .iter()
                        .map(|cap| cap.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                    Err(e) => {
                        failed = true;
                        format!("ERR({})", e.code.as_str())
                    }
                };
                // Current canonical state from the real readback.
                let state = match adapter.read_device(entity_id) {
                    Ok(device_entity) => vacuum_state_value(&device_entity)
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_else(|| "UNKNOWN".to_string()),
                    Err(e) => {
                        failed = true;
                        format!("ERR({})", e.code.as_str())
                    }
                };
                println!(
                    "vacuum {entity_id}: availability={} capabilities={} state={}",
                    avail.as_str(),
                    caps,
                    state
                );
            }
            Err(e) => {
                println!(
                    "vacuum {entity_id}: availability FAIL ({})",
                    e.code.as_str()
                );
                failed = true;
            }
        }
    }

    // Observability: counters + redacted audit tail.
    for (key, value) in adapter.counters() {
        println!("counter {key}={value}");
    }
    for entry in adapter.audit().iter().rev().take(5) {
        println!(
            "audit {} {} {} {}",
            entry.correlation, entry.operation, entry.outcome, entry.detail
        );
    }

    if failed {
        println!("vacuum-diag: DEGRADED");
        ExitCode::from(1)
    } else {
        println!("vacuum-diag: ok");
        ExitCode::SUCCESS
    }
}

fn recover(adapter: &VacuumAdapter<HaVacuumTransport>) -> ExitCode {
    let cleared = adapter.recover();
    println!("recover: cleared {cleared} in-flight entries");
    status(adapter)
}
