//! EP-012 fabric vocabulary (SPEC-003 canonical terms; ADR-017).
//!
//! Vocabulary-locked enums for the fabric surface. Every enum parses
//! from its canonical SCREAMING_SNAKE_CASE wire string and rejects
//! unknown values (fail closed, no silent reinterpretation).

use serde::{Deserialize, Serialize};

/// Transport families owned by the fabric surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ApiTransport {
    /// Versioned HTTP REST (SPEC-003 required behavior 1).
    Rest,
    /// Bidirectional WebSocket sessions.
    WebSocket,
    /// MCP Streamable HTTP (specification 2025-11-25).
    McpStreamableHttp,
    /// A2A agent-to-agent protocol 1.0.1.
    A2A,
}

impl ApiTransport {
    /// Canonical wire value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rest => "REST",
            Self::WebSocket => "WEBSOCKET",
            Self::McpStreamableHttp => "MCP_STREAMABLE_HTTP",
            Self::A2A => "A2A",
        }
    }
}

impl std::str::FromStr for ApiTransport {
    type Err = FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "REST" => Ok(Self::Rest),
            "WEBSOCKET" => Ok(Self::WebSocket),
            "MCP_STREAMABLE_HTTP" => Ok(Self::McpStreamableHttp),
            "A2A" => Ok(Self::A2A),
            other => Err(FabricVocabularyError::unknown("ApiTransport", other)),
        }
    }
}

/// MCP protocol specification version (SPEC-003 required behavior 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum McpProtocolVersion {
    /// MCP Streamable HTTP specification 2025-11-25 (locked target).
    Spec2025_11_25,
}

impl McpProtocolVersion {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spec2025_11_25 => "2025-11-25",
        }
    }
}

impl std::str::FromStr for McpProtocolVersion {
    type Err = FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "2025-11-25" => Ok(Self::Spec2025_11_25),
            other => Err(FabricVocabularyError::unknown("McpProtocolVersion", other)),
        }
    }
}

/// A2A protocol version (SPEC-003 required behavior 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum A2AProtocolVersion {
    /// A2A protocol 1.0.1 (locked target).
    Spec1_0_1,
}

impl A2AProtocolVersion {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spec1_0_1 => "1.0.1",
        }
    }
}

impl std::str::FromStr for A2AProtocolVersion {
    type Err = FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "1.0.1" => Ok(Self::Spec1_0_1),
            other => Err(FabricVocabularyError::unknown("A2AProtocolVersion", other)),
        }
    }
}

/// A2A task stream state (SPEC-003 required behavior 3: streaming
/// status, cancellation, push notifications).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StreamState {
    Pending,
    Running,
    Completed,
    Cancelled,
    Failed,
}

impl StreamState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Running => "RUNNING",
            Self::Completed => "COMPLETED",
            Self::Cancelled => "CANCELLED",
            Self::Failed => "FAILED",
        }
    }
}

impl std::str::FromStr for StreamState {
    type Err = FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PENDING" => Ok(Self::Pending),
            "RUNNING" => Ok(Self::Running),
            "COMPLETED" => Ok(Self::Completed),
            "CANCELLED" => Ok(Self::Cancelled),
            "FAILED" => Ok(Self::Failed),
            other => Err(FabricVocabularyError::unknown("StreamState", other)),
        }
    }
}

/// Vocabulary parse/rejection error (fail closed on unknown values).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricVocabularyError {
    pub enum_name: &'static str,
    pub value: String,
}

impl FabricVocabularyError {
    pub(crate) fn unknown(enum_name: &'static str, value: &str) -> Self {
        Self {
            enum_name,
            value: value.to_string(),
        }
    }
}

impl std::fmt::Display for FabricVocabularyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unknown {} value: {:?}", self.enum_name, self.value)
    }
}

impl std::error::Error for FabricVocabularyError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_vocabulary_transport_round_trip() {
        for (wire, expected) in [
            ("REST", ApiTransport::Rest),
            ("WEBSOCKET", ApiTransport::WebSocket),
            ("MCP_STREAMABLE_HTTP", ApiTransport::McpStreamableHttp),
            ("A2A", ApiTransport::A2A),
        ] {
            assert_eq!(wire.parse::<ApiTransport>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
    }

    #[test]
    fn ep012_unit_vocabulary_rejects_unknown_transport() {
        let err = "HTTP".parse::<ApiTransport>().unwrap_err();
        assert!(err.to_string().contains("ApiTransport"));
        assert!(err.to_string().contains("HTTP"));
    }

    #[test]
    fn ep012_unit_vocabulary_protocol_versions_parse() {
        assert_eq!(
            "2025-11-25".parse::<McpProtocolVersion>().unwrap(),
            McpProtocolVersion::Spec2025_11_25
        );
        assert_eq!(
            "1.0.1".parse::<A2AProtocolVersion>().unwrap(),
            A2AProtocolVersion::Spec1_0_1
        );
        assert!("2024-11-05".parse::<McpProtocolVersion>().is_err());
        assert!("2.0.0".parse::<A2AProtocolVersion>().is_err());
    }

    #[test]
    fn ep012_unit_vocabulary_stream_state_rejects_unknown() {
        assert_eq!(
            "PENDING".parse::<StreamState>().unwrap(),
            StreamState::Pending
        );
        assert!("IDLE".parse::<StreamState>().is_err());
    }
}
