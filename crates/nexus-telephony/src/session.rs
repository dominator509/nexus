//! EP-025 call session, leg, and media bridge (SPEC-014 CallSession /
//! CallLeg canonical terms).

use serde::{Deserialize, Serialize};

use crate::error::CallError;
use crate::vocabulary::{
    CallDirection, CallLegId, CallSessionId, CallState, MediaCodec, MediaState, SipEndpointId,
};

/// One call leg: a real channel within a session (SPEC-014 CallLeg).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallLeg {
    pub id: CallLegId,
    pub session_id: CallSessionId,
    pub endpoint: SipEndpointId,
    pub state: CallState,
    /// Real Asterisk channel id when bound (never fabricated).
    pub channel_id: Option<String>,
}

impl CallLeg {
    pub fn new(
        id: CallLegId,
        session_id: CallSessionId,
        endpoint: SipEndpointId,
        state: CallState,
    ) -> Self {
        Self {
            id,
            session_id,
            endpoint,
            state,
            channel_id: None,
        }
    }

    pub fn bind_channel(&mut self, channel_id: impl Into<String>) -> Result<(), CallError> {
        if self.channel_id.is_some() {
            return Err(CallError::conflict("leg already bound to a real channel"));
        }
        self.channel_id = Some(channel_id.into());
        Ok(())
    }
}

/// Call session (SPEC-014 CallSession): a durable call workflow with
/// objective, participant, disclosure, consent, and transcript policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallSession {
    pub id: CallSessionId,
    pub direction: CallDirection,
    pub peer: SipEndpointId,
    pub state: CallState,
    pub media_state: MediaState,
    pub legs: Vec<CallLeg>,
    pub codec: Option<MediaCodec>,
    /// Provider correlation id (redacted surface only).
    pub correlation: Option<String>,
    pub recording_consented: bool,
    pub ai_disclosure_required: bool,
}

impl CallSession {
    pub fn new(
        id: CallSessionId,
        direction: CallDirection,
        peer: SipEndpointId,
        correlation: Option<String>,
        recording_consented: bool,
        ai_disclosure_required: bool,
    ) -> Self {
        Self {
            id,
            direction,
            peer,
            state: CallState::Requested,
            media_state: MediaState::None,
            legs: Vec::new(),
            codec: None,
            correlation,
            recording_consented,
            ai_disclosure_required,
        }
    }

    pub fn add_leg(&mut self, leg: CallLeg) {
        self.legs.push(leg);
    }
}

/// Media bridge port: attaches a real media path between Nexus voice
/// (STT/TTS) and an Asterisk channel (directive 4/9/10).
///
/// The bridge does NOT implement SIP/RTP itself; it orchestrates the
/// real Asterisk media boundary (ARI ExternalMedia / WebSocket where
/// supported by the pinned build, recorded in the Decision Log).
pub trait MediaBridge {
    /// Attach a media path for a session. Returns a bridge handle.
    fn attach(&self, session: &CallSessionId) -> Result<String, CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "media bridge has no implementation bound",
        ))
    }

    /// Detach and release the media path.
    fn detach(&self, session: &CallSessionId) -> Result<(), CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "media bridge has no implementation bound",
        ))
    }

    /// Current media verification state for a session.
    fn media_state(&self, session: &CallSessionId) -> Result<MediaState, CallError> {
        let _ = session;
        Err(CallError::unavailable(
            "media bridge has no implementation bound",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep025_unit_call_session_construction() {
        let session_id = CallSessionId::new("session/1").unwrap();
        let peer = SipEndpointId::new("endpoint-a").unwrap();
        let mut session = CallSession::new(
            session_id.clone(),
            CallDirection::Outbound,
            peer.clone(),
            Some("tel-1".into()),
            false,
            true,
        );
        assert_eq!(session.state, CallState::Requested);
        assert_eq!(session.media_state, MediaState::None);
        assert!(session.legs.is_empty());
        assert!(!session.recording_consented);
        assert!(session.ai_disclosure_required);

        let leg_id = CallLegId::new("leg/1").unwrap();
        let mut leg = CallLeg::new(leg_id, session_id.clone(), peer, CallState::InviteSent);
        leg.bind_channel("PJSIP/endpoint-a-00000001").unwrap();
        assert!(leg.channel_id.is_some());
        assert!(leg.bind_channel("again").is_err()); // conflict
        session.add_leg(leg);
        assert_eq!(session.legs.len(), 1);
    }

    #[test]
    fn ep025_unit_media_bridge_fails_closed() {
        struct Unbound;
        impl MediaBridge for Unbound {}

        let session = CallSessionId::new("session/2").unwrap();
        assert!(Unbound.attach(&session).is_err());
        assert!(Unbound.detach(&session).is_err());
        assert!(Unbound.media_state(&session).is_err());
    }

    #[test]
    fn ep025_unit_leg_requires_real_channel_for_bridge() {
        // A leg without a real channel id cannot represent a bridged
        // channel: binding is explicit and single-use.
        let leg = CallLeg::new(
            CallLegId::new("leg/2").unwrap(),
            CallSessionId::new("session/3").unwrap(),
            SipEndpointId::new("endpoint-b").unwrap(),
            CallState::Answered,
        );
        assert!(leg.channel_id.is_none());
    }
}
