//! nexus-test-performance: EP-040 performance budget evaluation root
//! (SPEC-008; TESTING.md performance layer).
//!
//! M1 owns the deterministic evaluation model for PerformanceBudget:
//! a budget is met only when a real observed value is within the declared
//! bound. Missing observation, stale observation, and unobserved budgets
//! fail closed. BUILD PASSED != RUNTIME SAFE: compile success never
//! satisfies a performance budget.
//!
//! Real performance harnesses, load generators, and hardware timing are
//! NOT asserted in M1; later milestones own live performance evidence.

use nexus_test_contract::error::{TestingError, TestingResult};
use nexus_test_contract::model::PerformanceBudget;
use nexus_test_contract::PerformanceBudgetPort;

/// Deterministic performance budget evaluator. Fail-closed on missing
/// observation; typed failure for over-budget evidence.
#[derive(Debug, Default)]
pub struct DeterministicBudgetEvaluator;

impl DeterministicBudgetEvaluator {
    pub fn new() -> Self {
        Self
    }

    /// Evaluate an observed value against a budget. Returns Ok only when
    /// the budget was observed and the observed value is within bound.
    pub fn evaluate_observed(
        &self,
        budget: &PerformanceBudget,
        observed: f64,
    ) -> TestingResult<()> {
        if budget.metric.trim().is_empty() {
            return Err(TestingError::validation("budget metric is required"));
        }
        if budget.max_value < 0.0 {
            return Err(TestingError::validation(
                "budget max_value must be non-negative",
            ));
        }
        if observed > budget.max_value {
            return Err(TestingError::policy(format!(
                "budget {} exceeded: observed {} > max {} {}",
                budget.id, observed, budget.max_value, budget.unit
            )));
        }
        Ok(())
    }
}

impl PerformanceBudgetPort for DeterministicBudgetEvaluator {
    fn evaluate(&self, budget: &PerformanceBudget) -> TestingResult<()> {
        match budget.observed_value {
            Some(v) => self.evaluate_observed(budget, v),
            None => Err(TestingError::missing_evidence(format!(
                "budget {} has no observed value; missing observation is never green",
                budget.id
            ))),
        }
    }
}

/// A budget is never met without a real observation (BUILD PASSED !=
/// RUNTIME SAFE). This is the model-level guard the evaluator enforces.
pub fn budget_met_fail_closed(budget: &PerformanceBudget) -> bool {
    budget.observed && budget.observed_value.is_some_and(|v| v <= budget.max_value)
}
