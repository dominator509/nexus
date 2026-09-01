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
use rustls_pki_types::{CertificateDer, PrivateKeyDer};
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
    /// Durable endpoint identity (documented enroll host_identifier)
    /// of the node that reported this result (AUD-035). Empty when no
    /// identity was bound; the adapter fails closed on that case.
    pub host_identifier: String,
    pub query_id: String,
    /// Observed rows (free-form osquery table rows; normalized at the
    /// adapter boundary, never a domain contract).
    pub rows: Vec<serde_json::Value>,
    /// SQLite status code; 0 = success, non-0 = query execution
    /// failure (documented).
    pub status: i64,
    /// REAL observation time (unix seconds) stamped by the collector
    /// when the distributed_write was received (AUD-037). 0 means no
    /// observation time was stamped; the adapter fails closed rather
    /// than fabricate one.
    pub observed_at: i64,
    /// Monotonic per-endpoint batch sequence (AUD-037): each
    /// distributed_write receipt increments it once and every result
    /// in that write carries the same sequence. Combined with the
    /// durable host identity and the row index it makes event ids
    /// collision-proof across batches and endpoints.
    pub batch_seq: u64,
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

/// TLS server identity of the collector endpoint (AUD-036). The
/// documented osquery TLS remote API is served over REAL TLS only -
/// never plaintext. The certificate is self-signed at construction
/// (rcgen, ServerAuth); nodes pin it via `certificate_der()`.
#[derive(Clone)]
struct EndpointTlsIdentity {
    server_config: Arc<rustls::ServerConfig>,
    cert_der: Vec<u8>,
}

impl std::fmt::Debug for EndpointTlsIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndpointTlsIdentity")
            .field("cert_der_len", &self.cert_der.len())
            .finish()
    }
}

/// Real osquery TLS remote API server (self-hosted collector).
#[derive(Debug, Clone)]
pub struct HttpOsqueryEndpoint {
    inner: Arc<Mutex<EndpointInner>>,
    /// AUD-036: TLS server identity. None means certificate generation
    /// failed and `serve()` fails closed - plaintext is never served.
    identity: Option<Arc<EndpointTlsIdentity>>,
}

#[derive(Debug)]
struct EndpointInner {
    enroll_secret: String,
    /// Durable endpoint identity bound at enrollment (documented
    /// host_identifier; AUD-035). Never minted, never replaced by a
    /// different host while a node is bound.
    host_identifier: Option<String>,
    node_key: Option<String>,
    queries: Vec<DistributedQuery>,
    observed: Vec<ObservedOsqueryResult>,
    served: usize,
    max_serves: usize,
    /// Monotonic batch sequence (AUD-037): incremented once per
    /// distributed_write receipt so event ids never collide across
    /// batches from the same endpoint.
    batch_seq: u64,
}

