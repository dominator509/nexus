//! EP-013 model gateway vocabulary (SPEC-009; ADR-018).
//!
//! Vocabulary-locked canonical classes for the model plane. Every enum
//! parses from its canonical SCREAMING_SNAKE_CASE wire string and
//! rejects unknown values (fail closed at the boundary).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Vocabulary error for the model gateway classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelGatewayVocabularyError(pub String);

impl fmt::Display for ModelGatewayVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ModelGatewayVocabularyError {}

impl ModelGatewayVocabularyError {
    pub fn unknown(class: &str, value: &str) -> Self {
        Self(format!("unknown {class} value: {value}"))
    }
}

macro_rules! vocabulary_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $wire:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = ModelGatewayVocabularyError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($wire => Ok(Self::$variant),)+
                    other => Err(ModelGatewayVocabularyError::unknown(stringify!($name), other)),
                }
            }
        }
    };
}

vocabulary_enum! {
    /// Effort tier (SPEC-009 required behavior 2): deterministic,
    /// non-thinking, high, max, specialist. Max is never the default
    /// for trivial work.
    EffortTier {
        Deterministic = "DETERMINISTIC",
        NonThinking = "NON_THINKING",
        High = "HIGH",
        Max = "MAX",
        Specialist = "SPECIALIST",
    }
}

vocabulary_enum! {
    /// Provider kind (SPEC-009; EP-013 node contract): Bifrost is the
    /// preferred gateway implementation; direct providers remain
    /// available for replacement and diagnostics.
    ProviderKind {
        Bifrost = "BIFROST",
        Deepseek = "DEEPSEEK",
        OpenaiCompatible = "OPENAI_COMPATIBLE",
        Venice = "VENICE",
        Xai = "XAI",
    }
}

vocabulary_enum! {
    /// Provider health state (SPEC-009 canonical term ProviderHealth).
    ProviderHealthState {
        Healthy = "HEALTHY",
        Degraded = "DEGRADED",
        Unhealthy = "UNHEALTHY",
        Unknown = "UNKNOWN",
    }
}

vocabulary_enum! {
    /// Escalation class (SPEC-009 canonical term Escalation):
    /// deterministic escalation on provider failure or policy denial.
    Escalation {
        None = "NONE",
        Retry = "RETRY",
        Failover = "FAILOVER",
        Human = "HUMAN",
        Disable = "DISABLE",
    }
}

vocabulary_enum! {
    /// Microbrain lifecycle (SPEC-009 canonical term Microbrain):
    /// begins in shadow, passes frozen and adversarial evals, then
    /// canaries low-risk traffic with DeepSeek fallback.
    Microbrain {
        Shadow = "SHADOW",
        Frozen = "FROZEN",
        Canary = "CANARY",
        Active = "ACTIVE",
    }
}

/// Model route class (SPEC-009 canonical term ModelRoute).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelRouteClass {
    Direct,
    Cached,
    Fallback,
    Escalated,
}

impl ModelRouteClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "DIRECT",
            Self::Cached => "CACHED",
            Self::Fallback => "FALLBACK",
            Self::Escalated => "ESCALATED",
        }
    }
}

impl std::str::FromStr for ModelRouteClass {
    type Err = ModelGatewayVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "DIRECT" => Ok(Self::Direct),
            "CACHED" => Ok(Self::Cached),
            "FALLBACK" => Ok(Self::Fallback),
            "ESCALATED" => Ok(Self::Escalated),
            other => Err(ModelGatewayVocabularyError::unknown(
                "ModelRouteClass",
                other,
            )),
        }
    }
}

/// Model gateway class (SPEC-009 canonical term ModelGateway).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelGatewayClass {
    Reflex,
    Bifrost,
    Direct,
}

impl ModelGatewayClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Reflex => "REFLEX",
            Self::Bifrost => "BIFROST",
            Self::Direct => "DIRECT",
        }
    }
}

