//! Agent card registry (SPEC-003 canonical term: Agent Card).

use crate::error::FabricError;
use serde::{Deserialize, Serialize};

/// Agent card identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCardId(pub String);

/// Agent card lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentCardState {
    Registered,
    Suspended,
    Revoked,
}

impl AgentCardState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Registered => "REGISTERED",
            Self::Suspended => "SUSPENDED",
            Self::Revoked => "REVOKED",
        }
    }
}

impl std::str::FromStr for AgentCardState {
    type Err = crate::vocabulary::FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "REGISTERED" => Ok(Self::Registered),
            "SUSPENDED" => Ok(Self::Suspended),
            "REVOKED" => Ok(Self::Revoked),
            other => Err(crate::vocabulary::FabricVocabularyError::unknown(
                "AgentCardState",
                other,
            )),
        }
    }
}

/// An agent card (A2A discovery metadata).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCard {
    pub card_id: AgentCardId,
    pub name: String,
    pub description: String,
    pub url: String,
    pub capabilities: Vec<String>,
    pub state: AgentCardState,
}

/// Provider-neutral agent card registry port.
pub trait AgentCardRegistry {
    /// Register or update a card; duplicate registration is a conflict.
    fn register(&mut self, card: AgentCard) -> Result<(), FabricError>;
    /// Look up a card by id.
    fn lookup(&self, card_id: &AgentCardId) -> Result<AgentCard, FabricError>;
    /// Suspend a card.
    fn suspend(&mut self, card_id: &AgentCardId) -> Result<(), FabricError>;
    /// Revoke a card (removes it from discovery).
    fn revoke(&mut self, card_id: &AgentCardId) -> Result<(), FabricError>;
    /// List cards visible to a tenant.
    fn list(&self, tenant_id: &str) -> Result<Vec<AgentCard>, FabricError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_agent_card_state_round_trip() {
        for (wire, expected) in [
            ("REGISTERED", AgentCardState::Registered),
            ("SUSPENDED", AgentCardState::Suspended),
            ("REVOKED", AgentCardState::Revoked),
        ] {
            assert_eq!(wire.parse::<AgentCardState>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("ACTIVE".parse::<AgentCardState>().is_err());
    }

    #[test]
    fn ep012_unit_agent_card_round_trip() {
        let card = AgentCard {
            card_id: AgentCardId("card-1".into()),
            name: "billing-agent".into(),
            description: "handles billing".into(),
            url: "https://agents.nexus.local/billing".into(),
            capabilities: vec!["fixture.billing.command".into()],
            state: AgentCardState::Registered,
        };
        let json = serde_json::to_value(&card).unwrap();
        let back: AgentCard = serde_json::from_value(json).unwrap();
        assert_eq!(back.name, "billing-agent");
        assert_eq!(back.state, AgentCardState::Registered);
    }
}
