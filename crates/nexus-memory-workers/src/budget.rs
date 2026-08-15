//! Deterministic context budget (SPEC-002; EP-016 M2).
//!
//! ContextEngine must respect bounded context size. The budget
//! allocates `max_items` across priority classes in a fixed order so
//! low-value retrieved memories cannot crowd out required state.

use nexus_context::ContextError;

/// Budget priority class (EP-016 Decision Log; SPEC-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BudgetClass {
    /// Required exact facts (direct entity/identifier matches).
    RequiredExact,
    /// Current objective / task state.
    ObjectiveState,
    /// Critical recent events.
    CriticalRecent,
    /// High-value retrieved memories.
    HighValueRetrieved,
    /// Supporting graph context.
    GraphContext,
    /// Optional semantic context.
    OptionalSemantic,
}

impl BudgetClass {
    pub const ALL: [BudgetClass; 6] = [
        BudgetClass::RequiredExact,
        BudgetClass::ObjectiveState,
        BudgetClass::CriticalRecent,
        BudgetClass::HighValueRetrieved,
        BudgetClass::GraphContext,
        BudgetClass::OptionalSemantic,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RequiredExact => "REQUIRED_EXACT",
            Self::ObjectiveState => "OBJECTIVE_STATE",
            Self::CriticalRecent => "CRITICAL_RECENT",
            Self::HighValueRetrieved => "HIGH_VALUE_RETRIEVED",
            Self::GraphContext => "GRAPH_CONTEXT",
            Self::OptionalSemantic => "OPTIONAL_SEMANTIC",
        }
    }
}

/// A per-class budget share.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetShare {
    pub class: BudgetClass,
    pub max: usize,
}

/// Deterministic budget allocation over the priority classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBudget {
    pub total: usize,
    pub shares: Vec<BudgetShare>,
}

impl ContextBudget {
    /// Allocate `max_items` deterministically. Required exact facts get
    /// the largest share; optional semantic context the smallest.
    /// Allocation is a pure function of `max_items`.
    pub fn allocate(max_items: usize) -> Result<Self, ContextError> {
        if max_items == 0 {
            return Err(ContextError::validation(
                "budget max_items must be positive",
                Some("context-budget".into()),
            ));
        }
        // Fixed proportional weights (sum 100).
        let weights: [(BudgetClass, u32); 6] = [
            (BudgetClass::RequiredExact, 30),
            (BudgetClass::ObjectiveState, 20),
            (BudgetClass::CriticalRecent, 20),
            (BudgetClass::HighValueRetrieved, 15),
            (BudgetClass::GraphContext, 10),
            (BudgetClass::OptionalSemantic, 5),
        ];
        let mut shares = Vec::with_capacity(weights.len());
        let mut remaining = max_items;
        for (index, (class, weight)) in weights.iter().enumerate() {
            // Deterministic floor allocation; the final class absorbs the
            // remainder so total is exact. Required exact facts always
            // get at least one slot so low-value retrieved memories can
            // never crowd out required state (SPEC-002 budget order).
            let raw = (max_items as u64 * u64::from(*weight)) / 100;
            let mut max = raw as usize;
            if index == 0 && max == 0 && max_items >= 1 {
                max = 1;
            }
            if index == weights.len() - 1 {
                max = remaining;
            }
            shares.push(BudgetShare { class: *class, max });
            remaining = remaining.saturating_sub(max);
        }
        Ok(Self {
            total: max_items,
            shares,
        })
    }

    /// The per-class cap for a class.
    pub fn cap(&self, class: BudgetClass) -> usize {
        self.shares
            .iter()
            .find(|s| s.class == class)
            .map(|s| s.max)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep016_unit_budget_allocates_deterministically() {
        let budget = ContextBudget::allocate(100).unwrap();
        assert_eq!(budget.total, 100);
        let sum: usize = budget.shares.iter().map(|s| s.max).sum();
        assert_eq!(sum, 100);
        // Required exact gets the largest share.
        assert!(
            budget.cap(BudgetClass::RequiredExact) >= budget.cap(BudgetClass::HighValueRetrieved)
        );
    }

    #[test]
    fn ep016_unit_budget_small_total_still_exact() {
        let budget = ContextBudget::allocate(3).unwrap();
        let sum: usize = budget.shares.iter().map(|s| s.max).sum();
        assert_eq!(sum, 3);
        // Low-value classes may receive zero but required exact always
        // has room (floor allocation).
        assert!(budget.cap(BudgetClass::RequiredExact) > 0);
    }

    #[test]
    fn ep016_unit_budget_rejects_zero() {
        assert!(ContextBudget::allocate(0).is_err());
    }

    #[test]
    fn ep016_unit_budget_ordering_never_reversed() {
        let budget = ContextBudget::allocate(200).unwrap();
        for pair in BudgetClass::ALL.windows(2) {
            assert!(budget.cap(pair[0]) >= budget.cap(pair[1]));
        }
    }
}
