//! EP-044 M3 integration: real control-plane binary over real HTTP.
//!
//! Spawns the actual `nexus-control-plane` binary as a child process,
//! drives real HTTP/1.1 requests over `TcpStream`, asserts the
//! canonical response shapes, and terminates it gracefully. No mocks,
//! no placeholders: the process under test is the production binary.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// Serializes port-using tests (bind-once doctrine; see failure suite).
fn port_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn pick_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    port
}

fn start_server() -> (Child, u16) {
    let port = pick_port();
    let child = Command::new(env!("CARGO_BIN_EXE_nexus-control-plane"))
        .env("NEXUS_CONTROL_PLANE_BIND", format!("127.0.0.1:{port}"))
        .env("NEXUS_BASE_DOMAIN", "nexus.test")
        .env("NEXUS_TENANT_ID", "018f0f6f-9c1e-7b6e-8000-000000000001")
        .env("NEXUS_CAPABILITY_SOURCE", "core")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn control-plane binary");
    // Wait until the listener is accepting.
    let mut up = false;
    for _ in 0..60 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            up = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(up, "control-plane binary did not start listening");
    (child, port)
}

fn http_get(port: u16, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: nexus.test\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut buf = String::new();
    stream.read_to_string(&mut buf).unwrap();
    let status_line = buf.lines().next().unwrap_or_default().to_string();
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("000")
        .parse()
        .unwrap_or(0);
    (status, buf)
}

#[test]
fn ep044_integration_healthz_returns_healthy() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    let (status, body) = http_get(port, "/healthz");
    assert_eq!(status, 200);
    assert!(body.contains(r#"{"status":"healthy"}"#), "body: {body}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_readyz_returns_ready() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    let (status, body) = http_get(port, "/readyz");
    assert_eq!(status, 200);
    assert!(body.contains(r#"{"ready":true}"#), "body: {body}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_capabilities_non_empty() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    let (status, body) = http_get(port, "/v1/capabilities");
    assert_eq!(status, 200);
    assert!(
        body.contains(r#""capabilities":["#) && body.contains("health"),
        "body: {body}"
    );
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_unknown_route_not_found() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    let (status, _body) = http_get(port, "/v1/nope");
    assert_eq!(status, 404);
    let _ = child.kill();
    let _ = child.wait();
}

// Compile-time bound: the server state must satisfy axum's State
// extractor requirements (Clone + Send + Sync + 'static).
#[test]
fn ep044_integration_server_state_bounds() {
    fn assert_bounds<T: Clone + Send + Sync + 'static>() {}
    assert_bounds::<nexus_control_plane::composition::RuntimeComposition>();
    assert_bounds::<nexus_control_plane::telemetry::RuntimeTelemetry>();
}

// ---------------------------------------------------------------------------
// RX-008 AUD-083/AUD-084 live-fire: the composed SPEC-003 surfaces must be
// reachable over real HTTP through the real binary, and the runtime must
// emit the startup telemetry line without leaking the tenant id.
// ---------------------------------------------------------------------------

fn http_post(port: u16, path: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: nexus.test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut buf = String::new();
    stream.read_to_string(&mut buf).unwrap();
    let status: u16 = buf
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .nth(1)
        .unwrap_or("000")
        .parse()
        .unwrap_or(0);
    (status, buf)
}

#[test]
fn ep044_integration_discover_returns_composed_capabilities() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    let (status, body) = http_get(port, "/v1/discover");
    assert_eq!(status, 200, "body: {body}");
    assert!(body.contains("runtime.health"), "body: {body}");
    assert!(body.contains("runtime.capabilities"), "body: {body}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_mcp_surface_live() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    // Initialize an MCP session through the real engine.
    let (status, body) = http_post(
        port,
        "/v1/mcp/initialize",
        r#"{"session_id":"sess-live-1"}"#,
    );
    assert_eq!(status, 200, "body: {body}");
    assert!(body.contains("2025-11-25"), "body: {body}");
    // List tools for the session.
    let (status, body) = http_post(port, "/v1/mcp/tools", r#"{"session_id":"sess-live-1"}"#);
    assert_eq!(status, 200, "body: {body}");
    assert!(body.contains("runtime.health"), "body: {body}");
    // Call the real health tool through the real engine.
    let (status, body) = http_post(
        port,
        "/v1/mcp/call",
        r#"{"session_id":"sess-live-1","call_id":"call-1","tool":"runtime.health","arguments":{}}"#,
    );
    assert_eq!(status, 200, "body: {body}");
    assert!(body.contains("\"healthy\""), "body: {body}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_a2a_surface_live() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    let submit = r#"{"task_id":"task-live-1","tenant_id":"018f0f6f-9c1e-7b6e-8000-000000000001","principal_id":"p-live","messages":[{"message_id":"req-1","role":"user","parts":[{"text":"run"}]}]}"#;
    let (status, body) = http_post(port, "/v1/a2a/tasks", submit);
    assert_eq!(status, 200, "body: {body}");
    let (status, body) = http_post(port, "/v1/a2a/tasks/task-live-1/run", "{}");
    assert_eq!(status, 200, "body: {body}");
    assert!(body.contains("task-live-1"), "body: {body}");
    let (status, body) = http_get(port, "/v1/a2a/tasks/task-live-1/stream");
    assert_eq!(status, 200, "body: {body}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_artifact_surface_live() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    let (status, body) = http_post(
        port,
        "/v1/artifacts",
        r#"{"content":"hello-rx008","content_type":"text/plain"}"#,
    );
    assert_eq!(status, 200, "body: {body}");
    // sha256("hello-rx008") is deterministic; the id is hash-bound.
    let id = body
        .split("\"artifact_id\":\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap_or("")
        .to_string();
    assert!(id.starts_with("sha256:"), "body: {body}");
    let (status, body) = http_get(port, &format!("/v1/artifacts/{id}"));
    assert_eq!(status, 200, "body: {body}");
    assert!(body.contains(&id), "body: {body}");
    // A fabricated id fails closed with 404.
    let (status, _) = http_get(port, &format!("/v1/artifacts/sha256:{}", "0".repeat(64)));
    assert_eq!(status, 404);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_event_surface_live() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    let event = r#"{"event_id":"0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073","event_type":"runtime.started","tenant_id":"018f0f6f-9c1e-7b6e-8000-000000000001","payload":{"state":"starting"}}"#;
    let (status, body) = http_post(port, "/v1/events", event);
    assert_eq!(status, 200, "body: {body}");
    assert!(body.contains("PENDING"), "body: {body}");
    let (status, body) = http_get(port, "/v1/events/pending");
    assert_eq!(status, 200, "body: {body}");
    assert!(body.contains("outbox-"), "body: {body}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_telemetry_startup_surface_live() {
    let _guard = port_lock().lock().unwrap();
    let (mut child, port) = start_server();
    let (status, body) = http_get(port, "/v1/telemetry/startup");
    assert_eq!(status, 200, "body: {body}");
    assert!(body.contains("nexus-control-plane"), "body: {body}");
    assert!(body.contains("startup"), "body: {body}");
    // AUD-083: the tenant id must never appear in emitted telemetry.
    assert!(
        !body.contains("018f0f6f-9c1e-7b6e-8000-000000000001"),
        "tenant leaked into telemetry: {body}"
    );
    let _ = child.kill();
    let _ = child.wait();
}
