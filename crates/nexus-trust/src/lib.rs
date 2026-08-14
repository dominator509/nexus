//! Nexus trust domain (EP-009).
//!
//! Owns the provider-neutral trust model: secret references (never
//! values in domain records), bootstrap and device secret stores,
//! certificate authority, service identity, private mesh control, and
//! the short-lived capability token issuer (SPEC-005, SPEC-020). This
//! crate may import `nexus-domain` (typed IDs and canonical
//! vocabulary), `nexus-identity` (principals, devices, trust levels),
//! and `nexus-auth` (authentication strength) plus serde only. No
//! infrastructure, database, network, or vendor crate may be imported
//! here; the dependency-direction tests enforce this boundary.
//!
//! INV-003 + SPEC-005: no long-lived universal bearer token exists;
//! secrets are referenced by name and never enter model context;
//! services use mTLS and short-lived credentials; Headscale-compatible
//! WireGuard and standard mTLS paths coexist.

#![forbid(unsafe_code)]

pub mod bootstrap;
pub mod device;
pub mod error;
pub mod mesh;
pub mod pki;
pub mod secret;
pub mod token;
pub mod vocabulary;

pub use bootstrap::{BootstrapBundle, BootstrapSecretStore, BootstrapSecretStoreError};
pub use device::{DeviceSecretStore, DeviceSecretStoreError, DeviceSecretValue};
pub use error::{TrustError, TrustErrorCode};
pub use mesh::{MeshController, MeshControllerError, MeshNode, WireGuardConfig};
pub use pki::{
    Certificate, CertificateAuthority, CertificateAuthorityError, ServiceIdentity,
    ServiceIdentityError,
};
pub use secret::{SecretReference, SecretStore, SecretStoreError, SecretValue};
pub use token::{CapabilityToken, CapabilityTokenIssuer, CapabilityTokenIssuerError};
pub use vocabulary::{
    CertificateState, MeshNodeState, SecretState, ServiceIdentityState, TokenState, TrustZone,
    TrustZoneError,
};

#[cfg(test)]
mod lib_tests;
