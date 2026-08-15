//! EP-017 M4 registry failure and abuse suite (SPEC-010 behaviors
//! 1-2; ADR-024).
//!
//! Proves the capability-based registry fails safely: unknown
//! capabilities never select, unavailable/suspended/revoked agents are
//! excluded, the deterministic tie-break never depends on vendor name,
//! and no fabricated signal ever promotes an ineligible card.

use nexus_agents::{
    AgentBudget, AgentBudgetClass, AgentCapability, AgentCard, AgentCardId, AgentCardState,
    AgentRegistry, AgentsErrorCode, CapabilityRequest,
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

fn request(capability: AgentCapability, tenant: &str) -> CapabilityRequest {
    CapabilityRequest {
        request_id: "r-1".into(),
        correlation_id: "c-1".into(),
        tenant_id: tenant.into(),
        principal_id: "p-1".into(),
        objective_id: "o-1".into(),
        task_id: "t-1".into(),
        capability,
        required_permissions: vec![],
        budget: AgentBudget::new(AgentBudgetClass::TotalTokens, 1000),
    }
}

fn implement_card(id: &str) -> AgentCard {
    card(
        id,
        vec![AgentCapability::Implement.as_str().into()],
        AgentCardState::Registered,
    )
}

#[test]
fn ep017_failure_unknown_capability_never_selects() {
    // No card declares ORCHESTRATE; selection must be empty, never a
    // fallback to a named agent.
    let mut registry = DeterministicAgentRegistry::new();
    registry.register(implement_card("codex-1")).unwrap();
    let ranked = registry
        .select_for_capability(&request(AgentCapability::Orchestrate, "t-1"))
        .unwrap();
    assert!(ranked.is_empty());
}

#[test]
fn ep017_failure_unavailable_agent_excluded() {
    let mut registry = DeterministicAgentRegistry::new();
    registry.register(implement_card("down-1")).unwrap();
    registry.register(implement_card("ok-1")).unwrap();
    registry.set_signals(
        &AgentCardId("down-1".into()),
        CardSignals {
            available: false,
            ..CardSignals::defaults()
        },
    );
    let ranked = registry
        .select_for_capability(&request(AgentCapability::Implement, "t-1"))
        .unwrap();
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].card_id, "ok-1");
}

#[test]
fn ep017_failure_suspended_and_revoked_agents_excluded() {
    let mut registry = DeterministicAgentRegistry::new();
    registry.register(implement_card("suspended-1")).unwrap();
    registry.register(implement_card("revoked-1")).unwrap();
    registry.register(implement_card("ok-1")).unwrap();
    // Force non-REGISTERED states through the selector directly (the
    // registry stores state on the card; suspended/revoked cards must
    // never be eligible).
    let cards = vec![
        card(
            "suspended-1",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Suspended,
        ),
        card(
            "revoked-1",
            vec![AgentCapability::Implement.as_str().into()],
            AgentCardState::Revoked,
        ),
        implement_card("ok-1"),
    ];
    let signals = HashMap::new();
    let ranked = AgentSelector::select(&cards, &signals, AgentCapability::Implement, "t-1");
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].card_id, "ok-1");
}

#[test]
fn ep017_failure_tie_break_is_card_id_not_vendor_name() {
    // Two cards with identical signals and identical scores: the
    // tie-break is the card id, deterministic, independent of the
    // vendor-style name.
    let cards = vec![
        implement_card("z-codex"),
        implement_card("a-openclaw"),
        implement_card("m-claude"),
    ];
    let signals = HashMap::new();
    let a = AgentSelector::select(&cards, &signals, AgentCapability::Implement, "t-1");
    let b = AgentSelector::select(&cards, &signals, AgentCapability::Implement, "t-1");
    assert_eq!(a.len(), 3);
    // Identical inputs -> identical orderings (byte-for-byte ids).
    let ids_a: Vec<&str> = a.iter().map(|s| s.card_id.as_str()).collect();
    let ids_b: Vec<&str> = b.iter().map(|s| s.card_id.as_str()).collect();
    assert_eq!(ids_a, ids_b);
    // Deterministic lexicographic tie-break: a-openclaw first.
    assert_eq!(a[0].card_id, "a-openclaw");
}

#[test]
fn ep017_failure_no_vendor_name_special_case_bypasses_capability() {
    // A card named like a well-known vendor must NOT be selected for a
    // capability it does not declare. Named-peer selection is
    // forbidden by SPEC-010 behavior 2.
    let mut registry = DeterministicAgentRegistry::new();
    registry
        .register(card("codex", vec![], AgentCardState::Registered))
        .unwrap();
    registry.register(implement_card("worker-1")).unwrap();
    let ranked = registry
        .select_for_capability(&request(AgentCapability::Implement, "t-1"))
        .unwrap();
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].card_id, "worker-1");
}

#[test]
fn ep017_failure_duplicate_registration_conflict() {
    let mut registry = DeterministicAgentRegistry::new();
    registry.register(implement_card("codex-1")).unwrap();
    let error = registry.register(implement_card("codex-1")).unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Conflict);
}

#[test]
fn ep017_failure_unregister_missing_not_found() {
    let mut registry = DeterministicAgentRegistry::new();
    let error = registry.unregister("nope").unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::NotFound);
}

#[test]
fn ep017_failure_empty_tenant_rejected() {
    let mut registry = DeterministicAgentRegistry::new();
    registry.register(implement_card("codex-1")).unwrap();
    let error = registry.list("").unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Validation);
}

#[test]
fn ep017_failure_empty_capability_request_rejected() {
    let mut registry = DeterministicAgentRegistry::new();
    registry.register(implement_card("codex-1")).unwrap();
    let mut req = request(AgentCapability::Implement, "t-1");
    req.tenant_id = String::new();
    let error = registry.select_for_capability(&req).unwrap_err();
    assert_eq!(error.code, AgentsErrorCode::Validation);
}

#[test]
fn ep017_failure_no_fabricated_signals_for_unmeasured_card() {
    // A card with NO injected signals uses the documented defaults;
    // the selector never invents availability or a perfect score to
    // promote it above a measured card.
    let mut registry = DeterministicAgentRegistry::new();
    registry.register(implement_card("unmeasured")).unwrap();
    registry.register(implement_card("measured")).unwrap();
    registry.set_signals(
        &AgentCardId("measured".into()),
        CardSignals {
            quality: 1.0,
            cost: 0.0,
            trust_tier: 10,
            available: true,
            historical_success: 1.0,
        },
    );
    let ranked = registry
        .select_for_capability(&request(AgentCapability::Implement, "t-1"))
        .unwrap();
    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].card_id, "measured");
}
