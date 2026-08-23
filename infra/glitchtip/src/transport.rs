//! Minimal HTTP/1.1 transport over `std::net` for the
//! GlitchTip/Sentry-compatible incident sink (EP-038 M3).
//!
//! The repo precedent (EP-037 SeaweedFS connector) hand-rolls
//! SigV4/HTTP over `std::net::TcpStream`; we follow the same rule:
//! no HTTP client SDK dependency for one small POST endpoint.
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
//!
//! Every connection is fresh per request: no persistent socket state
//! is retained, so a provider restart cannot leave a stale socket.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use nexus_observability::model::short_fingerprint;

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
    /// Transport-level failure (refused / timeout / malformed).
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

/// Post one envelope to the DSN's envelope endpoint.
///
/// `authorization_header` carries the `X-Sentry-Auth` value built by
/// the caller (it includes the public key; we never log it here).
pub fn post_envelope(
    dsn: &Dsn,
    envelope: &str,
    authorization_header: &str,
    content_type: &str,
) -> DeliveryOutcome {
    let host = dsn.host().to_string();
    let path = dsn.envelope_path();

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

    let stream = match TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT) {
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

    let host_header = host.clone();
    let request = format!(
        "POST {path} HTTP/1.1\r\n\
         Host: {host_header}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         X-Sentry-Auth: {authorization_header}\r\n\
         Connection: close\r\n\
         \r\n\
         {envelope}",
        envelope.len()
    );

    let mut stream = stream;
    if let Err(e) = stream.write_all(request.as_bytes()) {
        return DeliveryOutcome::Failed {
            kind: TransportFailure::ExternalProvider,
            detail: format!("write: {e}"),
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
}
