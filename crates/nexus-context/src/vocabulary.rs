//! EP-016 context vocabulary (SPEC-002; ADR-023).
//!
//! New vocabulary-locked names owned by the context engine plane. The
//! canonical memory vocabulary (`MemoryRecord`, `MemoryProposal`,
//! `MemoryQuery`, `MemoryCandidate`, `MemoryType`, `Sensitivity`,
//! `RetentionPolicy`, `EmbeddingRef`) comes from `nexus-data` and
//! `nexus-domain`; `ContextCapsule` and the capsule service contract
//! come from `nexus-fabric`. Every enum here rejects unknown classes at
//! parse time.

use serde::{Deserialize, Serialize};

/// Context vocabulary parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextVocabularyError(pub String);

impl ContextVocabularyError {
    pub fn unknown(class: &str, value: &str) -> Self {
        Self(format!("unknown {class}: {value}"))
    }
}

impl std::fmt::Display for ContextVocabularyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ContextVocabularyError {}

/// Purpose limitation class (SPEC-020; ADR-023). Context construction
/// is purpose-limited: a capsule may only carry data whose declared
/// purpose permits the current use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ContextPurpose {
    TaskExecution,
    Planning,
    Search,
    Notification,
    SystemMaintenance,
}

impl ContextPurpose {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TaskExecution => "TASK_EXECUTION",
            Self::Planning => "PLANNING",
            Self::Search => "SEARCH",
            Self::Notification => "NOTIFICATION",
            Self::SystemMaintenance => "SYSTEM_MAINTENANCE",
        }
    }
}

impl std::fmt::Display for ContextPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ContextPurpose {
    type Err = ContextVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "TASK_EXECUTION" => Ok(Self::TaskExecution),
            "PLANNING" => Ok(Self::Planning),
            "SEARCH" => Ok(Self::Search),
            "NOTIFICATION" => Ok(Self::Notification),
            "SYSTEM_MAINTENANCE" => Ok(Self::SystemMaintenance),
            other => Err(ContextVocabularyError::unknown("ContextPurpose", other)),
        }
    }
}

/// Bounded graph expansion mode (SPEC-002 behavior 7; ADR-023). The
/// graph-aware context construction never expands past the declared
/// hop bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GraphExpansionMode {
    Direct,
    OneHop,
    TwoHop,
}

impl GraphExpansionMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "DIRECT",
            Self::OneHop => "ONE_HOP",
            Self::TwoHop => "TWO_HOP",
        }
    }
}

impl std::fmt::Display for GraphExpansionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for GraphExpansionMode {
    type Err = ContextVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "DIRECT" => Ok(Self::Direct),
            "ONE_HOP" => Ok(Self::OneHop),
            "TWO_HOP" => Ok(Self::TwoHop),
            other => Err(ContextVocabularyError::unknown("GraphExpansionMode", other)),
        }
    }
}

/// Privacy filter decision (SPEC-020, INV-007; ADR-023). A candidate is
/// either allowed, redacted (metadata only), or denied outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PrivacyFilterDecision {
    Allow,
    Redact,
    Deny,
}

impl PrivacyFilterDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "ALLOW",
            Self::Redact => "REDACT",
            Self::Deny => "DENY",
        }
    }
}

impl std::fmt::Display for PrivacyFilterDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PrivacyFilterDecision {
    type Err = ContextVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ALLOW" => Ok(Self::Allow),
            "REDACT" => Ok(Self::Redact),
            "DENY" => Ok(Self::Deny),
            other => Err(ContextVocabularyError::unknown(
                "PrivacyFilterDecision",
                other,
            )),
        }
    }
}

/// Consolidation execution mode (ADR-023). Model-assisted consolidation
/// is the preferred mode; the deterministic fallback satisfies the same
/// proposal contract when model evaluation is unavailable (node
/// contract fallback). Models can never write canonical memory directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConsolidationMode {
    ModelAssisted,
    DeterministicFallback,
    Skipped,
}

impl ConsolidationMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModelAssisted => "MODEL_ASSISTED",
            Self::DeterministicFallback => "DETERMINISTIC_FALLBACK",
            Self::Skipped => "SKIPPED",
        }
    }
}

impl std::fmt::Display for ConsolidationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ConsolidationMode {
    type Err = ContextVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "MODEL_ASSISTED" => Ok(Self::ModelAssisted),
            "DETERMINISTIC_FALLBACK" => Ok(Self::DeterministicFallback),
            "SKIPPED" => Ok(Self::Skipped),
            other => Err(ContextVocabularyError::unknown("ConsolidationMode", other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep016_unit_context_purpose_round_trip() {
        for value in [
            ContextPurpose::TaskExecution,
            ContextPurpose::Planning,
            ContextPurpose::Search,
            ContextPurpose::Notification,
            ContextPurpose::SystemMaintenance,
        ] {
            assert_eq!(value.as_str().parse::<ContextPurpose>().unwrap(), value);
            let v = serde_json::to_value(value).unwrap();
            let back: ContextPurpose = serde_json::from_value(v).unwrap();
            assert_eq!(back, value);
        }
        assert!("UNKNOWN".parse::<ContextPurpose>().is_err());
    }

    #[test]
    fn ep016_unit_graph_expansion_mode_round_trip() {
        for value in [
            GraphExpansionMode::Direct,
            GraphExpansionMode::OneHop,
            GraphExpansionMode::TwoHop,
        ] {
            assert_eq!(value.as_str().parse::<GraphExpansionMode>().unwrap(), value);
            let v = serde_json::to_value(value).unwrap();
            let back: GraphExpansionMode = serde_json::from_value(v).unwrap();
            assert_eq!(back, value);
        }
        assert!("EIGHT_HOPS".parse::<GraphExpansionMode>().is_err());
    }

    #[test]
    fn ep016_unit_privacy_filter_decision_round_trip() {
        for value in [
            PrivacyFilterDecision::Allow,
            PrivacyFilterDecision::Redact,
            PrivacyFilterDecision::Deny,
        ] {
            assert_eq!(
                value.as_str().parse::<PrivacyFilterDecision>().unwrap(),
                value
            );
            let v = serde_json::to_value(value).unwrap();
            let back: PrivacyFilterDecision = serde_json::from_value(v).unwrap();
            assert_eq!(back, value);
        }
        assert!("MAYBE".parse::<PrivacyFilterDecision>().is_err());
    }

    #[test]
    fn ep016_unit_consolidation_mode_round_trip() {
        for value in [
            ConsolidationMode::ModelAssisted,
            ConsolidationMode::DeterministicFallback,
            ConsolidationMode::Skipped,
        ] {
            assert_eq!(value.as_str().parse::<ConsolidationMode>().unwrap(), value);
            let v = serde_json::to_value(value).unwrap();
            let back: ConsolidationMode = serde_json::from_value(v).unwrap();
            assert_eq!(back, value);
        }
        assert!("MAGIC".parse::<ConsolidationMode>().is_err());
    }
}
