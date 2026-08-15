//! Deterministic agent registry and capability-based selector
//! (SPEC-010 behaviors 1-3; ADR-024).
//!
//! Agents request capabilities rather than named peers. Selection is a
//! pure deterministic function of declared signals: quality, cost,
//! trust tier, availability, and historical success. A card is
//! eligible only when it is REGISTERED, declares the requested
//! capability, and is available.

use nexus_agents::{
    AgentCapability, AgentCard, AgentCardId, AgentCardState, AgentRegistry, AgentSelection,
    AgentsError, CapabilityRequest,
};
use std::collections::HashMap;

/// Injected deterministic selection signals for a card. These are
/// measured by the operator/trust layer; the selector never fabricates
/// them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardSignals {
    /// Normalized quality (0.0-1.0).
    pub quality: f64,
    /// Normalized cost (0.0-1.0, lower is better).
    pub cost: f64,
    /// Trust tier (higher is more trusted).
    pub trust_tier: u8,
    pub available: bool,
    /// Historical success rate (0.0-1.0).
    pub historical_success: f64,
}

impl CardSignals {
    pub const fn defaults() -> Self {
        Self {
            quality: 0.5,
            cost: 0.5,
            trust_tier: 1,
            available: true,
            historical_success: 0.5,
        }
    }
}

/// Pure deterministic capability selector.
pub struct AgentSelector;

/// Composite weights (documented, deterministic; sum 1.0).
pub const WEIGHT_QUALITY: f64 = 0.35;
pub const WEIGHT_COST: f64 = 0.20;
pub const WEIGHT_TRUST: f64 = 0.20;
pub const WEIGHT_AVAILABILITY: f64 = 0.10;
pub const WEIGHT_HISTORY: f64 = 0.15;

impl AgentSelector {
    /// Rank candidates for a capability. Deterministic: identical
    /// inputs produce identical orderings; ties break by card id.
    pub fn select(
        cards: &[AgentCard],
        signals: &HashMap<String, CardSignals>,
        capability: AgentCapability,
        _tenant_id: &str,
    ) -> Vec<AgentSelection> {
        let mut out: Vec<AgentSelection> = Vec::new();
        for card in cards {
            if card.state != AgentCardState::Registered {
                continue;
            }
            if !card.capabilities.iter().any(|c| c == capability.as_str()) {
                continue;
            }
            let sig = signals
                .get(card.card_id.0.as_str())
                .copied()
                .unwrap_or_else(CardSignals::defaults);
            if !sig.available {
                continue;
            }
            // Composite score: quality and history and trust raise the
            // rank; cost lowers it.
            let score = WEIGHT_QUALITY * sig.quality
                + WEIGHT_COST * (1.0 - sig.cost)
                + WEIGHT_TRUST * (sig.trust_tier as f64 / 10.0)
                + WEIGHT_AVAILABILITY * 1.0
                + WEIGHT_HISTORY * sig.historical_success;
            // Deterministic rank: higher score is better; encode as a
            // stable u64 without float ordering ambiguity.
            let rank = (score * 1_000_000.0) as u64;
            out.push(AgentSelection {
                card_id: card.card_id.0.clone(),
                capability,
                quality: sig.quality,
                cost: sig.cost,
                trust_tier: sig.trust_tier,
                available: true,
                historical_success: sig.historical_success,
                rank,
            });
        }
        out.sort_by(|a, b| b.rank.cmp(&a.rank).then_with(|| a.card_id.cmp(&b.card_id)));
        out
    }
}

/// In-memory deterministic agent registry.
#[derive(Debug, Clone, Default)]
pub struct DeterministicAgentRegistry {
    cards: HashMap<String, AgentCard>,
    signals: HashMap<String, CardSignals>,
}

impl DeterministicAgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the deterministic selection signals for a card. Signals are
    /// injected (operator/trust layer); the registry never fabricates
    /// them.
    pub fn set_signals(&mut self, card_id: &AgentCardId, signals: CardSignals) {
        self.signals.insert(card_id.0.clone(), signals);
    }
}