impl std::str::FromStr for ModelGatewayClass {
    type Err = ModelGatewayVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "REFLEX" => Ok(Self::Reflex),
            "BIFROST" => Ok(Self::Bifrost),
            "DIRECT" => Ok(Self::Direct),
            other => Err(ModelGatewayVocabularyError::unknown(
                "ModelGatewayClass",
                other,
            )),
        }
    }
}

/// ReflexProvider class (SPEC-009 canonical term ReflexProvider).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReflexProviderClass {
    DeepseekV4Flash,
    Bifrost,
    Custom,
}

impl ReflexProviderClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DeepseekV4Flash => "DEEPSEEK_V4_FLASH",
            Self::Bifrost => "BIFROST",
            Self::Custom => "CUSTOM",
        }
    }
}

impl std::str::FromStr for ReflexProviderClass {
    type Err = ModelGatewayVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "DEEPSEEK_V4_FLASH" => Ok(Self::DeepseekV4Flash),
            "BIFROST" => Ok(Self::Bifrost),
            "CUSTOM" => Ok(Self::Custom),
            other => Err(ModelGatewayVocabularyError::unknown(
                "ReflexProviderClass",
                other,
            )),
        }
    }
}

/// Cache hit ratio (SPEC-009 canonical term CacheHitRatio): hit prompt
/// tokens divided by total prompt tokens; cacheable reflex traffic
/// targets at least 0.97.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct CacheHitRatio {
    hit_tokens: u64,
    total_tokens: u64,
}

impl CacheHitRatio {
    pub fn new(hit_tokens: u64, total_tokens: u64) -> Self {
        Self {
            hit_tokens,
            total_tokens,
        }
    }

    pub fn hit_tokens(&self) -> u64 {
        self.hit_tokens
    }

    pub fn total_tokens(&self) -> u64 {
        self.total_tokens
    }

    /// Ratio in [0.0, 1.0]; 0.0 when there are no tokens.
    pub fn ratio(&self) -> f64 {
        if self.total_tokens == 0 {
            0.0
        } else {
            self.hit_tokens as f64 / self.total_tokens as f64
        }
    }

