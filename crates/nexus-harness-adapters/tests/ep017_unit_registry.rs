//! EP-017 M2 registry and selector tests (SPEC-010; ADR-024).
//!
//! Proves the deterministic capability-based selection behavior:
//! ranking, capability filtering, availability filtering, card state
//! filtering, tenant listing, and registry lifecycle.

use nexus_agents::{
    AgentBudget, AgentBudgetClass, AgentCapability, AgentCard, AgentCardId, AgentCardState,
    AgentRegistry, CapabilityRequest,
};
use nexus_harness_adapters::{AgentSelector, CardSignals, DeterministicAgentRegistry};
use std::collections::HashMap;

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

fn request(capability: AgentCapability) -> CapabilityRequest {
    CapabilityRequest {
        request_id: "cr-1".into(),
        correlation_id: "c-1".into(),
        tenant_id: "t-1".into(),
        principal_id: "p-1".into(),
        objective_id: "o-1".into(),
        task_id: "task-1".into(),
        capability,
        required_permissions: vec![],
        budget: AgentBudget::new(AgentBudgetClass::TotalTokens, 1000),
    }
}

#[test]
fn ep017_unit_registry_register_conflict_unregister() {
    let mut registry = DeterministicAgentRegistry::new();
    registry
        .register(card(
            "c1",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Registered,
        ))
        .unwrap();
    let error = registry
        .register(card("c1", vec![], AgentCardState::Registered))
        .unwrap_err();
    assert_eq!(error.code.as_str(), "CONFLICT");
    let got = registry.get("c1").unwrap();
    assert_eq!(got.card_id.0, "c1");
    registry.unregister("c1").unwrap();
    let error = registry.unregister("c1").unwrap_err();
    assert_eq!(error.code.as_str(), "NOT_FOUND");
}

#[test]
fn ep017_unit_registry_rejects_empty_name() {
    let mut registry = DeterministicAgentRegistry::new();
    let mut c = card("c1", vec![], AgentCardState::Registered);
    c.name = String::new();
    let error = registry.register(c).unwrap_err();
    assert_eq!(error.code.as_str(), "VALIDATION");
}

#[test]
fn ep017_unit_registry_list_sorted() {
    let mut registry = DeterministicAgentRegistry::new();
    registry
        .register(card("z", vec![], AgentCardState::Registered))
        .unwrap();
    registry
        .register(card("a", vec![], AgentCardState::Registered))
        .unwrap();
    let cards = registry.list("t-1").unwrap();
    assert_eq!(cards[0].card_id.0, "a");
    assert_eq!(cards[1].card_id.0, "z");
}

#[test]
fn ep017_unit_selector_deterministic_same_input_same_order() {
    let cards = vec![
        card(
            "a",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Registered,
        ),
        card(
            "b",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Registered,
        ),
    ];
    let mut signals = HashMap::new();
    signals.insert(
        "a".into(),
        CardSignals {
            quality: 0.8,
            ..CardSignals::defaults()
        },
    );
    signals.insert(
        "b".into(),
        CardSignals {
            quality: 0.6,
            ..CardSignals::defaults()
        },
    );
    let first = AgentSelector::select(&cards, &signals, AgentCapability::Implement, "t-1");
    let second = AgentSelector::select(&cards, &signals, AgentCapability::Implement, "t-1");
    assert_eq!(first, second);
    assert_eq!(first[0].card_id, "a");
}

#[test]
fn ep017_unit_selector_excludes_suspended_and_revoked() {
    let cards = vec![
        card(
            "s",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Suspended,
        ),
        card(
            "r",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Revoked,
        ),
        card(
            "ok",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Registered,
        ),
    ];
    let ranked = AgentSelector::select(&cards, &HashMap::new(), AgentCapability::Implement, "t-1");
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].card_id, "ok");
}

#[test]
fn ep017_unit_selector_tie_breaks_by_card_id() {
    let cards = vec![
        card(
            "b",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Registered,
        ),
        card(
            "a",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Registered,
        ),
    ];
    let ranked = AgentSelector::select(&cards, &HashMap::new(), AgentCapability::Implement, "t-1");
    assert_eq!(ranked[0].card_id, "a");
    assert_eq!(ranked[1].card_id, "b");
}

#[test]
fn ep017_unit_registry_selection_via_trait() {
    let mut registry = DeterministicAgentRegistry::new();
    registry
        .register(card(
            "codex-1",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Registered,
        ))
        .unwrap();
    registry.set_signals(
        &AgentCardId("codex-1".into()),
        CardSignals {
            quality: 0.95,
            ..CardSignals::defaults()
        },
    );
    let ranked = registry
        .select_for_capability(&request(AgentCapability::Implement))
        .unwrap();
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].card_id, "codex-1");
    assert_eq!(ranked[0].capability, AgentCapability::Implement);
}

#[test]
fn ep017_unit_selection_empty_when_no_eligible_agent() {
    let registry = DeterministicAgentRegistry::new();
    let ranked = registry
        .select_for_capability(&request(AgentCapability::Implement))
        .unwrap();
    assert!(ranked.is_empty());
}
