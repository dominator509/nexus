//! EP-028 Hydra transport integration tests (M3).
//!
//! The production HTTP transport under test is REAL
//! (`HttpHydraTransport`, reqwest blocking). The peer is a controlled
//! local HTTP fixture over REAL std::net sockets that emits REAL
//! Hydra-shaped responses (the versioned canonical surface from
//! schemas/hydra/): 200 JSON context, 200 capability ads, 200 action
//! envelope, 401, 404, 409, 429, 5xx, malformed JSON, silent peer.
//! Mocks control the peer only; the transport is never mocked.
//!
//! These tests also validate the REAL Rust serialization output
//! against the REAL canonical JSON Schemas under schemas/hydra/ using
//! the REAL JSON Schema 2020-12 validator (jsonschema crate), proving
//! the Rust serde surface and the cross-language schema contract do
//! not drift.
//!
//! Certification boundary: these fixtures prove request construction,
//! response/status semantics, classification, and failure behavior
//! over real HTTP. They NEVER certify a real Hydra/CRM provider; real
//! provider certification is DEFERRED (no Hydra component is selected
//! in COMPONENT_REGISTRY; Postiz is EP-029's node).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use nexus_hydra::{BusinessContext, HydraCapabilityMap, HydraContextProjection, HydraErrorCode};
use nexus_hydra_connector::{HttpHydraTransport, HydraTransport};

const CANARY_TOKEN: &str = "EP028PW_CANARY_8b41";

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

fn spawn_server<F>(handler: F) -> (u16, JoinHandle<()>)
where
    F: Fn(&str, &str) -> (u16, &'static str, String) + Send + 'static,
{
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
        let (method, path) = parse_request_line(&head);
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

fn spawn_silent_server() -> (u16, JoinHandle<()>) {
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
        // Accept the connection but never respond, and keep the socket
        // OPEN past the transport's timeout (a true silent peer; a
        // closed socket would surface as ExternalProvider instead of
        // Timeout).
        let mut buf = [0u8; 1];
        let _ = stream.read(&mut buf);
        thread::sleep(Duration::from_secs(5));
    });
    (port, handle)
}

