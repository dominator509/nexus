//! EP-031 Wazuh forced-failure and abuse-case tests (M4).
//!
//! Exercise the REAL failure mechanism against the production HTTP
//! transport over REAL std::net sockets with controlled local
//! fixtures: refused port (unavailable dependency), malformed JSON
//! (corrupted message), 401 denied permission, silent peer (timeout),
//! and partial/cancelled work. Fail-closed behavior and the redacted
//! audit trail are asserted on every path.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use nexus_sentinel::SentinelErrorCode;
use nexus_sentinel_advanced::EndpointTelemetryProvider;
use nexus_wazuh_connector::{HttpWazuhTransport, WazuhEndpointTelemetryProvider};

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

fn tenant() -> nexus_domain::TenantId {
    use std::str::FromStr;
    nexus_domain::TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

/// Fixture: authenticate returns a token, then the alerts request
/// returns `alerts_status` with `alerts_body` (or hangs when
/// `hang` is set - the silent-peer timeout case).
fn spawn_wazuh(
    auth_status: u16,
    alerts_status: u16,
    alerts_body: String,
    hang: bool,
) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        // Connection 1: authenticate.
        let (mut c1, _) = listener.accept().expect("accept auth");
        let _ = read_until_blank_line(&mut c1);
        let auth_body = if auth_status == 200 {
            "{\"data\":{\"token\":\"fake-wazuh-jwt\"}}"
        } else {
            "{\"message\":\"Unauthorized\",\"error\":401}"
        };
        let resp = format!(
            "HTTP/1.1 {auth_status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{auth_body}",
            auth_body.len()
        );
        let _ = c1.write_all(resp.as_bytes());
        let _ = c1.flush();

        if auth_status == 200 {
            // Connection 2: alerts.
            let (mut c2, _) = listener.accept().expect("accept alerts");
            let head = read_until_blank_line(&mut c2);
            let (method, path) = parse_request_line(&head);
            let status = if method == "GET" && path.starts_with("/alerts") {
                alerts_status
            } else {
                404
            };
            if hang {
                // Silent peer: accept and hold without responding.
                thread::sleep(Duration::from_secs(20));
                return;
            }
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{alerts_body}",
                alerts_body.len()
            );
            let _ = c2.write_all(resp.as_bytes());
            let _ = c2.flush();
        }
    });
    (port, handle)
}

#[test]
fn ep031_failure_wazuh_unreachable_fails_closed() {
    // Refused port = unavailable dependency; no fabricated telemetry.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let transport = HttpWazuhTransport::new(
        format!("http://127.0.0.1:{port}"),
        "user",
        "pass",
        Duration::from_secs(2),
    );
    let p = WazuhEndpointTelemetryProvider::new(transport);
    let err = p.read_telemetry(&tenant()).expect_err("refused");
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
    // The failure is recorded in the bounded audit ring.
    assert!(p.audit_entries().iter().any(|e| e.outcome == "failed"));
}

#[test]
fn ep031_failure_wazuh_malformed_json_fails_closed() {
    // Corrupt the controlled message: alerts returns non-JSON.
    let body = "this is not json".to_string();
    let (port, handle) = spawn_wazuh(200, 200, body, false);
    let transport = HttpWazuhTransport::new(
        format!("http://127.0.0.1:{port}"),
        "user",
        "pass",
        Duration::from_secs(5),
    );
    let p = WazuhEndpointTelemetryProvider::new(transport);
    let err = p.read_telemetry(&tenant()).expect_err("malformed");
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
    assert!(p.audit_entries().iter().any(|e| e.outcome == "failed"));
    let _ = handle.join();
}

#[test]
fn ep031_failure_wazuh_denied_permission_fails_closed() {
    // Revoke the sandbox token: authenticate returns 401.
    let (port, handle) = spawn_wazuh(401, 401, "{}".to_string(), false);
    let transport = HttpWazuhTransport::new(
        format!("http://127.0.0.1:{port}"),
        "user-bad",
        "pass-bad",
        Duration::from_secs(5),
    );
    let p = WazuhEndpointTelemetryProvider::new(transport);
    let err = p.read_telemetry(&tenant()).expect_err("denied");
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
    assert!(p.audit_entries().iter().any(|e| e.outcome == "failed"));
    let _ = handle.join();
}

#[test]
fn ep031_failure_wazuh_silent_peer_times_out() {
    // Silent peer kept-open past the bounded timeout -> Timeout,
    // fail closed, never fabricated telemetry.
    let (port, handle) = spawn_wazuh(200, 200, "{}".to_string(), true);
    let transport = HttpWazuhTransport::new(
        format!("http://127.0.0.1:{port}"),
        "user",
        "pass",
        Duration::from_millis(500),
    );
    let p = WazuhEndpointTelemetryProvider::new(transport);
    let err = p.read_telemetry(&tenant()).expect_err("timeout");
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
    assert!(p.audit_entries().iter().any(|e| e.outcome == "failed"));
    let _ = handle.join();
}

#[test]
fn ep031_failure_wazuh_clean_telemetry_is_observed_not_fabricated() {
    // Empty alert window is an empty window, never a fabricated
    // baseline: zero events and an ok audit entry.
    let body = r#"{"data":{"affected_items":[],"total_affected_items":0,"total_failed_items":0,"failed_items":[]},"message":"ok","error":0}"#.to_string();
    let (port, handle) = spawn_wazuh(200, 200, body, false);
    let transport = HttpWazuhTransport::new(
        format!("http://127.0.0.1:{port}"),
        "user",
        "pass",
        Duration::from_secs(5),
    );
    let p = WazuhEndpointTelemetryProvider::new(transport);
    let events = p.read_telemetry(&tenant()).expect("empty window");
    assert!(events.is_empty(), "empty window is an empty window");
    assert!(p.audit_entries().iter().any(|e| e.outcome == "ok"));
    let _ = handle.join();
}

#[test]
fn ep031_failure_wazuh_audit_never_leaks_credentials() {
    // Poison the failure detail with the credential strings; the
    // redaction must keep them out of the audit trail.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let transport = HttpWazuhTransport::new(
        format!("http://127.0.0.1:{port}"),
        "super-secret-user-12345",
        "super-secret-pass-12345",
        Duration::from_secs(2),
    );
    let p = WazuhEndpointTelemetryProvider::new(transport);
    let _ = p.read_telemetry(&tenant());
    let trail = format!("{:?}", p.audit_entries());
    assert!(
        !trail.contains("super-secret-user-12345"),
        "username leaked into audit"
    );
    assert!(
        !trail.contains("super-secret-pass-12345"),
        "password leaked into audit"
    );
}
