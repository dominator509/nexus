//! Model budget contract (EP-013 node contract `ModelBudget`).

use crate::error::ModelGatewayError;
use serde::{Deserialize, Serialize};

/// Budget decision (fail closed on exhaustion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BudgetDecision {
    Allowed,
    Denied,
}

impl BudgetDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "ALLOWED",
            Self::Denied => "DENIED",
        }
    }
}

/// Deterministic budget ledger: tokens and cost accounting per
/// tenant/budget reference. A model request that exceeds the declared
/// budget fails closed BEFORE any provider call.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetLedger {
    pub budget_ref: String,
    pub token_budget: u64,
    pub tokens_used: u64,
}

impl BudgetLedger {
    pub fn new(budget_ref: impl Into<String>, token_budget: u64) -> Self {
        Self {
            budget_ref: budget_ref.into(),
            token_budget,
            tokens_used: 0,
        }
    }

    pub fn remaining(&self) -> u64 {
        self.token_budget.saturating_sub(self.tokens_used)
    }

    /// Check a prospective token cost against the remaining budget.
    pub fn check(&self, cost: u64) -> BudgetDecision {
        if cost > self.remaining() {
            BudgetDecision::Denied
        } else {
            BudgetDecision::Allowed
        }
    }

    /// Record usage (caller must have checked first).
    pub fn record(&mut self, cost: u64) -> Result<(), ModelGatewayError> {
        if self.check(cost) == BudgetDecision::Denied {
            return Err(ModelGatewayError::new(
                crate::error::ModelGatewayErrorCode::Conflict,
                format!("budget exhausted for {}", self.budget_ref),
                None,
                None,
                None,
                Some(self.budget_ref.clone()),
            ));
        }
        self.tokens_used += cost;
        Ok(())
    }
}

/// Model budget port: the gateway asks the budget before routing.
pub trait ModelBudget {
    /// Check whether the request may proceed against the declared
    /// budget.
    fn check(
        &self,
        request: &crate::model::ModelRequest,
    ) -> Result<BudgetDecision, ModelGatewayError>;

    /// Record usage after a provider call (idempotent by request id).
    fn record(
        &mut self,
        request: &crate::model::ModelRequest,
        usage: &crate::model::UsageReport,
    ) -> Result<(), ModelGatewayError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep013_unit_budget_ledger_check_and_record() {
        let mut ledger = BudgetLedger::new("b-1", 100);
        assert_eq!(ledger.remaining(), 100);
        assert_eq!(ledger.check(50), BudgetDecision::Allowed);
        ledger.record(50).unwrap();
        assert_eq!(ledger.remaining(), 50);
        assert_eq!(ledger.check(60), BudgetDecision::Denied);
        assert!(ledger.record(60).is_err());
        assert_eq!(ledger.remaining(), 50);
    }

    #[test]
    fn ep013_unit_budget_ledger_exact_boundary() {
        let mut ledger = BudgetLedger::new("b-2", 10);
        assert_eq!(ledger.check(10), BudgetDecision::Allowed);
        ledger.record(10).unwrap();
        assert_eq!(ledger.remaining(), 0);
        assert_eq!(ledger.check(1), BudgetDecision::Denied);
    }

    #[test]
    fn ep013_unit_budget_ledger_zero_budget_denies() {
        let ledger = BudgetLedger::new("b-3", 0);
        assert_eq!(ledger.check(1), BudgetDecision::Denied);
    }

    #[test]
    fn ep013_unit_budget_decision_round_trip() {
        assert_eq!(BudgetDecision::Allowed.as_str(), "ALLOWED");
        assert_eq!(BudgetDecision::Denied.as_str(), "DENIED");
        let v = serde_json::to_value(BudgetDecision::Denied).unwrap();
        assert_eq!(v, serde_json::json!("DENIED"));
    }
}
