//! EP-025 `asterisk-diag` operations diagnostic (M2).
//!
//! status: real ARI health, channel/session count, governed
//! capability set, and per-session canonical state. Safe telemetry
//! only (directive 24) - never credentials, raw audio, or private
//! caller information.
//!
//! recover: bounded actions only - reconnect/health probe and a
//! fresh readback. It NEVER originates, answers, hangs up, plays,
//! or sends DTMF (directive V).

use std::process::ExitCode;
use std::time::Duration;

use nexus_asterisk::{AsteriskAdapter, RestAriTransport};
use nexus_telephony::{CallCapability, CallPolicy, DisclosurePolicy};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("status");
    let base_url =
        std::env::var("NEXUS_ARI_URL").unwrap_or_else(|_| "http://127.0.0.1:8088".into());
    let user = std::env::var("NEXUS_ARI_USER").unwrap_or_else(|_| "nexus".into());
    let password = std::env::var("NEXUS_ARI_PASSWORD").unwrap_or_default();
    let timeout = std::env::var("NEXUS_ARI_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(10);

    let transport = match RestAriTransport::new(
        base_url.clone(),
        user.clone(),
        password.clone(),
        Duration::from_secs(timeout),
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("asterisk-diag: transport init failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    // The ARI password is used ONLY for the transport; it is never
    // placed in diagnostics.
    let policy = CallPolicy {
        allowed_capabilities: vec![
            CallCapability::Dial,
            CallCapability::Answer,
            CallCapability::Hangup,
            CallCapability::Transfer,
            CallCapability::Dtmf,
            CallCapability::Hold,
            CallCapability::Status,
        ],
        max_duration_seconds: 3600,
        cost_cap: 1.0,
        disclosure: DisclosurePolicy::new(false, true, "US", 0).unwrap_or_else(|_| {
            DisclosurePolicy::new(false, true, "ZZ", 0).expect("valid fallback disclosure")
        }),
    };
    let adapter = AsteriskAdapter::new(Box::new(transport), policy);

    match mode {
        "status" => status(&adapter),
        "recover" => recover(&adapter),
        other => {
            eprintln!("asterisk-diag: unknown mode {other:?} (status|recover)");
            ExitCode::FAILURE
        }
    }
}

fn status(adapter: &AsteriskAdapter) -> ExitCode {
    match adapter.provider_available() {
        Ok(true) => {
            println!("provider: AVAILABLE");
            match adapter.list_sessions() {
                Ok(sessions) => {
                    println!("channels: {}", sessions.len());
                    for session in sessions {
                        println!(
                            "session {} state {} legs {}",
                            session.id.as_str(),
                            session.state.as_str(),
                            session.legs.len()
                        );
                    }
                    // M4: bounded provider surface (directive W) - real
                    // PJSIP endpoint/contact state and bridge count from
                    // Asterisk's own ARI state, safe telemetry only.
                    for ep in ["endpoint-a", "endpoint-b", "endpoint-c", "endpoint-d"] {
                        match adapter.endpoint_state(ep) {
                            Ok(e) => println!(
                                "endpoint {ep}: {}",
                                e.state.as_deref().unwrap_or("unknown")
                            ),
                            Err(e) => {
                                println!("endpoint {ep}: error {} {}", e.code.as_str(), e.message)
                            }
                        }
                    }
                    match adapter.transport().list_bridges() {
                        Ok(bridges) => println!("bridges: {}", bridges.len()),
                        Err(e) => println!("bridges: error {} {}", e.code.as_str(), e.message),
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    println!("provider: DEGRADED");
                    println!("error: {} {}", e.code.as_str(), e.message);
                    ExitCode::FAILURE
                }
            }
        }
        Ok(false) => {
            println!("provider: UNAVAILABLE");
            ExitCode::FAILURE
        }
        Err(e) => {
            println!("provider: UNAVAILABLE");
            println!("error: {} {}", e.code.as_str(), e.message);
            ExitCode::FAILURE
        }
    }
}

fn recover(adapter: &AsteriskAdapter) -> ExitCode {
    // Bounded recovery: reconnect/health probe + fresh readback ONLY.
    // Never originates/answers/hangs up/plays/DTMF.
    match adapter.provider_available() {
        Ok(true) => {
            println!("recover: ok");
            println!("provider: AVAILABLE");
            ExitCode::SUCCESS
        }
        Ok(false) => {
            println!("recover: FAIL - provider unavailable");
            ExitCode::FAILURE
        }
        Err(e) => {
            println!("recover: FAIL - {} {}", e.code.as_str(), e.message);
            ExitCode::FAILURE
        }
    }
}
