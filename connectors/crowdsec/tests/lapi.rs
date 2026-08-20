//! EP-031 CrowdSec real-socket integration tests (M3).
//!
//! The production HTTP transport (HttpCrowdSecTransport) is exercised
//! over REAL std::net sockets against controlled local fixtures
//! emitting REAL CrowdSec LAPI-shaped responses (documented surface:
//! POST /v1/watchers/login -> {"token": ...}; GET /v1/decisions?ip=
//! -> {"decisions": [...]}). Mocks control the PEER only; the
//! transport under test is never mocked.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use nexus_crowdsec_connector::{CrowdSecTransport, HttpCrowdSecTransport};
use nexus_sentinel::SentinelErrorCode;

fn read_until_blank_line(stream: &mut TcpStream) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => break,
            Ok(_) => {
                buf.push(byte[0]);
                if buf.ends_with(b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    buf
}

fn parse_request_line(head: &[u8]) -> (String, String) {
    let text = String::from_utf8_lossy(head);
    let line = text.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    (method, path)
}

/// Spawn a one-shot LAPI fixture that handles login then decisions.
fn spawn_lapi(login_status: u16, decisions_body: String) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        // Connection 1: watcher login.
        let (mut c1, _) = listener.accept().expect("accept login");
        let _ = read_until_blank_line(&mut c1);
        let login_body = if login_status == 200 {
            "{\"code\":200,\"token\":\"fake-jwt-token\"}"
        } else {
            "{\"code\":401,\"message\":\"bad credentials\"}"
        };
        let resp = format!(
            "HTTP/1.1 {login_status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{login_body}",
            login_body.len()
        );
        let _ = c1.write_all(resp.as_bytes());
        let _ = c1.flush();

        if login_status == 200 {
            // Connection 2: decisions query.
            let (mut c2, _) = listener.accept().expect("accept decisions");
            let head = read_until_blank_line(&mut c2);
            let (method, path) = parse_request_line(&head);
            let status = if method == "GET" && path.starts_with("/v1/decisions") {
                200
            } else {
                404
            };
            let body = if status == 200 {
                decisions_body.as_str()
            } else {
                "{}"
            };
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = c2.write_all(resp.as_bytes());
            let _ = c2.flush();
        }
    });
    (port, handle)
}

#[test]
fn ep031_integration_crowdsec_lapi_full_login_and_ban_decision_over_real_socket() {
    let body = r#"{"decisions":[{"id":1,"origin":"cscli","type":"ban","scope":"Ip","value":"1.2.3.4","duration":"4h0m0s","scenario":"ssh-bf","action":"ban","created_at":"2026-08-20T00:00:00Z"}]}"#;
    let (port, handle) = spawn_lapi(200, body.to_string());
    let mut transport = HttpCrowdSecTransport::new(
        format!("http://127.0.0.1:{port}"),
        "machine-1",
        "password-1",
        Duration::from_secs(5),
    );
    let decisions = transport.decisions_for("1.2.3.4").expect("decisions");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].action, "ban");
    assert_eq!(decisions[0].value, "1.2.3.4");
    assert_eq!(decisions[0].scenario, "ssh-bf");
    let _ = handle.join();
}

#[test]
fn ep031_integration_crowdsec_lapi_clean_reputation_returns_empty() {
    let body = r#"{"decisions":[]}"#;
    let (port, handle) = spawn_lapi(200, body.to_string());
    let mut transport = HttpCrowdSecTransport::new(
        format!("http://127.0.0.1:{port}"),
        "machine-1",
        "password-1",
        Duration::from_secs(5),
    );
    let decisions = transport.decisions_for("192.0.2.99").expect("decisions");
    assert!(
        decisions.is_empty(),
        "clean reputation is absence of evidence"
    );
    let _ = handle.join();
}

#[test]
fn ep031_integration_crowdsec_lapi_login_rejected_fails_closed() {
    let (port, handle) = spawn_lapi(401, "{}".to_string());
    let mut transport = HttpCrowdSecTransport::new(
        format!("http://127.0.0.1:{port}"),
        "machine-bad",
        "password-bad",
        Duration::from_secs(5),
    );
    let err = transport
        .decisions_for("1.2.3.4")
        .expect_err("login rejected");
    assert_eq!(err.code, SentinelErrorCode::Authorization);
    let _ = handle.join();
}

#[test]
fn ep031_integration_crowdsec_lapi_unreachable_fails_closed() {
    // Bind then drop: the port is refused, never a fabricated
    // reputation verdict.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let mut transport = HttpCrowdSecTransport::new(
        format!("http://127.0.0.1:{port}"),
        "machine-1",
        "password-1",
        Duration::from_secs(2),
    );
    let err = transport.decisions_for("1.2.3.4").expect_err("refused");
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
}
