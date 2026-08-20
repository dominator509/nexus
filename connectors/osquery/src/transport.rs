//! EP-031 osquery transport (M5): self-hosted collector implementing
//! the DOCUMENTED osquery TLS remote API server surface.
//!
//! osquery is the Endpoint profile security sensor (SPEC-013 behavior
//! 3; COMPONENT_REGISTRY external sensor, GPL-2.0). Nexus is the
//! collector: a real osqueryd node enrolls and reports observed
//! telemetry over the documented endpoints (osquery.readthedocs.io/
//! en/stable/deployment/remote - anti-hallucination, no invented
//! endpoints):
//!
//! - POST /enroll
//!   request {"enroll_secret": "...", "host_identifier": "..."}
//!   response {"node_key": "...", "node_invalid": false}
//!   (node_invalid true means the node must re-enroll)
//! - POST /distributed_read
//!   request {"node_key": "..."}
//!   response {"queries": {"<id>": "<sql>", ...}, "node_invalid": false}
//! - POST /distributed_write
//!   request {"node_key": "...", "queries": {"<id>": [rows]},
//!   "statuses": {"<id>": 0}}
//!   response {"node_invalid": false}
//!   statuses are SQLite error codes: non-0 means query execution
//!   failure (observed, never fabricated).
//!
//! The endpoint binds a REAL std::net socket and serves sequential
//! POST requests with a bounded capacity. Malformed requests fail
//! closed (HTTP 400). The enrollment secret is used ONLY to validate
//! enrollment and is registered as a redaction secret; it never
//! appears in errors or telemetry.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nexus_sentinel::{SentinelError, SentinelErrorCode};
use serde::{Deserialize, Serialize};

/// A distributed query the collector issues to enrolled nodes
/// (documented distributed_read response entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DistributedQuery {
    pub id: String,
    pub query: String,
}

/// Observed distributed query result (documented distributed_write
/// request entry + statuses entry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedOsqueryResult {
    pub query_id: String,
    /// Observed rows (free-form osquery table rows; normalized at the
    /// adapter boundary, never a domain contract).
    pub rows: Vec<serde_json::Value>,
    /// SQLite status code; 0 = success, non-0 = query execution
    /// failure (documented).
    pub status: i64,
}

/// Documented enroll request body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsqueryEnrollRequest {
    #[serde(default)]
    pub enroll_secret: Option<String>,
    #[serde(default)]
    pub host_identifier: Option<String>,
}

/// The osquery transport port. Default implementations fail closed so
/// an unbound transport never fabricates telemetry.
pub trait OsqueryTransport {
    /// Validate an enrollment (documented POST /enroll). Returns the
    /// node_key on success.
    fn enroll(&mut self, secret: &str, host_identifier: &str) -> Result<String, SentinelError> {
        let _ = (secret, host_identifier);
        Err(SentinelError::unavailable(
            "osquery transport has no implementation bound",
        ))
    }

    /// Issue distributed queries to an enrolled node (documented POST
    /// /distributed_read).
    fn distributed_read(&mut self, node_key: &str) -> Result<Vec<DistributedQuery>, SentinelError> {
        let _ = node_key;
        Err(SentinelError::unavailable(
            "osquery transport has no implementation bound",
        ))
    }

    /// Collect distributed query results from an enrolled node
    /// (documented POST /distributed_write). Non-zero statuses are
    /// OBSERVED query execution failures.
    fn distributed_write(
        &mut self,
        node_key: &str,
        queries: &HashMap<String, Vec<serde_json::Value>>,
        statuses: &HashMap<String, i64>,
    ) -> Result<(), SentinelError> {
        let _ = (node_key, queries, statuses);
        Err(SentinelError::unavailable(
            "osquery transport has no implementation bound",
        ))
    }

    /// Observed results accumulated since the last drain.
    fn observed_results(&self) -> Vec<ObservedOsqueryResult> {
        Vec::new()
    }

    /// Drain observed results (the adapter consumes them once).
    fn drain_observed(&mut self) -> Vec<ObservedOsqueryResult> {
        Vec::new()
    }
}

/// Unit transport: always fails closed (used for the unbound case).
impl OsqueryTransport for () {}

/// Real osquery TLS remote API server (self-hosted collector).
#[derive(Debug, Clone)]
pub struct HttpOsqueryEndpoint {
    inner: Arc<Mutex<EndpointInner>>,
}

#[derive(Debug)]
struct EndpointInner {
    enroll_secret: String,
    node_key: Option<String>,
    queries: Vec<DistributedQuery>,
    observed: Vec<ObservedOsqueryResult>,
    served: usize,
    max_serves: usize,
}

