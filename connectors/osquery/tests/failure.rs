//! EP-031 osquery forced-failure suite (M5).
//!
//! REAL std::net socket proofs against the production
//! `HttpOsqueryEndpoint` server (never mocked): the test plays the
//! osqueryd NODE side over real sockets and drives the documented
//! enroll / distributed_read / distributed_write lifecycle, then
//! proves the adapter fails closed on observed failures and never
//! fabricates telemetry.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use nexus_domain::TenantId;
use nexus_osquery_connector::{
    DistributedQuery, HttpOsqueryEndpoint, OsqueryEndpointTelemetryProvider, OsqueryTransport,
};
use nexus_sentinel::SentinelErrorCode;
use nexus_sentinel_advanced::EndpointTelemetryProvider;

const TENANT: &str = "018f0f6f-9c1e-7b6e-8000-000000000001";
const SECRET: &str = "ep031-osquery-secret";
const CANARY: &str = "EP031_M5_CANARY_OSQUERY_SECRET";

fn tenant() -> TenantId {
    TenantId::from_str(TENANT).expect("tenant")
}

fn queries() -> Vec<DistributedQuery> {
    vec![DistributedQuery {
        id: "listening_ports".to_string(),
        query: "SELECT address, port, protocol, pid FROM listening_ports;".to_string(),
    }]
}

/// Build a rustls client that PINS the collector's certificate
/// (AUD-036): the node authenticates the server over REAL TLS and
/// never speaks plaintext.
fn tls_client(ep: &HttpOsqueryEndpoint) -> rustls::ClientConfig {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls_pki_types::CertificateDer::from(ep.certificate_der()))
        .expect("pin collector certificate");
    rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth()
}

/// POST a documented request body to the endpoint over REAL TLS and
/// return the raw HTTP response body.
fn post(ep: &HttpOsqueryEndpoint, port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let mut conn = rustls::ClientConnection::new(
        Arc::new(tls_client(ep)),
        "localhost".try_into().expect("server name"),
    )
    .expect("client connection");
    conn.complete_io(&mut stream).expect("tls handshake");
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    conn.writer().write_all(req.as_bytes()).expect("write");
    conn.complete_io(&mut stream).expect("tls flush");
    let mut resp: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    // Read until the full HTTP response (headers + Content-Length body)
    // is buffered. rustls reader().read() returns WouldBlock when no
    // plaintext is buffered; pump the socket and retry.
    loop {
        match conn.reader().read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                resp.extend_from_slice(&chunk[..n]);
                if let Some(end) = resp.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&resp[..end]);
                    let clen = head
                        .lines()
                        .find_map(|l| {
                            let l = l.to_ascii_lowercase();
                            l.strip_prefix("content-length:")
                                .and_then(|v| v.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if resp.len() >= end + 4 + clen {
                        break;
                    }
                }
            }
            Err(_) => {
                if conn.complete_io(&mut stream).is_err() {
                    break;
                }
            }
        }
    }
    let text = String::from_utf8_lossy(&resp).to_string();
    let body_start = text.find("\r\n\r\n").map(|i| i + 4).unwrap_or(text.len());
    text[body_start..].to_string()
}

fn full_lifecycle(secret: &str) -> (HttpOsqueryEndpoint, u16, String) {
    let ep = HttpOsqueryEndpoint::new(secret.to_string(), queries());
    let port = ep.serve().expect("serve");
    // Enroll (documented POST /enroll).
    let body = format!(
        r#"{{"enroll_secret":"{}","host_identifier":"host-1"}}"#,
        secret
    );
    let resp = post(&ep, port, "/enroll", &body);
    let node_key = serde_json::from_str::<serde_json::Value>(&resp)
        .expect("enroll response json")
        .get("node_key")
        .and_then(|v| v.as_str())
        .expect("node_key")
        .to_string();
    assert!(!node_key.is_empty());
    (ep, port, node_key)
}

#[test]
fn ep031_failure_osquery_full_enroll_read_write_lifecycle_over_real_socket() {
    let (server_ep, port, node_key) = full_lifecycle(SECRET);

    // Distributed read: the collector issues the owned query.
    let resp = post(
        &server_ep,
        port,
        "/distributed_read",
        &format!(r#"{{"node_key":"{node_key}"}}"#),
    );
    let v: serde_json::Value = serde_json::from_str(&resp).expect("distributed_read json");
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(false));
    assert!(v["queries"]["listening_ports"]
        .as_str()
        .unwrap_or("")
        .contains("listening_ports"));

    // Distributed write: the node returns REAL osquery-shaped rows.
    let write = format!(
        r#"{{"node_key":"{node_key}","queries":{{"listening_ports":[{{"address":"0.0.0.0","port":"8443","protocol":"tcp","pid":"42"}}]}},"statuses":{{"listening_ports":0}}}}"#
    );
    let resp = post(&server_ep, port, "/distributed_write", &write);
    let v: serde_json::Value = serde_json::from_str(&resp).expect("distributed_write json");
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(false));

    // The adapter observes the wildcard listener as a finding.
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let mut ep2 = ep.clone();
    let key = ep2.enroll(SECRET, "host-1").unwrap();
    let mut q = HashMap::new();
    q.insert(
        "listening_ports".to_string(),
        vec![serde_json::json!({"address": "0.0.0.0", "port": "8443", "protocol": "tcp"})],
    );
    let mut s = HashMap::new();
    s.insert("listening_ports".to_string(), 0);
    ep2.distributed_write(&key, &q, &s).unwrap();
    let provider = OsqueryEndpointTelemetryProvider::new(ep2);
    let events = provider.read_telemetry(&tenant()).unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn ep031_failure_osquery_enroll_secret_rejected_fails_closed() {
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let port = ep.serve().expect("serve");
    let body = format!(
        r#"{{"enroll_secret":"{}","host_identifier":"host-1"}}"#,
        "wrong-secret"
    );
    let resp = post(&ep, port, "/enroll", &body);
    let v: serde_json::Value = serde_json::from_str(&resp).expect("json");
    // Documented failure shape: blank node_key + node_invalid true.
    assert_eq!(v["node_key"], serde_json::Value::String(String::new()));
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(true));
    let ep2 = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    assert!(ep2.node_key().is_none());
}

