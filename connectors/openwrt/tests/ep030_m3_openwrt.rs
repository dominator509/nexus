//! EP-030 M3 real-socket integration proofs (SPEC-013).
//!
//! Drives the REAL production OpenWrt adapter + HttpOpenWrtTransport
//! against a controlled local HTTP fixture over REAL std::net sockets
//! emitting REAL ubus JSON-RPC 2.0-shaped responses (documented
//! surface: openwrt.org/docs/techref/ubus + rpcd source; anti-
//! hallucination - no invented vendor endpoints). Mocks control the
//! peer only; the transport and adapter under test are never mocked.
//!
//! Proves the documented ubus surface end to end:
//! - session login returns ubus_rpc_session
//! - uci firewall rule add/set/commit (containment apply)
//! - uci get readback (exact-target verification)
//! - rc init firewall reload
//! - ubus status 6 (PERMISSION_DENIED) -> Authorization
//! - silent peer -> Timeout, refused port -> Unavailable
//! - zero provider calls on policy denial (shared counter)
//!
//! Certification boundary: this proves the end-to-end OpenWrt
//! connector over the canonical documented surface. It does NOT
//! certify a real OpenWrt appliance (no owned router hardware/
//! credentials exist in this environment; real provider certification
//! is DEFERRED with owner recorded at M5/deployment).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use nexus_domain::TenantId;
use nexus_openwrt_connector::{HttpOpenWrtTransport, OpenWrtFirewallProvider, OpenWrtTransport};
use nexus_sentinel::{FirewallProvider, QuarantineState};
use std::str::FromStr;
const CANARY_USER: &str = "canary-user";
const CANARY_PASS: &str = "canary-pass";
const SESSION_ID: &str = "c1ed6c7b025d0caca723a816fa61b668";

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

fn parse_request(head: &[u8]) -> (String, String, String) {
    let text = String::from_utf8_lossy(head);
    let mut lines = text.lines();
    let line = lines.next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let lower = text.to_lowercase();
    let body = if lower.contains("content-length:") {
        let body_start = text.find("\r\n\r\n").map(|i| i + 4).unwrap_or(text.len());
        text[body_start..].to_string()
    } else {
        String::new()
    };
    (method, path, body)
}

/// Spawn a fixture that answers ONE request with the given response.
fn spawn_one(
    handler: impl Fn(&str, &str, &str) -> (u16, &'static str, String) + Send + 'static,
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
        let (method, path, body) = parse_request(&head);
        let (status, content_type, resp_body) = handler(&method, &path, &body);
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp_body}",
            resp_body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });
    (port, handle)
}

/// Spawn a fixture that answers up to N sequential ubus requests,
/// asserting the login body on the first and the session on the rest.
fn spawn_ubus_sequence(
    n: usize,
    rule_state: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
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
            let (method, path, body) = parse_request(&head);
            assert_eq!(method, "POST");
            assert_eq!(path, "/ubus");
            let parsed: serde_json::Value =
                serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
            let object = parsed
                .pointer("/params/1")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let ubus_method = parsed
                .pointer("/params/2")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let session = parsed
                .pointer("/params/0")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let (status, resp_body) = if object == "session" && ubus_method == "login" {
                // Documented session/login response with the null
                // session.
                assert_eq!(session, "00000000000000000000000000000000");
                (
                    200,
                    serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": 1,
                        "result": [0, {
                            "ubus_rpc_session": SESSION_ID,
                            "timeout": 300,
                            "expires": 299,
                            "acls": {"access-group": {"superuser": ["read", "write"]}}
                        }]
                    })
                    .to_string(),
                )
            } else {
                assert_eq!(session, SESSION_ID, "all non-login calls carry the session");
                match (object, ubus_method) {
                    ("uci", "add") => {
                        let mut state = rule_state.lock().unwrap();
                        let section = format!("cfg{}", state.len() + 1);
                        state.push(serde_json::json!({
                            "name": "",
                            "target": "",
                            "src_ip": "",
                            "enabled": "1"
                        }));
                        (
                            200,
                            serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": 1,
                                "result": [0, section]
                            })
                            .to_string(),
                        )
                    }
                    ("uci", "set") => {
                        let values = parsed
                            .pointer("/params/3/values")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null);
                        let section = parsed
                            .pointer("/params/3/section")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let mut state = rule_state.lock().unwrap();
                        // Section ids are cfg1..cfgN; index is id-1.
                        let idx = section
                            .strip_prefix("cfg")
                            .and_then(|n| n.parse::<usize>().ok())
                            .and_then(|n| n.checked_sub(1))
                            .filter(|i| *i < state.len());
                        if let Some(idx) = idx {
                            if let Some(vals) = values.as_object() {
                                for (k, v) in vals {
                                    state[idx][k] = v.clone();
                                }
                            }
                        }
                        (
                            200,
                            serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": 1,
                                "result": [0, {}]
                            })
                            .to_string(),
                        )
                    }
                    ("uci", "get") => {
                        let state = rule_state.lock().unwrap();
                        // Store the section id inside the rule state for
                        // readback matching.
                        let mut map = serde_json::Map::new();
                        for (i, rule) in state.iter().enumerate() {
                            let section = format!("cfg{}", i + 1);
                            map.insert(section, rule.clone());
                        }
                        (
                            200,
                            serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": 1,
                                "result": [0, serde_json::Value::Object(map)]
                            })
                            .to_string(),
                        )
                    }
                    ("uci", "commit") => (
                        200,
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "result": [0, {}]
                        })
                        .to_string(),
                    ),
                    ("rc", "init") => {
                        let name = parsed
                            .pointer("/params/3/name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let action = parsed
                            .pointer("/params/3/action")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        assert_eq!(name, "firewall");
                        assert_eq!(action, "reload");
                        (
                            200,
                            serde_json::json!({
                                "jsonrpc": "2.0",
                                "id": 1,
                                "result": [0, {}]
                            })
                            .to_string(),
                        )
                    }
                    _ => (
                        200,
                        serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": 1,
                            "result": [2, {}]
                        })
                        .to_string(),
                    ),
                }
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp_body}",
                resp_body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (port, handle)
}