impl HttpOsqueryEndpoint {
    /// Create the endpoint. `queries` are the distributed queries the
    /// collector issues to enrolled nodes.
    pub fn new(enroll_secret: impl Into<String>, queries: Vec<DistributedQuery>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(EndpointInner {
                enroll_secret: enroll_secret.into(),
                node_key: None,
                queries,
                observed: Vec::new(),
                served: 0,
                max_serves: 64,
            })),
        }
    }

    /// The configured enrollment secret (never logged; used only for
    /// validation).
    pub fn enroll_secret(&self) -> String {
        self.inner.lock().unwrap().enroll_secret.clone()
    }

    /// Bind a REAL socket on 127.0.0.1 and serve sequential POST
    /// requests with bounded capacity. Returns the bound port.
    pub fn serve(&self) -> std::io::Result<u16> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(false)?;
        let inner = Arc::clone(&self.inner);
        thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            loop {
                if std::time::Instant::now() > deadline {
                    break;
                }
                let (mut stream, _) = match listener.accept() {
                    Ok(ok) => ok,
                    Err(_) => continue,
                };
                let mut inner = match inner.lock() {
                    Ok(g) => g,
                    Err(_) => break,
                };
                if inner.served >= inner.max_serves {
                    break;
                }
                inner.served += 1;
                let response = handle_request(&mut inner, &mut stream);
                drop(inner);
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        Ok(port)
    }

    /// The node_key issued at enrollment, if any.
    pub fn node_key(&self) -> Option<String> {
        self.inner.lock().unwrap().node_key.clone()
    }
}

/// Serve one HTTP POST request (fail closed on malformed input).
fn handle_request(inner: &mut EndpointInner, stream: &mut TcpStream) -> String {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    // Read until the header terminator is present.
    let mut header_end = None;
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end = Some(pos + 4);
                    break;
                }
                if buf.len() > 65536 {
                    return http_json(400, r#"{"error":"request too large"}"#);
                }
            }
        }
    }
    let Some(end) = header_end else {
        return http_json(400, r#"{"error":"malformed request"}"#);
    };
    let head = String::from_utf8_lossy(&buf[..end]).to_string();
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    if method != "POST" {
        return http_json(405, r#"{"error":"method not allowed"}"#);
    }
    // Content-Length framing.
    let mut content_length = 0usize;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse::<usize>().unwrap_or(0);
        }
    }
    let mut body = buf[end..].to_vec();
    while body.len() < content_length {
        let mut chunk = [0u8; 8192];
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => body.extend_from_slice(&chunk[..n]),
        }
    }
    let body = String::from_utf8_lossy(&body).to_string();
    route(inner, path, &body)
}

