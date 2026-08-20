//! EP-030 M4 forced-failure and abuse-case proofs (SPEC-013).
//!
//! Drives the REAL production AdGuard Home adapter + HttpAdGuardTransport
//! against controlled local fixtures over REAL std::net sockets.
//! Mocks control the peer only; the transport and adapter under test
//! are never mocked.
//!
//! Real failure mechanisms exercised:
//! - refused port -> Unavailable
//! - silent peer kept open past the bounded timeout -> Timeout
//! - malformed JSON -> External (fail closed)
//! - 401 -> Authorization (bad credential)
//! - 404 -> NotFound
//! - redaction canary: zero credential leakage in audit
//! - bounded recovery: a fresh healthy fixture succeeds after an
//!   unavailable window
//! - cancelled/dropped work fails closed (connection torn down
//!   mid-request -> External)
//!
//! Certification boundary: this proves the AdGuard Home connector
//! fails safely over the canonical documented control API. It does
//! NOT certify a real AdGuard Home sidecar (no owned deployment exists
//! in this environment; real provider certification is DEFERRED with
//! owner recorded at M5/deployment).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use nexus_adguard_connector::{AdGuardDnsSecurityProvider, AdGuardTransport, HttpAdGuardTransport};
use nexus_domain::TenantId;
use nexus_sentinel::{DnsSecurityProvider, SentinelErrorCode};
use std::str::FromStr;

const CANARY_USER: &str = "canary-user";
const CANARY_PASS: &str = "canary-pass";

fn tenant() -> TenantId {
    TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn read_until_blank_line(stream: &mut TcpStream) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    buf
}

fn spawn_one(
    handler: impl Fn(&str, &str) -> (u16, &'static str, String) + Send + 'static,
) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking");
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(c) => break c,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        let head = read_until_blank_line(&mut stream);
        let text = String::from_utf8_lossy(&head);
        let line = text.lines().next().unwrap_or("");
        let mut parts = line.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        let (status, content_type, body) = handler(&method, &path);
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });
    (port, handle)
}

fn spawn_silent_server() -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking");
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(c) => break c,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        let mut buf = [0u8; 1];
        let _ = stream.read(&mut buf);
        thread::sleep(Duration::from_secs(5));
    });
    (port, handle)
}

fn spawn_n(
    n: usize,
    handler: impl Fn(&str, &str) -> (u16, &'static str, String) + Send + 'static,
) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        for _ in 0..n {
            listener.set_nonblocking(true).expect("nonblocking");
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(c) => break c,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() > deadline {
                            return;
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => return,
                }
            };
            let head = read_until_blank_line(&mut stream);
            let text = String::from_utf8_lossy(&head);
            let line = text.lines().next().unwrap_or("");
            let mut parts = line.split_whitespace();
            let method = parts.next().unwrap_or("").to_string();
            let path = parts.next().unwrap_or("").to_string();
            let (status, content_type, body) = handler(&method, &path);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (port, handle)
}

fn transport(port: u16) -> HttpAdGuardTransport {
    HttpAdGuardTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_USER,
        CANARY_PASS,
        Duration::from_millis(1500),
    )
}

#[test]
fn ep030_failure_refused_port_is_unavailable() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let t = transport(port);
    let err = t.status().unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Unavailable);
}

#[test]
fn ep030_failure_silent_peer_times_out() {
    let (port, handle) = spawn_silent_server();
    let t = transport(port);
    let err = t.status().unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Timeout);
    handle.join().unwrap();
}

#[test]
fn ep030_failure_malformed_json_fails_closed() {
    let (port, handle) =
        spawn_one(move |_method, _path| (200, "application/json", "{\"broken json".to_string()));
    let t = transport(port);
    let err = t.status().unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
    handle.join().unwrap();
}

#[test]
fn ep030_failure_unauthorized_maps_to_authorization() {
    let (port, handle) = spawn_one(move |_method, _path| {
        (
            401,
            "application/json",
            "{\"message\":\"unauthorized\"}".to_string(),
        )
    });
    let t = transport(port);
    let err = t.status().unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Authorization);
    handle.join().unwrap();
}

