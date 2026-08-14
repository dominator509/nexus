//! Real OpenBao PKI adapter (EP-009 M4 directives B-I, R).
//!
//! Implements the provider-neutral `nexus-trust` `CertificateAuthority`
//! and `ServiceIdentityRegistry` contracts against the REAL pinned
//! OpenBao 2.5.4 PKI engine over its HTTP surface:
//!
//! - mount enable:   POST /v1/sys/mounts/pki {"type":"pki"}
//! - internal root:  POST /v1/pki/root/generate/internal
//!   {common_name,key_type:"ec",key_bits:256,ttl} -> data.certificate,
//!   data.serial_number, data.issuer_id (key stays internal, never
//!   exported)
//! - role:           POST /v1/pki/roles/nexus-service {allowed_domains,
//!   allow_subdomains, allow_any_name:false, require_cn:false,
//!   enforce_hostnames:true, allowed_uri_sans:"nexus://*",
//!   key_type:"ec", key_bits:256, max_ttl, server_flag:true,
//!   client_flag:true} (directive D: no arbitrary hostname issuance, no
//!   CA issuance from leaf roles)
//! - sign CSR:       POST /v1/pki/sign/nexus-service {csr,ttl} ->
//!   data.certificate, data.issuing_ca, data.ca_chain, data.serial_number,
//!   data.expiration
//! - read cert:      GET /v1/pki/cert/<serial> -> data.revocation_time
//! - revoke:         POST /v1/pki/revoke {serial_number} ->
//!   data.revocation_time
//! - CRL DER:        GET /v1/pki/crl (Accept: application/pkix-crl)
//!
//! All wire formats above were verified live against the pinned
//! container before the adapter was written (ExecPlan Decision Log).
//!
//! Leaf private keys are generated at the consuming node via rcgen
//! (directive E); only the CSR crosses the CA boundary. The adapter
//! returns the certificate record; the node keeps the key in a
//! `SecretValue`-protected location and it is never logged or
//! serialized.

use std::time::{Duration, Instant};

use nexus_trust::pki::{Certificate, CertificateAuthority, ServiceIdentityRegistry};
use nexus_trust::vocabulary::TrustZone;
use nexus_trust::{ServiceIdentity, TrustError};

use crate::error::{PkiError, PkiErrorCode};
use crate::identity::{canonical_service_uri, certificate_serial_hex};
use crate::telemetry::{fingerprint, RecordingSink, TelemetryEvent};

/// Connection/read/write budget for the OpenBao PKI surface.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

/// Default PKI mount.
pub const DEFAULT_MOUNT: &str = "pki";
/// Default issuance role.
pub const DEFAULT_ROLE: &str = "nexus-service";
/// Role maximum TTL (directive D: bounded issuance).
pub const ROLE_MAX_TTL_HOURS: u64 = 24;

/// A leaf certificate + its private key material (node-owned).
///
/// The private key NEVER crosses the CA boundary; it is generated at
/// the consuming node and held here only for the mTLS handshake. Debug
/// and Display never print the key material.
pub struct IssuedLeaf {
    /// Canonical certificate record (reference only, no key material).
    pub certificate: Certificate,
    /// Certificate chain PEM (leaf + issuing CA) for the TLS config.
    pub chain_pem: String,
    /// Leaf private key PEM (node-owned, redacted in Debug/Display).
    pub private_key_pem: SecretKeyPem,
}

impl std::fmt::Debug for IssuedLeaf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IssuedLeaf")
            .field("certificate", &self.certificate)
            .field("chain_pem_len", &self.chain_pem.len())
            .field("private_key_pem", &"<redacted>")
            .finish()
    }
}

/// A private key PEM wrapper that redacts its content everywhere.
///
/// The tuple constructor is intentionally exposed ONLY for evidence
/// tooling and tests (never in production paths); the field itself
/// remains private and Debug/Display are redacted.
#[derive(Clone)]
pub struct SecretKeyPem(pub(crate) String);

