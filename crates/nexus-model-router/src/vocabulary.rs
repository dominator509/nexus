//! EP-015 router vocabulary (SPEC-009, SPEC-025; ADR-022).
//!
//! New vocabulary-locked names owned by the model router plane. The
//! canonical `Route`, `Risk`, and `Privacy` classes come from
//! `nexus-domain`; `EffortTier`/`ProviderHealth`/`CacheHitRatio` from
//! `nexus-model-gateway`; the `ReflexProvider` contract from
//! `nexus-reflex`. Every enum here rejects unknown classes at parse time.

use serde::{Deserialize, Serialize};

/// Router vocabulary parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouterVocabularyError(pub String);

impl RouterVocabularyError {
    pub fn unknown(class: &str, value: &str) -> Self {
        Self(format!("unknown {class}: {value}"))
    }
}

impl std::fmt::Display for RouterVocabularyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for RouterVocabularyError {}

/// Routing decision class (SPEC-009 canonical term ModelRoute; ADR-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RoutingDecisionClass {
    Routed,
    Fallback,
    Escalated,
    Rejected,
    Shadow,
}

impl RoutingDecisionClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Routed => "ROUTED",
            Self::Fallback => "FALLBACK",
            Self::Escalated => "ESCALATED",
            Self::Rejected => "REJECTED",
            Self::Shadow => "SHADOW",
        }
    }
}

impl std::str::FromStr for RoutingDecisionClass {
    type Err = RouterVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ROUTED" => Ok(Self::Routed),
            "FALLBACK" => Ok(Self::Fallback),
            "ESCALATED" => Ok(Self::Escalated),
            "REJECTED" => Ok(Self::Rejected),
            "SHADOW" => Ok(Self::Shadow),
            other => Err(RouterVocabularyError::unknown(
                "RoutingDecisionClass",
                other,
            )),
        }
    }
}

/// Router strategy class (SPEC-009: RouteLLM/LLMRouter replaceable;
/// ADR-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RouterStrategyClass {
    Policy,
    RouteLlm,
    LlmRouter,
    Microbrain,
}

impl RouterStrategyClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Policy => "POLICY",
            Self::RouteLlm => "ROUTE_LLM",
            Self::LlmRouter => "LLM_ROUTER",
            Self::Microbrain => "MICROBRAIN",
        }
    }
}

impl std::str::FromStr for RouterStrategyClass {
    type Err = RouterVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "POLICY" => Ok(Self::Policy),
            "ROUTE_LLM" => Ok(Self::RouteLlm),
            "LLM_ROUTER" => Ok(Self::LlmRouter),
            "MICROBRAIN" => Ok(Self::Microbrain),
            other => Err(RouterVocabularyError::unknown("RouterStrategyClass", other)),
        }
    }
}

/// Escalation reason (SPEC-009 canonical term Escalation; ADR-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EscalationReason {
    Ambiguity,
    Risk,
    Privacy,
    Budget,
    Unavailable,
    Cost,
    Latency,
    Security,
    Certification,
    OutOfDistribution,
}

impl EscalationReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ambiguity => "AMBIGUITY",
            Self::Risk => "RISK",
            Self::Privacy => "PRIVACY",
            Self::Budget => "BUDGET",
            Self::Unavailable => "UNAVAILABLE",
            Self::Cost => "COST",
            Self::Latency => "LATENCY",
            Self::Security => "SECURITY",
            Self::Certification => "CERTIFICATION",
            Self::OutOfDistribution => "OUT_OF_DISTRIBUTION",
        }
    }
}

impl std::str::FromStr for EscalationReason {
    type Err = RouterVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "AMBIGUITY" => Ok(Self::Ambiguity),
            "RISK" => Ok(Self::Risk),
            "PRIVACY" => Ok(Self::Privacy),
            "BUDGET" => Ok(Self::Budget),
            "UNAVAILABLE" => Ok(Self::Unavailable),
            "COST" => Ok(Self::Cost),
            "LATENCY" => Ok(Self::Latency),
            "SECURITY" => Ok(Self::Security),
            "CERTIFICATION" => Ok(Self::Certification),
            "OUT_OF_DISTRIBUTION" => Ok(Self::OutOfDistribution),
            other => Err(RouterVocabularyError::unknown("EscalationReason", other)),
        }
    }
}

/// Microbrain state (SPEC-025; ADR-022). The Microbrain begins in
/// shadow and can remain disabled; promotion is gated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MicrobrainState {
    Disabled,
    Shadow,
    Canary,
    Active,
    PromotionGated,
}

impl MicrobrainState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "DISABLED",
            Self::Shadow => "SHADOW",
            Self::Canary => "CANARY",
            Self::Active => "ACTIVE",
            Self::PromotionGated => "PROMOTION_GATED",
        }
    }
}