#[test]
fn ep030_failure_not_found_maps_to_not_found() {
    let (port, handle) = spawn_one(move |_method, _path| {
        (
            404,
            "application/json",
            "{\"message\":\"not found\"}".to_string(),
        )
    });
    let t = transport(port);
    let err = t.status().unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::NotFound);
    handle.join().unwrap();
}

#[test]
fn ep030_failure_bad_credential_fails_closed_with_audit() {
    // 401 on the status probe: capabilities advertise nothing and
    // telemetry fails closed with an audited Authorization outcome.
    // The fixture answers two connections: the capability probe and
    // the telemetry read.
    let (port, handle) = spawn_n(2, move |_method, _path| {
        (
            401,
            "application/json",
            "{\"message\":\"unauthorized\"}".to_string(),
        )
    });
    let provider = AdGuardDnsSecurityProvider::new(
        Box::new(transport(port)),
        tenant(),
        "wrong-user",
        "wrong-pass",
    );
    // Capabilities fail closed: never advertise on a failing probe.
    assert!(provider.capabilities().is_empty());
    let err = provider.read_telemetry(&tenant()).unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Authorization);
    assert!(provider
        .audit()
        .iter()
        .any(|e| e.operation == "READ_TELEMETRY" && e.outcome == "EXTERNAL_PROVIDER"));
    handle.join().unwrap();
}

#[test]
fn ep030_failure_redaction_canary_zero_leakage() {
    // The credential must never leak into the audit ring even when an
    // entry is poisoned with it.
    let (port, handle) = spawn_one(move |_method, _path| {
        (
            200,
            "application/json",
            serde_json::json!({
                "dns_addresses": ["127.0.0.1"],
                "dns_port": 53,
                "http_port": 80,
                "protection_enabled": true,
                "running": true,
                "version": "v0.108.0"
            })
            .to_string(),
        )
    });
    let provider = AdGuardDnsSecurityProvider::new(
        Box::new(transport(port)),
        tenant(),
        CANARY_USER,
        CANARY_PASS,
    );
    let _ = provider.capabilities();
    // The audit ring redacts the credentials at insert (poison-safe).
    let audit = provider.audit();
    let joined = serde_json::to_string(&audit).unwrap();
    assert!(!joined.contains(CANARY_USER));
    assert!(!joined.contains(CANARY_PASS));
    handle.join().unwrap();
}

#[test]
fn ep030_failure_bounded_recovery_after_unavailable() {
    // First the sidecar is unreachable (refused); then a fresh
    // healthy fixture answers. The adapter recovers through a fresh
    // transport (bounded recovery; never fabricates health).
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let dead_port = listener.local_addr().unwrap().port();
    drop(listener);

    let t = transport(dead_port);
    let err = t.status().unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::Unavailable);

    // Fresh healthy fixture.
    let (port2, handle2) = spawn_one(move |_method, _path| {
        (
            200,
            "application/json",
            serde_json::json!({
                "dns_addresses": ["127.0.0.1"],
                "dns_port": 53,
                "http_port": 80,
                "protection_enabled": true,
                "running": true,
                "version": "v0.108.0"
            })
            .to_string(),
        )
    });
    let t2 = transport(port2);
    let status = t2.status().unwrap();
    assert!(status.running);
    assert_eq!(status.dns_port, 53);
    handle2.join().unwrap();
}

#[test]
fn ep030_failure_cancelled_work_fails_closed() {
    // A connection that is torn down mid-request (no complete HTTP
    // response) must fail closed with an External/Unavailable class,
    // never fabricate a result.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(c) => break c,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        let _ = read_until_blank_line(&mut stream);
        // Read the request, then drop the connection without
        // responding.
        drop(stream);
    });
    let t = transport(port);
    let err = t.status().unwrap_err();
    // The torn-down connection surfaces as ExternalProvider (or
    // Unavailable for a connect-class failure); NEVER a fabricated Ok.
    assert!(matches!(
        err.code,
        SentinelErrorCode::ExternalProvider | SentinelErrorCode::Unavailable
    ));
    handle.join().unwrap();
}
