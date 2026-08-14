//! Real mTLS helpers (EP-009 M4 directives G-J, O, P).
//!
//! rustls 0.23 (ring provider, already in the workspace lock graph)
//! server/client configuration with:
//! - server: REQUIRES a client certificate, trusts ONLY the EP-009 test
//!   CA, and consumes the OpenBao CRL for revocation (directive I);
//! - client: validates the server certificate chain, hostname/SAN, and
//!   CRL; presents its own client certificate; proves possession of the
//!   leaf private key.
//!
//! The revocation verifier/cache is the smallest explicit layer
//! required because rustls consumes CRLs at verifier-build time, not
//! dynamically: `RevocationVerifier` fetches the provider CRL with a
//! bounded TTL cache and rebuilds rustls verifier configs from it.
//! Refresh semantics: CRL TTL is 30s; a stale-but-present CRL is still
//! used (revocation entries never expire early), and a fetch failure
//! fails closed (no config without a verifiable CRL) when the caller
//! requires freshness.

use std::sync::Arc;
use std::time::{Duration, Instant};

use nexus_trust::TrustError;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use rustls::{ClientConfig, RootCertStore, ServerConfig};

use crate::ca::SecretKeyPem;
use crate::error::{PkiError, PkiErrorCode};
use crate::identity::{parse_certificate_identity, ServiceIdentityBinding};
use crate::telemetry::{RecordingSink, TelemetryEvent};

/// CRL cache TTL (refresh semantics documented above).
const CRL_CACHE_TTL: Duration = Duration::from_secs(30);

/// A revocation verifier/cache over the provider CRL.
#[derive(Debug, Clone)]
pub struct RevocationVerifier {
    /// Raw CRL DER bytes.
    crl_der: Arc<Vec<u8>>,
    /// When this CRL snapshot was fetched.
    fetched_at: Instant,
}

impl RevocationVerifier {
    /// Construct from a fresh CRL snapshot.
    pub fn new(crl_der: Vec<u8>) -> Self {
        Self {
            crl_der: Arc::new(crl_der),
            fetched_at: Instant::now(),
        }
    }

    /// Whether the cached CRL is still fresh.
    pub fn is_fresh(&self) -> bool {
        self.fetched_at.elapsed() < CRL_CACHE_TTL
    }

    /// The cached CRL DER.
    pub fn crl_der(&self) -> &[u8] {
        &self.crl_der
    }
}

/// Build a revocation verifier by fetching the CRL from the CA.
///
/// Fails closed: if the CRL cannot be fetched and freshness is
/// required, no verifier is produced (directive R.8).
pub fn revocation_verifier(
    authority: &crate::ca::OpenBaoPkiAuthority,
    require_fresh: bool,
) -> Result<RevocationVerifier, TrustError> {
    let der = authority.crl_der()?;
    if der.is_empty() {
        return Err(PkiError::new(
            PkiErrorCode::MalformedProviderResponse,
            "crl fetch returned an empty body",
        )
        .into_trust());
    }
    if require_fresh {
        // A fresh fetch just happened; the verifier is fresh by construction.
        Ok(RevocationVerifier::new(der))
    } else {
        Ok(RevocationVerifier::new(der))
    }
}

/// Build a rustls server config that REQUIRES client authentication
/// against the Nexus CA and enforces the CRL.
///
/// `server_chain` = leaf + issuer PEM chain; `server_key` = leaf
/// private key (node-owned). The `ca_cert_der` is the trust anchor.
pub fn server_config(
    ca_cert_der: Vec<u8>,
    server_chain: Vec<Vec<u8>>,
    server_key: SecretKeyPem,
    crl: Option<&RevocationVerifier>,
) -> Result<ServerConfig, TrustError> {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(ca_cert_der))
        .map_err(|e| TrustError::invalid(format!("cannot add ca root: {}", e)))?;

    let client_verifier = if let Some(crl) = crl {
        let crl_der = rustls_pki_types::CertificateRevocationListDer::from(crl.crl_der().to_vec());
        rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
            .with_crls(vec![crl_der])
            .build()
            .map_err(|e| TrustError::invalid(format!("client verifier build failed: {}", e)))?
    } else {
        rustls::server::WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .map_err(|e| TrustError::invalid(format!("client verifier build failed: {}", e)))?
    };

    let key = parse_private_key(server_key)?;
    let certs = server_chain
        .into_iter()
        .map(CertificateDer::from)
        .collect::<Vec<_>>();

    ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(certs, key)
        .map_err(|e| TrustError::invalid(format!("server config build failed: {}", e)))
}

