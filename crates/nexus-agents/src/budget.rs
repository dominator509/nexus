//! Agent budget (SPEC-010; ADR-024).
//!
//! Budgets are fixed, declared limits Nexus owns and enforces
//! fail-closed. A budget tracks a class, a hard limit, and current
//! usage; usage above the limit is rejected, never silently exceeded.

use crate::error::AgentsError;
use crate::vocabulary::AgentBudgetClass;
use serde::{Deserialize, Serialize};

/// A single declared budget limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBudget {
    pub class: AgentBudgetClass,
    pub limit: u64,
    pub used: u64,
}

impl AgentBudget {
    pub fn new(class: AgentBudgetClass, limit: u64) -> Self {
        Self {
            class,
            limit,
            used: 0,
        }
    }

    /// Canonical invariants: the limit is positive and usage never
    /// exceeds it.
    pub fn validate(&self) -> Result<(), AgentsError> {
        if self.limit == 0 {
            return Err(AgentsError::validation(
                "budget limit must be positive",
                Some("agent-budget".into()),
            ));
        }
        if self.used > self.limit {
            return Err(AgentsError::validation(
                "budget usage exceeds limit",
                Some("agent-budget".into()),
            ));
        }
        Ok(())
    }

    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.used)
    }

    pub fn exhausted(&self) -> bool {
        self.used >= self.limit
    }

    /// Consume budget; fails closed when the limit would be exceeded.
    pub fn consume(&mut self, amount: u64) -> Result<(), AgentsError> {
        let next = self.used.saturating_add(amount);
        if next > self.limit {
            return Err(AgentsError::policy(
                "budget exhausted",
                Some("agent-budget".into()),
            ));
        }
        self.used = next;
        Ok(())
    }
}
