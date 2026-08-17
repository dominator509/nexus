//! EP-025 provider-neutral telephony contracts (SPEC-014).
//!
//! Asterisk 22 LTS is the telephony gateway; SIP carriers are
//! providers. Audio streams through the Nexus voice pipeline over a
//! secure media bridge. Calls are durable workflows with objective,
//! participant, disclosure, consent, interruption, escalation,
//! summary, transcript policy, and cost cap.
//!
//! Permanent invariants (owner directive, EP-025):
//! - CALL REQUESTED != SIP INVITE SENT != REMOTE RINGING != ANSWERED
//!   != MEDIA ESTABLISHED != TWO-WAY AUDIO VERIFIED != CALL COMPLETED.
//! - SIP SIGNALING IS NOT MEDIA CERTIFICATION: a 200/ANSWER proves
//!   signaling, never audio. Two-way means TWO directions.
//! - ASTERISK SAYS ANSWERED != NEXUS PROVED CONVERSATION. SIP WORKS
//!   != PSTN WORKS. RTP EXISTS != AUDIO IS INTELLIGIBLE.
//! - Real signaling, real media, real audio, real readback, real
//!   failure. No fabricated call state.
//! - Caller ID and SIP display identity are untrusted/advisory inputs
//!   (directive 16): a displayed number never authenticates a Nexus
//!   user or bypasses EP-008.
//! - Carrier credentials remain isolated (acceptance obligation 2).
//! - Recording and AI disclosure follow policy and jurisdiction
//!   configuration (acceptance obligation 4).

#![forbid(unsafe_code)]

pub mod error;
pub mod provider;
pub mod session;
pub mod verifier;
pub mod vocabulary;

pub use error::{CallError, CallErrorCode};
pub use provider::{AsteriskProvider, SipCarrierProvider, TelephonyProvider};
pub use session::{CallLeg, CallSession, MediaBridge};
pub use verifier::{CallVerification, CallVerifier};
pub use vocabulary::{
    CallCapability, CallCommand, CallDirection, CallLegId, CallOutcome, CallPolicy,
    CallPrivacyClass, CallSessionId, CallState, CarrierId, DisclosurePolicy, MediaCodec,
    MediaState, SipEndpointId, TranscriptArtifact, TranscriptId,
};
