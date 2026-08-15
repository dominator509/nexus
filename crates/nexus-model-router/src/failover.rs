//! Provider failover plane (EP-015 M5; ADR-022; LF-021).
//!
//! Production failover surface for the model router. After the
//! deterministic router selects a route, `route_with_failover` attempts
//! the configured primary `ReflexProvider` through its real transport,
//! classifies the typed failure with `ProviderFailoverPolicy`, and -
//! only when the failure class is failover-eligible (UNAVAILABLE or
//! TIMEOUT) - selects and attempts the configured secondary provider.
//!
//! Invariants (LF-021):
//! - The real router observes the primary failure and decides to fail
//!   over; the proof never calls the secondary directly.
//! - Budgets (cost/latency/attempts) carry forward across attempts; the
//!   secondary receives the remaining budget, never a fresh cap.
//! - The Nexus trace/correlation id is never replaced.
//! - Every provider result passes the same canonical
//!   `NexusControlObject` validation contract (the provider's own
//!   validator; malformed/contract-invalid responses fail closed).
//! - Security policy dominates availability: a prohibited secondary is
//!   never used (same `RoutePolicy::override_security` and
//!   `EscalationPolicy` surfaces the primary route passed).
//! - Every path fails closed with bounded attempts; no provider
//!   cycling, no route recursion, no fabricated control object.

use crate::config::RouterPolicyConfig;
use crate::decision::RoutingDecision;
use crate::escalation::EscalationOutcome;
use crate::features::RoutingFeatures;
use crate::router::{DeterministicModelRouter, NexusModelRouter};
use crate::vocabulary::{FailoverStage, ProviderFailureClass, RoutingDecisionClass};
use nexus_domain::vocabulary::Route;
use nexus_reflex::{ReflexError, ReflexErrorCode, ReflexProvider, ReflexRequest};
use serde::{Deserialize, Serialize};

/// Deterministic provider failover policy (config-driven from the
/// canonical `config/models/router/policy.json` `failover` section;
/// ADR-022). `new()` uses the code defaults that match the artifact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProviderFailoverPolicy {
    /// Maximum provider attempts per request (primary + failover).
    pub max_provider_attempts: usize,
    /// Cost-budget units (0..=1) consumed by each provider attempt.
    pub per_attempt_cost: f64,
    /// Latency-budget milliseconds consumed by each provider attempt.
    pub per_attempt_latency_ms: u64,
}

impl ProviderFailoverPolicy {
    pub fn new() -> Self {
        Self::from_config(&RouterPolicyConfig::default())
    }

    pub fn from_config(config: &RouterPolicyConfig) -> Self {
        Self {
            max_provider_attempts: config.failover.max_provider_attempts,
            per_attempt_cost: config.failover.attempt_cost,
            per_attempt_latency_ms: config.failover.attempt_latency_ms,
        }
    }

    /// Classify a typed provider failure. Only UNAVAILABLE and TIMEOUT
    /// are failover-eligible; contract, rate, external, policy, budget,
    /// and security failures never cause provider hopping.
    pub fn classify(&self, error: &ReflexError) -> ProviderFailureClass {
        match error.code {
            ReflexErrorCode::Unavailable => ProviderFailureClass::Unavailable,
            ReflexErrorCode::Timeout => ProviderFailureClass::Timeout,
            ReflexErrorCode::RateLimited => ProviderFailureClass::RateLimited,
            ReflexErrorCode::ExternalProvider => ProviderFailureClass::External,
            _ => ProviderFailureClass::Contract,
        }
    }

    /// Failover eligibility: availability and timeout are eligible;
    /// everything else fails closed without provider hopping.
    pub fn is_failover_eligible(&self, class: ProviderFailureClass) -> bool {
        matches!(
            class,
            ProviderFailureClass::Unavailable | ProviderFailureClass::Timeout
        )
    }

    /// Cost-budget milli-units consumed per attempt (budget caps use
    /// `cost * 1000` units; see `EscalationPolicy`).
    pub fn per_attempt_cost_milli(&self) -> u64 {
        (self.per_attempt_cost * 1000.0) as u64
    }
}

impl Default for ProviderFailoverPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed fail-closed failure for the failover plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailoverFailure {
    pub class: ProviderFailureClass,
    /// Redacted deterministic reason. Never credentials, prompts, or
    /// provider endpoint details.
    pub reason: String,
}

