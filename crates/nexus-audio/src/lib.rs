//! EP-022 nexus-audio: voice satellites, Bluetooth endpoints, AEC,
//! endpoint transfer, and room routing (SPEC-012).
//!
//! The crate owns the provider-neutral audio endpoint contracts. Domain
//! rules are pure; provider behavior plugs in behind the port traits
//! (Assist satellite, Wyoming, Bluetooth) and is never certified until
//! real provider or hardware evidence exists (Reality rule).

#![forbid(unsafe_code)]

pub mod aec;
pub mod bluetooth;
pub mod endpoint;
pub mod error;
pub mod router;
pub mod satellite;
pub mod transfer;
pub mod vocabulary;

pub use aec::{AecProfile, EchoCancellationProfile};
pub use bluetooth::{BluetoothDeviceRef, BluetoothEndpointProvider, BluetoothState};
pub use endpoint::{AudioEndpoint, AudioEndpointId, AudioRoomId, EndpointAvailability};
pub use error::{AudioError, AudioErrorCode};
pub use router::{DeterministicRouter, EndpointRouter, RouterPolicy, RoutingInput, RoutingOutput};
pub use satellite::{AssistSatelliteProvider, VoiceSatellite, VoiceSatelliteId, WyomingProvider};
pub use transfer::{ConversationContext, ConversationTransfer, DeterministicTransfer};
pub use vocabulary::{
    require_hardware_class, require_role, EndpointRole, HardwareClass, HARDWARE_CLASSES,
};
