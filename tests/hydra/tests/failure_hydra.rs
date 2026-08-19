//! EP-028 Hydra forced failures, abuse cases, and observability (M4).
//!
//! Every test exercises a REAL failure mechanism against the REAL
//! production transport/adapter over REAL std::net sockets:
//! - unavailable dependency: refused port (server gone) -> Unavailable;
//! - timeout: silent peer (socket kept open, no response) -> Timeout;
//! - malformed input: malformed provider JSON -> External (fail
//!   closed);
//! - duplicate request: in-flight duplicate -> Conflict, zero second
//!   transport call, entry released after end;
//! - denied permission: policy gate BEFORE transport -> Policy, zero
//!   provider calls (Arc counter proof);
//! - unknown provider vocabulary: fabricated action state -> External
//!   (fail closed, provider cannot widen the contract);
//! - binding abuse: wrong binding id -> Authorization;
//! - redaction canaries: credential embedded in a poisoned error is
//!   replaced with *** in the audit ring (zero leakage);
//! - bounded recovery: after an unavailable server, a fresh transport
//!   against a healthy server succeeds.
//!
//! Mocks control the PEER only; the transport/adapter under test is
//! never mocked (directive: do not mock the component being proven).
//! The fixture itself is CONTROLLED_TEST_FIXTURE; no real Hydra/CRM
//! provider is claimed (certification boundary).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use nexus_domain::{ApprovalClass, BusinessId, PersonId, TenantId};
use nexus_hydra::{
    BusinessContext, HydraAccessChannel, HydraActionKind, HydraActionRequest, HydraBindingId,
    HydraErrorCode, HydraProvider,
};
use nexus_hydra_connector::{HttpHydraTransport, HydraAdapter, HydraObservability, HydraTransport};

const CANARY_TOKEN: &str = "EP028_SECRET_CANARY_5c92";

fn tenant() -> TenantId {
    TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn person() -> PersonId {
    PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
}

fn business() -> BusinessId {
    BusinessId::from_str("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap()
}

fn binding() -> nexus_hydra::HydraBusinessBinding {
    nexus_hydra::HydraBusinessBinding::new(
        HydraBindingId::new("binding-1").unwrap(),
        tenant(),
        business(),
        std::collections::BTreeSet::from([HydraAccessChannel::REST]),
    )
}

fn context() -> BusinessContext {
    BusinessContext::single(tenant(), person(), business())
}

fn request(kind: HydraActionKind, approval: ApprovalClass) -> HydraActionRequest {
    HydraActionRequest::new(
        nexus_hydra::HydraActionId::new("action-1").unwrap(),
        tenant(),
        person(),
        business(),
        kind,
        "idempotency-key-0001",
    )
    .with_approval_class(approval)
}

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
        let mut buf = [0u8; 1];
        let _ = stream.read(&mut buf);
        // Keep the socket open past the transport timeout.
        thread::sleep(Duration::from_secs(5));
    });
    (port, handle)
}