/// Outcome of a failover-routed request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailoverOutcome {
    pub request_id: String,
    pub correlation_id: String,
    pub decision: RoutingDecision,
    /// Validated final control object (None when fail-closed).
    pub final_reflex: Option<nexus_reflex::ReflexDecision>,
    /// Typed fail-closed failure (None when a provider succeeded).
    pub failure: Option<FailoverFailure>,
    /// Provider attempts consumed (bounded by `max_provider_attempts`).
    pub provider_attempts: usize,
    pub max_provider_attempts: usize,
    /// Cost budget remaining after the final attempt (None = unbounded).
    pub remaining_budget: Option<u64>,
    /// Budget carried into the secondary attempt (None unless failover
    /// actually selected a secondary). Proves the secondary received
    /// the remaining budget, never a fresh cap.
    pub secondary_received_budget: Option<u64>,
    /// Latency budget remaining after the final attempt (ms).
    pub remaining_latency_ms: u64,
}

impl FailoverOutcome {
    fn new(
        request_id: &str,
        correlation_id: &str,
        decision: RoutingDecision,
        policy: &ProviderFailoverPolicy,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            correlation_id: correlation_id.into(),
            decision,
            final_reflex: None,
            failure: None,
            provider_attempts: 0,
            max_provider_attempts: policy.max_provider_attempts,
            remaining_budget: None,
            secondary_received_budget: None,
            remaining_latency_ms: 0,
        }
    }
}