/// Server that accepts MULTIPLE sequential connections and dispatches
/// each request to the handler (needed for submit-then-readback).
fn spawn_multi_server<F>(handler: F) -> (u16, JoinHandle<()>)
where
    F: Fn(&str, &str) -> (u16, &'static str, String) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        for _ in 0..2 {
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
            let (method, path) = parse_request_line(&head);
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

fn context() -> BusinessContext {
    BusinessContext::single(
        nexus_domain::TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
        nexus_domain::PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        nexus_domain::BusinessId::from_str("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap(),
    )
}

const CONTEXT_JSON: &str = r#"{
  "binding_id": "binding-1",
  "business_id": "018f0f6f-9c1e-7b6e-8000-000000000003",
  "customers": [
    {
      "customer_reference_id": "cust-1",
      "business_id": "018f0f6f-9c1e-7b6e-8000-000000000003",
      "hydra_person_id": "018f0f6f-9c1e-7b6e-8000-000000000002",
      "resolution": "DETERMINISTIC"
    }
  ],
  "campaigns": [
    {
      "campaign_id": "camp-1",
      "business_id": "018f0f6f-9c1e-7b6e-8000-000000000003",
      "name": "Q3",
      "state": "ACTIVE"
    }
  ],
  "observed_at": "2026-08-19T00:00:00Z"
}"#;

#[test]
fn ep028_integration_read_context_real_http() {
    let (port, handle) = spawn_server(|method, path| {
        assert_eq!(method, "GET");
        assert_eq!(path, "/v1/context");
        (200, "application/json", CONTEXT_JSON.to_string())
    });
    let transport = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let projection = transport.read_context(&context()).expect("read context");
    assert_eq!(projection.customers.len(), 1);
    assert_eq!(projection.customers[0].resolution.as_str(), "DETERMINISTIC");
    assert_eq!(projection.campaigns.len(), 1);
    assert_eq!(projection.campaigns[0].name, "Q3");
    handle.join().unwrap();
}

#[test]
fn ep028_integration_capabilities_real_http() {
    let (port, handle) = spawn_server(|method, path| {
        assert_eq!(method, "GET");
        assert_eq!(path, "/v1/capabilities");
        (
            200,
            "application/json",
            r#"[{"kind":"READ_CONTEXT","available":true},{"kind":"EXECUTE_UPDATE","available":false},{"kind":"FABRICATED_KIND","available":true}]"#.to_string(),
        )
    });
    let transport = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let map = transport.capabilities().expect("capabilities");
    // READ_CONTEXT is advertised available; EXECUTE_UPDATE is
    // advertised but unavailable; a fabricated provider capability is
    // never advertised (fail closed - provider vocabulary cannot widen
    // the contract).
    assert!(map.is_available(nexus_hydra::HydraCapabilityKind::ReadContext));
    assert!(!map.is_available(nexus_hydra::HydraCapabilityKind::ExecuteUpdate));
    handle.join().unwrap();
}

#[test]
fn ep028_integration_submit_and_readback_real_http() {
    let (port, handle) = spawn_multi_server(|method, path| {
        if method == "POST" && path == "/v1/actions" {
            (
                200,
                "application/json",
                r#"{"action_id":"action-1","state":"SUBMITTED"}"#.to_string(),
            )
        } else if method == "GET" && path == "/v1/actions/action-1" {
            (
                200,
                "application/json",
                r#"{"action_id":"action-1","state":"EXECUTED"}"#.to_string(),
            )
        } else {
            (404, "application/json", r#"{}"#.to_string())
        }
    });
    let transport = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let payload = serde_json::json!({"action_id":"action-1"});
    let state = transport.submit_action(&payload).expect("submit");
    assert_eq!(state, "SUBMITTED");
    let readback = transport.read_action("action-1").expect("readback");
    assert_eq!(readback, "EXECUTED");
    handle.join().unwrap();
}

#[test]
fn ep028_integration_status_classification_real_http() {
    for (status, expected) in [
        (401, HydraErrorCode::Authorization),
        (403, HydraErrorCode::Authorization),
        (404, HydraErrorCode::NotFound),
        (409, HydraErrorCode::Conflict),
        (429, HydraErrorCode::RateLimit),
        (500, HydraErrorCode::Unavailable),
        (503, HydraErrorCode::Unavailable),
    ] {
        let (port, handle) =
            spawn_server(move |_method, _path| (status, "application/json", r#"{}"#.to_string()));
        let transport = HttpHydraTransport::new(
            format!("http://127.0.0.1:{port}"),
            CANARY_TOKEN,
            Duration::from_secs(5),
        );
        let err = transport.read_context(&context()).unwrap_err();
        assert_eq!(err.code, expected, "status {status}");
        handle.join().unwrap();
    }
}

#[test]
fn ep028_integration_malformed_json_fails_closed_real_http() {
    let (port, handle) =
        spawn_server(|_method, _path| (200, "application/json", "not-json{".to_string()));
    let transport = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let err = transport.read_context(&context()).unwrap_err();
    assert_eq!(err.code, HydraErrorCode::ExternalProvider);
    handle.join().unwrap();
}

#[test]
fn ep028_integration_silent_peer_times_out_real_http() {
    let (port, handle) = spawn_silent_server();
    let transport = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_millis(300),
    );
    let err = transport.read_context(&context()).unwrap_err();
    assert_eq!(err.code, HydraErrorCode::Timeout);
    handle.join().unwrap();
}

#[test]
fn ep028_integration_refused_port_unavailable_real_http() {
    // Bind a listener, learn the port, drop it, then connect: refused.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    let transport = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let err = transport.read_context(&context()).unwrap_err();
    assert_eq!(err.code, HydraErrorCode::Unavailable);
}

#[test]
fn ep028_integration_business_context_scope_validated_before_transport() {
    // A malformed single-business context (missing business id) must
    // fail at validation BEFORE any transport call, even against a
    // server that would otherwise answer.
    let (port, handle) =
        spawn_server(|_method, _path| (200, "application/json", CONTEXT_JSON.to_string()));
    let transport = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let bad = BusinessContext {
        tenant_id: context().tenant_id,
        principal_id: context().principal_id,
        scope: nexus_hydra::BusinessScope::SingleBusiness,
        business_id: None,
        correlation: None,
    };
    let err = transport.read_context(&bad).unwrap_err();
    assert_eq!(err.code, HydraErrorCode::Validation);
    handle.join().unwrap();
}

// ------------------------------------------------------------------
// Schema parity: real Rust serialization validates against the REAL
// canonical schemas under schemas/hydra/.
// ------------------------------------------------------------------

fn root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("connectors dir")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn load_schema(relative: &str) -> serde_json::Value {
    let path = root().join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read schema {relative}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("cannot parse schema {relative}: {e}"))
}

fn validator_for(schema: &serde_json::Value) -> jsonschema::Validator {
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .build(schema)
        .expect("canonical schema must compile")
}

#[test]
fn ep028_integration_context_projection_matches_canonical_schema() {
    let schema = load_schema("schemas/hydra/context-projection.schema.json");
    let validator = validator_for(&schema);
    let projection: HydraContextProjection =
        serde_json::from_str(CONTEXT_JSON).expect("canonical shape parses");
    let json = serde_json::to_value(&projection).expect("serialize");
    let result = validator.validate(&json);
    assert!(
        result.is_ok(),
        "real Rust serialization must validate against the canonical schema: {result:?}"
    );
}

#[test]
fn ep028_integration_capability_map_matches_canonical_schema() {
    let schema = load_schema("schemas/hydra/capability-map.schema.json");
    let validator = validator_for(&schema);
    let mut map = HydraCapabilityMap::new();
    map.advertise(
        nexus_hydra::HydraCapabilityKind::ReadContext,
        nexus_domain::Availability::Available,
    );
    let json = serde_json::to_value(&map).expect("serialize");
    let result = validator.validate(&json);
    assert!(
        result.is_ok(),
        "real capability map must validate against the canonical schema: {result:?}"
    );
}

#[test]
fn ep028_integration_action_request_matches_canonical_schema() {
    let schema = load_schema("schemas/hydra/action-request.schema.json");
    let validator = validator_for(&schema);
    let req = nexus_hydra::HydraActionRequest::new(
        nexus_hydra::HydraActionId::new("action-1").unwrap(),
        context().tenant_id,
        context().principal_id,
        context().business_id.expect("business"),
        nexus_hydra::HydraActionKind::ReadContext,
        "idempotency-key-0001",
    );
    let json = serde_json::to_value(&req).expect("serialize");
    let result = validator.validate(&json);
    assert!(
        result.is_ok(),
        "real action request must validate against the canonical schema: {result:?}"
    );
}