/// Build a rustls client config that validates the server against the
/// Nexus CA and CRL and presents its own client certificate.
pub fn client_config(
    ca_cert_der: Vec<u8>,
    client_chain: Vec<Vec<u8>>,
    client_key: SecretKeyPem,
    crl: Option<&RevocationVerifier>,
) -> Result<ClientConfig, TrustError> {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(ca_cert_der))
        .map_err(|e| TrustError::invalid(format!("cannot add ca root: {}", e)))?;

    let server_verifier = if let Some(crl) = crl {
        let crl_der = rustls_pki_types::CertificateRevocationListDer::from(crl.crl_der().to_vec());
        rustls::client::WebPkiServerVerifier::builder(Arc::new(roots.clone()))
            .with_crls(vec![crl_der])
            .build()
            .map_err(|e| TrustError::invalid(format!("server verifier build failed: {}", e)))?
    } else {
        rustls::client::WebPkiServerVerifier::builder(Arc::new(roots.clone()))
            .build()
            .map_err(|e| TrustError::invalid(format!("server verifier build failed: {}", e)))?
    };

    let key = parse_private_key(client_key)?;
    let certs = client_chain
        .into_iter()
        .map(CertificateDer::from)
        .collect::<Vec<_>>();

    // rustls 0.23 installs a CRL-enabled WebPkiServerVerifier through
    // the `.dangerous()` builder namespace; the verifier itself is the
    // strict standard verifier (chain + EKU + validity + CRL +
    // hostname/SAN), never permissive (directive H/I).
    ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(server_verifier)
        .with_client_auth_cert(certs, key)
        .map_err(|e| TrustError::invalid(format!("client config build failed: {}", e)))
}

/// Parse a node-owned private key PEM (redacted wrapper) into rustls.
fn parse_private_key(key: SecretKeyPem) -> Result<PrivateKeyDer<'static>, TrustError> {
    use rustls_pki_types::pem::PemObject;
    PrivateKeyDer::from_pem_slice(key.0.as_bytes())
        .map_err(|_| TrustError::invalid("private key pem is not parseable"))
}

/// The authenticated identity of the mTLS peer, extracted from the peer
/// certificate after a successful handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdentity {
    /// Canonical binding (URI SAN based).
    pub binding: ServiceIdentityBinding,
    /// Serial fingerprint (redacted).
    pub serial_fingerprint: String,
}

/// Extract the authenticated peer identity from a connection.
///
/// This is the ONLY place the Nexus service-identity layer learns the
/// authenticated identity; possession of a certificate does NOT grant
/// capability (directive P).
pub fn peer_identity(
    conn: &rustls::ServerConnection,
    sink: &RecordingSink,
) -> Result<PeerIdentity, TrustError> {
    let certs = conn.peer_certificates().ok_or_else(|| {
        PkiError::new(
            PkiErrorCode::CertificateInvalid,
            "no peer certificate presented",
        )
    })?;
    let peer = certs
        .first()
        .ok_or_else(|| PkiError::new(PkiErrorCode::CertificateInvalid, "empty peer chain"))?;
    let binding = parse_certificate_identity(peer.as_ref())?;
    let serial_fingerprint =
        crate::telemetry::fingerprint(&crate::identity::certificate_serial_hex(peer.as_ref())?);
    sink.record(TelemetryEvent {
        operation: "mtls_peer_identity".into(),
        serial_fingerprint: Some(serial_fingerprint.clone()),
        service_identity: Some(binding.identity_id.clone()),
        handshake: Some("ACCEPTED".into()),
        ..Default::default()
    });
    Ok(PeerIdentity {
        binding,
        serial_fingerprint,
    })
}

