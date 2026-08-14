//! Bootstrap secret store (SPEC-005 behavior 6; SOPS + age).
//!
//! The bootstrap path protects cold-start configuration with SOPS and
//! age before OpenBao is available. `BootstrapSecretStore` is the
//! provider-neutral port: it loads a sealed bootstrap envelope and
//! resolves the small set of initial secrets (age identity, database
//! password, OpenBao unseal material, service credentials). The adapter
//! (infra/openbao, M2) implements real SOPS+age decryption; this module
//! owns the contract and the typed error surface.

use serde::{Deserialize, Serialize};

use crate::TrustError;
use crate::secret::SecretReference;

/// A sealed bootstrap envelope reference.
///
/// Points at a SOPS-encrypted file by path plus the age identity
/// reference needed to open it. The envelope never contains plaintext.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapBundle {
    /// Path of the sealed SOPS file (e.g. `config/sops/bootstrap.yaml`).
    pub sealed_path: String,
    /// Reference to the age identity secret used to open the envelope.
    pub age_identity: SecretReference,
    /// Secret references declared in the bootstrap envelope.
    pub secrets: Vec<SecretReference>,
}

impl BootstrapBundle {
    /// Construct a bundle; rejects empty path.
    pub fn new(
        sealed_path: impl Into<String>,
        age_identity: SecretReference,
        secrets: Vec<SecretReference>,
    ) -> Result<Self, BootstrapSecretStoreError> {
        let sealed_path = sealed_path.into();
        if sealed_path.trim().is_empty() {
            return Err(BootstrapSecretStoreError::EmptyPath);
        }
        Ok(Self {
            sealed_path,
            age_identity,
            secrets,
        })
    }
}

/// Provider-neutral bootstrap secret store port.
pub trait BootstrapSecretStore {
    /// Load the sealed envelope and return its declared references.
    fn load(&self, bundle: &BootstrapBundle) -> Result<Vec<SecretReference>, TrustError>;
    /// Resolve one bootstrap secret by reference.
    fn get(
        &self,
        bundle: &BootstrapBundle,
        reference: &SecretReference,
    ) -> Result<Vec<u8>, TrustError>;
}

/// Bootstrap store construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapSecretStoreError {
    /// Sealed path was empty/whitespace.
    EmptyPath,
}

impl std::fmt::Display for BootstrapSecretStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("bootstrap sealed path must not be empty")
    }
}

impl std::error::Error for BootstrapSecretStoreError {}
