//! Device secret store (SPEC-005 behavior 6).
//!
//! Mobile and desktop use platform secure stores (Secure Enclave /
//! Keychain, Android Keystore, OS keychain). `DeviceSecretStore` is the
//! provider-neutral port: device-scoped secrets are referenced by name
//! and resolved only inside the device boundary. Connector code receives
//! references, never durable plaintext.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::TrustError;

/// A device-scoped secret reference (includes the owning device).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DeviceSecretReference {
    /// Canonical device identifier (NexusId string).
    pub device_id: String,
    /// Secret key within the device store.
    pub key: String,
}

impl DeviceSecretReference {
    /// Construct a reference; rejects empty device id/key.
    pub fn new(
        device_id: impl Into<String>,
        key: impl Into<String>,
    ) -> Result<Self, DeviceSecretStoreError> {
        let device_id = device_id.into();
        let key = key.into();
        if device_id.trim().is_empty() {
            return Err(DeviceSecretStoreError::EmptyDeviceId);
        }
        if key.trim().is_empty() {
            return Err(DeviceSecretStoreError::EmptyKey);
        }
        Ok(Self { device_id, key })
    }
}

impl fmt::Display for DeviceSecretReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "device:{}:{}", self.device_id, self.key)
    }
}

/// An opaque device secret value (redacted Debug, never serialized).
#[derive(Clone, PartialEq, Eq)]
pub struct DeviceSecretValue {
    bytes: Vec<u8>,
}

impl DeviceSecretValue {
    /// Wrap raw bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    /// The underlying bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Payload length.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Whether the payload is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl fmt::Debug for DeviceSecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DeviceSecretValue({} bytes)", self.bytes.len())
    }
}

impl Serialize for DeviceSecretValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("<redacted>")
    }
}

impl<'de> Deserialize<'de> for DeviceSecretValue {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(serde::de::Error::custom(
            "DeviceSecretValue must never be deserialized",
        ))
    }
}

/// Provider-neutral device secret store port.
pub trait DeviceSecretStore {
    /// Read a device-scoped secret.
    fn get(&self, reference: &DeviceSecretReference) -> Result<DeviceSecretValue, TrustError>;
    /// Write a device-scoped secret.
    fn put(
        &self,
        reference: &DeviceSecretReference,
        value: DeviceSecretValue,
    ) -> Result<(), TrustError>;
    /// Delete a device-scoped secret.
    fn delete(&self, reference: &DeviceSecretReference) -> Result<(), TrustError>;
}

/// Device secret store construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSecretStoreError {
    /// Device id was empty/whitespace.
    EmptyDeviceId,
    /// Key was empty/whitespace.
    EmptyKey,
}

impl fmt::Display for DeviceSecretStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::EmptyDeviceId => "device id must not be empty",
            Self::EmptyKey => "device secret key must not be empty",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for DeviceSecretStoreError {}