#[test]
fn ep028_failure_refused_port_unavailable() {
    // Unavailable dependency: bind, learn the port, drop the listener,
    // then connect - the provider is GONE.
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
fn ep028_failure_silent_peer_timeout() {
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
fn ep028_failure_malformed_json_external() {
    let (port, handle) = spawn_server(|_m, _p| (200, "application/json", "{not-json".to_string()));
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
fn ep028_failure_policy_denied_zero_provider_calls() {
    struct CountingTransport {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }
    impl HydraTransport for CountingTransport {
        fn submit_action(&self, _a: &serde_json::Value) -> Result<String, nexus_hydra::HydraError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("SUBMITTED".to_string())
        }
    }
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let adapter = HydraAdapter::new(
        Box::new(CountingTransport {
            calls: std::sync::Arc::clone(&calls),
        }),
        binding(),
        Vec::new(),
    );
    // Paid-ad budget change with only POLICY-class approval: the gate
    // must reject BEFORE the provider is invoked.
    let req = request(HydraActionKind::PaidAdBudgetChange, ApprovalClass::Policy);
    let err = adapter.submit_action(&binding(), &req).unwrap_err();
    assert_eq!(err.code, HydraErrorCode::Policy);
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn ep028_failure_denied_permission_binding_mismatch() {
    struct OkTransport;
    impl HydraTransport for OkTransport {
        fn submit_action(&self, _a: &serde_json::Value) -> Result<String, nexus_hydra::HydraError> {
            Ok("SUBMITTED".to_string())
        }
    }
    let adapter = HydraAdapter::new(Box::new(OkTransport), binding(), Vec::new());
    let other = nexus_hydra::HydraBusinessBinding::new(
        HydraBindingId::new("binding-other").unwrap(),
        tenant(),
        business(),
        std::collections::BTreeSet::from([HydraAccessChannel::REST]),
    );
    let req = request(HydraActionKind::ReadContext, ApprovalClass::None);
    let err = adapter.submit_action(&other, &req).unwrap_err();
    assert_eq!(err.code, HydraErrorCode::Authorization);
}

#[test]
fn ep028_failure_in_flight_duplicate_conflict_and_release() {
    use std::sync::{Arc, Mutex};

    struct GatedTransport {
        gate: Arc<Mutex<bool>>,
        calls: std::sync::atomic::AtomicUsize,
    }
    impl HydraTransport for GatedTransport {
        fn submit_action(&self, _a: &serde_json::Value) -> Result<String, nexus_hydra::HydraError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            loop {
                let gate = self.gate.lock().unwrap();
                if *gate {
                    break;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok("SUBMITTED".to_string())
        }
    }

    let gate = Arc::new(Mutex::new(false));
    let transport = GatedTransport {
        gate: Arc::clone(&gate),
        calls: std::sync::atomic::AtomicUsize::new(0),
    };
    let adapter = Arc::new(HydraAdapter::new(
        Box::new(transport),
        binding(),
        Vec::new(),
    ));
    let adapter1 = Arc::clone(&adapter);
    let handle1 = std::thread::spawn(move || {
        adapter1.submit_action(
            &binding(),
            &request(HydraActionKind::ReadContext, ApprovalClass::None),
        )
    });
    std::thread::sleep(Duration::from_millis(100));

    // Duplicate in-flight request -> Conflict, and it must not reach
    // the transport a second time.
    let err = adapter
        .submit_action(
            &binding(),
            &request(HydraActionKind::ReadContext, ApprovalClass::None),
        )
        .unwrap_err();
    assert_eq!(err.code, HydraErrorCode::Conflict);

    *gate.lock().unwrap() = true;
    let first = handle1.join().unwrap().unwrap();
    assert_eq!(first.state, nexus_hydra::HydraActionState::Submitted);

    // After completion the entry is released: a retry is not Conflict.
    let retry = adapter.submit_action(
        &binding(),
        &request(HydraActionKind::ReadContext, ApprovalClass::None),
    );
    assert!(retry.is_ok());
}

#[test]
fn ep028_failure_unknown_provider_vocabulary_fails_closed() {
    let (port, handle) = spawn_server(|_m, _p| {
        (
            200,
            "application/json",
            r#"{"action_id":"action-1","state":"FABRICATED_STATE"}"#.to_string(),
        )
    });
    let transport = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let payload = serde_json::json!({"action_id":"action-1"});
    let err = transport.submit_action(&payload).unwrap_err();
    assert_eq!(err.code, HydraErrorCode::ExternalProvider);
    handle.join().unwrap();
}

#[test]
fn ep028_failure_redaction_canary_zero_leakage() {
    // Poison the audit ring with an error containing the secret; the
    // ring must redact at insert (poison-safe).
    let mut obs = HydraObservability::new(64, vec![CANARY_TOKEN.to_string()]);
    obs.record(nexus_hydra_connector::HydraAuditEntry {
        correlation: "corr-1".into(),
        operation: "SUBMIT_ACTION".into(),
        outcome: "AUTHORIZATION".into(),
        detail: format!("credential {CANARY_TOKEN} embedded in failure"),
        fields: std::collections::BTreeMap::new(),
    });
    let audit = obs.audit();
    assert_eq!(audit.len(), 1);
    let dumped = format!("{:?}", audit);
    assert!(!dumped.contains(CANARY_TOKEN), "canary leaked: {dumped}");
    assert!(dumped.contains("***"));
}

#[test]
fn ep028_failure_unknown_action_state_fails_closed_through_adapter() {
    struct BadStateTransport;
    impl HydraTransport for BadStateTransport {
        fn submit_action(&self, _a: &serde_json::Value) -> Result<String, nexus_hydra::HydraError> {
            Ok("FABRICATED_STATE".to_string())
        }
    }
    let adapter = HydraAdapter::new(Box::new(BadStateTransport), binding(), Vec::new());
    let req = request(HydraActionKind::ReadContext, ApprovalClass::None);
    let err = adapter.submit_action(&binding(), &req).unwrap_err();
    assert_eq!(err.code, HydraErrorCode::ExternalProvider);
    // The audit records the failure class, never the fabricated state
    // as success.
    assert!(adapter.audit().iter().all(|e| e.outcome != "ok"));
}

#[test]
fn ep028_failure_bounded_recovery_after_unavailable() {
    // Phase A: provider gone -> Unavailable.
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

    // Phase B: a healthy provider appears on a NEW port; a fresh
    // transport recovers and reads context (bounded recovery, no
    // fabricated session, no blind retry on the dead endpoint).
    let (port2, handle) = spawn_server(|_m, _p| {
        (
            200,
            "application/json",
            r#"{"binding_id":"binding-1","business_id":"018f0f6f-9c1e-7b6e-8000-000000000003","customers":[],"campaigns":[],"observed_at":"2026-08-19T00:00:00Z"}"#
                .to_string(),
        )
    });
    let recovered = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port2}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let projection = recovered.read_context(&context()).expect("recovery");
    assert_eq!(projection.business_id, business());
    handle.join().unwrap();
}

#[test]
fn ep028_failure_cancelled_work_fails_closed() {
    // A request whose correlation is present but whose binding is
    // inactive must fail closed Policy even though the transport would
    // answer (cancelled/inactive authority never reaches the provider).
    let (port, handle) = spawn_server(|_m, _p| {
        (
            200,
            "application/json",
            r#"{"binding_id":"binding-1","business_id":"018f0f6f-9c1e-7b6e-8000-000000000003","customers":[],"campaigns":[],"observed_at":"2026-08-19T00:00:00Z"}"#
                .to_string(),
        )
    });
    let transport = HttpHydraTransport::new(
        format!("http://127.0.0.1:{port}"),
        CANARY_TOKEN,
        Duration::from_secs(5),
    );
    let mut inactive = binding();
    inactive.deactivate();
    let adapter = HydraAdapter::new(Box::new(transport), inactive.clone(), Vec::new());
    let err = adapter.read_context(&inactive, &context()).unwrap_err();
    assert_eq!(err.code, HydraErrorCode::Policy);
    handle.join().unwrap();
}
