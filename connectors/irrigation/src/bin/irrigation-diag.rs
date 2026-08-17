//! EP-024 irrigation operations diagnostic + bounded recovery
//! (SPEC-011; M4 directive 6).
//!
//! Real production diagnostic over the REAL irrigation adapter:
//!   - status: auth check, zone discovery, per-zone availability and
//!     capabilities, observability counters + redacted audit tail;
//!   - recover: bounded recovery (clears stuck in-flight entries),
//!     then re-checks availability.
//!
//! Output is redacted (the provider token never appears; secrets are
//! redacted by the observability ring).
//!
//! Env: NEXUS_HA_BASE + NEXUS_HA_TOKEN (fixture bootstrap) or
//! IRRIGATION_BASE_URL / IRRIGATION_TOKEN.

use std::env;
use std::process::ExitCode;

use nexus_devices::vocabulary::DeviceAvailability;
use nexus_devices::IrrigationProvider;
use nexus_irrigation::{
    irrigation_zone_id, HaIrrigationTransport, IrrigationAdapter, IrrigationZoneSelector,
};

const ZONE_A: &str = "input_boolean.nexus_zone_a";
const ZONE_B: &str = "input_boolean.nexus_zone_b";

fn main() -> ExitCode {
    let base = env::var("IRRIGATION_BASE_URL")
        .or_else(|_| env::var("NEXUS_HA_BASE"))
        .unwrap_or_else(|_| "http://127.0.0.1:8125".to_string());
    let token = env::var("IRRIGATION_TOKEN")
        .or_else(|_| env::var("NEXUS_HA_TOKEN"))
        .unwrap_or_default();
    if token.is_empty() {
        eprintln!("irrigation-diag: FAIL - no token (set IRRIGATION_TOKEN or NEXUS_HA_TOKEN)");
        return ExitCode::from(2);
    }

    let transport = HaIrrigationTransport::new(base, token.clone());
    let adapter = IrrigationAdapter::new(
        transport,
        IrrigationZoneSelector::entities([ZONE_A.to_string(), ZONE_B.to_string()]),
    )
    .with_observability_secrets(vec![token]);

    let mode = env::args().nth(1).unwrap_or_else(|| "status".to_string());
    match mode.as_str() {
        "status" => status(&adapter),
        "recover" => recover(&adapter),
        other => {
            eprintln!("irrigation-diag: unknown mode {other:?} (status|recover)");
            ExitCode::from(2)
        }
    }
}

fn status(adapter: &IrrigationAdapter<HaIrrigationTransport>) -> ExitCode {
    // Auth check (real provider boundary).
    let base = env::var("IRRIGATION_BASE_URL")
        .or_else(|_| env::var("NEXUS_HA_BASE"))
        .unwrap_or_else(|_| "http://127.0.0.1:8125".to_string());
    let token = env::var("IRRIGATION_TOKEN")
        .or_else(|_| env::var("NEXUS_HA_TOKEN"))
        .unwrap_or_default();
    let probe = HaIrrigationTransport::new(base, token);
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
    for entity_id in [ZONE_A, ZONE_B] {
        let zone = match irrigation_zone_id(entity_id) {
            Ok(zone) => zone,
            Err(e) => {
                println!("zone {entity_id}: id error ({e})");
                failed = true;
                continue;
            }
        };
        match adapter.availability(&zone) {
            Ok(avail) => {
                // Only AVAILABLE is healthy: a zone the provider does
                // not actually report as present + usable is DEGRADED
                // (never a false "ok" from a fresh readback that is
                // unavailable/not-found).
                if avail != DeviceAvailability::Available {
                    failed = true;
                }
                let caps = match adapter.capabilities(&zone) {
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
                println!(
                    "zone {entity_id}: availability={} capabilities={}",
                    avail.as_str(),
                    caps
                );
            }
            Err(e) => {
                println!("zone {entity_id}: availability FAIL ({})", e.code.as_str());
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
        println!("irrigation-diag: DEGRADED");
        ExitCode::from(1)
    } else {
        println!("irrigation-diag: ok");
        ExitCode::SUCCESS
    }
}

fn recover(adapter: &IrrigationAdapter<HaIrrigationTransport>) -> ExitCode {
    let cleared = adapter.recover();
    println!("recover: cleared {cleared} in-flight entries");
    status(adapter)
}