impl HttpOsqueryEndpoint {
    /// Create the endpoint. `queries` are the distributed queries the
    /// collector issues to enrolled nodes. A self-signed TLS server
    /// identity (ServerAuth, SAN localhost + 127.0.0.1) is generated;
    /// `serve()` speaks the documented osquery TLS remote API over
    /// REAL TLS only (AUD-036) - plaintext is never served.
    pub fn new(enroll_secret: impl Into<String>, queries: Vec<DistributedQuery>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(EndpointInner {
                enroll_secret: enroll_secret.into(),
                host_identifier: None,
                node_key: None,
                queries,
                observed: Vec::new(),
                served: 0,
                max_serves: 64,
                batch_seq: 0,
            })),
            identity: generate_tls_identity().map(Arc::new),
        }
    }

    /// The configured enrollment secret (never logged; used only for
    /// validation).
    pub fn enroll_secret(&self) -> String {
        self.inner.lock().unwrap().enroll_secret.clone()
    }

    /// The server certificate (DER) nodes pin to authenticate the
    /// collector (AUD-036). Empty when TLS identity generation failed.
    pub fn certificate_der(&self) -> Vec<u8> {
        self.identity
            .as_ref()
            .map(|id| id.cert_der.clone())
            .unwrap_or_default()
    }

    /// Bind a REAL socket on 127.0.0.1 and serve sequential POST
    /// requests over REAL TLS with bounded capacity. Returns the bound
    /// port. FAILS CLOSED (io error) without a TLS identity - the
    /// documented osquery TLS remote API is never served as plaintext
    /// (AUD-036).
    pub fn serve(&self) -> std::io::Result<u16> {
        let Some(identity) = self.identity.clone() else {
            return Err(std::io::Error::other(
                "osquery endpoint has no TLS identity; refusing plaintext serve (AUD-036)",
            ));
        };
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        listener.set_nonblocking(false)?;
        let inner = Arc::clone(&self.inner);
        let server_config = Arc::clone(&identity.server_config);
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
                let _ = handle_tls_request(&mut inner, &mut stream, &server_config);
            }
        });
        Ok(port)
    }

    /// The node_key issued at enrollment, if any.
    pub fn node_key(&self) -> Option<String> {
        self.inner.lock().unwrap().node_key.clone()
    }

    /// The durable endpoint identity bound at enrollment (AUD-035).
    pub fn host_identifier(&self) -> Option<String> {
        self.inner.lock().unwrap().host_identifier.clone()
    }
}

/// Generate a self-signed TLS server identity for the collector
/// (AUD-036): rcgen KeyPair + ServerAuth certificate with SAN
/// localhost + 127.0.0.1, loaded into a rustls ServerConfig (ring
/// provider). None on any generation/load failure - the endpoint then
/// refuses to serve (fail closed, no plaintext fallback).
fn generate_tls_identity() -> Option<EndpointTlsIdentity> {
    let key_pair = rcgen::KeyPair::generate().ok()?;
    let mut params = rcgen::CertificateParams::new(Vec::new()).ok()?;
    params
        .subject_alt_names
        .push(rcgen::SanType::DnsName("localhost".try_into().ok()?));
    params
        .subject_alt_names
        .push(rcgen::SanType::IpAddress("127.0.0.1".parse().ok()?));
    params.is_ca = rcgen::IsCa::NoCa;
    params
        .extended_key_usages
        .push(rcgen::ExtendedKeyUsagePurpose::ServerAuth);
    let cert = params.self_signed(&key_pair).ok()?;
    let cert_der = cert.der().to_vec();
    let key_der = key_pair.serialize_der();
    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(cert_der.clone())],
            PrivateKeyDer::Pkcs8(key_der.into()),
        )
        .ok()?;
    Some(EndpointTlsIdentity {
        server_config: Arc::new(server_config),
        cert_der,
    })
}

