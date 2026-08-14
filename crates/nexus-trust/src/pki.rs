//! Certificate authority and service identity (SPEC-005 behavior 7).
//!
//! Services authenticate each other with mTLS using short-lived
//! certificates issued by the Nexus certificate authority. `Certificate`
//! records a short-lived mTLS certificate; `ServiceIdentity` is the
//! canonical service principal bound to issued certificates. The CA port
//! issues, verifies, and revokes certificates; the service identity port
//! registers and rotates service identities.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::TrustError;
use crate::vocabulary::{CertificateState, ServiceIdentityState, TrustZone};

/// A short-lived mTLS certificate record.
///
/// The record references the certificate material; it never embeds
/// private keys (secrets stay in the store, referenced by name).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Certificate {
    /// Certificate identifier.
    pub certificate_id: String,
    /// Subject service identity.
    pub subject: String,
    /// Trust zone the certificate is valid in.
    pub zone: TrustZone,
    /// Not-before time, unix seconds.
    pub not_before_unix_s: i64,
    /// Not-after time, unix seconds (short-lived).
    pub not_after_unix_s: i64,
    /// Reference to the certificate material in the secret store.
    pub material_reference: String,
    /// Current certificate state.
    pub state: CertificateState,
}

impl Certificate {
    /// Construct a certificate; rejects empty fields and inverted times.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        certificate_id: impl Into<String>,
        subject: impl Into<String>,
        zone: TrustZone,
        not_before_unix_s: i64,
        not_after_unix_s: i64,
        material_reference: impl Into<String>,
    ) -> Result<Self, CertificateAuthorityError> {
        let certificate_id = certificate_id.into();
        let subject = subject.into();
        let material_reference = material_reference.into();
        if certificate_id.trim().is_empty()
            || subject.trim().is_empty()
            || material_reference.trim().is_empty()
        {
            return Err(CertificateAuthorityError::EmptyField);
        }
        if not_after_unix_s <= not_before_unix_s {
            return Err(CertificateAuthorityError::InvertedTimes);
        }
        Ok(Self {
            certificate_id,
            subject,
            zone,
            not_before_unix_s,
            not_after_unix_s,
            material_reference,
            state: CertificateState::Active,
        })
    }

    /// Whether the certificate is valid at a time (active, in window).
    pub fn is_valid_at(&self, now_unix_s: i64) -> bool {
        self.state == CertificateState::Active
            && now_unix_s >= self.not_before_unix_s
            && now_unix_s < self.not_after_unix_s
    }

    /// Mark revoked (idempotent).
    pub fn revoke(&mut self) {
        self.state = CertificateState::Revoked;
    }
}

/// A service identity (SPEC-005 canonical term).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceIdentity {
    /// Service identity identifier.
    pub identity_id: String,
    /// Tenant boundary.
    pub tenant_id: String,
    /// Canonical service name.
    pub name: String,
    /// Trust zone the service belongs to.
    pub zone: TrustZone,
    /// Current identity state.
    pub state: ServiceIdentityState,
}

impl ServiceIdentity {
    /// Construct an identity; rejects empty fields.
    pub fn new(
        identity_id: impl Into<String>,
        tenant_id: impl Into<String>,
        name: impl Into<String>,
        zone: TrustZone,
    ) -> Result<Self, ServiceIdentityError> {
        let identity_id = identity_id.into();
        let tenant_id = tenant_id.into();
        let name = name.into();
        if identity_id.trim().is_empty() || tenant_id.trim().is_empty() || name.trim().is_empty() {
            return Err(ServiceIdentityError::EmptyField);
        }
        Ok(Self {
            identity_id,
            tenant_id,
            name,
            zone,
            state: ServiceIdentityState::Active,
        })
    }
}

/// Provider-neutral certificate authority port.
pub trait CertificateAuthority {
    /// Issue a short-lived mTLS certificate for a service identity.
    fn issue(
        &self,
        subject: &str,
        zone: TrustZone,
        now_unix_s: i64,
        ttl_seconds: i64,
    ) -> Result<Certificate, TrustError>;
    /// Verify a certificate is currently valid.
    fn verify(&self, certificate: &Certificate, now_unix_s: i64) -> Result<(), TrustError>;
    /// Revoke a certificate before its natural expiry.
    fn revoke(&self, certificate_id: &str) -> Result<(), TrustError>;
}

/// Provider-neutral service identity port.
pub trait ServiceIdentityRegistry {
    /// Register a new service identity.
    fn register(&self, identity: ServiceIdentity) -> Result<(), TrustError>;
    /// Look up a service identity by id.
    fn lookup(&self, identity_id: &str) -> Result<ServiceIdentity, TrustError>;
    /// Suspend (stop new issuance without destroying the record).
    fn suspend(&self, identity_id: &str) -> Result<(), TrustError>;
    /// Revoke (terminate) a service identity.
    fn revoke(&self, identity_id: &str) -> Result<(), TrustError>;
}

/// Certificate construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateAuthorityError {
    /// A required field was empty/whitespace.
    EmptyField,
    /// Not-after is not after not-before.
    InvertedTimes,
}

impl fmt::Display for CertificateAuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyField => "certificate fields must not be empty",
            Self::InvertedTimes => "certificate not_after must be after not_before",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for CertificateAuthorityError {}

/// Service identity construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceIdentityError {
    /// A required field was empty/whitespace.
    EmptyField,
}

impl fmt::Display for ServiceIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("service identity fields must not be empty")
    }
}

impl std::error::Error for ServiceIdentityError {}
