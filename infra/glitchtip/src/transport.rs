//! Minimal HTTP/1.1 transport over `std::net` for the
//! GlitchTip/Sentry-compatible incident sink (EP-038 M3).
//!
//! The repo precedent (EP-037 SeaweedFS connector) hand-rolls
//! SigV4/HTTP over `std::net::TcpStream`; we follow the same rule:
//! no HTTP client SDK dependency for one small POST endpoint.
//!
//! TLS (AUD-055): the DSN scheme decides the transport. An `https`
//! DSN MUST negotiate TLS through rustls before a single envelope
//! byte is written; if TLS cannot be established the delivery fails
//! closed with `TransportFailure::ExternalProvider` (TLS detail) and
//! the envelope is NEVER sent in plaintext. An `http` DSN is accepted
//! ONLY for local fixtures and stays plaintext. The scheme is the
//! authority: a plaintext send on an https DSN is impossible by
//! construction.
//!
//! Failure mapping follows SPEC-006 and the M3 directive:
//!
//! - connection refused            -> Unavailable
//! - timeout                       -> Timeout
//! - 401/403                       -> Authorization
//! - 404 (project/endpoint absent) -> NotFound
//! - 429                           -> RateLimit
//! - 5xx                           -> ExternalProvider
//! - malformed response            -> ExternalProvider
//! - redaction denied              -> Policy (enforced before transport)
//! - TLS handshake/verify failure  -> ExternalProvider (TLS detail)
//!
//! Every connection is fresh per request: no persistent socket state
//! is retained, so a provider restart cannot leave a stale socket.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use nexus_observability::model::short_fingerprint;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use crate::dsn::Dsn;

/// Bounded connect timeout.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Bounded read timeout.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Outcome of a single envelope POST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliveryOutcome {
    /// The provider accepted the envelope (HTTP 2xx).
    Accepted { status: u16 },
    /// The provider returned a distinguishable failure.
    Rejected { status: u16, reason: String },
    /// Transport-level failure (refused / timeout / malformed / TLS).
    Failed {
        kind: TransportFailure,
        detail: String,
    },
}

/// Transport failure kinds (SPEC-006 vocabulary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportFailure {
    Unavailable,
    Timeout,
    NotFound,
    Authorization,
    RateLimit,
    ExternalProvider,
}

impl std::fmt::Display for TransportFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Unavailable => "Unavailable",
                Self::Timeout => "Timeout",
                Self::NotFound => "NotFound",
                Self::Authorization => "Authorization",
                Self::RateLimit => "RateLimit",
                Self::ExternalProvider => "ExternalProvider",
            }
        )
    }
}

/// Build a rustls client configuration that trusts the standard web
/// root store. Self-hosted GlitchTip deployments behind a private CA
/// can supply their own `RootCertStore` via [`post_envelope_with_roots`]
/// (or a dedicated constructor on the sink).
pub fn client_config(roots: RootCertStore) -> Result<Arc<ClientConfig>, String> {
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

/// Standard web root store (webpki-roots).
pub fn web_roots() -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    roots
}

/// Post one envelope to the DSN's envelope endpoint.
///
/// `authorization_header` carries the `X-Sentry-Auth` value built by
/// the caller (it includes the public key; we never log it here).
///
/// TLS policy: if `dsn.is_https()`, the request is sent through a
/// rustls `ClientConnection`; any TLS failure fails closed with
/// `ExternalProvider` (TLS detail) BEFORE the envelope is written.
/// If `dsn` is `http`, the request stays plaintext (local fixtures).
pub fn post_envelope(
    dsn: &Dsn,
    envelope: &str,
    authorization_header: &str,
    content_type: &str,
) -> DeliveryOutcome {
    post_envelope_with_config(dsn, envelope, authorization_header, content_type, None)
}

