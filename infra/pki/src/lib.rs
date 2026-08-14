//! EP-009 M4: certificate authority, service identity, and real mTLS.
//!
//! Responsibility split (directive A):
//! - OpenBao PKI engine: CA/issuer authority, issuance, policy, serial
//!   tracking, revocation, CRL.
//! - `nexus-trust`: provider-neutral `CertificateAuthority`,
//!   `ServiceIdentity`, certificate lifecycle state, canonical identity
//!   binding.
//! - this crate (`nexus-pki`): concrete PKI adapter (OpenBao PKI over
//!   HTTP), certificate parsing/validation integration, mTLS
//!   configuration helpers, and revocation verifier/cache.
//! - Headscale: mesh membership/connectivity only. A machine on the
//!   private mesh is NOT a Nexus service identity; possession of a
//!   certificate does NOT grant a Nexus capability (directive A/O/P).
//!
//! Canonical identity binding (directive C, ADR-014): every issued
//! certificate carries the deterministic URI SAN
//! `nexus://tenant/<tenant_id>/service/<identity_id>` derived from the
//! `ServiceIdentity` contract. mTLS validates the transport DNS SAN via
//! rustls; the Nexus identity layer binds the authenticated peer to the
//! canonical URI SAN and fails closed on mismatch.

pub mod ca;
pub mod error;
pub mod identity;
pub mod mtls;
pub mod telemetry;

#[cfg(test)]
mod lib_tests;

pub use ca::OpenBaoPkiAuthority;
pub use ca::SecretKeyPem;
pub use error::{PkiError, PkiErrorCode};
pub use identity::{canonical_service_uri, parse_certificate_identity, ServiceIdentityBinding};
pub use mtls::{
    client_config, revocation_verifier, server_config, MtlsHandshake, PeerIdentity,
    RevocationVerifier,
};
pub use telemetry::{fingerprint, RecordingSink, TelemetryEvent};
