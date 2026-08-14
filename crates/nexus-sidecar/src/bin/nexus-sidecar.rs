//! EP-011 M4 sidecar process (directive B/M/W).
//!
//! The real sidecar binary:
//!
//! - binds 127.0.0.1 with an ephemeral port (never 0.0.0.0);
//! - prints `PORT <n>` on stdout once ready;
//! - serves the hardened sidecar REST surface;
//! - shuts down cleanly on SIGTERM/SIGINT (listener released, no
//!   stale children, no leaked credential state).
//!
//! Configuration comes from the environment:
//!
//! - NEXUS_SIDECAR_BIND (default 127.0.0.1; loopback enforced)
//! - NEXUS_SIDECAR_PORT (default 0 = ephemeral)
//! - NEXUS_SIDECAR_TENANT (bound tenant, directive F)
//! - NEXUS_SIDECAR_CONNECTOR (connector id, directive G)
//! - NEXUS_SIDECAR_CAPABILITIES (comma list `id:CLASS`, directive G)
//! - NEXUS_SIDECAR_CREDENTIAL_SCOPE (comma list `connector:ref`, N)
//! - NEXUS_PROVIDER_URL (provider base URL, directive J)
//! - NEXUS_SIDECAR_MAX_REQUEST_BYTES / MAX_RESPONSE_BYTES (directive D)
//! - NEXUS_SIDECAR_READ_TIMEOUT_MS / PROVIDER_TIMEOUT_MS (directive U)
//! - NEXUS_SIDECAR_MAX_CONCURRENCY (directive T)
//! - NEXUS_SIDECAR_WEBHOOK_SECRET_HEX / WEBHOOK_FINGERPRINT (P/Q)

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use nexus_domain::vocabulary::CapabilityClass;
use nexus_sidecar::credential::CredentialScope;
use nexus_sidecar::dispatch::{CapabilityClassTable, ConnectorTable};
use nexus_sidecar::limits::Limits;
use nexus_sidecar::server::{SidecarConfig, SidecarServer};
use nexus_sidecar::tenant::TenantBinding;
use nexus_sidecar::webhook::{WebhookIngress, WebhookPolicy};
use nexus_sidecar::{SidecarError, SidecarErrorKind};

fn env_str(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty())
}

