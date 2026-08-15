//! Agent registry (SPEC-010 behaviors 1-3; ADR-024).
//!
//! Nexus owns canonical agent selection. Agents request capabilities
//! rather than named peers; the registry selects on quality, cost,
//! trust, availability, and historical success. Direct agent-to-agent
//! authority is forbidden; selection always returns a ranked,
//! deterministic `AgentSelection` that a later policy stage may
//! approve or deny. This file owns no provider behavior (M1 contract
//! boundary); the deterministic selector implementation is owned by
//! the EP-017 M2 crate boundary.

use crate::capability::CapabilityRequest;
use crate::error::AgentsError;
use crate::vocabulary::AgentCapability;
use nexus_fabric::AgentCard;
use serde::{Deserialize, Serialize};

/// A ranked, deterministic agent selection for a capability request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSelection {
    pub card_id: String,
    pub capability: AgentCapability,
    /// Normalized quality score (0.0-1.0); deterministic.
    pub quality: f64,
    /// Normalized cost score (0.0-1.0, lower is better).
    pub cost: f64,
    /// Trust tier (vocabulary locked; SPEC-010 canonical term
    /// `Skill Trust` for skills, agent trust by policy).
    pub trust_tier: u8,
    pub available: bool,
    /// Historical success rate (0.0-1.0).
    pub historical_success: f64,
    /// Composite deterministic rank (lower is better).
    pub rank: u64,
}

/// Provider-neutral agent registry port.
///
/// The registry tracks agent cards (nexus-fabric `AgentCard`) and
/// answers capability requests. Selection inputs are deterministic;
/// the composite rank is computed by the M2 selector, never by an
/// adapter.
pub trait AgentRegistry {
    /// Register or update an agent card; duplicate registration is a
    /// conflict.
    fn register(&mut self, card: AgentCard) -> Result<(), AgentsError>;

    /// Remove an agent card from the registry.
    fn unregister(&mut self, card_id: &str) -> Result<(), AgentsError>;

    /// Look up an agent card.
    fn get(&self, card_id: &str) -> Result<AgentCard, AgentsError>;

    /// List agent cards visible to a tenant.
    fn list(&self, tenant_id: &str) -> Result<Vec<AgentCard>, AgentsError>;

    /// Select ranked candidates for a capability request. Never
    /// returns a named agent without a capability match; returns an
    /// empty ranking when no eligible agent exists.
    fn select_for_capability(
        &self,
        request: &CapabilityRequest,
    ) -> Result<Vec<AgentSelection>, AgentsError>;
}
