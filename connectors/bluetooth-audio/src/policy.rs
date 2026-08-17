//! Bluetooth connect policy (SPEC-012 behavior 7, security
//! fail-closed). Default is deny; only an explicit allowlist grants
//! connect. A denied decision is a real policy failure, never a
//! fallthrough.

use std::collections::HashSet;

use nexus_audio::{AudioError, AudioErrorCode, BluetoothDeviceRef};

/// Policy decision port for Bluetooth connects.
pub trait BluetoothConnectPolicy: Send + Sync {
    fn allow_connect(&self, device: &BluetoothDeviceRef) -> Result<(), AudioError>;
}

/// Deny by default; allow only explicitly allowlisted devices.
#[derive(Debug, Default)]
pub struct DenyByDefaultPolicy {
    allowlist: HashSet<BluetoothDeviceRef>,
}

impl DenyByDefaultPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_allowed(mut self, devices: impl IntoIterator<Item = BluetoothDeviceRef>) -> Self {
        self.allowlist.extend(devices);
        self
    }
}

impl BluetoothConnectPolicy for DenyByDefaultPolicy {
    fn allow_connect(&self, device: &BluetoothDeviceRef) -> Result<(), AudioError> {
        if self.allowlist.contains(device) {
            Ok(())
        } else {
            Err(AudioError::new(
                AudioErrorCode::Policy,
                "bluetooth connect denied by policy: device not allowlisted",
                None,
                Some(Box::from(device.to_string())),
            ))
        }
    }
}
