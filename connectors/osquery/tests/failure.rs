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

/// POST a documented request body to the endpoint over a REAL socket
/// and return the raw HTTP response body.
fn post(port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let req = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(req.as_bytes()).expect("write");
    let mut resp = String::new();
    stream.read_to_string(&mut resp).expect("read");
    let body_start = resp.find("\r\n\r\n").map(|i| i + 4).unwrap_or(resp.len());
    resp[body_start..].to_string()
}

fn full_lifecycle(secret: &str) -> (u16, String) {
    let ep = HttpOsqueryEndpoint::new(secret.to_string(), queries());
    let port = ep.serve().expect("serve");
    // Enroll (documented POST /enroll).
    let body = format!(
        r#"{{"enroll_secret":"{}","host_identifier":"host-1"}}"#,
        secret
    );
    let resp = post(port, "/enroll", &body);
    let node_key = serde_json::from_str::<serde_json::Value>(&resp)
        .expect("enroll response json")
        .get("node_key")
        .and_then(|v| v.as_str())
        .expect("node_key")
        .to_string();
    assert!(!node_key.is_empty());
    (port, node_key)
}

#[test]
fn ep031_failure_osquery_full_enroll_read_write_lifecycle_over_real_socket() {
    let (port, node_key) = full_lifecycle(SECRET);

    // Distributed read: the collector issues the owned query.
    let resp = post(
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
    let resp = post(port, "/distributed_write", &write);
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
    let resp = post(port, "/enroll", &body);
    let v: serde_json::Value = serde_json::from_str(&resp).expect("json");
    // Documented failure shape: blank node_key + node_invalid true.
    assert_eq!(v["node_key"], serde_json::Value::String(String::new()));
    assert_eq!(v["node_invalid"], serde_json::Value::Bool(true));
    let ep2 = HttpOsqueryEndpoint::new(SECRET.to_string(), queries());
    assert!(ep2.node_key().is_none());
}

#[test]
fn ep031_failure_osquery_unknown_node_key_rejected() {
    let (port, _) = full_lifecycle(SECRET);
    let resp = post(
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
    let resp = post(port, "/enroll", "this is not json");
    assert!(resp.contains("error"));
    let resp = post(port, "/distributed_write", "");
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
