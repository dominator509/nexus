//! EP-022 conversation transfer (SPEC-012 behavior 8; acceptance
//! obligation 2).
//!
//! A conversation may transfer endpoints without losing context:
//! principal, objective, privacy policy, and transcript context are
//! preserved.

use nexus_domain::PersonId;

use crate::endpoint::{AudioEndpointId, AudioRoomId};
use crate::error::{AudioError, AudioErrorCode};

/// Immutable conversation context carried across endpoint transfers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationContext {
    pub session_id: String,
    pub principal: PersonId,
    pub objective: String,
    pub privacy_policy_id: String,
    pub room: Option<AudioRoomId>,
    pub transcript: Vec<(String, String)>,
    pub correlation_id: Option<Box<str>>,
}

impl ConversationContext {
    pub fn new(
        session_id: impl Into<String>,
        principal: PersonId,
        objective: impl Into<String>,
        privacy_policy_id: impl Into<String>,
    ) -> Result<Self, AudioError> {
        let session_id = session_id.into();
        let objective = objective.into();
        let privacy_policy_id = privacy_policy_id.into();
        if session_id.is_empty() {
            return Err(AudioError::new(
                AudioErrorCode::Validation,
                "session id must not be empty",
                None,
                None,
            ));
        }
        if objective.is_empty() {
            return Err(AudioError::new(
                AudioErrorCode::Validation,
                "objective must not be empty",
                None,
                None,
            ));
        }
        if privacy_policy_id.is_empty() {
            return Err(AudioError::new(
                AudioErrorCode::Validation,
                "privacy policy id must not be empty",
                None,
                None,
            ));
        }
        Ok(Self {
            session_id,
            principal,
            objective,
            privacy_policy_id,
            room: None,
            transcript: Vec::new(),
            correlation_id: None,
        })
    }

    pub fn with_room(mut self, room: AudioRoomId) -> Self {
        self.room = Some(room);
        self
    }

    pub fn with_correlation(mut self, correlation_id: Box<str>) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn append_transcript(&mut self, speaker: &str, text: &str) {
        self.transcript
            .push((speaker.to_string(), text.to_string()));
    }
}

/// Conversation transfer port (fail-closed default).
pub trait ConversationTransfer {
    fn transfer(
        &self,
        context: &ConversationContext,
        target: &AudioEndpointId,
    ) -> Result<ConversationContext, AudioError> {
        let _ = (context, target);
        Err(AudioError::unavailable(
            "conversation transfer has no implementation bound",
        ))
    }
}

/// Deterministic transfer implementation: context is preserved and the
/// room/principal binding survives; only the endpoint binding moves.
#[derive(Debug, Clone, Default)]
pub struct DeterministicTransfer;

impl ConversationTransfer for DeterministicTransfer {
    fn transfer(
        &self,
        context: &ConversationContext,
        _target: &AudioEndpointId,
    ) -> Result<ConversationContext, AudioError> {
        Ok(context.clone())
    }
}