/// Like [`post_envelope`] but with an explicit TLS client config
/// (custom root store for self-hosted private CAs). `None` uses the
/// standard web root store for https DSNs.
pub fn post_envelope_with_config(
    dsn: &Dsn,
    envelope: &str,
    authorization_header: &str,
    content_type: &str,
    tls_config: Option<Arc<ClientConfig>>,
) -> DeliveryOutcome {
    let host = dsn.host().to_string();

    // Resolve to a concrete SocketAddr (connect_timeout requires one).
    use std::net::ToSocketAddrs;
    let addr = match resolve_host(&host).to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(a) => a,
            None => {
                return DeliveryOutcome::Failed {
                    kind: TransportFailure::Unavailable,
                    detail: format!("resolve {}: no addresses", dsn.describe()),
                }
            }
        },
        Err(e) => {
            return DeliveryOutcome::Failed {
                kind: TransportFailure::Unavailable,
                detail: format!("resolve {}: {e}", dsn.describe()),
            }
        }
    };

    let mut stream = match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
        Ok(s) => s,
        Err(e) => {
            return DeliveryOutcome::Failed {
                kind: classify_connect_error(&e),
                detail: format!("connect {}: {}", dsn.describe(), e),
            }
        }
    };
    if let Err(e) = stream.set_read_timeout(Some(READ_TIMEOUT)) {
        return DeliveryOutcome::Failed {
            kind: TransportFailure::ExternalProvider,
            detail: format!("set read timeout: {e}"),
        };
    }
    if let Err(e) = stream.set_write_timeout(Some(READ_TIMEOUT)) {
        return DeliveryOutcome::Failed {
            kind: TransportFailure::ExternalProvider,
            detail: format!("set write timeout: {e}"),
        };
    }

    // AUD-055: the https DSN MUST negotiate TLS before any envelope
    // byte leaves this process. A plaintext send on an https DSN is
    // the audited defect; it is impossible by construction here.
    if dsn.is_https() {
        let config = match tls_config {
            Some(cfg) => cfg,
            None => match client_config(web_roots()) {
                Ok(cfg) => cfg,
                Err(e) => {
                    return DeliveryOutcome::Failed {
                        kind: TransportFailure::ExternalProvider,
                        detail: format!("tls client config: {e}"),
                    }
                }
            },
        };
        // ServerName from the host, stripping any explicit port.
        let server_name = match ServerName::try_from(host_without_port(&host).to_string()) {
            Ok(name) => name,
            Err(e) => {
                return DeliveryOutcome::Failed {
                    kind: TransportFailure::ExternalProvider,
                    detail: format!("tls server name {}: {e}", short_fingerprint(&host)),
                }
            }
        };
        let conn = match ClientConnection::new(config, server_name) {
            Ok(c) => c,
            Err(e) => {
                return DeliveryOutcome::Failed {
                    kind: TransportFailure::ExternalProvider,
                    detail: format!("tls handshake setup: {e}"),
                }
            }
        };
        let mut tls = StreamOwned::new(conn, stream);
        return write_and_read(
            dsn,
            &mut tls,
            envelope,
            authorization_header,
            content_type,
            &host,
        );
    }

    write_and_read(
        dsn,
        &mut stream,
        envelope,
        authorization_header,
        content_type,
        &host,
    )
}

/// Serialize + send the HTTP request over an already-established
/// stream (plain TCP or TLS), then read and classify the response.
fn write_and_read<W: Write + Read>(
    dsn: &Dsn,
    stream: &mut W,
    envelope: &str,
    authorization_header: &str,
    content_type: &str,
    host: &str,
) -> DeliveryOutcome {
    let host_header = host.to_string();
    let request = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: {host_header}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         X-Sentry-Auth: {authorization_header}\r\n\
         Connection: close\r\n\
         \r\n\
         {envelope}",
        envelope.len(),
        path = dsn.envelope_path()
    );

    if let Err(e) = stream.write_all(request.as_bytes()) {
        return DeliveryOutcome::Failed {
            kind: TransportFailure::ExternalProvider,
            detail: format!("write: {e}"),
        };
    }
    if let Err(e) = stream.flush() {
        return DeliveryOutcome::Failed {
            kind: TransportFailure::ExternalProvider,
            detail: format!("flush: {e}"),
        };
    }

    let mut response = Vec::new();
    if let Err(e) = stream.read_to_end(&mut response) {
        let kind = if is_timeout(&e) {
            TransportFailure::Timeout
        } else {
            TransportFailure::ExternalProvider
        };
        return DeliveryOutcome::Failed {
            kind,
            detail: format!("read: {e}"),
        };
    }

    parse_response(&response)
}