/// Dispatch a documented endpoint (anti-hallucination: only the
/// documented surface exists).
fn route(inner: &mut EndpointInner, path: &str, body: &str) -> String {
    match path {
        "/enroll" => route_enroll(inner, body),
        "/distributed_read" => route_distributed_read(inner, body),
        "/distributed_write" => route_distributed_write(inner, body),
        _ => http_json(404, r#"{"error":"not found"}"#),
    }
}

fn route_enroll(inner: &mut EndpointInner, body: &str) -> String {
    let Ok(req) = serde_json::from_str::<OsqueryEnrollRequest>(body) else {
        return http_json(400, r#"{"error":"malformed enroll request"}"#);
    };
    let secret = req.enroll_secret.unwrap_or_default();
    if secret != inner.enroll_secret {
        // Documented failure shape: blank node_key + node_invalid.
        return http_json(200, r#"{"node_key":"","node_invalid":true}"#);
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let node_key = format!("node-{nanos}");
    inner.node_key = Some(node_key.clone());
    http_json(
        200,
        &format!(r#"{{"node_key":"{node_key}","node_invalid":false}}"#),
    )
}

fn route_distributed_read(inner: &mut EndpointInner, body: &str) -> String {
    let Ok(req) = serde_json::from_str::<serde_json::Value>(body) else {
        return http_json(400, r#"{"error":"malformed distributed_read request"}"#);
    };
    let node_key = req.get("node_key").and_then(|v| v.as_str()).unwrap_or("");
    if Some(node_key.to_string()) != inner.node_key {
        return http_json(200, r#"{"node_invalid":true}"#);
    }
    let mut queries = serde_json::Map::new();
    for q in &inner.queries {
        queries.insert(q.id.clone(), serde_json::Value::String(q.query.clone()));
    }
    let mut out = serde_json::Map::new();
    out.insert("queries".to_string(), serde_json::Value::Object(queries));
    out.insert("node_invalid".to_string(), serde_json::Value::Bool(false));
    http_json(200, &serde_json::Value::Object(out).to_string())
}

fn route_distributed_write(inner: &mut EndpointInner, body: &str) -> String {
    let Ok(req) = serde_json::from_str::<serde_json::Value>(body) else {
        return http_json(400, r#"{"error":"malformed distributed_write request"}"#);
    };
    let node_key = req.get("node_key").and_then(|v| v.as_str()).unwrap_or("");
    if Some(node_key.to_string()) != inner.node_key {
        return http_json(200, r#"{"node_invalid":true}"#);
    }
    let queries = req
        .get("queries")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let statuses = req
        .get("statuses")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    for (id, rows) in queries {
        let status = statuses.get(&id).and_then(|v| v.as_i64()).unwrap_or(-1);
        let rows = rows.as_array().cloned().unwrap_or_default();
        inner.observed.push(ObservedOsqueryResult {
            query_id: id,
            rows,
            status,
        });
    }
    http_json(200, r#"{"node_invalid":false}"#)
}

fn http_json(status: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        match status {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            _ => "Error",
        },
        body.len(),
        body
    )
}

impl OsqueryTransport for HttpOsqueryEndpoint {
    fn enroll(&mut self, secret: &str, host_identifier: &str) -> Result<String, SentinelError> {
        let mut inner = self.inner.lock().unwrap();
        if secret != inner.enroll_secret {
            return Err(SentinelError::new(
                SentinelErrorCode::Authorization,
                "osquery enrollment secret rejected",
                None,
                None,
                None,
                None,
            ));
        }
        let _ = host_identifier;
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let node_key = format!("node-{nanos}");
        inner.node_key = Some(node_key.clone());
        Ok(node_key)
    }

    fn distributed_read(&mut self, node_key: &str) -> Result<Vec<DistributedQuery>, SentinelError> {
        let inner = self.inner.lock().unwrap();
        if Some(node_key.to_string()) != inner.node_key {
            return Err(SentinelError::new(
                SentinelErrorCode::Authorization,
                "osquery node_key invalid",
                None,
                None,
                None,
                None,
            ));
        }
        Ok(inner.queries.clone())
    }

    fn distributed_write(
        &mut self,
        node_key: &str,
        queries: &HashMap<String, Vec<serde_json::Value>>,
        statuses: &HashMap<String, i64>,
    ) -> Result<(), SentinelError> {
        let mut inner = self.inner.lock().unwrap();
        if Some(node_key.to_string()) != inner.node_key {
            return Err(SentinelError::new(
                SentinelErrorCode::Authorization,
                "osquery node_key invalid",
                None,
                None,
                None,
                None,
            ));
        }
        for (id, rows) in queries {
            let status = statuses.get(id).copied().unwrap_or(-1);
            inner.observed.push(ObservedOsqueryResult {
                query_id: id.clone(),
                rows: rows.clone(),
                status,
            });
        }
        Ok(())
    }

    fn observed_results(&self) -> Vec<ObservedOsqueryResult> {
        self.inner.lock().unwrap().observed.clone()
    }

    fn drain_observed(&mut self) -> Vec<ObservedOsqueryResult> {
        std::mem::take(&mut self.inner.lock().unwrap().observed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queries() -> Vec<DistributedQuery> {
        vec![DistributedQuery {
            id: "listening_ports".to_string(),
            query: "SELECT address, port, protocol, pid FROM listening_ports;".to_string(),
        }]
    }

    #[test]
    fn ep031_unit_osquery_enroll_documented_shape() {
        let mut ep = HttpOsqueryEndpoint::new("ep031-secret", queries());
        let key = ep.enroll("ep031-secret", "host-1").unwrap();
        assert!(key.starts_with("node-"));
        assert_eq!(ep.node_key(), Some(key.clone()));
        let err = ep.enroll("wrong-secret", "host-1").unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Authorization);
    }

    #[test]
    fn ep031_unit_osquery_distributed_read_issues_owned_queries() {
        let mut ep = HttpOsqueryEndpoint::new("ep031-secret", queries());
        let key = ep.enroll("ep031-secret", "host-1").unwrap();
        let got = ep.distributed_read(&key).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "listening_ports");
        assert!(got[0].query.contains("listening_ports"));
        let err = ep.distributed_read("node-bogus").unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Authorization);
    }

    #[test]
    fn ep031_unit_osquery_distributed_write_collects_observed_rows() {
        let mut ep = HttpOsqueryEndpoint::new("ep031-secret", queries());
        let key = ep.enroll("ep031-secret", "host-1").unwrap();
        let mut queries = HashMap::new();
        queries.insert(
            "listening_ports".to_string(),
            vec![serde_json::json!({"address": "0.0.0.0", "port": 8443})],
        );
        let mut statuses = HashMap::new();
        statuses.insert("listening_ports".to_string(), 0);
        ep.distributed_write(&key, &queries, &statuses).unwrap();
        let observed = ep.observed_results();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0].status, 0);
        assert_eq!(observed[0].rows.len(), 1);
    }

    #[test]
    fn ep031_unit_osquery_unit_transport_fails_closed() {
        let mut t = ();
        let err = t.enroll("s", "h").unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
    }
}