impl std::str::FromStr for MicrobrainState {
    type Err = RouterVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "DISABLED" => Ok(Self::Disabled),
            "SHADOW" => Ok(Self::Shadow),
            "CANARY" => Ok(Self::Canary),
            "ACTIVE" => Ok(Self::Active),
            "PROMOTION_GATED" => Ok(Self::PromotionGated),
            other => Err(RouterVocabularyError::unknown("MicrobrainState", other)),
        }
    }
}

/// Shadow decision class (SPEC-025 canonical term ShadowDecision;
/// ADR-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShadowDecisionClass {
    Match,
    Diverge,
    Failed,
}

impl ShadowDecisionClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Match => "MATCH",
            Self::Diverge => "DIVERGE",
            Self::Failed => "FAILED",
        }
    }
}

impl std::str::FromStr for ShadowDecisionClass {
    type Err = RouterVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "MATCH" => Ok(Self::Match),
            "DIVERGE" => Ok(Self::Diverge),
            "FAILED" => Ok(Self::Failed),
            other => Err(RouterVocabularyError::unknown("ShadowDecisionClass", other)),
        }
    }
}

/// Provider failure class (EP-015 M5 failover plane; ADR-022; LF-021).
///
/// Typed classification of a provider attempt failure. Only
/// `Unavailable` and `Timeout` are failover-eligible; contract, rate,
/// policy, budget, and security failures never cause provider hopping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderFailureClass {
    Unavailable,
    Timeout,
    RateLimited,
    Contract,
    External,
    Rejected,
    BudgetExhausted,
    SecurityDenied,
}

impl ProviderFailureClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unavailable => "UNAVAILABLE",
            Self::Timeout => "TIMEOUT",
            Self::RateLimited => "RATE_LIMITED",
            Self::Contract => "CONTRACT",
            Self::External => "EXTERNAL",
            Self::Rejected => "REJECTED",
            Self::BudgetExhausted => "BUDGET_EXHAUSTED",
            Self::SecurityDenied => "SECURITY_DENIED",
        }
    }
}

impl std::str::FromStr for ProviderFailureClass {
    type Err = RouterVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "UNAVAILABLE" => Ok(Self::Unavailable),
            "TIMEOUT" => Ok(Self::Timeout),
            "RATE_LIMITED" => Ok(Self::RateLimited),
            "CONTRACT" => Ok(Self::Contract),
            "EXTERNAL" => Ok(Self::External),
            "REJECTED" => Ok(Self::Rejected),
            "BUDGET_EXHAUSTED" => Ok(Self::BudgetExhausted),
            "SECURITY_DENIED" => Ok(Self::SecurityDenied),
            other => Err(RouterVocabularyError::unknown(
                "ProviderFailureClass",
                other,
            )),
        }
    }
}

/// Failover audit stage (EP-015 M5; ADR-022; LF-021).
///
/// Ordered provider-attempt stages recorded on `RouteAuditRecord` for a
/// failover-routed request. The routing decision record itself is the
/// `route_requested`/`primary_selected` event (class + route tier); the
/// stages below track the provider attempts under that decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailoverStage {
    PrimarySelected,
    PrimaryAttempted,
    PrimaryFailed,
    FailoverEligible,
    SecondarySelected,
    SecondaryAttempted,
    SecondaryValidated,
    RouteCompleted,
    FailedClosed,
}

impl FailoverStage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PrimarySelected => "PRIMARY_SELECTED",
            Self::PrimaryAttempted => "PRIMARY_ATTEMPTED",
            Self::PrimaryFailed => "PRIMARY_FAILED",
            Self::FailoverEligible => "FAILOVER_ELIGIBLE",
            Self::SecondarySelected => "SECONDARY_SELECTED",
            Self::SecondaryAttempted => "SECONDARY_ATTEMPTED",
            Self::SecondaryValidated => "SECONDARY_VALIDATED",
            Self::RouteCompleted => "ROUTE_COMPLETED",
            Self::FailedClosed => "FAILED_CLOSED",
        }
    }
}

