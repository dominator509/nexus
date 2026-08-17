//! EP-024 media connector (SPEC-011 behaviors 1-3, 5).
//!
//! Provider-neutral media adapter core behind the nexus-devices
//! `MediaProvider` port. Home Assistant is the preferred transport for
//! commodity media devices; direct Sonos/TV transports exist only for
//! capability or reliability gaps (acceptance obligation 2). Unbound
//! transports fail closed and never fabricate devices, states, or
//! command acceptance.

#![forbid(unsafe_code)]

pub mod adapter;
pub mod error;
pub mod transport;

pub use adapter::MediaAdapter;
pub use error::{MediaError, MediaErrorCode};
pub use transport::{
    MediaCommand, MediaCommandReceipt, MediaCommandState, MediaState, MediaTransport,
};