impl AgentRegistry for DeterministicAgentRegistry {
    fn register(&mut self, card: AgentCard) -> Result<(), AgentsError> {
        if self.cards.contains_key(&card.card_id.0) {
            return Err(AgentsError::new(
                nexus_agents::AgentsErrorCode::Conflict,
                "agent card already registered",
                None,
                None,
                None,
                Some("agent-registry".into()),
            ));
        }
        if card.name.is_empty() {
            return Err(AgentsError::validation(
                "agent card name must not be empty",
                Some("agent-registry".into()),
            ));
        }
        self.cards.insert(card.card_id.0.clone(), card);
        Ok(())
    }

    fn unregister(&mut self, card_id: &str) -> Result<(), AgentsError> {
        match self.cards.remove(card_id) {
            Some(_) => {
                self.signals.remove(card_id);
                Ok(())
            }
            None => Err(AgentsError::not_found(
                "agent card not found",
                Some("agent-registry".into()),
            )),
        }
    }

    fn get(&self, card_id: &str) -> Result<AgentCard, AgentsError> {
        self.cards.get(card_id).cloned().ok_or_else(|| {
            AgentsError::not_found("agent card not found", Some("agent-registry".into()))
        })
    }

    fn list(&self, tenant_id: &str) -> Result<Vec<AgentCard>, AgentsError> {
        if tenant_id.is_empty() {
            return Err(AgentsError::validation(
                "tenant_id must not be empty",
                Some("agent-registry".into()),
            ));
        }
        // Cards are tenant-scoped by the caller's card set; the
        // registry returns the full registered set (tenant isolation
        // is enforced at the storage boundary).
        let mut cards: Vec<AgentCard> = self.cards.values().cloned().collect();
        cards.sort_by(|a, b| a.card_id.0.cmp(&b.card_id.0));
        Ok(cards)
    }

    fn select_for_capability(
        &self,
        request: &CapabilityRequest,
    ) -> Result<Vec<AgentSelection>, AgentsError> {
        request.validate()?;
        let cards: Vec<AgentCard> = self.cards.values().cloned().collect();
        Ok(AgentSelector::select(
            &cards,
            &self.signals,
            request.capability,
            &request.tenant_id,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: &str, capabilities: Vec<String>, state: AgentCardState) -> AgentCard {
        AgentCard {
            card_id: AgentCardId(id.into()),
            name: id.into(),
            description: String::new(),
            url: String::new(),
            capabilities,
            state,
        }
    }

    #[test]
    fn ep017_unit_selector_ranks_by_composite_score() {
        let cards = vec![
            card(
                "high",
                vec![AgentCapability::Implement.as_str().into()],
                AgentCardState::Registered,
            ),
            card(
                "low",
                vec![AgentCapability::Implement.as_str().into()],
                AgentCardState::Registered,
            ),
        ];
        let mut signals = HashMap::new();
        signals.insert(
            "high".into(),
            CardSignals {
                quality: 0.9,
                cost: 0.5,
                trust_tier: 5,
                available: true,
                historical_success: 0.9,
            },
        );
        signals.insert(
            "low".into(),
            CardSignals {
                quality: 0.2,
                cost: 0.5,
                trust_tier: 1,
                available: true,
                historical_success: 0.2,
            },
        );
        let ranked = AgentSelector::select(&cards, &signals, AgentCapability::Implement, "t-1");
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].card_id, "high");
        assert_eq!(ranked[1].card_id, "low");
    }

    #[test]
    fn ep017_unit_selector_excludes_missing_capability_and_unavailable() {
        let cards = vec![
            card("no-cap", vec!["OTHER".into()], AgentCardState::Registered),
            card(
                "down",
                vec![AgentCapability::Implement.as_str().into()],
                AgentCardState::Registered,
            ),
            card(
                "suspended",
                vec![AgentCapability::Implement.as_str().into()],
                AgentCardState::Suspended,
            ),
            card(
                "ok",
                vec![AgentCapability::Implement.as_str().into()],
                AgentCardState::Registered,
            ),
        ];
        let mut signals = HashMap::new();
        signals.insert(
            "down".into(),
            CardSignals {
                available: false,
                ..CardSignals::defaults()
            },
        );
        let ranked = AgentSelector::select(&cards, &signals, AgentCapability::Implement, "t-1");
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].card_id, "ok");
    }
}