fn env_u64(name: &str, default: u64) -> u64 {
    env_str(name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn env_usize(name: &str, default: usize) -> usize {
    env_str(name)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_class(value: &str) -> Result<CapabilityClass, SidecarError> {
    match value {
        "QUERY" => Ok(CapabilityClass::Query),
        "COMMAND" => Ok(CapabilityClass::Command),
        "WORKFLOW" => Ok(CapabilityClass::Workflow),
        "STREAM" => Ok(CapabilityClass::Stream),
        "ADMINISTRATIVE" => Ok(CapabilityClass::Administrative),
        other => Err(SidecarError::validation(
            format!("unknown capability class: {other}"),
            None,
        )),
    }
}

fn build_config() -> Result<SidecarConfig, SidecarError> {
    // Loopback-only bind (directive B): the sidecar must never expose
    // 0.0.0.0. Any non-loopback bind address is rejected.
    let bind_ip = env_str("NEXUS_SIDECAR_BIND")
        .and_then(|v| v.parse::<IpAddr>().ok())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    if !bind_ip.is_loopback() {
        return Err(SidecarError::new(
            SidecarErrorKind::Validation,
            format!("sidecar bind must be loopback, got {bind_ip}"),
            None,
            None,
            None,
        ));
    }
    let port = env_u64("NEXUS_SIDECAR_PORT", 0) as u16;
    let bind = SocketAddr::new(bind_ip, port);

    let tenant_id = env_str("NEXUS_SIDECAR_TENANT")
        .ok_or_else(|| SidecarError::validation("NEXUS_SIDECAR_TENANT is required", None))?;
    let tenant = TenantBinding::new(tenant_id)?;

    let connector_id = env_str("NEXUS_SIDECAR_CONNECTOR")
        .ok_or_else(|| SidecarError::validation("NEXUS_SIDECAR_CONNECTOR is required", None))?;

    let mut table = CapabilityClassTable::new();
    let mut connector = ConnectorTable::new(connector_id.clone());
    if let Some(caps) = env_str("NEXUS_SIDECAR_CAPABILITIES") {
        for entry in caps.split(',') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            let Some((id, class)) = entry.split_once(':') else {
                return Err(SidecarError::validation(
                    format!("malformed capability entry: {entry}"),
                    None,
                ));
            };
            connector.register(id.trim(), parse_class(class.trim())?);
        }
    }
    table.insert(connector);

    // Optional second connector (directive G/N cross-connector proofs):
    // `NEXUS_SIDECAR_CONNECTOR_EXTRA` + `NEXUS_SIDECAR_CAPABILITIES_EXTRA`.
    if let Some(extra_connector) = env_str("NEXUS_SIDECAR_CONNECTOR_EXTRA") {
        let mut extra = ConnectorTable::new(extra_connector.clone());
        if let Some(caps) = env_str("NEXUS_SIDECAR_CAPABILITIES_EXTRA") {
            for entry in caps.split(',') {
                let entry = entry.trim();
                if entry.is_empty() {
                    continue;
                }
                let Some((id, class)) = entry.split_once(':') else {
                    return Err(SidecarError::validation(
                        format!("malformed extra capability entry: {entry}"),
                        None,
                    ));
                };
                extra.register(id.trim(), parse_class(class.trim())?);
            }
        }
        table.insert(extra);
    }

    let mut credentials = CredentialScope::new();
    if let Some(scope) = env_str("NEXUS_SIDECAR_CREDENTIAL_SCOPE") {
        for entry in scope.split(',') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            let Some((conn, reference)) = entry.split_once(':') else {
                return Err(SidecarError::validation(
                    format!("malformed credential scope entry: {entry}"),
                    None,
                ));
            };
            credentials.grant(conn.trim(), reference.trim());
        }
    }

    let provider_base_url = env_str("NEXUS_PROVIDER_URL")
        .ok_or_else(|| SidecarError::validation("NEXUS_PROVIDER_URL is required", None))?;

    let limits = Limits::new(
        env_u64("NEXUS_SIDECAR_MAX_REQUEST_BYTES", 64 * 1024),
        env_u64("NEXUS_SIDECAR_MAX_RESPONSE_BYTES", 64 * 1024),
        env_usize("NEXUS_SIDECAR_MAX_CONCURRENCY", 16),
        Duration::from_millis(env_u64("NEXUS_SIDECAR_READ_TIMEOUT_MS", 10_000)),
        Duration::from_millis(env_u64("NEXUS_SIDECAR_PROVIDER_TIMEOUT_MS", 5_000)),
    );

    let webhook = match (
        env_str("NEXUS_SIDECAR_WEBHOOK_SECRET_HEX"),
        env_str("NEXUS_SIDECAR_WEBHOOK_FINGERPRINT"),
    ) {
        (Some(secret), Some(fp)) => {
            let policy = WebhookPolicy::new(secret, fp)?;
            let dedupe = env_usize("NEXUS_SIDECAR_WEBHOOK_DEDUPE", 4096);
            Some(std::sync::Arc::new(std::sync::Mutex::new(
                WebhookIngress::new(policy, dedupe),
            )))
        }
        (None, None) => None,
        _ => {
            return Err(SidecarError::validation(
                "webhook secret and fingerprint must be provided together",
                None,
            ));
        }
    };

    // Owned poller (directive R/S): provisioned when a state dir and
    // source are supplied; otherwise POLL fails closed.
    let poller = match (
        env_str("NEXUS_SIDECAR_STATE_DIR"),
        env_str("NEXUS_SIDECAR_SOURCE"),
    ) {
        (Some(state_dir), Some(source)) => {
            let checkpoint = env_str("NEXUS_SIDECAR_CHECKPOINT")
                .unwrap_or_else(|| "checkpoint.ckpt".to_string());
            Some(nexus_sidecar::poller::PollSource::new(
                state_dir, source, checkpoint, limits,
            )?)
        }
        _ => None,
    };

    Ok(SidecarConfig {
        bind,
        limits,
        tenant,
        dispatch: table,
        credentials,
        provider_base_url,
        webhook,
        poller,
        concurrency: std::sync::Arc::new(tokio::sync::Semaphore::new(limits.max_concurrency)),
    })
}

#[tokio::main]
async fn main() {
    let config = match build_config() {
        Ok(c) => c,
        Err(err) => {
            eprintln!("sidecar config error: {}", err.message);
            std::process::exit(2);
        }
    };

    let server = match SidecarServer::new(config) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("sidecar startup error: {}", err.message);
            std::process::exit(2);
        }
    };

    // Bind a probe listener first to learn the ephemeral port, then
    // hand the actual port to the server. Simpler: bind in the server
    // and print after readiness via a channel is complex; instead we
    // bind here and pass the bound address through config.
    //
    // The server binds again on the same address; to keep the PORT
    // contract deterministic we pre-bind and release (TOCTOU is
    // acceptable for a loopback test fixture; the real deployment
    // binds a fixed port).
    let probe = tokio::net::TcpListener::bind(server.config_bind())
        .await
        .expect("probe bind failed");
    let actual = probe.local_addr().expect("local addr");
    drop(probe);

    let server = server.with_bind(actual);

    println!("PORT {}", actual.port());
    // Ready telemetry.
    server
        .sink()
        .emit(&nexus_sidecar::telemetry::TelemetryEntry {
            event: nexus_sidecar::telemetry::TelemetryEvent::SidecarReady,
            connector_fingerprint: Some(nexus_sidecar::telemetry::fingerprint(
                &server.connector_id(),
            )),
            capability_id: None,
            class: None,
            transport: Some("REST".to_string()),
            result_class: None,
            latency_ms: None,
            correlation_id: None,
            tenant_fingerprint: None,
            detail: Some(format!("port={}", actual.port())),
        });

    // Controlled shutdown (directive M/W): SIGTERM/SIGINT release the
    // listener and exit cleanly.
    //
    // Shutdown ownership (directive C):
    //   OWNED and terminated/awaited by this shutdown path:
    //     - HTTP listener socket: released by process exit (verified
    //       by the mid-request shutdown test rebinding the old port);
    //     - in-flight request tasks: dropped with the runtime;
    //     - webhook ingress/replay state: in-memory Arc, freed at exit
    //       (replay defense is process-lifetime only);
    //     - poller state + checkpoint writer: in-memory / per-request,
    //       no persistent worker to await;
    //     - credential broker handles: in-memory scope table, freed at
    //       exit;
    //     - telemetry/log drain: SIDECAR_STOPPED flushed synchronously
    //       before exit;
    //     - signal handler + timeout/background tasks: terminated at
    //       exit.
    //   NOT owned and therefore NOT terminated here:
    //     - the fixture/provider process (the sidecar is a client; the
    //       caller owns provider lifecycle);
    //     - caller-owned connections after the exit boundary (they
    //       observe canonical termination semantics).
    // Shutdown is bounded and immediate: arbitrary in-flight provider
    // work is not awaited (the M4 contract does not promise graceful
    // completion of provider work); the process exits deterministically
    // and the kernel releases every owned socket.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let tx = shutdown_tx;
        tokio::spawn(async move {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                    .expect("sigterm handler");
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                    .expect("sigint handler");
            tokio::select! {
                _ = sigterm.recv() => {}
                _ = sigint.recv() => {}
            }
            let _ = tx.send(());
        });
    }

    let sink = server.sink();
    let connector_id = server.connector_id();

    tokio::select! {
        _ = shutdown_rx => {
            sink.emit(&nexus_sidecar::telemetry::TelemetryEntry {
                event: nexus_sidecar::telemetry::TelemetryEvent::SidecarStopped,
                connector_fingerprint: Some(nexus_sidecar::telemetry::fingerprint(&connector_id)),
                capability_id: None,
                class: None,
                transport: Some("REST".to_string()),
                result_class: None,
                latency_ms: None,
                correlation_id: None,
                tenant_fingerprint: None,
                detail: Some("signal received".to_string()),
            });
            // Controlled shutdown: release the listener and exit
            // cleanly. The runtime's spawned connection tasks must not
            // keep the process alive; explicit exit(0) is the
            // deterministic clean-shutdown contract (directive M/W).
            std::process::exit(0);
        }
        result = server.serve() => {
            if let Err(err) = result {
                eprintln!("sidecar serve error: {}", err.message);
                std::process::exit(2);
            }
        }
    }
}
