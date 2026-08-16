//! EP-022 Bluetooth endpoint port (SPEC-012 behaviors 6, 8).
//!
//! Reconnect and endpoint transfer must preserve conversation context;
//! the provider port exposes connect/disconnect/reconnect and carries
//! a stable device reference so context survives reconnects.

use std::fmt;

use crate::error::AudioError;

/// Typed Bluetooth device reference (stable across reconnects).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BluetoothDeviceRef(String);

impl BluetoothDeviceRef {
    pub fn new(value: impl Into<String>) -> Result<Self, AudioError> {
        let value = value.into();
        if value.is_empty() || value.len() > 128 {
            return Err(AudioError::new(
                crate::error::AudioErrorCode::Validation,
                "bluetooth device ref must be 1..=128 characters",
                None,
                None,
            ));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BluetoothDeviceRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Canonical Bluetooth endpoint state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BluetoothState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

impl BluetoothState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Disconnected => "DISCONNECTED",
            Self::Connecting => "CONNECTING",
            Self::Connected => "CONNECTED",
            Self::Reconnecting => "RECONNECTING",
        }
    }
}

/// Bluetooth endpoint provider port (fail-closed).
pub trait BluetoothEndpointProvider {
    fn connect(&self, device: &BluetoothDeviceRef) -> Result<(), AudioError> {
        let _ = device;
        Err(AudioError::unavailable(
            "bluetooth provider has no implementation bound",
        ))
    }

    fn disconnect(&self, device: &BluetoothDeviceRef) -> Result<(), AudioError> {
        let _ = device;
        Err(AudioError::unavailable(
            "bluetooth provider has no implementation bound",
        ))
    }

    fn state(&self, device: &BluetoothDeviceRef) -> Result<BluetoothState, AudioError> {
        let _ = device;
        Err(AudioError::unavailable(
            "bluetooth provider has no implementation bound",
        ))
    }
}
