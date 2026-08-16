//! EP-022 Assist satellite adapter core (SPEC-012 behaviors 3, 6, 9;
//! node contract acceptance obligations 2 and 3).
//!
//! Real production adapter behavior behind the `AssistSatelliteProvider`
//! port family: local wake gating, hardware mute authority, room-local
//! ephemeral capture, visible satellite state, and conversation context
//! survival across reconnect and transfer.
//!
//! Permanent invariants (Reality rule, SPEC-012):
//!
//! - Raw room audio is ephemeral by default and never continuously
//!   streamed to cloud (behavior 4).
//! - Hardware mute is authoritative: a fixed microphone with hardware
//!   mute cannot be captured from software (behavior 9).
//! - A satellite is only locally functional when its wake gate is
//!   actually bound; an unbound gate fails closed (UNAVAILABLE) and
//!   never fabricates a trigger (Reality rule).
//! - Conversation context (principal, objective, privacy policy,
//!   transcript, correlation) survives reconnect and endpoint transfer
//!   without implicit privacy upgrades.
//!
//! This crate is the M2 adapter core. Bluetooth/Wyoming transports and
//! real OS-level audio I/O belong to M3/M4/M5 and are never claimed
//! here.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod transport;

pub use adapter::{CaptureDecision, SatelliteCapture, SatelliteState, WakeDecision, WakeGate};
pub use transport::{AudioFrameSink, AudioSource, SourceEvent, WakeEvent};