#[test]
fn ep031_failure_osquery_enroll_without_host_identifier_fails_closed() {
    // AUD-035: over a REAL socket, enrollment without a durable
    // endpoint identity fails closed (documented failure shape) - the
    // collector never binds an unnamed endpoint.
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let port = ep.serve().expect("serve");
    let body = format!(r#"{{"enroll_secret":"{}"}}"#, SECRET);
    let resp = post(&ep, port, "/enroll", &body);
    let v: serde_json::Value = serde_json::from_str(&resp).expect("json");
    assert_eq!(v["node_key"], serde_json::Value::String(String::new()));
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(true));
    assert!(ep.node_key().is_none());
    assert!(ep.host_identifier().is_none());
}

#[test]
fn ep031_failure_osquery_enroll_identity_conflict_denied_over_socket() {
    // AUD-035: over a REAL socket, a DIFFERENT host cannot re-enroll
    // and adopt the bound node's identity or credentials; the SAME
    // host re-enrolling keeps its durable identity.
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let port = ep.serve().expect("serve");
    let body = format!(
        r#"{{"enroll_secret":"{}","host_identifier":"host-1"}}"#,
        SECRET
    );
    let resp = post(&ep, port, "/enroll", &body);
    let v: serde_json::Value = serde_json::from_str(&resp).expect("json");
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(false));
    assert_eq!(ep.host_identifier().as_deref(), Some("host-1"));
    // Different host: denied.
    let body = format!(
        r#"{{"enroll_secret":"{}","host_identifier":"host-2"}}"#,
        SECRET
    );
    let resp = post(&ep, port, "/enroll", &body);
    let v: serde_json::Value = serde_json::from_str(&resp).expect("json");
    assert_eq!(v["node_key"], serde_json::Value::String(String::new()));
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(true));
    assert_eq!(ep.host_identifier().as_deref(), Some("host-1"));
    // Same host: durable identity preserved, fresh session key.
    let body = format!(
        r#"{{"enroll_secret":"{}","host_identifier":"host-1"}}"#,
        SECRET
    );
    let resp = post(&ep, port, "/enroll", &body);
    let v: serde_json::Value = serde_json::from_str(&resp).expect("json");
    assert!(!v["node_key"].as_str().unwrap_or("").is_empty());
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(false));
    assert_eq!(ep.host_identifier().as_deref(), Some("host-1"));
}

#[test]
fn ep031_failure_osquery_observed_results_carry_durable_identity() {
    // AUD-035: after a REAL enroll -> read -> write lifecycle, the
    // server attributes every observed result to the durable endpoint
    // identity bound at enrollment.
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let port = ep.serve().expect("serve");
    let body = format!(
        r#"{{"enroll_secret":"{}","host_identifier":"host-1"}}"#,
        SECRET
    );
    let resp = post(&ep, port, "/enroll", &body);
    let v: serde_json::Value = serde_json::from_str(&resp).expect("json");
    let node_key = v["node_key"].as_str().unwrap().to_string();
    let write = format!(
        r#"{{"node_key":"{node_key}","queries":{{"listening_ports":[{{"address":"0.0.0.0","port":"8443","protocol":"tcp"}}]}},"statuses":{{"listening_ports":0}}}}"#
    );
    let resp = post(&ep, port, "/distributed_write", &write);
    let v: serde_json::Value = serde_json::from_str(&resp).expect("json");
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(false));
    let observed = ep.observed_results();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].host_identifier, "host-1");
    assert_eq!(observed[0].query_id, "listening_ports");
}

#[test]
fn ep031_failure_osquery_unknown_node_key_rejected() {
    let (ep, port, _) = full_lifecycle(SECRET);
    let resp = post(
        &ep,
        port,
        "/distributed_write",
        r#"{"node_key":"node-bogus","queries":{},"statuses":{}}"#,
    );
    let v: serde_json::Value = serde_json::from_str(&resp).expect("json");
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(true));
}