impl std::str::FromStr for FailoverStage {
    type Err = RouterVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PRIMARY_SELECTED" => Ok(Self::PrimarySelected),
            "PRIMARY_ATTEMPTED" => Ok(Self::PrimaryAttempted),
            "PRIMARY_FAILED" => Ok(Self::PrimaryFailed),
            "FAILOVER_ELIGIBLE" => Ok(Self::FailoverEligible),
            "SECONDARY_SELECTED" => Ok(Self::SecondarySelected),
            "SECONDARY_ATTEMPTED" => Ok(Self::SecondaryAttempted),
            "SECONDARY_VALIDATED" => Ok(Self::SecondaryValidated),
            "ROUTE_COMPLETED" => Ok(Self::RouteCompleted),
            "FAILED_CLOSED" => Ok(Self::FailedClosed),
            other => Err(RouterVocabularyError::unknown("FailoverStage", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep015_unit_routing_decision_class_round_trip() {
        for value in [
            RoutingDecisionClass::Routed,
            RoutingDecisionClass::Fallback,
            RoutingDecisionClass::Escalated,
            RoutingDecisionClass::Rejected,
            RoutingDecisionClass::Shadow,
        ] {
            assert_eq!(
                value.as_str().parse::<RoutingDecisionClass>().unwrap(),
                value
            );
            let v = serde_json::to_value(value).unwrap();
            let back: RoutingDecisionClass = serde_json::from_value(v).unwrap();
            assert_eq!(back, value);
        }
        assert!("UNKNOWN".parse::<RoutingDecisionClass>().is_err());
    }

    #[test]
    fn ep015_unit_router_strategy_class_round_trip() {
        for value in [
            RouterStrategyClass::Policy,
            RouterStrategyClass::RouteLlm,
            RouterStrategyClass::LlmRouter,
            RouterStrategyClass::Microbrain,
        ] {
            assert_eq!(
                value.as_str().parse::<RouterStrategyClass>().unwrap(),
                value
            );
        }
        assert!("AUTOPILOT".parse::<RouterStrategyClass>().is_err());
    }

    #[test]
    fn ep015_unit_escalation_reason_round_trip() {
        for value in [
            EscalationReason::Ambiguity,
            EscalationReason::Risk,
            EscalationReason::Privacy,
            EscalationReason::Budget,
            EscalationReason::Unavailable,
            EscalationReason::Cost,
            EscalationReason::Latency,
            EscalationReason::Security,
            EscalationReason::Certification,
            EscalationReason::OutOfDistribution,
        ] {
            assert_eq!(value.as_str().parse::<EscalationReason>().unwrap(), value);
        }
        assert!("NOT_A_REASON".parse::<EscalationReason>().is_err());
    }

    #[test]
    fn ep015_unit_microbrain_state_round_trip() {
        for value in [
            MicrobrainState::Disabled,
            MicrobrainState::Shadow,
            MicrobrainState::Canary,
            MicrobrainState::Active,
            MicrobrainState::PromotionGated,
        ] {
            assert_eq!(value.as_str().parse::<MicrobrainState>().unwrap(), value);
        }
        assert!("ON_FIRE".parse::<MicrobrainState>().is_err());
    }

    #[test]
    fn ep015_unit_shadow_decision_class_round_trip() {
        for value in [
            ShadowDecisionClass::Match,
            ShadowDecisionClass::Diverge,
            ShadowDecisionClass::Failed,
        ] {
            assert_eq!(
                value.as_str().parse::<ShadowDecisionClass>().unwrap(),
                value
            );
        }
        assert!("MAYBE".parse::<ShadowDecisionClass>().is_err());
    }

    #[test]
    fn ep015_unit_provider_failure_class_round_trip() {
        for value in [
            ProviderFailureClass::Unavailable,
            ProviderFailureClass::Timeout,
            ProviderFailureClass::RateLimited,
            ProviderFailureClass::Contract,
            ProviderFailureClass::External,
            ProviderFailureClass::Rejected,
            ProviderFailureClass::BudgetExhausted,
            ProviderFailureClass::SecurityDenied,
        ] {
            assert_eq!(
                value.as_str().parse::<ProviderFailureClass>().unwrap(),
                value
            );
            let v = serde_json::to_value(value).unwrap();
            let back: ProviderFailureClass = serde_json::from_value(v).unwrap();
            assert_eq!(back, value);
        }
        assert!("ON_FIRE".parse::<ProviderFailureClass>().is_err());
    }

    #[test]
    fn ep015_unit_failover_stage_round_trip() {
        for value in [
            FailoverStage::PrimarySelected,
            FailoverStage::PrimaryAttempted,
            FailoverStage::PrimaryFailed,
            FailoverStage::FailoverEligible,
            FailoverStage::SecondarySelected,
            FailoverStage::SecondaryAttempted,
            FailoverStage::SecondaryValidated,
            FailoverStage::RouteCompleted,
            FailoverStage::FailedClosed,
        ] {
            assert_eq!(value.as_str().parse::<FailoverStage>().unwrap(), value);
            let v = serde_json::to_value(value).unwrap();
            let back: FailoverStage = serde_json::from_value(v).unwrap();
            assert_eq!(back, value);
        }
        assert!("NOPE".parse::<FailoverStage>().is_err());
    }
}
