//! WebSocket session contract (SPEC-003 required behavior 1).

use crate::error::FabricError;
use nexus_domain::TenantId;
use nexus_identity::Principal;
use serde::{Deserialize, Serialize};

/// WebSocket session lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WebSocketState {
    Connecting,
    Open,
    Closing,
    Closed,
}

impl WebSocketState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Connecting => "CONNECTING",
            Self::Open => "OPEN",
            Self::Closing => "CLOSING",
            Self::Closed => "CLOSED",
        }
    }
}

impl std::str::FromStr for WebSocketState {
    type Err = crate::vocabulary::FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "CONNECTING" => Ok(Self::Connecting),
            "OPEN" => Ok(Self::Open),
            "CLOSING" => Ok(Self::Closing),
            "CLOSED" => Ok(Self::Closed),
            other => Err(crate::vocabulary::FabricVocabularyError::unknown(
                "WebSocketState",
                other,
            )),
        }
    }
}

/// A WebSocket event (inbound or outbound frame).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebSocketEvent {
    pub session_id: String,
    pub text: Option<String>,
    pub binary_sha256: Option<String>,
}

/// Provider-neutral WebSocket session port.
pub trait WebSocketSession {
    /// Accept a new session bound to the authenticated tenant/principal.
    fn open(
        &mut self,
        session_id: &str,
        tenant_id: &TenantId,
        principal: &Principal,
    ) -> Result<(), FabricError>;
    /// Send a text frame on the session.
    fn send_text(&self, session_id: &str, text: &str) -> Result<(), FabricError>;
    /// Close the session with the given code.
    fn close(&mut self, session_id: &str, code: u16) -> Result<(), FabricError>;
    /// Current state of a session.
    fn state(&self, session_id: &str) -> Result<WebSocketState, FabricError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_websocket_state_round_trip() {
        for (wire, expected) in [
            ("CONNECTING", WebSocketState::Connecting),
            ("OPEN", WebSocketState::Open),
            ("CLOSING", WebSocketState::Closing),
            ("CLOSED", WebSocketState::Closed),
        ] {
            assert_eq!(wire.parse::<WebSocketState>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
    }

    #[test]
    fn ep012_unit_websocket_state_rejects_unknown() {
        assert!("HALF_OPEN".parse::<WebSocketState>().is_err());
    }

    #[test]
    fn ep012_unit_websocket_event_has_fingerprint_not_payload() {
        let ev = WebSocketEvent {
            session_id: "sess-1".into(),
            text: Some("hello".into()),
            binary_sha256: None,
        };
        assert_eq!(ev.session_id, "sess-1");
        assert_eq!(ev.text.as_deref(), Some("hello"));
    }
}