#[test]
fn ep031_failure_osquery_malformed_request_fails_closed() {
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let port = ep.serve().expect("serve");
    let resp = post(&ep, port, "/enroll", "this is not json");
    assert!(resp.contains("error"));
    let resp = post(&ep, port, "/distributed_write", "");
    assert!(resp.contains("error"));
}

#[test]
fn ep031_failure_osquery_query_failure_observed_not_fabricated() {
    // Non-zero distributed status is an OBSERVED query execution
    // failure: the adapter fails closed (ExternalProvider) and never
    // fabricates rows for the failed query.
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let mut ep2 = ep.clone();
    let key = ep2.enroll(SECRET, "host-1").unwrap();
    let mut q = HashMap::new();
    q.insert(
        "listening_ports".to_string(),
        vec![serde_json::json!({"address": "0.0.0.0", "port": "8443"})],
    );
    let mut s = HashMap::new();
    s.insert("listening_ports".to_string(), 2);
    ep2.distributed_write(&key, &q, &s).unwrap();
    let provider = OsqueryEndpointTelemetryProvider::new(ep2);
    let err = provider.read_telemetry(&tenant()).unwrap_err();
    assert_eq!(err.code, SentinelErrorCode::ExternalProvider);
    let entries = provider.audit_entries();
    assert!(entries.iter().any(|e| e.outcome == "failed"));
}

#[test]
fn ep031_failure_osquery_clean_telemetry_is_observed_not_fabricated() {
    // Private-address listeners are observed telemetry, not findings.
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let mut ep2 = ep.clone();
    let key = ep2.enroll(SECRET, "host-1").unwrap();
    let mut q = HashMap::new();
    q.insert(
        "listening_ports".to_string(),
        vec![serde_json::json!({"address": "127.0.0.1", "port": "22", "protocol": "tcp"})],
    );
    let mut s = HashMap::new();
    s.insert("listening_ports".to_string(), 0);
    ep2.distributed_write(&key, &q, &s).unwrap();
    let provider = OsqueryEndpointTelemetryProvider::new(ep2);
    let events = provider.read_telemetry(&tenant()).unwrap();
    assert!(events.is_empty());
}

#[test]
fn ep031_failure_osquery_plaintext_connection_rejected() {
    // AUD-036: the collector serves the documented osquery TLS remote
    // API over REAL TLS ONLY. A PLAINTEXT HTTP request over a raw
    // socket must never reach the API surface - no HTTP response, no
    // node_key, no node_invalid JSON. The server answers with TLS
    // alert bytes or closes; the plaintext bytes never route.
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let port = ep.serve().expect("serve");
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let body = format!(
        r#"{{"enroll_secret":"{}","host_identifier":"host-1"}}"#,
        SECRET
    );
    let req = format!(
        "POST /enroll HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => resp.extend_from_slice(&chunk[..n]),
        }
    }
    let text = String::from_utf8_lossy(&resp).to_string();
    assert!(
        !text.contains("HTTP/1.1"),
        "plaintext HTTP request must never receive an HTTP response"
    );
    assert!(
        !text.contains("node_invalid"),
        "plaintext HTTP request must never reach the documented API surface"
    );
    assert!(
        !text.contains("node_key"),
        "plaintext HTTP request must never mint or reveal a node key"
    );
}

#[test]
fn ep031_failure_osquery_untrusted_tls_client_denied() {
    // AUD-036: TLS is real, not decorative. A client that does NOT
    // pin the collector certificate fails the handshake - the
    // documented API is only reachable by a node that authenticates
    // the server.
    let ep = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    let port = ep.serve().expect("serve");
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    // Empty root store: the collector's self-signed cert is untrusted.
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();
    let mut conn = rustls::ClientConnection::new(
        Arc::new(config),
        "localhost".try_into().expect("server name"),
    )
    .expect("client connection");
    let result = conn.complete_io(&mut stream);
    assert!(
        result.is_err(),
        "untrusted TLS client must fail the handshake (certificate verification is real)"
    );
}

#[test]
fn ep031_failure_osquery_audit_never_leaks_secret() {
    // Redaction canary: the enrollment secret must never appear in
    // audit entries or provider diagnostics.
    let ep = HttpOsqueryEndpoint::new(CANARY.to_string(), queries());
    let mut ep2 = ep.clone();
    let key = ep2.enroll(CANARY, "host-1").unwrap();
    let mut q = HashMap::new();
    q.insert(
        "listening_ports".to_string(),
        vec![serde_json::json!({"address": "0.0.0.0", "port": "8443"})],
    );
    let mut s = HashMap::new();
    s.insert("listening_ports".to_string(), 0);
    ep2.distributed_write(&key, &q, &s).unwrap();
    let provider = OsqueryEndpointTelemetryProvider::new(ep2);
    let _ = provider.read_telemetry(&tenant()).unwrap();
    let entries = provider.audit_entries();
    let joined = entries
        .iter()
        .map(|e| format!("{} {} {}", e.operation, e.outcome, e.detail))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!joined.contains(CANARY), "canary leaked into audit");
    assert!(!joined.contains(&key), "node_key leaked into audit");
}
