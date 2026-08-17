//! EP-023 Frigate/go2rtc provider adapter (SPEC-021).
//!
//! Real production adapter behavior behind the `CameraProvider` /
//! `FrigateProvider` ports from nexus-vision: provider health, camera
//! discovery, stream metadata, camera/state mapping, event retrieval,
//! live stream references, snapshot/reference handling, availability,
//! provider errors, and two-way-audio capability metadata.
//!
//! The transport port is the infrastructure boundary between the
//! adapter and Frigate. The real implementation (`RestTransport`)
//! targets the documented Frigate HTTP API and the embedded go2rtc
//! stream API. Controlled fixtures are acceptable for deterministic
//! unit behavior; provider/media certification requires the real
//! Frigate instance and real media flow (M3/M5).
//!
//! Permanent invariants (SPEC-021 / owner directive):
//! - No unverified RTSP or ONVIF claim is made: stream references stay
//!   `Unverified` until real go2rtc/media evidence exists.
//! - A camera that exists in configuration is not automatically
//!   reachable or streaming; the three states map truthfully.
//! - Visitor identity is advisory only and never authorizes.
//! - Two-way audio is never advertised from metadata alone.
//! - RTSP credentials and tokens are never logged or serialized.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod availability;
pub mod redact;
pub mod transport;

pub use adapter::FrigateAdapter;
pub use availability::CameraAvailability;
pub use redact::redact_url;
pub use transport::{
    FrigateCameraConfig, FrigateConfig, FrigateEvent, FrigateTransport, Go2RtcProducer,
    Go2RtcStreamInfo, RestTransport,
};
