//! Deterministic effort policy (SPEC-009 required behavior 2; ADR-021).
//!
//! Effort tiers are selected by policy, not by the caller's whim. The
//! policy maps a request's effort inputs to an `EffortTier`; MAX is
//! never the default for trivial work. A request may also carry an
//! explicit tier, but the policy still governs what is permitted.

use crate::error::ReflexError;
use crate::vocabulary::EffortSelectionClass;
use nexus_model_gateway::vocabulary::EffortTier;
use serde::{Deserialize, Serialize};

/// Inputs consumed by the deterministic effort policy.
///
/// Kept intentionally small and typed: the policy only needs the
/// request's deterministic classification (is this a deterministic
/// task? is this trivial work?) to select a tier. Provider-neutral.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffortInput {
    /// True when the task is fully deterministic (no model needed).
    pub deterministic: bool,
    /// True when the task is trivial work (low value, low risk).
    pub trivial: bool,
    /// Optional explicit tier requested by the caller.
    pub explicit_tier: Option<EffortTier>,
}

impl EffortInput {
    pub fn new(tier: EffortTier) -> Self {
        Self {
            deterministic: tier == EffortTier::Deterministic,
            trivial: false,
            explicit_tier: Some(tier),
        }
    }

    pub fn deterministic() -> Self {
        Self {
            deterministic: true,
            trivial: false,
            explicit_tier: Some(EffortTier::Deterministic),
        }
    }

    pub fn trivial() -> Self {
        Self {
            deterministic: false,
            trivial: true,
            explicit_tier: None,
        }
    }

    /// The effective tier after policy selection.
    pub fn tier(&self) -> EffortTier {
        EffortPolicy::select(self)
    }
}

/// Deterministic effort-tier selection policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EffortPolicy;

impl EffortPolicy {
    pub fn new() -> Self {
        Self
    }

    /// Select the effort tier for an input.
    ///
    /// Rules (SPEC-009 required behavior 2):
    /// - Deterministic tasks resolve to `Deterministic` (bypass model).
    /// - Trivial work is NEVER `Max`; it resolves to `NonThinking`.
    /// - An explicit non-deterministic tier is honored (caller intent),
    ///   but `Max` still requires the task not be trivial.
    /// - Default is `High`.
    pub fn select(input: &EffortInput) -> EffortTier {
        if input.deterministic {
            return EffortTier::Deterministic;
        }
        if input.trivial {
            // Max is never the default for trivial work.
            return EffortTier::NonThinking;
        }
        match input.explicit_tier {
            Some(EffortTier::Deterministic) => EffortTier::Deterministic,
            Some(EffortTier::NonThinking) => EffortTier::NonThinking,
            Some(EffortTier::High) => EffortTier::High,
            Some(EffortTier::Max) => EffortTier::Max,
            Some(EffortTier::Specialist) => EffortTier::Specialist,
            None => EffortTier::High,
        }
    }

    /// Classify how a tier was chosen.
    pub fn selection_class(input: &EffortInput) -> EffortSelectionClass {
        if input.explicit_tier.is_some() {
            EffortSelectionClass::Explicit
        } else {
            EffortSelectionClass::PolicySelected
        }
    }

    /// Validate an explicit tier for a request.
    ///
    /// Trivial work cannot request `Max`; a deterministic request
    /// cannot request a thinking tier (it would defeat the bypass).
    pub fn validate(input: &EffortInput) -> Result<(), ReflexError> {
        if input.trivial && input.explicit_tier == Some(EffortTier::Max) {
            return Err(ReflexError::validation(
                "max effort is not permitted for trivial work",
                Some("effort-policy".into()),
            ));
        }
        if input.deterministic
            && matches!(
                input.explicit_tier,
                Some(EffortTier::NonThinking)
                    | Some(EffortTier::High)
                    | Some(EffortTier::Max)
                    | Some(EffortTier::Specialist)
            )
        {
            return Err(ReflexError::validation(
                "deterministic task cannot request a thinking effort tier",
                Some("effort-policy".into()),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep014_unit_deterministic_tasks_resolve_to_deterministic() {
        assert_eq!(
            EffortPolicy::select(&EffortInput::deterministic()),
            EffortTier::Deterministic
        );
    }

    #[test]
    fn ep014_unit_trivial_work_is_never_max() {
        // Trivial work with NO explicit tier -> NonThinking.
        assert_eq!(
            EffortPolicy::select(&EffortInput::trivial()),
            EffortTier::NonThinking
        );
        // Even an explicit Max on trivial work is rejected by validate.
        let mut input = EffortInput::trivial();
        input.explicit_tier = Some(EffortTier::Max);
        assert!(EffortPolicy::validate(&input).is_err());
        // select() is total and MUST NOT return Max for trivial work.
        assert_eq!(EffortPolicy::select(&input), EffortTier::NonThinking);
    }

    #[test]
    fn ep014_unit_default_is_high() {
        let input = EffortInput {
            deterministic: false,
            trivial: false,
            explicit_tier: None,
        };
        assert_eq!(EffortPolicy::select(&input), EffortTier::High);
        assert_eq!(
            EffortPolicy::selection_class(&input),
            EffortSelectionClass::PolicySelected
        );
    }

    #[test]
    fn ep014_unit_explicit_tiers_are_honored() {
        for tier in [
            EffortTier::NonThinking,
            EffortTier::High,
            EffortTier::Max,
            EffortTier::Specialist,
        ] {
            let input = EffortInput::new(tier);
            assert_eq!(EffortPolicy::select(&input), tier);
            assert_eq!(
                EffortPolicy::selection_class(&input),
                EffortSelectionClass::Explicit
            );
        }
    }

    #[test]
    fn ep014_unit_deterministic_task_rejects_thinking_tier() {
        let mut input = EffortInput::deterministic();
        input.explicit_tier = Some(EffortTier::High);
        assert!(EffortPolicy::validate(&input).is_err());
    }

    #[test]
    fn ep014_unit_effort_input_serde_round_trip() {
        let input = EffortInput::new(EffortTier::High);
        let v = serde_json::to_value(&input).unwrap();
        let back: EffortInput = serde_json::from_value(v).unwrap();
        assert_eq!(back, input);
        assert_eq!(back.tier(), EffortTier::High);
    }

    #[test]
    fn ep014_unit_effort_tier_rejects_unknown() {
        assert!("ULTRA".parse::<EffortTier>().is_err());
    }
}