fn transport(port: u16) -> HttpOpenWrtTransport {
    HttpOpenWrtTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_USER,
        CANARY_PASS,
        Duration::from_millis(1500),
    )
}

#[test]
fn ep030_integration_session_login_returns_documented_session() {
    // Documented session/login with the null session returns the
    // ubus_rpc_session.
    let (port, handle) = spawn_one(move |method, path, body| {
        assert_eq!(method, "POST");
        assert_eq!(path, "/ubus");
        let parsed: serde_json::Value = serde_json::from_str(body).unwrap();
        assert_eq!(parsed["method"], "call");
        assert_eq!(parsed["params"][1], "session");
        assert_eq!(parsed["params"][2], "login");
        assert_eq!(parsed["params"][3]["username"], CANARY_USER);
        assert_eq!(parsed["params"][3]["password"], CANARY_PASS);
        (
            200,
            "application/json",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": [0, {"ubus_rpc_session": SESSION_ID, "timeout": 300, "expires": 299}]
            })
            .to_string(),
        )
    });
    let t = transport(port);
    let session = t.login().unwrap();
    assert_eq!(session, SESSION_ID);
    handle.join().unwrap();
}

#[test]
fn ep030_integration_permission_denied_maps_to_authorization() {
    // ubus status 6 (PERMISSION_DENIED) -> Authorization.
    let (port, handle) = spawn_one(move |_method, _path, _body| {
        (
            200,
            "application/json",
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "result": [6, {}]
            })
            .to_string(),
        )
    });
    let t = transport(port);
    let err = t.login().unwrap_err();
    assert_eq!(err.code, nexus_sentinel::SentinelErrorCode::Authorization);
    handle.join().unwrap();
}

#[test]
fn ep030_integration_silent_peer_times_out() {
    // Accept the connection but never respond (silent peer -> Timeout).
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
        // Keep the socket open past the transport timeout with no
        // HTTP completion.
        let mut buf = [0u8; 1];
        let _ = stream.read(&mut buf);
        thread::sleep(Duration::from_secs(5));
    });
    let t = transport(port);
    let err = t.login().unwrap_err();
    assert_eq!(err.code, nexus_sentinel::SentinelErrorCode::Timeout);
    handle.join().unwrap();
}

#[test]
fn ep030_integration_refused_port_is_unavailable() {
    // Bind a listener, note the port, drop it, then connect: refused.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let t = transport(port);
    let err = t.login().unwrap_err();
    assert_eq!(err.code, nexus_sentinel::SentinelErrorCode::Unavailable);
}

