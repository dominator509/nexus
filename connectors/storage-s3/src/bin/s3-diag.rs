//! EP-037 M5 `s3-diag` operations diagnostic (ExecPlan M5 CONTENT 3:
//! operations; SPEC-024 adapter diagnosis).
//!
//! status: configured -> reachable (TCP) -> provider responding
//! (real production probe: canary PUT/GET with digest verification +
//! DELETE) -> probe verified. Never "endpoint configured -> healthy":
//! unreachable or unverified providers exit nonzero with an explicit
//! truthful status.
//!
//! recover: bounded actions only - re-run the read/write probe after a
//! provider restart; never an infinite reconnect loop, never credential
//! rotation, never destructive actions.
//!
//! Safe telemetry only: credentials are used for the transport and
//! NEVER printed; payload content is never printed.

use std::process::ExitCode;
use std::time::Duration;

use nexus_provider_storage_s3::{S3ArtifactStore, S3CompatibilityProfile, S3Config};

fn config_from_env() -> S3Config {
    let endpoint =
        std::env::var("NEXUS_S3_ENDPOINT").unwrap_or_else(|_| "127.0.0.1:9000".to_string());
    let access = std::env::var("NEXUS_S3_ACCESS_KEY").unwrap_or_default();
    let secret = std::env::var("NEXUS_S3_PW_KEY").unwrap_or_default();
    let prefix = std::env::var("NEXUS_S3_BUCKET_PREFIX").unwrap_or_else(|_| "n".to_string());
    let region = std::env::var("NEXUS_S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let profile = match std::env::var("NEXUS_S3_PROFILE").as_deref() {
        Ok("MINIO") => S3CompatibilityProfile::MinIo,
        Ok("SEAWEEDFS") => S3CompatibilityProfile::SeaweedFs,
        Ok("AWS_S3") => S3CompatibilityProfile::AwsS3,
        Ok("R2") => S3CompatibilityProfile::R2,
        Ok("B2") => S3CompatibilityProfile::B2,
        _ => S3CompatibilityProfile::Generic,
    };
    let timeout = std::env::var("NEXUS_S3_TIMEOUT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);
    S3Config::new(endpoint, access, secret, prefix)
        .with_region(region)
        .with_profile(profile)
        .with_timeouts(Duration::from_secs(3), Duration::from_secs(timeout))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("status");
    match mode {
        "status" => status(),
        "recover" => recover(),
        other => {
            eprintln!("s3-diag: unknown mode {other:?} (status|recover)");
            ExitCode::FAILURE
        }
    }
}

fn configured(config: &S3Config) -> bool {
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
    println!("provider: s3-compatible");
    println!("profile: {}", config.profile.as_str());
    println!("region: {}", config.region);
    println!("endpoint: {}", config.endpoint);

    // reachable: bounded TCP connect through the transport.
    let store = match S3ArtifactStore::open(config) {
        Ok(s) => s,
        Err(e) => {
            println!("reachable: false ({e})");
            println!("provider_responding: false");
            println!("probe_verified: false");
            println!("state: DEGRADED");
            return ExitCode::FAILURE;
        }
    };
    println!("reachable: true");

    // provider_responding + probe_verified: real production probe
    // (PUT -> GET -> digest verify -> DELETE). healthz alone is never
    // readiness.
    match store.diag_probe() {
        Ok(()) => {
            println!("provider_responding: true");
            println!("probe_verified: true");
            println!("state: READY");
            ExitCode::SUCCESS
        }
        Err(e) => {
            println!("provider_responding: false");
            println!("probe_verified: false");
            println!("state: DEGRADED ({e})");
            ExitCode::FAILURE
        }
    }
}

fn recover() -> ExitCode {
    // Bounded recovery: re-run the production probe (bounded attempts
    // with backoff). Never an infinite loop, never destructive.
    let config = config_from_env();
    if !configured(&config) {
        println!("recover: configured: false; nothing to probe");
        return ExitCode::FAILURE;
    }
    let store = match S3ArtifactStore::open(config) {
        Ok(s) => s,
        Err(e) => {
            println!("recover: cannot open adapter: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut last = String::new();
    for attempt in 1..=5u32 {
        match store.diag_probe() {
            Ok(()) => {
                println!("recover: probe verified after {attempt} attempt(s)");
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                last = format!("{e}");
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    }
    println!("recover: FAILED after 5 attempts (last: {last})");
    ExitCode::FAILURE
}