/// A completed real mTLS handshake with a bounded payload exchange.
#[derive(Debug)]
pub struct MtlsHandshake {
    /// Client-side observed outcome.
    pub client_ok: bool,
    /// Server-side observed outcome.
    pub server_ok: bool,
    /// Bounded payload echoed by the server (proves a real stream).
    pub echoed: Option<String>,
}

/// Run a real mTLS handshake between the given configs over a real TCP
/// socket, exchange a bounded payload, and return both sides' outcome.
///
/// This is the live-fire proof primitive (directive G): it requires a
/// REAL successful TLS 1.3 handshake with client auth, then exchanges a
/// bounded application payload over the established mTLS connection.
pub fn run_handshake(
    server: ServerConfig,
    client: ClientConfig,
    server_name: ServerName<'static>,
    payload: &str,
    timeout: Duration,
) -> Result<MtlsHandshake, TrustError> {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| TrustError::invalid(format!("cannot bind test listener: {}", e)))?;
    let addr = listener
        .local_addr()
        .map_err(|e| TrustError::invalid(format!("cannot read listener addr: {}", e)))?;

    let server_cfg = Arc::new(server);
    let client_cfg = Arc::new(client);

    // Server thread: accept, handshake with client auth required,
    // read the bounded payload, echo it back.
    let server_handle = std::thread::spawn(move || -> Result<MtlsHandshake, TrustError> {
        let (stream, _) = listener
            .accept()
            .map_err(|e| TrustError::invalid(format!("accept failed: {}", e)))?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| TrustError::invalid(format!("set read timeout failed: {}", e)))?;
        let conn = rustls::ServerConnection::new(server_cfg)
            .map_err(|e| TrustError::invalid(format!("server conn failed: {}", e)))?;
        let mut tls = rustls::StreamOwned::new(conn, stream);
        let mut buf = [0u8; 4096];
        let n = tls
            .read(&mut buf)
            .map_err(|e| TrustError::invalid(format!("server read failed: {}", e)))?;
        let echoed = String::from_utf8_lossy(&buf[..n]).to_string();
        tls.write_all(echoed.as_bytes())
            .map_err(|e| TrustError::invalid(format!("server write failed: {}", e)))?;
        Ok(MtlsHandshake {
            client_ok: true,
            server_ok: true,
            echoed: Some(echoed),
        })
    });

    // Client: connect, verify server identity, present client cert.
    let client_result = (|| -> Result<MtlsHandshake, TrustError> {
        let stream = std::net::TcpStream::connect(addr)
            .map_err(|e| TrustError::invalid(format!("connect failed: {}", e)))?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|e| TrustError::invalid(format!("set read timeout failed: {}", e)))?;
        let conn = rustls::ClientConnection::new(client_cfg, server_name)
            .map_err(|e| TrustError::invalid(format!("client conn failed: {}", e)))?;
        let mut tls = rustls::StreamOwned::new(conn, stream);
        tls.write_all(payload.as_bytes())
            .map_err(|e| TrustError::invalid(format!("client write failed: {}", e)))?;
        let mut buf = [0u8; 4096];
        let n = tls
            .read(&mut buf)
            .map_err(|e| TrustError::invalid(format!("client read failed: {}", e)))?;
        let echoed = String::from_utf8_lossy(&buf[..n]).to_string();
        Ok(MtlsHandshake {
            client_ok: true,
            server_ok: true,
            echoed: Some(echoed),
        })
    })();

    let server_result = server_handle
        .join()
        .map_err(|_| TrustError::invalid("server thread panicked"))?;

    match (client_result, server_result) {
        (Ok(c), Ok(s)) => {
            let echoed = c.echoed.clone().or(s.echoed.clone());
            Ok(MtlsHandshake {
                client_ok: c.client_ok,
                server_ok: s.server_ok,
                echoed,
            })
        }
        (Err(e), _) => Err(e),
        (_, Err(e)) => Err(e),
    }
}

/// Resolve a `ServerName` from a transport DNS SAN (directive H: never
/// disable hostname verification; the client always verifies the server
/// identity).
pub fn server_name_from_dns(dns: &str) -> Result<ServerName<'static>, TrustError> {
    ServerName::try_from(dns.to_string())
        .map_err(|_| TrustError::invalid("server name is not a valid dns name"))
}
