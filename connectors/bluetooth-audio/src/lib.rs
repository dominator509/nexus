//! EP-022 Bluetooth audio connector (SPEC-012 behaviors 6, 8; node
//! contract acceptance obligation 2).
//!
//! Real provider behavior behind the `BluetoothEndpointProvider` port:
//! a minimal REAL D-Bus client probes the system bus for the org.bluez
//! name owner; when BlueZ is absent (as on this host) every operation
//! fails closed with a typed UNAVAILABLE error, an audit record, and
//! metric counters. No fabricated connectivity is ever claimed: the
//! CONNECTED state is reachable only through a real certified
//! transport, which is explicitly deferred.
//!
//! M4 is the forced-failure milestone: this crate proves that
//! dependency, policy, security, timeout, and resource faults all fail
//! safely (SPEC-012 behavior 7).
//!
//! Permanent invariants (Reality rule, SPEC-012):
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.
//! - BlueZ absence is proven by a real D-Bus call on the real system
//!   bus (GetNameOwner -> NameHasNoOwner), never assumed or simulated.
//! - A failed connect rolls back to DISCONNECTED (no partial side
//!   effect).
//! - Policy defaults to deny; only an explicit allowlist grants
//!   connect.
//! - Error and audit payloads are redacted; raw audio and credentials
//!   never enter messages or records.

#![forbid(unsafe_code)]

pub mod audit;
pub mod connector;
pub mod dbus;
pub mod policy;
pub mod probe;
pub mod state;

pub use audit::{IncidentRecord, IncidentRecorder, Metrics, MetricsSnapshot};
pub use connector::BluetoothAudioConnector;
pub use dbus::{BusError, DbusClient};
pub use policy::{BluetoothConnectPolicy, DenyByDefaultPolicy};
pub use probe::{BlueZPresence, BlueZProbe};
pub use state::ConnectorStateMachine;