/// Strip an explicit `:port` suffix from a host for TLS server-name
/// construction. IPv6 literals (`::1`) are left intact; only a
/// single `:digits` port suffix is stripped.
fn host_without_port(host: &str) -> &str {
    if let Some(idx) = host.rfind(':') {
        // A second colon anywhere means an IPv6 literal, not a port.
        if host[..idx].contains(':') {
            return host;
        }
        let suffix = &host[idx + 1..];
        if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) {
            return &host[..idx];
        }
    }
    host
}

/// Parse an HTTP response into a delivery outcome.
fn parse_response(response: &[u8]) -> DeliveryOutcome {
    let text = String::from_utf8_lossy(response);
    let status_line = match text.lines().next() {
        Some(l) => l,
        None => {
            return DeliveryOutcome::Failed {
                kind: TransportFailure::ExternalProvider,
                detail: "malformed response: no status line".to_string(),
            }
        }
    };
    let status: u16 = match status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
    {
        Some(code) => code,
        None => {
            return DeliveryOutcome::Failed {
                kind: TransportFailure::ExternalProvider,
                detail: format!(
                    "malformed response: unparseable status line {}",
                    short_fingerprint(status_line)
                ),
            }
        }
    };
    match status {
        200..=299 => DeliveryOutcome::Accepted { status },
        401 | 403 => DeliveryOutcome::Rejected {
            status,
            reason: TransportFailure::Authorization.to_string(),
        },
        404 => DeliveryOutcome::Rejected {
            status,
            reason: TransportFailure::NotFound.to_string(),
        },
        429 => DeliveryOutcome::Rejected {
            status,
            reason: TransportFailure::RateLimit.to_string(),
        },
        500..=599 => DeliveryOutcome::Rejected {
            status,
            reason: TransportFailure::ExternalProvider.to_string(),
        },
        _ => DeliveryOutcome::Rejected {
            status,
            reason: "unexpected status".to_string(),
        },
    }
}

fn resolve_host(host: &str) -> String {
    // `TcpStream::connect_timeout` accepts "host:port" strings via
    // `ToSocketAddrs`; the DSN host may already include a port.
    if host.contains(':') {
        host.to_string()
    } else {
        // Default GlitchTip port when the DSN omits one.
        format!("{host}:8000")
    }
}

fn classify_connect_error(e: &std::io::Error) -> TransportFailure {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::ConnectionRefused | ErrorKind::AddrNotAvailable | ErrorKind::NotFound => {
            TransportFailure::Unavailable
        }
        ErrorKind::TimedOut => TransportFailure::Timeout,
        _ => TransportFailure::ExternalProvider,
    }
}