    /// True when the cacheable traffic target (>= 0.97) is met.
    pub fn meets_cache_target(&self) -> bool {
        self.ratio() >= 0.97
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep013_unit_effort_tier_round_trip() {
        for (wire, expected) in [
            ("DETERMINISTIC", EffortTier::Deterministic),
            ("NON_THINKING", EffortTier::NonThinking),
            ("HIGH", EffortTier::High),
            ("MAX", EffortTier::Max),
            ("SPECIALIST", EffortTier::Specialist),
        ] {
            assert_eq!(wire.parse::<EffortTier>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("ULTRA".parse::<EffortTier>().is_err());
        assert!("".parse::<EffortTier>().is_err());
    }

    #[test]
    fn ep013_unit_effort_tier_ordering() {
        // Deterministic < NonThinking < High < Max < Specialist.
        assert!(EffortTier::Deterministic < EffortTier::NonThinking);
        assert!(EffortTier::NonThinking < EffortTier::High);
        assert!(EffortTier::High < EffortTier::Max);
        assert!(EffortTier::Max < EffortTier::Specialist);
    }

    #[test]
    fn ep013_unit_provider_kind_round_trip() {
        assert_eq!(
            "BIFROST".parse::<ProviderKind>().unwrap(),
            ProviderKind::Bifrost
        );
        assert_eq!(
            "DEEPSEEK".parse::<ProviderKind>().unwrap(),
            ProviderKind::Deepseek
        );
        assert_eq!(
            "OPENAI_COMPATIBLE".parse::<ProviderKind>().unwrap(),
            ProviderKind::OpenaiCompatible
        );
        assert!("GEMINI".parse::<ProviderKind>().is_err());
    }

    #[test]
    fn ep013_unit_provider_health_state_round_trip() {
        assert_eq!(
            "HEALTHY".parse::<ProviderHealthState>().unwrap(),
            ProviderHealthState::Healthy
        );
        assert_eq!(
            "DEGRADED".parse::<ProviderHealthState>().unwrap(),
            ProviderHealthState::Degraded
        );
        assert_eq!(
            "UNHEALTHY".parse::<ProviderHealthState>().unwrap(),
            ProviderHealthState::Unhealthy
        );
        assert_eq!(
            "UNKNOWN".parse::<ProviderHealthState>().unwrap(),
            ProviderHealthState::Unknown
        );
        assert!("DOWN".parse::<ProviderHealthState>().is_err());
    }

    #[test]
    fn ep013_unit_escalation_round_trip() {
        for (wire, expected) in [
            ("NONE", Escalation::None),
            ("RETRY", Escalation::Retry),
            ("FAILOVER", Escalation::Failover),
            ("HUMAN", Escalation::Human),
            ("DISABLE", Escalation::Disable),
        ] {
            assert_eq!(wire.parse::<Escalation>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("UNKNOWN".parse::<Escalation>().is_err());
    }

    #[test]
    fn ep013_unit_microbrain_round_trip() {
        for (wire, expected) in [
            ("SHADOW", Microbrain::Shadow),
            ("FROZEN", Microbrain::Frozen),
            ("CANARY", Microbrain::Canary),
            ("ACTIVE", Microbrain::Active),
        ] {
            assert_eq!(wire.parse::<Microbrain>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("PROD".parse::<Microbrain>().is_err());
    }

    #[test]
    fn ep013_unit_route_class_round_trip() {
        assert_eq!(
            "DIRECT".parse::<ModelRouteClass>().unwrap(),
            ModelRouteClass::Direct
        );
        assert_eq!(
            "CACHED".parse::<ModelRouteClass>().unwrap(),
            ModelRouteClass::Cached
        );
        assert_eq!(
            "FALLBACK".parse::<ModelRouteClass>().unwrap(),
            ModelRouteClass::Fallback
        );
        assert_eq!(
            "ESCALATED".parse::<ModelRouteClass>().unwrap(),
            ModelRouteClass::Escalated
        );
        assert!("ANY".parse::<ModelRouteClass>().is_err());
    }

    #[test]
    fn ep013_unit_gateway_class_round_trip() {
        assert_eq!(
            "REFLEX".parse::<ModelGatewayClass>().unwrap(),
            ModelGatewayClass::Reflex
        );
        assert_eq!(
            "BIFROST".parse::<ModelGatewayClass>().unwrap(),
            ModelGatewayClass::Bifrost
        );
        assert_eq!(
            "DIRECT".parse::<ModelGatewayClass>().unwrap(),
            ModelGatewayClass::Direct
        );
        assert!("GATEWAY".parse::<ModelGatewayClass>().is_err());
    }

    #[test]
    fn ep013_unit_reflex_provider_class_round_trip() {
        assert_eq!(
            "DEEPSEEK_V4_FLASH".parse::<ReflexProviderClass>().unwrap(),
            ReflexProviderClass::DeepseekV4Flash
        );
        assert_eq!(
            "BIFROST".parse::<ReflexProviderClass>().unwrap(),
            ReflexProviderClass::Bifrost
        );
        assert_eq!(
            "CUSTOM".parse::<ReflexProviderClass>().unwrap(),
            ReflexProviderClass::Custom
        );
        assert!("CLAUDE".parse::<ReflexProviderClass>().is_err());
    }

    #[test]
    fn ep013_unit_cache_hit_ratio() {
        let r = CacheHitRatio::new(97, 100);
        assert!((r.ratio() - 0.97).abs() < 1e-9);
        assert!(r.meets_cache_target());
        let low = CacheHitRatio::new(50, 100);
        assert!(!low.meets_cache_target());
        let empty = CacheHitRatio::new(0, 0);
        assert_eq!(empty.ratio(), 0.0);
        assert!(!empty.meets_cache_target());
    }
}