impl DeterministicModelRouter {
    /// Route a request with real provider failover (LF-021; ADR-022).
    ///
    /// The deterministic router selects the primary route; the primary
    /// `ReflexProvider` is attempted through its real transport; a
    /// typed failover-eligible failure (UNAVAILABLE/TIMEOUT) selects
    /// the configured secondary provider, which is attempted with the
    /// remaining budgets and the same Nexus trace id. Every provider
    /// result passes the same canonical `NexusControlObject` validation
    /// contract. Security policy dominates: a prohibited secondary is
    /// never used. All paths fail closed with bounded attempts.
    #[allow(clippy::too_many_arguments)]
    pub fn route_with_failover(
        &mut self,
        request_id: &str,
        correlation_id: &str,
        features: &RoutingFeatures,
        request: &ReflexRequest,
        primary: &mut dyn ReflexProvider,
        secondary: &mut dyn ReflexProvider,
        secondary_route: Route,
    ) -> Result<FailoverOutcome, crate::error::RouterError> {
        let policy = ProviderFailoverPolicy::new();

        // The real router decides the primary route (the decision audit
        // record is the route_requested/primary_selected event).
        let decision = self.route(request_id, correlation_id, features)?;
        let mut outcome =
            FailoverOutcome::new(request_id, correlation_id, decision.clone(), &policy);
        let primary_id = primary.provider_id().to_string();
        let secondary_id = secondary.provider_id().to_string();

        // Budgets start at the request's caps and carry forward.
        let mut remaining_budget = features.budget;
        let mut remaining_latency_ms = features.latency_ms;
        let mut attempts = 0usize;

        // Non-failover: a deterministic policy denial (REJECTED,
        // FALLBACK, ESCALATED) never triggers a provider attempt.
        if decision.class != RoutingDecisionClass::Routed {
            outcome.failure = Some(FailoverFailure {
                class: ProviderFailureClass::Rejected,
                reason: format!(
                    "router decision {}; no provider attempt",
                    decision.class.as_str()
                ),
            });
            outcome.remaining_budget = remaining_budget;
            outcome.remaining_latency_ms = remaining_latency_ms;
            self.emit_failover_stage(
                &decision,
                FailoverStage::FailedClosed,
                Some(primary_id),
                Some(ProviderFailureClass::Rejected),
            );
            return Ok(outcome);
        }

        // Primary attempt (the failed attempt consumes per-attempt
        // budget and one of the bounded provider attempts).
        if attempts >= policy.max_provider_attempts {
            outcome.failure = Some(FailoverFailure {
                class: ProviderFailureClass::BudgetExhausted,
                reason: "no provider attempts remaining".into(),
            });
            outcome.remaining_budget = remaining_budget;
            outcome.remaining_latency_ms = remaining_latency_ms;
            self.emit_failover_stage(
                &decision,
                FailoverStage::FailedClosed,
                Some(primary_id),
                Some(ProviderFailureClass::BudgetExhausted),
            );
            return Ok(outcome);
        }
        attempts += 1;
        remaining_budget =
            remaining_budget.map(|b| b.saturating_sub(policy.per_attempt_cost_milli()));
        remaining_latency_ms = remaining_latency_ms.saturating_sub(policy.per_attempt_latency_ms);
        self.emit_failover_stage(
            &decision,
            FailoverStage::PrimarySelected,
            Some(primary_id.clone()),
            None,
        );
        self.emit_failover_stage(
            &decision,
            FailoverStage::PrimaryAttempted,
            Some(primary_id.clone()),
            None,
        );
        match primary.reflex(request) {
            Ok(reflex) => {
                outcome.final_reflex = Some(reflex);
                outcome.provider_attempts = attempts;
                outcome.remaining_budget = remaining_budget;
                outcome.remaining_latency_ms = remaining_latency_ms;
                self.emit_failover_stage(
                    &decision,
                    FailoverStage::RouteCompleted,
                    Some(primary_id),
                    None,
                );
                return Ok(outcome);
            }
            Err(err) => {
                let class = policy.classify(&err);
                outcome.provider_attempts = attempts;
                self.emit_failover_stage(
                    &decision,
                    FailoverStage::PrimaryFailed,
                    Some(primary_id.clone()),
                    Some(class),
                );
                if !policy.is_failover_eligible(class) {
                    // Typed non-failover failure: fail closed; the
                    // secondary is never attempted.
                    outcome.failure = Some(FailoverFailure {
                        class,
                        reason: format!(
                            "primary provider failed with non-failover class {}",
                            class.as_str()
                        ),
                    });
                    outcome.remaining_budget = remaining_budget;
                    outcome.remaining_latency_ms = remaining_latency_ms;
                    self.emit_failover_stage(
                        &decision,
                        FailoverStage::FailedClosed,
                        Some(primary_id),
                        Some(class),
                    );
                    return Ok(outcome);
                }
                self.emit_failover_stage(
                    &decision,
                    FailoverStage::FailoverEligible,
                    Some(primary_id.clone()),
                    Some(class),
                );
            }
        }

        // Security policy dominates availability: the secondary's tier
        // must pass the same production policy surfaces the primary
        // route passed. A prohibited secondary is never used.
        let secured = self.policy().override_security(features, secondary_route)?;
        if secured != secondary_route
            || !matches!(
                self.escalation().escalate(features, secondary_route),
                EscalationOutcome::None
            )
        {
            outcome.failure = Some(FailoverFailure {
                class: ProviderFailureClass::SecurityDenied,
                reason: "secondary provider prohibited by security policy".into(),
            });
            outcome.remaining_budget = remaining_budget;
            outcome.remaining_latency_ms = remaining_latency_ms;
            self.emit_failover_stage(
                &decision,
                FailoverStage::FailedClosed,
                Some(secondary_id),
                Some(ProviderFailureClass::SecurityDenied),
            );
            return Ok(outcome);
        }

        // Budgets carry forward: the secondary receives the remaining
        // budgets (never a fresh cap) and must still fit within them.
        outcome.secondary_received_budget = remaining_budget;
        if attempts >= policy.max_provider_attempts
            || remaining_budget.is_some_and(|b| b < policy.per_attempt_cost_milli())
            || remaining_latency_ms < policy.per_attempt_latency_ms
        {
            outcome.failure = Some(FailoverFailure {
                class: ProviderFailureClass::BudgetExhausted,
                reason: "remaining budget insufficient for secondary attempt".into(),
            });
            outcome.remaining_budget = remaining_budget;
            outcome.remaining_latency_ms = remaining_latency_ms;
            self.emit_failover_stage(
                &decision,
                FailoverStage::FailedClosed,
                Some(secondary_id),
                Some(ProviderFailureClass::BudgetExhausted),
            );
            return Ok(outcome);
        }

        // Secondary attempt.
        attempts += 1;
        remaining_budget =
            remaining_budget.map(|b| b.saturating_sub(policy.per_attempt_cost_milli()));
        remaining_latency_ms = remaining_latency_ms.saturating_sub(policy.per_attempt_latency_ms);
        self.emit_failover_stage(
            &decision,
            FailoverStage::SecondarySelected,
            Some(secondary_id.clone()),
            None,
        );
        self.emit_failover_stage(
            &decision,
            FailoverStage::SecondaryAttempted,
            Some(secondary_id.clone()),
            None,
        );
        match secondary.reflex(request) {
            Ok(reflex) => {
                outcome.final_reflex = Some(reflex);
                outcome.provider_attempts = attempts;
                outcome.remaining_budget = remaining_budget;
                outcome.remaining_latency_ms = remaining_latency_ms;
                self.emit_failover_stage(
                    &decision,
                    FailoverStage::SecondaryValidated,
                    Some(secondary_id.clone()),
                    None,
                );
                self.emit_failover_stage(
                    &decision,
                    FailoverStage::RouteCompleted,
                    Some(secondary_id),
                    None,
                );
                Ok(outcome)
            }
            Err(err) => {
                let class = policy.classify(&err);
                outcome.failure = Some(FailoverFailure {
                    class,
                    reason: format!(
                        "secondary provider failed; fail closed ({})",
                        class.as_str()
                    ),
                });
                outcome.provider_attempts = attempts;
                outcome.remaining_budget = remaining_budget;
                outcome.remaining_latency_ms = remaining_latency_ms;
                self.emit_failover_stage(
                    &decision,
                    FailoverStage::FailedClosed,
                    Some(secondary_id),
                    Some(class),
                );
                Ok(outcome)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_reflex::{ReflexError, ReflexErrorCode};

    fn error(code: ReflexErrorCode) -> ReflexError {
        // Redacted deterministic error; the classifier only reads the
        // typed code.
        match code {
            ReflexErrorCode::Unavailable => ReflexError::unavailable("primary unavailable", None),
            ReflexErrorCode::Timeout => ReflexError::timeout("primary timed out", None),
            ReflexErrorCode::Validation => ReflexError::validation("contract invalid", None),
            ReflexErrorCode::RateLimited => ReflexError::rate_limited("rate limited", None),
            _ => ReflexError::internal("unexpected", None),
        }
    }

    #[test]
    fn ep015_unit_failover_policy_classifies_typed_failures() {
        let policy = ProviderFailoverPolicy::new();
        assert_eq!(
            policy.classify(&error(ReflexErrorCode::Unavailable)),
            ProviderFailureClass::Unavailable
        );
        assert_eq!(
            policy.classify(&error(ReflexErrorCode::Timeout)),
            ProviderFailureClass::Timeout
        );
        assert_eq!(
            policy.classify(&error(ReflexErrorCode::Validation)),
            ProviderFailureClass::Contract
        );
        assert_eq!(
            policy.classify(&error(ReflexErrorCode::RateLimited)),
            ProviderFailureClass::RateLimited
        );
    }

    #[test]
    fn ep015_unit_failover_policy_eligibility_is_locked() {
        let policy = ProviderFailoverPolicy::new();
        // Only availability and timeout are failover-eligible.
        assert!(policy.is_failover_eligible(ProviderFailureClass::Unavailable));
        assert!(policy.is_failover_eligible(ProviderFailureClass::Timeout));
        for class in [
            ProviderFailureClass::RateLimited,
            ProviderFailureClass::Contract,
            ProviderFailureClass::External,
            ProviderFailureClass::Rejected,
            ProviderFailureClass::BudgetExhausted,
            ProviderFailureClass::SecurityDenied,
        ] {
            assert!(
                !policy.is_failover_eligible(class),
                "{class:?} must not fail over"
            );
        }
    }

    #[test]
    fn ep015_unit_failover_policy_budget_math_is_deterministic() {
        let policy = ProviderFailoverPolicy::new();
        // Canonical artifact: 2 attempts max, 0.1 cost per attempt
        // (100 milli-units), 100 ms latency per attempt.
        assert_eq!(policy.max_provider_attempts, 2);
        assert_eq!(policy.per_attempt_cost_milli(), 100);
        // A primary failure consumes one attempt + 100 milli-cost + 100 ms.
        let initial_budget = Some(1000u64);
        let after_primary =
            initial_budget.map(|b| b.saturating_sub(policy.per_attempt_cost_milli()));
        assert_eq!(after_primary, Some(900));
        assert!(after_primary < initial_budget);
    }

    #[test]
    fn ep015_unit_failover_policy_from_canonical_artifact_matches_defaults() {
        let file = ProviderFailoverPolicy::from_config(
            &crate::config::RouterPolicyConfig::from_canonical_file().unwrap(),
        );
        assert_eq!(file, ProviderFailoverPolicy::new());
    }
}
