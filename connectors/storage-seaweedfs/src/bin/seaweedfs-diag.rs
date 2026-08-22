//! EP-037 M4 `seaweedfs-diag` operations diagnostic (ExecPlan M4
//! CONTENT 6: "Add an operations diagnostic and bounded recovery
//! command for every new service or provider").
//!
//! status: configured -> reachable (TCP) -> provider responding
//! (GET /healthz on the S3 gateway) -> read/write probe verified
//! (canary object PUT/GET with digest verification). Never
//! "endpoint configured -> healthy": unreachable or unverified
//! providers exit nonzero with an explicit truthful status.
//!
//! recover: bounded actions only - re-run the read/write probe after
//! a provider restart; never an infinite reconnect loop, never
//! credential rotation, never destructive actions.
//!
//! Safe telemetry only: credentials are used for the transport and
//! NEVER printed; payload content is never printed.

use std::process::ExitCode;
use std::time::Duration;

use nexus_provider_storage_seaweedfs::{SeaweedFsArtifactStore, SeaweedFsConfig};

fn config_from_env() -> SeaweedFsConfig {
    let endpoint =
        std::env::var("NEXUS_SEAWEEDFS_ENDPOINT").unwrap_or_else(|_| "127.0.0.1:8333".to_string());
    let access = std::env::var("NEXUS_SEAWEEDFS_ACCESS_KEY").unwrap_or_default();
    let secret = std::env::var("NEXUS_SEAWEEDFS_PW_KEY").unwrap_or_default();
    let prefix = std::env::var("NEXUS_SEAWEEDFS_BUCKET_PREFIX").unwrap_or_else(|_| "n".to_string());
    let timeout = std::env::var("NEXUS_SEAWEEDFS_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);
    SeaweedFsConfig::new(endpoint, access, secret, prefix)
        .with_timeouts(Duration::from_secs(3), Duration::from_secs(timeout))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("status");
    match mode {
        "status" => status(),
        "recover" => recover(),
        other => {
            eprintln!("seaweedfs-diag: unknown mode {other:?} (status|recover)");
            ExitCode::FAILURE
        }
    }
}

fn configured(config: &SeaweedFsConfig) -> bool {
    !config.endpoint.trim().is_empty()
        && !config.access_key.trim().is_empty()
        && !config.secret_key.trim().is_empty()
}

fn status() -> ExitCode {
    let config = config_from_env();
    if !configured(&config) {
        println!("configured: false (endpoint or credentials missing)");
        println!("reachable: false");
        println!("provider_responding: false");
        println!("probe_verified: false");
        println!("state: DEGRADED");
        return ExitCode::FAILURE;
    }
    println!("configured: true");
    println!("provider: seaweedfs:s3-gateway");
    println!("endpoint: {}", config.endpoint);

    // reachable: bounded TCP connect through the transport.
    let store = match SeaweedFsArtifactStore::open(config) {
        Ok(s) => s,
        Err(e) => {
            println!("reachable: false ({e})");
            println!("provider_responding: false");
            println!("probe_verified: false");
            println!("state: DEGRADED");
            return ExitCode::FAILURE;
        }
    };

    // responding + probe verified: real read/write probe with digest
    // verification (an endpoint that accepts TCP but never answers or
    // returns wrong bytes is DEGRADED, never healthy).
    match store.diag_probe() {
        Ok(()) => {
            println!("reachable: true");
            println!("provider_responding: true");
            println!("probe_verified: true");
            println!("state: OK");
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("reachable: true");
            println!("provider_responding: true");
            println!("probe_verified: false ({e})");
            println!("state: DEGRADED");
            ExitCode::FAILURE
        }
    }
}

fn recover() -> ExitCode {
    // Bounded recovery: re-run the probe once after a provider restart.
    // If the provider is still down, report truthfully; never loop.
    let config = config_from_env();
    if !configured(&config) {
        println!("recover: FAIL - provider not configured");
        return ExitCode::FAILURE;
    }
    let store = match SeaweedFsArtifactStore::open(config) {
        Ok(s) => s,
        Err(e) => {
            println!("recover: FAIL - provider unreachable ({e})");
            return ExitCode::FAILURE;
        }
    };
    match store.diag_probe() {
        Ok(()) => {
            println!("recover: OK - provider responding, probe verified");
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("recover: FAIL - provider not verified ({e})");
            ExitCode::FAILURE
        }
    }
}
