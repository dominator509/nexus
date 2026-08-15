//! EP-017 agent orchestrator vocabulary (SPEC-010; ADR-024).
//!
//! Vocabulary-locked enums owned by EP-017. Every enum rejects unknown
//! values at parse time; a new synonym requires an ADR and schema
//! update. Canonical terms from SPEC-003 (Agent Card, Artifact
//! Manifest) and SPEC-001 (ObjectiveId, TaskId) live in nexus-fabric
//! and nexus-domain and are re-exported, never redefined.

use crate::error::AgentsVocabularyError;
use serde::{Deserialize, Serialize};

/// Agent task lifecycle state (SPEC-010; ADR-024). Terminal outcomes
/// mirror SPEC-006 ActionLifecycle; durable engine states CANCELLED and
/// FAILED are explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentTaskState {
    Requested,
    Assigned,
    Running,
    Paused,
    WaitingInput,
    Reviewing,
    Cancelled,
    Succeeded,
    Failed,
}

impl AgentTaskState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "REQUESTED",
            Self::Assigned => "ASSIGNED",
            Self::Running => "RUNNING",
            Self::Paused => "PAUSED",
            Self::WaitingInput => "WAITING_INPUT",
            Self::Reviewing => "REVIEWING",
            Self::Cancelled => "CANCELLED",
            Self::Succeeded => "SUCCEEDED",
            Self::Failed => "FAILED",
        }
    }

    /// Terminal states can never transition again.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Cancelled | Self::Succeeded | Self::Failed)
    }
}

impl std::fmt::Display for AgentTaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AgentTaskState {
    type Err = AgentsVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "REQUESTED" => Ok(Self::Requested),
            "ASSIGNED" => Ok(Self::Assigned),
            "RUNNING" => Ok(Self::Running),
            "PAUSED" => Ok(Self::Paused),
            "WAITING_INPUT" => Ok(Self::WaitingInput),
            "REVIEWING" => Ok(Self::Reviewing),
            "CANCELLED" => Ok(Self::Cancelled),
            "SUCCEEDED" => Ok(Self::Succeeded),
            "FAILED" => Ok(Self::Failed),
            other => Err(AgentsVocabularyError::unknown("AgentTaskState", other)),
        }
    }
}

/// Harness adapter kind (SPEC-010; ADR-024). Concrete adapter
/// implementations live in the EP-017 M2 crate boundary; this kind is
/// the vocabulary-locked identity used by the registry and task
/// contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentAdapterKind {
    Codex,
    ClaudeCode,
    Hermes,
    OpenClaw,
}

impl AgentAdapterKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "CODEX",
            Self::ClaudeCode => "CLAUDE_CODE",
            Self::Hermes => "HERMES",
            Self::OpenClaw => "OPENCLAW",
        }
    }

    pub const ALL: [AgentAdapterKind; 4] = [
        AgentAdapterKind::Codex,
        AgentAdapterKind::ClaudeCode,
        AgentAdapterKind::Hermes,
        AgentAdapterKind::OpenClaw,
    ];
}

impl std::fmt::Display for AgentAdapterKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AgentAdapterKind {
    type Err = AgentsVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "CODEX" => Ok(Self::Codex),
            "CLAUDE_CODE" => Ok(Self::ClaudeCode),
            "HERMES" => Ok(Self::Hermes),
            "OPENCLAW" => Ok(Self::OpenClaw),
            other => Err(AgentsVocabularyError::unknown("AgentAdapterKind", other)),
        }
    }
}

/// Agent capability requested by a task (SPEC-010 behavior 2; ADR-024).
/// Agents request capabilities rather than named peers; Nexus selects
/// the adapter by quality, cost, trust, availability, and historical
/// success. Capabilities are vocabulary locked and reject unknown
/// values at parse time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentCapability {
    Orchestrate,
    Implement,
    Review,
    Test,
    Execute,
    Summarize,
    Artifact,
}

impl AgentCapability {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Orchestrate => "ORCHESTRATE",
            Self::Implement => "IMPLEMENT",
            Self::Review => "REVIEW",
            Self::Test => "TEST",
            Self::Execute => "EXECUTE",
            Self::Summarize => "SUMMARIZE",
            Self::Artifact => "ARTIFACT",
        }
    }

    pub const ALL: [AgentCapability; 7] = [
        AgentCapability::Orchestrate,
        AgentCapability::Implement,
        AgentCapability::Review,
        AgentCapability::Test,
        AgentCapability::Execute,
        AgentCapability::Summarize,
        AgentCapability::Artifact,
    ];
}

impl std::fmt::Display for AgentCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AgentCapability {
    type Err = AgentsVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ORCHESTRATE" => Ok(Self::Orchestrate),
            "IMPLEMENT" => Ok(Self::Implement),
            "REVIEW" => Ok(Self::Review),
            "TEST" => Ok(Self::Test),
            "EXECUTE" => Ok(Self::Execute),
            "SUMMARIZE" => Ok(Self::Summarize),
            "ARTIFACT" => Ok(Self::Artifact),
            other => Err(AgentsVocabularyError::unknown("AgentCapability", other)),
        }
    }
}

/// Delegation lifecycle (SPEC-010 canonical term `Delegation`;
/// ADR-024). Direct agent-to-agent authority is forbidden: every
/// delegation is recorded by Nexus and passes Nexus policy and
/// correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DelegationState {
    Proposed,
    Accepted,
    Active,
    Completed,
    Revoked,
    Failed,
}

impl DelegationState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "PROPOSED",
            Self::Accepted => "ACCEPTED",
            Self::Active => "ACTIVE",
            Self::Completed => "COMPLETED",
            Self::Revoked => "REVOKED",
            Self::Failed => "FAILED",
        }
    }
}

impl std::fmt::Display for DelegationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for DelegationState {
    type Err = AgentsVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PROPOSED" => Ok(Self::Proposed),
            "ACCEPTED" => Ok(Self::Accepted),
            "ACTIVE" => Ok(Self::Active),
            "COMPLETED" => Ok(Self::Completed),
            "REVOKED" => Ok(Self::Revoked),
            "FAILED" => Ok(Self::Failed),
            other => Err(AgentsVocabularyError::unknown("DelegationState", other)),
        }
    }
}

/// Agent budget class (SPEC-010; ADR-024). Budgets are fixed, declared
/// limits; Nexus is the canonical budget owner and enforces them
/// fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentBudgetClass {
    TotalTokens,
    TotalCost,
    MaxConcurrent,
    MaxDurationSecs,
}

impl AgentBudgetClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TotalTokens => "TOTAL_TOKENS",
            Self::TotalCost => "TOTAL_COST",
            Self::MaxConcurrent => "MAX_CONCURRENT",
            Self::MaxDurationSecs => "MAX_DURATION_SECS",
        }
    }

    pub const ALL: [AgentBudgetClass; 4] = [
        AgentBudgetClass::TotalTokens,
        AgentBudgetClass::TotalCost,
        AgentBudgetClass::MaxConcurrent,
        AgentBudgetClass::MaxDurationSecs,
    ];
}

impl std::fmt::Display for AgentBudgetClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AgentBudgetClass {
    type Err = AgentsVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "TOTAL_TOKENS" => Ok(Self::TotalTokens),
            "TOTAL_COST" => Ok(Self::TotalCost),
            "MAX_CONCURRENT" => Ok(Self::MaxConcurrent),
            "MAX_DURATION_SECS" => Ok(Self::MaxDurationSecs),
            other => Err(AgentsVocabularyError::unknown("AgentBudgetClass", other)),
        }
    }
}
