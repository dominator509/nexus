//! EP-044 control-plane runtime binary (ADR-019 `ControlPlaneServer`).
//!
//! The real runnable Nexus Control Plane Runtime. Reads canonical
//! configuration from the environment (`NEXUS_BASE_DOMAIN`,
//! `NEXUS_SMOKE_URL`, `NEXUS_CONTROL_PLANE_BIND`, `NEXUS_TENANT_ID`,
//! `NEXUS_CAPABILITY_SOURCE`), composes the real capability source and
//! lifecycle, and serves `/healthz`, `/readyz`, `/v1/capabilities`
//! until a shutdown signal arrives. No placeholder mode; the binary IS
//! the runtime.

use std::sync::Arc;

use nexus_control_plane::capabilities::{CapabilityListSource, ConfiguredCapabilityList};
use nexus_control_plane::config::ControlPlaneConfig;
use nexus_control_plane::server::ControlPlaneServer;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() {
    let base_domain = env_or("NEXUS_BASE_DOMAIN", "nexus.test");
    let bind_address = env_or("NEXUS_CONTROL_PLANE_BIND", "127.0.0.1:8443");
    let tenant_id = env_or("NEXUS_TENANT_ID", "018f0f6f-9c1e-7b6e-8000-000000000001");
    let capability_source = env_or("NEXUS_CAPABILITY_SOURCE", "core");

    let config = match ControlPlaneConfig::new(
        &base_domain,
        &bind_address,
        &tenant_id,
        &capability_source,
    ) {
        Ok(cfg) => cfg,
        Err(err) => {
            eprintln!("control-plane: invalid config: {err}");
            std::process::exit(2);
        }
    };

    // Canonical base URL for the runtime smoke (NEXUS_SMOKE_URL wins).
    let _smoke_url = env_or("NEXUS_SMOKE_URL", &config.base_url());

    // Real capability source: deterministic core list. Never invented at
    // request time (SPEC-003 discovery; EP-044 composition root).
    let capabilities: Box<dyn CapabilityListSource + Send + Sync> =
        Box::new(ConfiguredCapabilityList::new(
            capability_source.clone(),
            vec!["health".to_string(), "capabilities".to_string()],
        ));

    let server = ControlPlaneServer::new(config, capabilities);
    let lifecycle = Arc::clone(server.lifecycle());

    println!(
        "control-plane: starting on {} (base {})",
        server.config().bind_address,
        server.config().base_url()
    );

    let shutdown = async {
        let _ = tokio::signal::ctrl_c().await;
        eprintln!("control-plane: shutdown signal received");
    };

    match server.serve(shutdown).await {
        Ok(()) => {
            let state = lifecycle.lock().expect("lifecycle lock").state();
            println!("control-plane: stopped (state {state})");
        }
        Err(err) => {
            eprintln!("control-plane: {err}");
            std::process::exit(1);
        }
    }
}