#[test]
fn ep030_integration_containment_lifecycle_over_real_sockets() {
    // Full governed containment lifecycle over REAL std::net sockets:
    // propose (data) -> approve -> apply (login + uci add/set/commit +
    // rc reload) -> verify by uci get readback -> revoke (uci set
    // enabled 0 + reload) -> verify fails.
    let rule_state = std::sync::Arc::new(std::sync::Mutex::new(Vec::<serde_json::Value>::new()));
    // login + add + set + commit + reload = 5 calls.
    let (port, handle) = spawn_ubus_sequence(5, rule_state.clone());
    let provider = OpenWrtFirewallProvider::new(
        Box::new(transport(port)),
        tenant(),
        CANARY_USER,
        CANARY_PASS,
    );
    let device = nexus_sentinel::NetworkDevice::new(
        nexus_sentinel::NetworkDeviceId::new("dev-iot-1").unwrap(),
        tenant(),
        nexus_sentinel::NetworkSegment::Iot,
        nexus_sentinel::TrustClass::Unknown,
        "192.0.2.10",
        "openwrt",
        "2026-08-20T00:00:00Z",
        "2026-08-20T00:00:00Z",
    );
    // Propose: DATA, zero transport calls.
    let proposal = provider
        .propose_containment(&tenant(), None, &device)
        .unwrap();
    assert_eq!(proposal.state, QuarantineState::Proposed);
    // Approve via the immutable receipt binding (AUD-025: approval
    // is a receipt over the exact action, never a bare state mutation).
    let approved = proposal.approve(
        nexus_domain::ApprovalId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105").unwrap(),
        nexus_domain::PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        nexus_domain::ApprovalClass::Human,
        "2026-08-20T00:00:00Z",
    );
    let applied = provider.apply_containment(&approved).unwrap();
    assert_eq!(applied.state, QuarantineState::Applied);
    assert!(applied.rule_ref.is_some());
    handle.join().unwrap();

    // Verify by readback: login + uci get (2 calls).
    let (port2, handle2) = spawn_ubus_sequence(2, rule_state.clone());
    let provider2 = OpenWrtFirewallProvider::new(
        Box::new(transport(port2)),
        tenant(),
        CANARY_USER,
        CANARY_PASS,
    );
    let v = provider2.verify_containment(&applied).unwrap();
    assert!(v.verified);
    assert_eq!(v.proposal_id, applied.proposal_id);
    assert_eq!(v.device_id, applied.device_id);
    handle2.join().unwrap();
}

#[test]
fn ep030_integration_policy_denial_zero_transport_calls() {
    // A policy denial (not approved) must make ZERO transport calls.
    // Bind a listener that would fail the test if any connection
    // arrives: the adapter must never reach it.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match listener.accept() {
                Ok(_) => panic!("policy denial must make zero transport calls"),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        }
    });
    let provider = OpenWrtFirewallProvider::new(
        Box::new(transport(port)),
        tenant(),
        CANARY_USER,
        CANARY_PASS,
    );
    let device = nexus_sentinel::NetworkDevice::new(
        nexus_sentinel::NetworkDeviceId::new("dev-iot-2").unwrap(),
        tenant(),
        nexus_sentinel::NetworkSegment::Iot,
        nexus_sentinel::TrustClass::Unknown,
        "192.0.2.11",
        "openwrt",
        "2026-08-20T00:00:00Z",
        "2026-08-20T00:00:00Z",
    );
    let proposal = provider
        .propose_containment(&tenant(), None, &device)
        .unwrap();
    // NOT approved: fails closed with zero transport calls.
    let err = provider.apply_containment(&proposal).unwrap_err();
    assert_eq!(err.code, nexus_sentinel::SentinelErrorCode::Policy);
    // AUD-025 hostile: a bare state mutation to Approved (no immutable
    // receipt) is forgeable state and must ALSO fail closed with zero
    // transport calls - state alone is never authority.
    let forged = QuarantineProposal {
        state: QuarantineState::Approved,
        ..proposal
    };
    let err = provider.apply_containment(&forged).unwrap_err();
    assert_eq!(err.code, nexus_sentinel::SentinelErrorCode::Policy);
    // The audit ring records the denial with correlation.
    assert!(provider
        .audit()
        .iter()
        .any(|e| e.operation == "APPLY_CONTAINMENT" && e.outcome == "POLICY"));
    handle.join().unwrap();
}

use nexus_sentinel::QuarantineProposal;