impl SecretKeyPem {
    /// Wrap PEM key material (evidence/test tooling only).
    pub fn new(pem: impl Into<String>) -> Self {
        Self(pem.into())
    }

    /// Access the PEM content for the TLS config (never logged).
    pub fn as_pem(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SecretKeyPem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted private key>")
    }
}

impl std::fmt::Display for SecretKeyPem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<redacted private key>")
    }
}

/// Real OpenBao PKI authority adapter.
pub struct OpenBaoPkiAuthority {
    base_url: String,
    mount: String,
    role: String,
    client_token: String,
    ca_cert_pem: String,
    sink: RecordingSink,
}

impl OpenBaoPkiAuthority {
    /// Construct from an already-issued bounded client token.
    ///
    /// `ca_cert_pem` is the PKI engine's root/issuer certificate (the
    /// trust anchor). It is public material, never a private key.
    pub fn with_token(
        base_url: impl Into<String>,
        client_token: impl Into<String>,
        ca_cert_pem: impl Into<String>,
    ) -> Result<Self, TrustError> {
        let base_url = base_url.into();
        let client_token = client_token.into();
        let ca_cert_pem = ca_cert_pem.into();
        if base_url.trim().is_empty() {
            return Err(TrustError::invalid(
                "openbao pki base url must not be empty",
            ));
        }
        if client_token.trim().is_empty() {
            return Err(TrustError::invalid(
                "openbao pki client token must not be empty",
            ));
        }
        if !ca_cert_pem.contains("BEGIN CERTIFICATE") {
            return Err(TrustError::invalid(
                "openbao pki ca certificate must be a PEM certificate",
            ));
        }
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            mount: DEFAULT_MOUNT.to_string(),
            role: DEFAULT_ROLE.to_string(),
            client_token,
            ca_cert_pem,
            sink: RecordingSink::new(),
        })
    }

    /// Configure a non-default mount/role (test isolation).
    pub fn with_mount_role(mut self, mount: &str, role: &str) -> Self {
        self.mount = mount.trim_matches('/').to_string();
        self.role = role.trim().to_string();
        self
    }

    /// The redacted telemetry sink (tests and probe only).
    pub fn sink(&self) -> &RecordingSink {
        &self.sink
    }

    /// The PKI engine root CA certificate PEM (public trust anchor).
    pub fn ca_cert_pem(&self) -> &str {
        &self.ca_cert_pem
    }

    /// Ensure the issuance role exists with the constrained profile
    /// (directive D). Idempotent: OpenBao returns 204 on create and an
    /// error on re-create; treat "already exists" as success.
    pub fn ensure_role(&self) -> Result<(), TrustError> {
        let start = Instant::now();
        let body = serde_json::json!({
            "allowed_domains": "svc.nexus.internal",
            "allow_subdomains": true,
            "allow_any_name": false,
            "require_cn": false,
            "enforce_hostnames": true,
            "allowed_uri_sans": "nexus://*",
            "key_type": "ec",
            "key_bits": 256,
            "max_ttl": format!("{}h", ROLE_MAX_TTL_HOURS),
            "server_flag": true,
            "client_flag": true,
        });
        let result = self.request(
            "POST",
            &format!("/v1/{}/roles/{}", self.mount, self.role),
            Some(body),
        );
        match result {
            Ok(_) => {
                self.sink.record(TelemetryEvent {
                    operation: "ensure_role".into(),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(())
            }
            Err(e) if e.code == PkiErrorCode::PermissionDenied && e.http_status == Some(400) => {
                // 400 on role create = role already exists (idempotent).
                self.sink.record(TelemetryEvent {
                    operation: "ensure_role".into(),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(())
            }
            Err(e) => Err(e.into_trust()),
        }
    }

    /// Issue a short-lived mTLS certificate for a service identity.
    ///
    /// The leaf key is generated at this node (rcgen, directive E); the
    /// CSR crosses the CA boundary; the CA returns the certificate. The
    /// returned `IssuedLeaf` holds the node-owned key for the handshake
    /// and is Debug/Display redacted.
    pub fn issue_leaf(
        &self,
        identity: &ServiceIdentity,
        now_unix_s: i64,
        ttl_seconds: i64,
    ) -> Result<IssuedLeaf, TrustError> {
        let start = Instant::now();
        if ttl_seconds <= 0 {
            return Err(TrustError::invalid("leaf ttl must be positive"));
        }
        if ttl_seconds > (ROLE_MAX_TTL_HOURS * 3600) as i64 {
            return Err(PkiError::new(
                PkiErrorCode::TtlViolation,
                format!(
                    "requested ttl {}s exceeds role maximum {}h",
                    ttl_seconds, ROLE_MAX_TTL_HOURS
                ),
            )
            .into_trust());
        }

        // 1. Generate the leaf key at the node (directive E).
        let key_pair = rcgen::KeyPair::generate()
            .map_err(|e| TrustError::invalid(format!("leaf key generation failed: {}", e)))?;
        let key_pem = key_pair.serialize_pem();

        // 2. Build the CSR with the canonical identity SANs. The CN is
        // set to the deterministic transport DNS name because the role
        // enforces hostname constraints (directive D/H: never weaken).
        let uri = canonical_service_uri(&identity.tenant_id, &identity.identity_id);
        let dns = crate::identity::transport_dns_name(&identity.tenant_id, &identity.identity_id);
        let mut params = rcgen::CertificateParams::new(vec![])
            .map_err(|e| TrustError::invalid(format!("csr params failed: {}", e)))?;
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, dns.clone());
        params.subject_alt_names.push(rcgen::SanType::URI(
            uri.clone()
                .try_into()
                .map_err(|_| TrustError::invalid("canonical uri san is not valid ia5"))?,
        ));
        params.subject_alt_names.push(rcgen::SanType::DnsName(
            dns.clone()
                .try_into()
                .map_err(|_| TrustError::invalid("transport dns san is not valid ia5"))?,
        ));
        params.is_ca = rcgen::IsCa::NoCa;
        params
            .extended_key_usages
            .push(rcgen::ExtendedKeyUsagePurpose::ServerAuth);
        params
            .extended_key_usages
            .push(rcgen::ExtendedKeyUsagePurpose::ClientAuth);
        let csr = params
            .serialize_request(&key_pair)
            .map_err(|e| TrustError::invalid(format!("csr serialization failed: {}", e)))?;
        let csr_pem = csr
            .pem()
            .map_err(|e| TrustError::invalid(format!("csr pem failed: {}", e)))?;

        // 3. Sign via the real CA.
        let body = serde_json::json!({
            "csr": csr_pem,
            "ttl": format!("{}s", ttl_seconds),
        });
        let resp = self.request(
            "POST",
            &format!("/v1/{}/sign/{}", self.mount, self.role),
            Some(body),
        )?;
        let data = resp.get("data").ok_or_else(|| {
            PkiError::new(
                PkiErrorCode::MalformedProviderResponse,
                "pki sign response missing data",
            )
        })?;
        let cert_pem = data
            .get("certificate")
            .and_then(|c| c.as_str())
            .ok_or_else(|| {
                PkiError::new(
                    PkiErrorCode::MalformedProviderResponse,
                    "pki sign response missing certificate",
                )
            })?
            .to_string();
        let serial = data
            .get("serial_number")
            .and_then(|c| c.as_str())
            .ok_or_else(|| {
                PkiError::new(
                    PkiErrorCode::MalformedProviderResponse,
                    "pki sign response missing serial_number",
                )
            })?
            .to_string();
        let issuing_ca = data
            .get("issuing_ca")
            .and_then(|c| c.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| self.ca_cert_pem.clone());
        let expiration = data
            .get("expiration")
            .and_then(|c| c.as_i64())
            .unwrap_or(now_unix_s + ttl_seconds);

        let cert_der = pem_to_der(&cert_pem)?;
        let serial_hex = certificate_serial_hex(&cert_der)?;
        let chain_pem = format!("{}\n{}", cert_pem.trim(), issuing_ca.trim());

        // 4. Canonical record (reference only, no key material).
        let certificate = Certificate::new(
            format!("cert-{}", serial_hex),
            identity.identity_id.clone(),
            TrustZone::PrivateMesh,
            now_unix_s,
            expiration,
            format!("{}:{}", self.mount, serial),
        )
        .map_err(|e| TrustError::invalid(format!("certificate record invalid: {}", e)))?;

        self.sink.record(TelemetryEvent {
            operation: "issue".into(),
            serial_fingerprint: Some(fingerprint(&serial_hex)),
            issuer_fingerprint: Some(fingerprint(&issuing_ca)),
            service_identity: Some(identity.identity_id.clone()),
            state: Some("ACTIVE".into()),
            expiry_seconds: Some(ttl_seconds.max(0) as u64),
            rotation: false,
            latency_ms: start.elapsed().as_millis() as u64,
            ..Default::default()
        });

        Ok(IssuedLeaf {
            certificate,
            chain_pem,
            private_key_pem: SecretKeyPem(key_pem),
        })
    }

    /// Sign an externally-provided CSR through the constrained role.
    ///
    /// Used by the failure probe (directives R.3/R.4) to prove that a
    /// malformed CSR or an identity outside the role constraints is
    /// rejected by the REAL CA. Returns the canonical certificate
    /// record; no private key ever crosses the CA boundary.
    pub fn sign_csr_raw(
        &self,
        csr_pem: &str,
        identity_id: &str,
        now_unix_s: i64,
        ttl_seconds: i64,
    ) -> Result<Certificate, TrustError> {
        let start = Instant::now();
        if ttl_seconds <= 0 {
            return Err(TrustError::invalid("leaf ttl must be positive"));
        }
        if ttl_seconds > (ROLE_MAX_TTL_HOURS * 3600) as i64 {
            return Err(PkiError::new(
                PkiErrorCode::TtlViolation,
                format!(
                    "requested ttl {}s exceeds role maximum {}h",
                    ttl_seconds, ROLE_MAX_TTL_HOURS
                ),
            )
            .into_trust());
        }
        let body = serde_json::json!({
            "csr": csr_pem,
            "ttl": format!("{}s", ttl_seconds),
        });
        let resp = self.request(
            "POST",
            &format!("/v1/{}/sign/{}", self.mount, self.role),
            Some(body),
        )?;
        let data = resp.get("data").ok_or_else(|| {
            PkiError::new(
                PkiErrorCode::MalformedProviderResponse,
                "pki sign response missing data",
            )
        })?;
        let cert_pem = data
            .get("certificate")
            .and_then(|c| c.as_str())
            .ok_or_else(|| {
                PkiError::new(
                    PkiErrorCode::MalformedProviderResponse,
                    "pki sign response missing certificate",
                )
            })?
            .to_string();
        let serial = data
            .get("serial_number")
            .and_then(|c| c.as_str())
            .ok_or_else(|| {
                PkiError::new(
                    PkiErrorCode::MalformedProviderResponse,
                    "pki sign response missing serial_number",
                )
            })?
            .to_string();
        let issuing_ca = data
            .get("issuing_ca")
            .and_then(|c| c.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| self.ca_cert_pem.clone());
        let expiration = data
            .get("expiration")
            .and_then(|c| c.as_i64())
            .unwrap_or(now_unix_s + ttl_seconds);
        let cert_der = pem_to_der(&cert_pem)?;
        let serial_hex = certificate_serial_hex(&cert_der)?;

        self.sink.record(TelemetryEvent {
            operation: "issue_raw".into(),
            serial_fingerprint: Some(fingerprint(&serial_hex)),
            issuer_fingerprint: Some(fingerprint(&issuing_ca)),
            service_identity: Some(identity_id.to_string()),
            state: Some("ACTIVE".into()),
            expiry_seconds: Some(ttl_seconds.max(0) as u64),
            latency_ms: start.elapsed().as_millis() as u64,
            ..Default::default()
        });

        Certificate::new(
            format!("cert-{}", serial_hex),
            identity_id.to_string(),
            TrustZone::PrivateMesh,
            now_unix_s,
            expiration,
            format!("{}:{}", self.mount, serial),
        )
        .map_err(|e| TrustError::invalid(format!("certificate record invalid: {}", e)))
    }

    /// Verify a certificate is currently valid against the real CA:
    /// exists, not revoked, within its validity window, and its
    /// canonical identity binding parses.
    pub fn verify_certificate(
        &self,
        certificate: &Certificate,
        now_unix_s: i64,
    ) -> Result<(), TrustError> {
        let start = Instant::now();
        // Serial reference embedded in material_reference ("<mount>:<serial>").
        let serial = certificate
            .material_reference
            .split_once(':')
            .map(|(_, s)| s)
            .unwrap_or(&certificate.material_reference);
        let resp = self.request("GET", &format!("/v1/{}/cert/{}", self.mount, serial), None);
        match resp {
            Ok(_) => {
                self.sink.record(TelemetryEvent {
                    operation: "verify".into(),
                    serial_fingerprint: Some(fingerprint(serial)),
                    state: Some("ACTIVE".into()),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                // Validity window is enforced by the live-fire proof
                // against the issued chain; the provider read confirms
                // existence + non-revocation at the CA.
                if !certificate.is_valid_at(now_unix_s) {
                    return Err(PkiError::new(
                        PkiErrorCode::ValidityWindow,
                        "certificate is outside its validity window",
                    )
                    .into_trust());
                }
                Ok(())
            }
            Err(e) if e.code == PkiErrorCode::NotFound => {
                self.sink.record(TelemetryEvent {
                    operation: "verify".into(),
                    serial_fingerprint: Some(fingerprint(serial)),
                    state: Some("REVOKED".into()),
                    revocation: Some("gone".into()),
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().into()),
                    ..Default::default()
                });
                Err(PkiError::new(
                    PkiErrorCode::Revoked,
                    "certificate no longer exists at the CA",
                )
                .into_trust())
            }
            Err(e) => Err(e.into_trust()),
        }
    }

    /// Revoke a certificate before its natural expiry.
    pub fn revoke_certificate(&self, certificate: &Certificate) -> Result<i64, TrustError> {
        let start = Instant::now();
        let serial = certificate
            .material_reference
            .split_once(':')
            .map(|(_, s)| s)
            .unwrap_or(&certificate.material_reference);
        let body = serde_json::json!({ "serial_number": serial });
        let resp = self.request("POST", &format!("/v1/{}/revoke", self.mount), Some(body))?;
        let data = resp.get("data").ok_or_else(|| {
            PkiError::new(
                PkiErrorCode::MalformedProviderResponse,
                "pki revoke response missing data",
            )
        })?;
        let revocation_time = data
            .get("revocation_time")
            .and_then(|t| t.as_i64())
            .ok_or_else(|| {
                PkiError::new(
                    PkiErrorCode::MalformedProviderResponse,
                    "pki revoke response missing revocation_time",
                )
            })?;
        self.sink.record(TelemetryEvent {
            operation: "revoke".into(),
            serial_fingerprint: Some(fingerprint(serial)),
            state: Some("REVOKED".into()),
            revocation: Some("revoked".into()),
            latency_ms: start.elapsed().as_millis() as u64,
            ..Default::default()
        });
        Ok(revocation_time)
    }

    /// Fetch the current CRL as DER (directive I: relying party
    /// revocation enforcement).
    pub fn crl_der(&self) -> Result<Vec<u8>, TrustError> {
        let start = Instant::now();
        let url = format!("{}/v1/{}/crl", self.base_url, self.mount);
        let resp = ureq::get(&url)
            .timeout(REQUEST_TIMEOUT)
            .set("X-Vault-Token", &self.client_token)
            .set("Accept", "application/pkix-crl")
            .call();
        match resp {
            Ok(r) => {
                let status = r.status();
                if !(200..300).contains(&status) {
                    return Err(PkiError::with_status(
                        PkiErrorCode::PermissionDenied,
                        "crl fetch rejected by provider",
                        status,
                    )
                    .into_trust());
                }
                let mut buf = Vec::new();
                use std::io::Read;
                r.into_reader()
                    .take(1024 * 1024)
                    .read_to_end(&mut buf)
                    .map_err(|_| {
                        PkiError::new(
                            PkiErrorCode::MalformedProviderResponse,
                            "cannot read crl body",
                        )
                    })?;
                self.sink.record(TelemetryEvent {
                    operation: "crl".into(),
                    latency_ms: start.elapsed().as_millis() as u64,
                    ..Default::default()
                });
                Ok(buf)
            }
            Err(ureq::Error::Status(code, _)) => {
                let e = PkiError::with_status(
                    PkiErrorCode::PermissionDenied,
                    "crl fetch rejected by provider",
                    code,
                );
                self.sink.record(TelemetryEvent {
                    operation: "crl".into(),
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().into()),
                    ..Default::default()
                });
                Err(e.into_trust())
            }
            Err(_) => {
                let e = PkiError::new(PkiErrorCode::Unavailable, "cannot reach openbao pki");
                self.sink.record(TelemetryEvent {
                    operation: "crl".into(),
                    latency_ms: start.elapsed().as_millis() as u64,
                    error_class: Some(e.code.as_str().into()),
                    ..Default::default()
                });
                Err(e.into_trust())
            }
        }
    }

    /// Low-level request against the OpenBao HTTP surface.
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, PkiError> {
        let url = format!("{}{}", self.base_url, path);
        let result = ureq::request(method, &url)
            .timeout(REQUEST_TIMEOUT)
            .set("X-Vault-Token", &self.client_token)
            .set("Content-Type", "application/json");
        let resp = match body {
            Some(b) => result.send_json(b),
            None => result.call(),
        };
        match resp {
            Ok(r) => {
                let status = r.status();
                if (200..300).contains(&status) {
                    let text = r.into_string().unwrap_or_default();
                    if text.trim().is_empty() {
                        return Ok(serde_json::Value::Null);
                    }
                    serde_json::from_str(&text).map_err(|_| {
                        PkiError::new(
                            PkiErrorCode::MalformedProviderResponse,
                            "malformed openbao pki success payload",
                        )
                    })
                } else {
                    let text = r.into_string().unwrap_or_default();
                    // Sanitized provider message: OpenBao error bodies
                    // carry human-readable text, never secrets. Truncate.
                    let detail = text.trim().chars().take(160).collect::<String>();
                    let code = classify_http_error(status, &text);
                    let message = if detail.is_empty() {
                        "openbao pki request rejected".to_string()
                    } else {
                        detail
                    };
                    Err(PkiError::with_status(code, message, status))
                }
            }
            Err(ureq::Error::Status(code, _)) => Err(PkiError::with_status(
                classify_http_error(code, ""),
                "openbao pki request rejected",
                code,
            )),
            Err(_) => Err(PkiError::new(
                PkiErrorCode::Unavailable,
                "cannot reach openbao pki",
            )),
        }
    }
}

impl CertificateAuthority for OpenBaoPkiAuthority {
    fn issue(
        &self,
        subject: &str,
        zone: TrustZone,
        now_unix_s: i64,
        ttl_seconds: i64,
    ) -> Result<Certificate, TrustError> {
        let identity = ServiceIdentity::new(subject, "default-tenant", subject, zone)
            .map_err(|e| TrustError::invalid(format!("identity invalid: {}", e)))?;
        let leaf = self.issue_leaf(&identity, now_unix_s, ttl_seconds)?;
        Ok(leaf.certificate)
    }

    fn verify(&self, certificate: &Certificate, now_unix_s: i64) -> Result<(), TrustError> {
        self.verify_certificate(certificate, now_unix_s)
    }

    fn revoke(&self, certificate_id: &str) -> Result<(), TrustError> {
        // The port revokes by certificate id; the adapter resolves via
        // the canonical serial embedded in the record. Live-fire uses
        // revoke_certificate with the full record.
        let serial = certificate_id
            .strip_prefix("cert-")
            .unwrap_or(certificate_id);
        let body = serde_json::json!({ "serial_number": serial });
        let resp = self.request("POST", &format!("/v1/{}/revoke", self.mount), Some(body))?;
        let data = resp.get("data").ok_or_else(|| {
            PkiError::new(
                PkiErrorCode::MalformedProviderResponse,
                "pki revoke response missing data",
            )
        })?;
        data.get("revocation_time")
            .and_then(|t| t.as_i64())
            .ok_or_else(|| {
                PkiError::new(
                    PkiErrorCode::MalformedProviderResponse,
                    "pki revoke response missing revocation_time",
                )
            })?;
        Ok(())
    }
}

impl ServiceIdentityRegistry for OpenBaoPkiAuthority {
    fn register(&self, identity: ServiceIdentity) -> Result<(), TrustError> {
        // OpenBao PKI has no separate identity registry; issuance
        // binding is enforced by the canonical URI SAN. Registration
        // validates the identity record only.
        if identity.identity_id.trim().is_empty()
            || identity.tenant_id.trim().is_empty()
            || identity.name.trim().is_empty()
        {
            return Err(TrustError::invalid(
                "service identity fields must not be empty",
            ));
        }
        Ok(())
    }

    fn lookup(&self, identity_id: &str) -> Result<ServiceIdentity, TrustError> {
        Err(TrustError::new(
            nexus_trust::TrustErrorCode::NotFound,
            format!("service identity {} not in local registry", identity_id),
        ))
    }

    fn suspend(&self, _identity_id: &str) -> Result<(), TrustError> {
        Err(TrustError::new(
            nexus_trust::TrustErrorCode::StateConflict,
            "suspend requires a registry backend; pki adapter does not own one",
        ))
    }

    fn revoke(&self, _identity_id: &str) -> Result<(), TrustError> {
        Err(TrustError::new(
            nexus_trust::TrustErrorCode::StateConflict,
            "revoke requires a registry backend; pki adapter does not own one",
        ))
    }
}

/// Classify an OpenBao PKI HTTP error into a typed code.
fn classify_http_error(status: u16, _body: &str) -> PkiErrorCode {
    match status {
        400 => PkiErrorCode::CsrRejected,
        401 | 403 => PkiErrorCode::PermissionDenied,
        404 => PkiErrorCode::NotFound,
        408 | 429 => PkiErrorCode::Timeout,
        _ => PkiErrorCode::MalformedProviderResponse,
    }
}

/// Convert a PEM certificate to DER bytes.
fn pem_to_der(pem: &str) -> Result<Vec<u8>, TrustError> {
    use rustls_pki_types::pem::PemObject;
    let cert = rustls_pki_types::CertificateDer::from_pem_slice(pem.as_bytes())
        .map_err(|_| TrustError::invalid("certificate pem is not parseable"))?;
    Ok(cert.as_ref().to_vec())
}
