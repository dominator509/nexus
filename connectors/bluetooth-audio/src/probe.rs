//! Real BlueZ presence probe on the system bus.

use std::time::Duration;

use nexus_audio::{AudioError, AudioErrorCode};

use crate::dbus::{BusError, DbusClient};

/// Default system bus address per the D-Bus specification.
pub const DEFAULT_SYSTEM_BUS_ADDRESS: &str = "unix:path=/run/dbus/system_bus_socket";

/// Honest result of probing the system bus for BlueZ.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueZPresence {
    /// org.bluez has a real owner on the bus.
    Present,
    /// org.bluez has no owner on the bus (the real forced-failure
    /// substrate on this host).
    Absent,
}

/// A probe that talks to a real bus daemon over a real Unix socket.
#[derive(Debug, Clone)]
pub struct BlueZProbe {
    address: String,
    timeout: Duration,
}

impl BlueZProbe {
    /// Use DBUS_SYSTEM_BUS_ADDRESS when set, else the standard system
    /// bus socket path.
    pub fn system_default() -> Self {
        let address = std::env::var("DBUS_SYSTEM_BUS_ADDRESS")
            .unwrap_or_else(|_| DEFAULT_SYSTEM_BUS_ADDRESS.to_string());
        Self {
            address,
            timeout: Duration::from_secs(5),
        }
    }

    /// Probe an explicit bus address (test injection and diagnostics).
    pub fn with_address(address: impl Into<String>, timeout: Duration) -> Self {
        Self {
            address: address.into(),
            timeout,
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Real probe: authenticate to the bus, verify the bus itself is
    /// alive (canary), then resolve org.bluez.
    pub fn probe(&self) -> Result<BlueZPresence, AudioError> {
        let mut client = DbusClient::connect(&self.address, self.timeout).map_err(map_bus)?;
        let canary = client
            .get_name_owner("org.freedesktop.DBus")
            .map_err(map_bus)?;
        if canary.is_empty() {
            return Err(AudioError::unavailable(
                "system bus canary returned an empty owner",
            ));
        }
        match client.get_name_owner("org.bluez") {
            Ok(owner) if !owner.is_empty() => Ok(BlueZPresence::Present),
            Ok(_) => Ok(BlueZPresence::Absent),
            Err(BusError::NameHasNoOwner) => Ok(BlueZPresence::Absent),
            Err(other) => Err(map_bus(other)),
        }
    }
}

fn map_bus(error: BusError) -> AudioError {
    match error {
        BusError::Connect(s) => AudioError::new(
            AudioErrorCode::Unavailable,
            format!("cannot connect to system bus: {s}"),
            None,
            None,
        ),
        BusError::Auth(s) => AudioError::new(
            AudioErrorCode::Authorization,
            format!("system bus authentication rejected: {s}"),
            None,
            None,
        ),
        BusError::Timeout => AudioError::new(
            AudioErrorCode::Timeout,
            "system bus probe timed out",
            None,
            None,
        ),
        BusError::NameHasNoOwner => AudioError::new(
            AudioErrorCode::NotFound,
            "name has no owner on the system bus",
            None,
            None,
        ),
        BusError::Malformed(s) => AudioError::new(
            AudioErrorCode::External,
            format!("malformed D-Bus reply: {s}"),
            None,
            None,
        ),
        BusError::Call {
            error_name,
            message,
        } => AudioError::new(
            AudioErrorCode::External,
            format!("D-Bus call error {error_name}: {message}"),
            None,
            None,
        ),
        BusError::Io(s) => AudioError::new(
            AudioErrorCode::Unavailable,
            format!("D-Bus I/O failure: {s}"),
            None,
            None,
        ),
    }
}
