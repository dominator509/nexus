//! EP-023 Frigate ops diagnostic and bounded recovery command (M4).
//!
//! status  - probe the REAL provider through the production adapter
//!           and print structured JSON: reachability, version, camera
//!           counts, live/degraded stream counts, go2rtc availability,
//!           safe error classification.
//! recover - bounded recovery: reconnect/refresh/rediscover only. Never
//!           restarts host infrastructure and never fabricates stream
//!           health. Reports exactly what a fresh observation shows.
//!
//! Secrets policy (directive O): all URLs pass through `redact_url`;
//! stdout/stderr must never contain RTSP passwords, bearer tokens,
//! query secrets, or camera media contents. The canary test asserts
//! zero occurrence of the actual secret values.
//!
//! Env: `FRIGATE_BASE_URL` (required) and optional `FRIGATE_TOKEN`.

use std::env;

use nexus_frigate::{FrigateAdapter, FrigateTransport, RestTransport};
use nexus_vision::provider::CameraProvider;

const BASE_URL_ENV: &str = "FRIGATE_BASE_URL";
const TOKEN_ENV: &str = "FRIGATE_TOKEN";

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str).unwrap_or("status");
    let base_url = match env::var(BASE_URL_ENV) {
        Ok(v) => v,
        Err(_) => {
            eprintln!("{BASE_URL_ENV} is required");
            std::process::exit(2);
        }
    };
    let mut transport =
        RestTransport::new(base_url).with_timeout(std::time::Duration::from_secs(5));
    if let Ok(token) = env::var(TOKEN_ENV) {
        if !token.is_empty() {
            transport = transport.with_token(token);
        }
    }
    match command {
        "status" => print_status(transport),
        "recover" => print_recovery(transport),
        other => {
            eprintln!("usage: frigate-diag (status|recover); got: {other}");
            std::process::exit(2);
        }
    }
}

fn print_status(transport: RestTransport) {
    let out = diag(transport, false);
    println!("{out}");
}

fn print_recovery(transport: RestTransport) {
    let out = diag(transport, true);
    println!("{out}");
}

/// Run the real diagnostic probe. `recover` adds the bounded recovery
/// note; the observation is identical (refresh/rediscover is the only
/// recovery EP-023 owns - never infrastructure restart).
fn diag(transport: RestTransport, recover: bool) -> serde_json::Value {
    let adapter = FrigateAdapter::new(transport);
    let mut value = serde_json::json!({
        "tool": "frigate-diag",
        "mode": if recover { "recover" } else { "status" },
    });

    match adapter.health() {
        Ok(()) => {
            value["provider_reachable"] = serde_json::Value::Bool(true);
        }
        Err(error) => {
            value["provider_reachable"] = serde_json::Value::Bool(false);
            value["error_code"] = serde_json::Value::String(error.code.as_str().to_string());
            value["error"] = serde_json::Value::String(redact(&error.message));
            if let Some(cid) = &error.correlation_id {
                value["correlation_id"] = serde_json::Value::String(cid.to_string());
            }
            value["action"] = serde_json::Value::String(
                "provider unreachable; fail closed; retry after connectivity restored".to_string(),
            );
            return value;
        }
    }

    match adapter.version() {
        Ok(version) => {
            value["frigate_version"] = serde_json::Value::String(redact(&version));
        }
        Err(error) => {
            value["frigate_version"] =
                serde_json::Value::String(format!("unavailable: {}", error.code.as_str()));
        }
    }

    match adapter.list_cameras() {
        Ok(cameras) => {
            value["camera_count"] = serde_json::Value::from(cameras.len());
            let mut live = 0usize;
            let mut degraded = 0usize;
            let mut unavailable = 0usize;
            for camera in &cameras {
                match adapter.availability(camera) {
                    Ok(avail) => {
                        let s = avail.as_str();
                        if s == "STREAMING" {
                            live += 1;
                        } else if s == "DEGRADED" {
                            degraded += 1;
                        } else if s == "UNAVAILABLE" {
                            unavailable += 1;
                        }
                    }
                    Err(_) => unavailable += 1,
                }
            }
            value["streams_live"] = serde_json::Value::from(live);
            value["streams_degraded"] = serde_json::Value::from(degraded);
            value["streams_unavailable"] = serde_json::Value::from(unavailable);
            value["camera_names"] = serde_json::Value::Array(
                cameras
                    .iter()
                    .map(|c| serde_json::Value::String(c.as_str().to_string()))
                    .collect(),
            );
        }
        Err(error) => {
            value["camera_count"] = serde_json::Value::from(0);
            value["error_code"] = serde_json::Value::String(error.code.as_str().to_string());
            value["error"] = serde_json::Value::String(redact(&error.message));
        }
    }

    value["metrics"] = adapter.metrics();

    // go2rtc availability via the real transport stream list (after
    // the metrics snapshot; into_inner consumes the adapter).
    let mut transport = adapter.into_inner();
    match transport.go2rtc_streams() {
        Ok(streams) => {
            value["go2rtc_available"] = serde_json::Value::Bool(true);
            value["go2rtc_stream_count"] = serde_json::Value::from(streams.len());
        }
        Err(error) => {
            value["go2rtc_available"] = serde_json::Value::Bool(false);
            value["go2rtc_error_code"] = serde_json::Value::String(error.code.as_str().to_string());
        }
    }

    value["action"] = serde_json::Value::String(match recover {
        true => "bounded recovery: fresh observation performed; no infrastructure restart owned"
            .to_string(),
        false => "diagnostic only".to_string(),
    });
    value
}

fn redact(text: &str) -> String {
    nexus_frigate::redact_url(text)
}
