//! EP-044 M3 integration: real control-plane binary over real HTTP.
//!
//! Spawns the actual `nexus-control-plane` binary as a child process,
//! drives real HTTP/1.1 requests over `TcpStream`, asserts the
//! canonical response shapes, and terminates it gracefully. No mocks,
//! no placeholders: the process under test is the production binary.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

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
    let (mut child, port) = start_server();
    let (status, body) = http_get(port, "/healthz");
    assert_eq!(status, 200);
    assert!(body.contains(r#"{"status":"healthy"}"#), "body: {body}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_readyz_returns_ready() {
    let (mut child, port) = start_server();
    let (status, body) = http_get(port, "/readyz");
    assert_eq!(status, 200);
    assert!(body.contains(r#"{"ready":true}"#), "body: {body}");
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep044_integration_capabilities_non_empty() {
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
    let (mut child, port) = start_server();
    let (status, _body) = http_get(port, "/v1/nope");
    assert_eq!(status, 404);
    let _ = child.kill();
    let _ = child.wait();
}
