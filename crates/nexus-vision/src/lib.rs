//! EP-023 nexus-vision: cameras, Frigate/go2rtc, Roku Home, visitor
//! identity, and two-way audio contracts (SPEC-021).
//!
//! The crate owns the provider-neutral vision contracts. Domain rules
//! are pure; provider behavior plugs in behind the port traits
//! (CameraProvider, FrigateProvider, RokuHomeProvider) and is never
//! certified until real provider or hardware evidence exists (Reality
//! rule).
//!
//! Permanent invariants (SPEC-021):
//! - No unverified RTSP or ONVIF claim is made: a StreamRef is
//!   Unverified unless real verification evidence exists.
//! - Known-person matching is advisory only and can never unlock or
//!   disarm by itself (behavior 6).
//! - Two-way audio is enabled only after live certification
//!   (behavior 7, acceptance obligation 4).
//! - Roku fallback order is fixed: verified local, authenticated
//!   vendor interface, Google Home bridge, browser automation, then
//!   unavailable (behavior 3).
//! - Browser automation is isolated, monitored, rate-limited, and
//!   never a stable API without certification (behavior 4).

#![forbid(unsafe_code)]

pub mod error;
pub mod event;
pub mod fallback;
pub mod identity;
pub mod provider;
pub mod stream;
pub mod two_way;
pub mod vocabulary;

pub use error::{VisionError, VisionErrorCode};
pub use event::{CameraEvent, ReviewItem, VisitorEvent};
pub use fallback::{BrowserAutomationPolicy, CameraFallbackPlan};
pub use identity::{KnownVisitor, VisitorIdentity};
pub use provider::{CameraProvider, FrigateProvider, RokuHomeProvider};
pub use stream::{StreamRef, VerificationStatus};
pub use two_way::{TwoWayAudioCapability, TwoWayAudioState};
pub use vocabulary::{CameraCapability, CameraId, PrivacyClass, RokuCapabilityTier};