/// Serve one HTTP POST request over REAL TLS (AUD-036: the documented
/// osquery TLS remote API is never served as plaintext). Completes the
/// TLS handshake (fail closed on any handshake error - plaintext bytes
/// never reach routing), reads the request through the TLS stream,
/// routes it, and writes the response through the same TLS connection.
/// Returns false on any TLS/IO failure (the connection is unusable).
fn handle_tls_request(
    inner: &mut EndpointInner,
    stream: &mut TcpStream,
    server_config: &Arc<rustls::ServerConfig>,
) -> bool {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
    let mut conn = match rustls::ServerConnection::new(server_config.clone()) {
        Ok(c) => c,
        Err(_) => return false,
    };
    // Complete the TLS handshake; a client that cannot complete a real
    // TLS handshake (e.g. plaintext bytes, untrusted client) fails
    // closed here and never reaches the documented API surface.
    loop {
        match conn.complete_io(stream) {
            Ok(_) => {
                if !conn.is_handshaking() {
                    break;
                }
            }
            Err(_) => return false,
        }
    }
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    // Read until the header terminator is present. The loop only exits
    // with the header-end offset (or fails closed); the terminator is
    // therefore always present when the request is routed.
    let header_end = loop {
        match conn.reader().read(&mut chunk) {
            Ok(0) => {
                // No plaintext buffered: pump the socket, retry.
                if conn.complete_io(stream).is_err() {
                    return false;
                }
                continue;
            }
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
                if buf.len() > 65536 {
                    return write_tls_response(
                        &mut conn,
                        stream,
                        http_json(400, r#"{"error":"request too large"}"#),
                    );
                }
            }
            Err(_) => {
                // WouldBlock: no plaintext buffered yet - pump the
                // socket and retry (the peer's request may not have
                // arrived in the handshake flight).
                if conn.complete_io(stream).is_err() {
                    return false;
                }
            }
        }
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.lines();
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");
    if method != "POST" {
        return write_tls_response(
            &mut conn,
            stream,
            http_json(405, r#"{"error":"method not allowed"}"#),
        );
    }
    // Content-Length framing.
    let mut content_length = 0usize;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse::<usize>().unwrap_or(0);
        }
    }
    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let mut chunk = [0u8; 8192];
        match conn.reader().read(&mut chunk) {
            Ok(0) => {
                if conn.complete_io(stream).is_err() {
                    return false;
                }
                continue;
            }
            Ok(n) => body.extend_from_slice(&chunk[..n]),
            Err(_) => {
                if conn.complete_io(stream).is_err() {
                    return false;
                }
            }
        }
    }
    let body = String::from_utf8_lossy(&body).to_string();
    let response = route(inner, path, &body);
    write_tls_response(&mut conn, stream, response)
}

