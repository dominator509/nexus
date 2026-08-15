//! Agent adapter canonical task contract (SPEC-010 behavior 4;
//! ADR-024).
//!
//! Every harness adapter (Codex, Claude Code, Hermes, OpenClaw)
//! implements `AgentAdapter`: start, message, progress, input request,
//! pause, cancel, resume, artifacts, tests, and review semantics where
//! the harness permits. The trait is provider-neutral; free-form
//! provider payloads are normalized at the infrastructure boundary and
//! never become domain contracts. This file owns no provider behavior
//! (M1 contract boundary); concrete harness adapters live in the
//! EP-017 M2 crate boundary.

use crate::artifact::AgentArtifact;
use crate::error::AgentsError;
use crate::task::AgentTask;
use crate::vocabulary::AgentAdapterKind;
use serde::{Deserialize, Serialize};

/// Opaque adapter session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AdapterSessionId(pub String);

/// Adapter session lifecycle. Mirrors SPEC-006 ActionLifecycle;
/// terminal outcomes are explicit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdapterSessionState {
    Starting,
    Running,
    WaitingInput,
    Paused,
    Cancelled,
    Succeeded,
    Failed,
}

impl AdapterSessionState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "STARTING",
            Self::Running => "RUNNING",
            Self::WaitingInput => "WAITING_INPUT",
            Self::Paused => "PAUSED",
            Self::Cancelled => "CANCELLED",
            Self::Succeeded => "SUCCEEDED",
            Self::Failed => "FAILED",
        }
    }
}

impl std::fmt::Display for AdapterSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A running adapter session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterSession {
    pub session_id: AdapterSessionId,
    pub kind: AgentAdapterKind,
    pub state: AdapterSessionState,
    pub task_id: String,
}

/// Start context: the canonical task contract's input boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterStartContext {
    pub task: AgentTask,
    /// Redacted objective/task brief; never raw secrets.
    pub brief: String,
    /// Working directory or repository reference when the harness
    /// supports isolated worktrees.
    pub workdir: Option<String>,
    pub extra: serde_json::Value,
}

/// A normalized adapter event (progress, input request, review
/// request, terminal outcome).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdapterEvent {
    Progress(AdapterProgress),
    InputRequest(String),
    ReviewRequest(AdapterReview),
    Artifact(AgentArtifact),
    Succeeded,
    Failed(String),
}

/// Normalized progress tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterProgress {
    pub session_id: AdapterSessionId,
    pub state: AdapterSessionState,
    /// Redacted one-line status; never raw content.
    pub status: String,
    pub percent: u8,
}

/// Normalized review request (Codex-implement / Claude-review loop).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterReview {
    pub session_id: AdapterSessionId,
    pub review_kind: String,
    pub target_artifact_ids: Vec<String>,
    pub verdict: Option<String>,
}

impl AdapterReview {
    pub fn validate(&self) -> Result<(), AgentsError> {
        if self.review_kind.is_empty() {
            return Err(AgentsError::validation(
                "review_kind must not be empty",
                Some("adapter-review".into()),
            ));
        }
        Ok(())
    }
}

/// The canonical task contract every harness adapter implements.
///
/// Provider-neutral by design: adapters take and return normalized
/// values, never provider payloads. Where a harness lacks a native
/// capability (for example no pause support), the adapter returns a
/// typed `Unavailable` error; it never simulates success.
pub trait AgentAdapter {
    /// The vocabulary-locked adapter kind.
    fn kind(&self) -> AgentAdapterKind;

    /// Declared capabilities this adapter can serve.
    fn capabilities(&self) -> &[crate::vocabulary::AgentCapability];

    /// Start a task session. The adapter owns the session lifecycle.
    fn start(&mut self, context: AdapterStartContext) -> Result<AdapterSession, AgentsError>;

    /// Send a message to a running session.
    fn message(
        &mut self,
        session: &AdapterSessionId,
        text: &str,
    ) -> Result<AdapterEvent, AgentsError>;

    /// Poll current progress.
    fn progress(&mut self, session: &AdapterSessionId) -> Result<AdapterProgress, AgentsError>;

    /// Ask the harness for human/adapter input (input request).
    fn input_request(
        &mut self,
        session: &AdapterSessionId,
        prompt: &str,
    ) -> Result<(), AgentsError>;

    /// Pause a session when the harness supports it.
    fn pause(&mut self, session: &AdapterSessionId) -> Result<(), AgentsError>;

    /// Cancel a session.
    fn cancel(&mut self, session: &AdapterSessionId) -> Result<(), AgentsError>;

    /// Resume a paused session.
    fn resume(&mut self, session: &AdapterSessionId) -> Result<AdapterEvent, AgentsError>;

    /// Return artifacts produced by the session.
    fn artifacts(&mut self, session: &AdapterSessionId) -> Result<Vec<AgentArtifact>, AgentsError>;

    /// Return test results produced by the session.
    fn tests(&mut self, session: &AdapterSessionId) -> Result<Vec<String>, AgentsError>;

    /// Request or submit a review (Codex-implement / Claude-review).
    fn review(
        &mut self,
        session: &AdapterSessionId,
        review: AdapterReview,
    ) -> Result<AdapterReview, AgentsError>;
}
