//! Secret references and secret stores (SPEC-005 behavior 6).
//!
//! A `SecretReference` names a secret by store and key; the value never
//! lives in the reference. `SecretValue` is an opaque byte buffer that
//! does not Debug-print its contents (never in logs, events, prompts, or
//! model context). `SecretStore` is the provider-neutral port;
//! `BootstrapSecretStore` and `DeviceSecretStore` are separate ports for
//! cold-start and platform-secure stores.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::TrustError;
use crate::vocabulary::SecretState;

/// A reference to a secret by store and key (SPEC-005 behavior 6).
///
/// The reference NEVER contains the secret value. Connectors and agents
/// receive references, not durable plaintext; resolution happens at the
/// last responsible service.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecretReference {
    /// Store name, e.g. `openbao`, `sops`, `device:<id>`.
    pub store: String,
    /// Canonical secret key within the store.
    pub key: String,
    /// Optional version (rotation); absence means "current".
    pub version: Option<String>,
}

impl SecretReference {
    /// Construct a reference; rejects empty store/key.
    pub fn new(
        store: impl Into<String>,
        key: impl Into<String>,
        version: Option<String>,
    ) -> Result<Self, SecretReferenceError> {
        let store = store.into();
        let key = key.into();
        if store.trim().is_empty() {
            return Err(SecretReferenceError::EmptyStore);
        }
        if key.trim().is_empty() {
            return Err(SecretReferenceError::EmptyKey);
        }
        Ok(Self {
            store,
            key,
            version,
        })
    }
}

impl fmt::Display for SecretReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.version {
            Some(v) => write!(f, "{}:{}@{}", self.store, self.key, v),
            None => write!(f, "{}:{}", self.store, self.key),
        }
    }
}

/// Secret-reference construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretReferenceError {
    /// Store name was empty/whitespace.
    EmptyStore,
    /// Key was empty/whitespace.
    EmptyKey,
}

impl fmt::Display for SecretReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyStore => "secret store must not be empty",
            Self::EmptyKey => "secret key must not be empty",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for SecretReferenceError {}

/// An opaque secret value.
///
/// Deliberately does NOT implement `Debug` with content: printing a
/// `SecretValue` yields only a length marker, so secrets never leak into
/// logs, events, or model context (SPEC-005 behavior 6; SECURITY.md
/// "Secrets").
#[derive(Clone, PartialEq, Eq)]
pub struct SecretValue {
    bytes: Vec<u8>,
}

impl SecretValue {
    /// Wrap raw bytes as a secret value.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// The underlying bytes (caller responsible for zeroing where the
    /// language permits).
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Length of the secret payload.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the payload is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretValue({} bytes)", self.bytes.len())
    }
}

impl Serialize for SecretValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("<redacted>")
    }
}

impl<'de> Deserialize<'de> for SecretValue {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "SecretValue must never be deserialized",
        ))
    }
}

/// Lifecycle state of a stored secret.
pub use crate::vocabulary::SecretState as StoredSecretState;

/// Provider-neutral secret store port (SPEC-005 behavior 6).
///
/// Fail closed: missing keys, provider errors, and revoked secrets are
/// typed errors, never empty successes. The store resolves references to
/// values; the domain and model layers only ever see references.
pub trait SecretStore {
    /// Resolve a reference to its current value.
    fn get(&self, reference: &SecretReference) -> Result<SecretValue, TrustError>;
    /// Store or update a secret under a reference.
    fn put(&self, reference: &SecretReference, value: SecretValue) -> Result<(), TrustError>;
    /// Rotate to a new version.
    fn rotate(&self, reference: &SecretReference, value: SecretValue) -> Result<(), TrustError>;
    /// Revoke a secret so the reference no longer resolves.
    fn revoke(&self, reference: &SecretReference) -> Result<(), TrustError>;
    /// Current state of a secret (Active/Rotating/Revoked).
    fn state(&self, reference: &SecretReference) -> Result<SecretState, TrustError>;
}

/// Secret store construction/validation errors (canonical surface).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretStoreError {
    /// Store name was empty/whitespace.
    EmptyStoreName,
    /// Reference was malformed.
    InvalidReference,
}

impl fmt::Display for SecretStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyStoreName => "store name must not be empty",
            Self::InvalidReference => "invalid secret reference",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for SecretStoreError {}
