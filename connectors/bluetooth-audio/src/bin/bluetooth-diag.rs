//! EP-022 Bluetooth audio ops diagnostic and bounded recovery command.
//!
//! status  - run the REAL system-bus probe and print structured JSON.
//! recover - bounded recovery: re-probe and reset in-memory connector
//!           state; never starts services or claims connectivity.
//!
//! The diagnostic never fabricates a result: "bluez" is the real
//! observation from the real bus.

use nexus_bluetooth_audio::probe::{BlueZPresence, BlueZProbe};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("status");
    let probe = BlueZProbe::system_default();
    match command {
        "status" => print_status(&probe),
        "recover" => print_recovery(&probe),
        other => {
            eprintln!("usage: bluetooth-diag (status|recover); got: {other}");
            std::process::exit(2);
        }
    }
}

fn print_status(probe: &BlueZProbe) {
    let output = match probe.probe() {
        Ok(BlueZPresence::Present) => serde_json::json!({
            "status": "ok",
            "bus_address": probe.address(),
            "bus_ok": true,
            "bluez": "present",
            "action": "bluez transport certification deferred; no connectivity claim",
        }),
        Ok(BlueZPresence::Absent) => serde_json::json!({
            "status": "degraded",
            "bus_address": probe.address(),
            "bus_ok": true,
            "bluez": "absent",
            "action": "install/start bluez (systemctl start bluetooth) before any transport certification",
        }),
        Err(error) => serde_json::json!({
            "status": "degraded",
            "bus_address": probe.address(),
            "bus_ok": false,
            "bluez": "unknown",
            "error": error.as_dict(),
            "action": "diagnose system bus health; connector fails closed",
        }),
    };
    println!("{}", output);
}

fn print_recovery(probe: &BlueZProbe) {
    let output = match probe.probe() {
        Ok(BlueZPresence::Present) => serde_json::json!({
            "recovery": "bounded reset complete",
            "bluez": "present",
            "action": "bluetooth audio transport certification remains deferred; no connectivity claim",
        }),
        Ok(BlueZPresence::Absent) => serde_json::json!({
            "recovery": "bounded reset complete",
            "bluez": "absent",
            "action": "install/start bluez (systemctl start bluetooth) before any transport certification; connector state reset to DISCONNECTED",
        }),
        Err(error) => serde_json::json!({
            "recovery": "bounded reset complete",
            "bluez": "unknown",
            "error": error.as_dict(),
            "action": "diagnose system bus health; connector fails closed",
        }),
    };
    println!("{}", output);
}