/// Write an HTTP response through the TLS connection and flush it to
/// the socket (fail closed on any TLS/IO error). `complete_io` also
/// reads; bound that wait so a client that stops sending after
/// `Connection: close` does not stall the serve loop.
fn write_tls_response(
    conn: &mut rustls::ServerConnection,
    stream: &mut TcpStream,
    response: String,
) -> bool {
    if conn.writer().write_all(response.as_bytes()).is_err() {
        return false;
    }
    stream
        .set_read_timeout(Some(Duration::from_millis(50)))
        .ok();
    conn.complete_io(stream).is_ok()
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
    // AUD-035: a durable endpoint identity is REQUIRED to enroll. The
    // documented host_identifier is that identity; without it the
    // collector cannot attribute telemetry to a durable endpoint, so
    // enrollment fails closed (documented failure shape).
    let host = req.host_identifier.unwrap_or_default();
    if host.trim().is_empty() {
        return http_json(200, r#"{"node_key":"","node_invalid":true}"#);
    }
    if let Some(bound) = &inner.host_identifier {
        if bound != &host {
            // Identity confusion: a different host must never adopt
            // the bound node's identity or credentials.
            return http_json(200, r#"{"node_key":"","node_invalid":true}"#);
        }
    }
    inner.host_identifier = Some(host);
    let node_key = mint_node_key();
    inner.node_key = Some(node_key.clone());
    http_json(
        200,
        &format!(r#"{{"node_key":"{node_key}","node_invalid":false}}"#),
    )
}

/// Mint a fresh session node_key (documented enroll response). The
/// node_key is a SESSION CREDENTIAL, never the durable identity: the
/// durable endpoint identity is the host_identifier bound at
/// enrollment (AUD-035).
fn mint_node_key() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("node-{nanos}")
}

/// REAL observation time (unix seconds) at the instant a
/// distributed_write is received (AUD-037). Never fabricated: 0 only
/// when the system clock is before the epoch (impossible in
/// practice), and the adapter fails closed on 0.
fn now_unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
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
            // AUD-035: every observed result is attributed to the
            // durable endpoint identity bound at enrollment.
            host_identifier: inner.host_identifier.clone().unwrap_or_default(),
            query_id: id,
            rows,
            status,
            // AUD-037: the collector stamps the REAL observation time
            // at write receipt - never a fabricated constant - and a
            // monotonic batch sequence so ids never collide across
            // batches.
            observed_at: now_unix_seconds(),
            batch_seq: inner.batch_seq,
        });
    }
    // AUD-037: each write receipt is one batch.
    inner.batch_seq += 1;
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
        // AUD-035: enrollment REQUIRES a durable endpoint identity
        // (the documented host_identifier). Without it the collector
        // cannot attribute telemetry to a durable endpoint, so
        // enrollment fails closed. A different host must never adopt
        // the bound node's identity or credentials.
        if host_identifier.trim().is_empty() {
            return Err(SentinelError::new(
                SentinelErrorCode::Authorization,
                "osquery enrollment requires a durable host identifier",
                None,
                None,
                None,
                None,
            ));
        }
        if let Some(bound) = &inner.host_identifier {
            if bound != host_identifier {
                return Err(SentinelError::new(
                    SentinelErrorCode::Authorization,
                    "osquery enrollment identity conflict: node already bound to a different host",
                    None,
                    None,
                    None,
                    None,
                ));
            }
        }
        inner.host_identifier = Some(host_identifier.to_string());
        let node_key = mint_node_key();
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
        let host_identifier = inner.host_identifier.clone().unwrap_or_default();
        let batch_seq = inner.batch_seq;
        for (id, rows) in queries {
            let status = statuses.get(id).copied().unwrap_or(-1);
            inner.observed.push(ObservedOsqueryResult {
                // AUD-035: attribute to the durable endpoint identity.
                host_identifier: host_identifier.clone(),
                query_id: id.clone(),
                rows: rows.clone(),
                status,
                // AUD-037: the collector stamps the REAL observation
                // time at write receipt - never a fabricated constant
                // - and a monotonic batch sequence so ids never
                // collide across batches.
                observed_at: now_unix_seconds(),
                batch_seq,
            });
        }
        // AUD-037: each write receipt is one batch.
        inner.batch_seq += 1;
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
        // AUD-035: every observed result carries the durable endpoint
        // identity bound at enrollment.
        assert_eq!(observed[0].host_identifier, "host-1");
    }

    #[test]
    fn ep031_unit_osquery_enroll_requires_durable_host_identity() {
        // AUD-035: enrollment without a durable endpoint identity
        // fails closed - the collector can never attribute telemetry
        // to an unnamed endpoint.
        let mut ep = HttpOsqueryEndpoint::new("ep031-secret", queries());
        let err = ep.enroll("ep031-secret", "").unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Authorization);
        let err = ep.enroll("ep031-secret", "   ").unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Authorization);
        assert!(ep.node_key().is_none(), "no node key minted");
        assert!(ep.host_identifier().is_none(), "no identity bound");
    }

    #[test]
    fn ep031_unit_osquery_enroll_identity_conflict_denied() {
        // AUD-035: a different host must never adopt the bound node's
        // identity or credentials; the SAME host re-enrolling keeps
        // its durable identity (session key rotates, identity does not).
        let mut ep = HttpOsqueryEndpoint::new("ep031-secret", queries());
        let key1 = ep.enroll("ep031-secret", "host-1").unwrap();
        assert!(key1.starts_with("node-"));
        let err = ep.enroll("ep031-secret", "host-2").unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Authorization);
        assert_eq!(ep.host_identifier().as_deref(), Some("host-1"));
        // Same host re-enrolls: durable identity preserved.
        let key2 = ep.enroll("ep031-secret", "host-1").unwrap();
        assert!(key2.starts_with("node-"));
        assert_eq!(ep.host_identifier().as_deref(), Some("host-1"));
    }

    #[test]
    fn ep031_unit_osquery_unit_transport_fails_closed() {
        let mut t = ();
        let err = t.enroll("s", "h").unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Unavailable);
    }
}