fn is_timeout(e: &std::io::Error) -> bool {
    use std::io::ErrorKind;
    matches!(e.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_accepts_200() {
        let outcome = parse_response(b"HTTP/1.1 200 OK\r\n\r\n");
        assert_eq!(outcome, DeliveryOutcome::Accepted { status: 200 });
    }

    #[test]
    fn parse_response_accepts_202() {
        let outcome = parse_response(b"HTTP/1.1 202 Accepted\r\n\r\n");
        assert_eq!(outcome, DeliveryOutcome::Accepted { status: 202 });
    }

    #[test]
    fn parse_response_maps_401_403_to_authorization() {
        assert_eq!(
            parse_response(b"HTTP/1.1 401 Unauthorized\r\n\r\n"),
            DeliveryOutcome::Rejected {
                status: 401,
                reason: "Authorization".to_string()
            }
        );
        assert_eq!(
            parse_response(b"HTTP/1.1 403 Forbidden\r\n\r\n"),
            DeliveryOutcome::Rejected {
                status: 403,
                reason: "Authorization".to_string()
            }
        );
    }

    #[test]
    fn parse_response_maps_404_to_not_found() {
        assert_eq!(
            parse_response(b"HTTP/1.1 404 Not Found\r\n\r\n"),
            DeliveryOutcome::Rejected {
                status: 404,
                reason: "NotFound".to_string()
            }
        );
    }

    #[test]
    fn parse_response_maps_429_to_rate_limit() {
        assert_eq!(
            parse_response(b"HTTP/1.1 429 Too Many Requests\r\n\r\n"),
            DeliveryOutcome::Rejected {
                status: 429,
                reason: "RateLimit".to_string()
            }
        );
    }

    #[test]
    fn parse_response_maps_5xx_to_external_provider() {
        assert_eq!(
            parse_response(b"HTTP/1.1 500 Internal Server Error\r\n\r\n"),
            DeliveryOutcome::Rejected {
                status: 500,
                reason: "ExternalProvider".to_string()
            }
        );
    }

    #[test]
    fn parse_response_malformed_no_status_line() {
        let outcome = parse_response(b"");
        match outcome {
            DeliveryOutcome::Failed { kind, .. } => {
                assert_eq!(kind, TransportFailure::ExternalProvider)
            }
            other => panic!("expected failure, got {other:?}"),
        }
    }

    #[test]
    fn parse_response_malformed_bad_status_code() {
        let outcome = parse_response(b"HTTP/1.1 ABC Nonsense\r\n\r\n");
        match outcome {
            DeliveryOutcome::Failed { kind, .. } => {
                assert_eq!(kind, TransportFailure::ExternalProvider)
            }
            other => panic!("expected failure, got {other:?}"),
        }
    }

    #[test]
    fn resolve_host_defaults_port() {
        assert_eq!(resolve_host("glitchtip.local"), "glitchtip.local:8000");
        assert_eq!(resolve_host("127.0.0.1:9000"), "127.0.0.1:9000");
    }

    #[test]
    fn host_without_port_strips_explicit_port() {
        assert_eq!(host_without_port("glitchtip.local"), "glitchtip.local");
        assert_eq!(host_without_port("glitchtip.local:443"), "glitchtip.local");
        assert_eq!(host_without_port("127.0.0.1:9000"), "127.0.0.1");
        assert_eq!(host_without_port("::1"), "::1");
    }

    /// A plain HTTP server must NOT accept an https DSN: the client
    /// speaks TLS, the server speaks plaintext, so the handshake fails
    /// and the delivery fails closed ExternalProvider. This is the
    /// AUD-055 hostile proof: an https DSN is never sent plaintext.
    #[test]
    fn https_dsn_against_plaintext_server_fails_closed_tls() {
        use std::io::Write as IoWrite;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let server = std::thread::spawn(move || {
            // Plaintext HTTP/1.1 server: it only ever sees garbage if
            // the client wrongly sends plaintext; a TLS client sends a
            // ClientHello which this server cannot parse as an HTTP
            // request line. We respond 400 to prove we were reached,
            // then the TLS handshake on the client side has already
            // failed or the client refused to send.
            if let Ok((mut stream, _)) = listener.accept() {
                let _ = stream.read(&mut [0u8; 1024]);
                let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
            }
        });

        let dsn = Dsn::parse(&format!(
            "https://0123456789abcdef0123456789abcdef@127.0.0.1:{}/42",
            addr.port()
        ))
        .expect("https dsn");
        assert!(dsn.is_https());
        let outcome = post_envelope(
            &dsn,
            "envelope",
            "X-Sentry-Auth: test",
            "application/x-sentry-envelope",
        );
        server.join().expect("server join");
        match outcome {
            DeliveryOutcome::Failed { kind, detail } => {
                assert_eq!(kind, TransportFailure::ExternalProvider);
                assert!(
                    detail.contains("tls")
                        || detail.contains("TLS")
                        || detail.contains("corrupt")
                        || detail.contains("invalid"),
                    "failure detail must evidence TLS-layer failure, got: {detail}"
                );
            }
            other => panic!("https-to-plaintext must fail closed, got {other:?}"),
        }
    }

    /// Positive TLS proof: an https DSN delivered to a REAL rustls TLS
    /// server (self-signed cert trusted via a custom root store) is
    /// Accepted. Proves the transport genuinely negotiates TLS instead
    /// of silently sending plaintext.
    #[test]
    fn https_dsn_delivers_over_real_tls_server() {
        use std::io::Write as IoWrite;
        use std::net::TcpListener;
        use std::sync::Arc;

        // Self-signed certificate + key (rcgen, same locked chain as
        // infra/pki). SAN must match the host we connect to.
        let certified_key = rcgen::KeyPair::generate().expect("keypair");
        let mut params = rcgen::CertificateParams::new(vec![]).expect("params");
        params.subject_alt_names.push(rcgen::SanType::DnsName(
            "localhost".try_into().expect("dns name"),
        ));
        params
            .subject_alt_names
            .push(rcgen::SanType::IpAddress(std::net::IpAddr::V4(
                std::net::Ipv4Addr::LOCALHOST,
            )));
        let cert = params
            .self_signed(&certified_key)
            .expect("self-signed cert");
        let cert_der = cert.der().clone();
        let key_der = rustls::pki_types::PrivateKeyDer::Pkcs8(
            rustls::pki_types::PrivatePkcs8KeyDer::from(certified_key.serialize_der()),
        );

        // rustls SERVER config.
        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der.clone()], key_der)
            .expect("server config");

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let conn =
                rustls::ServerConnection::new(Arc::new(server_config)).expect("server connection");
            let mut tls = rustls::StreamOwned::new(conn, stream);
            // Read the request line; we only need to prove bytes
            // arrived over TLS, then respond 200 and close cleanly.
            let mut buf = [0u8; 2048];
            let _ = tls.read(&mut buf);
            let _ = tls.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            tls.conn.send_close_notify();
            let _ = tls.flush();
        });

        // Client trusts the self-signed cert via a custom root store.
        let mut roots = RootCertStore::empty();
        roots.add(cert_der).expect("add root");
        let cfg = client_config(roots).expect("client config");

        let dsn = Dsn::parse(&format!(
            "https://0123456789abcdef0123456789abcdef@localhost:{}/42",
            addr.port()
        ))
        .expect("https dsn");
        let outcome = post_envelope_with_config(
            &dsn,
            "envelope-body",
            "X-Sentry-Auth: test",
            "application/x-sentry-envelope",
            Some(cfg),
        );
        server.join().expect("server join");
        assert_eq!(
            outcome,
            DeliveryOutcome::Accepted { status: 200 },
            "https DSN must deliver over real TLS"
        );
    }

    /// An https DSN with an empty host is rejected at parse time
    /// (fail closed before any connection is attempted).
    #[test]
    fn https_dsn_rejects_empty_host_at_parse() {
        let err = Dsn::parse("https://0123456789abcdef0123456789abcdef@/42").expect_err("dsn");
        assert_eq!(err.reason, "empty credential or host");
        // A valid https DSN is scheme-aware at the transport boundary.
        let dsn =
            Dsn::parse("https://0123456789abcdef0123456789abcdef@glitchtip.local/42").unwrap();
        assert!(dsn.is_https());
        assert_eq!(dsn.scheme(), "https");
    }
}
