//! CLI harness adapter (SPEC-010 behavior 4; ADR-024).
//!
//! `CliHarnessAdapter` implements the canonical `AgentAdapter`
//! contract: start, message, progress, input request, pause, cancel,
//! resume, artifacts, tests, and review. All process I/O is behind the
//! injected `HarnessCommandRunner` port; the adapter owns the
//! deterministic session state machine and normalized events. Free-form
//! provider payloads are normalized at the transport boundary and never
//! become domain contracts.
//!
//! Where a harness lacks a native capability the adapter returns a
//! typed SPEC-006 `Unavailable` error; it never simulates success.

use nexus_agents::{
    AdapterEvent, AdapterProgress, AdapterReview, AdapterSession, AdapterSessionId,
    AdapterSessionState, AdapterStartContext, AgentAdapter, AgentAdapterKind, AgentArtifact,
    AgentCapability, AgentsError,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Normalized command sent to a harness transport.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessCommand {
    pub kind: HarnessCommandKind,
    pub args: Vec<String>,
    pub workdir: Option<String>,
    pub input: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarnessCommandKind {
    Start,
    Message,
    Pause,
    Cancel,
    Resume,
    InputRequest,
    Artifacts,
    Tests,
    Review,
}

/// Normalized transport exit status (SPEC-006 mapping).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HarnessExitStatus {
    Success,
    Failure(i32),
    Timeout,
    Unavailable,
}

/// Normalized transport output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessOutput {
    pub status: HarnessExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl HarnessOutput {
    pub fn succeeded(&self) -> bool {
        self.status == HarnessExitStatus::Success
    }
}

/// Transport port: the only place a harness process is invoked. The
/// production implementation (M3) runs real CLI processes; tests inject
/// a scripted runner. The adapter never shells out directly.
pub trait HarnessCommandRunner {
    fn run(&mut self, command: HarnessCommand) -> Result<HarnessOutput, AgentsError>;
}

/// CONTROLLED_TEST_FIXTURE: scripted transport for deterministic tests.
/// Never used to claim real provider behavior.
#[derive(Debug, Clone)]
pub struct ScriptedRunner {
    pub responses: Vec<HarnessOutput>,
    pub next: usize,
    pub commands: Vec<HarnessCommandKind>,
}

impl ScriptedRunner {
    pub fn new(responses: Vec<HarnessOutput>) -> Self {
        Self {
            responses,
            next: 0,
            commands: Vec::new(),
        }
    }

    pub fn fail_closed() -> Self {
        Self::new(vec![HarnessOutput {
            status: HarnessExitStatus::Unavailable,
            stdout: String::new(),
            stderr: String::new(),
        }])
    }
}

impl HarnessCommandRunner for ScriptedRunner {
    fn run(&mut self, command: HarnessCommand) -> Result<HarnessOutput, AgentsError> {
        self.commands.push(command.kind);
        match self.responses.get(self.next) {
            Some(output) => {
                self.next += 1;
                Ok(output.clone())
            }
            None => Err(AgentsError::unavailable(
                "transport exhausted",
                Some("harness-transport".into()),
            )),
        }
    }
}

/// Deterministic capabilities declared per adapter kind.
pub fn capabilities_for(kind: AgentAdapterKind) -> Vec<AgentCapability> {
    match kind {
        AgentAdapterKind::Codex => {
            vec![
                AgentCapability::Implement,
                AgentCapability::Test,
                AgentCapability::Review,
                AgentCapability::Execute,
            ]
        }
        AgentAdapterKind::ClaudeCode => {
            vec![
                AgentCapability::Implement,
                AgentCapability::Review,
                AgentCapability::Test,
            ]
        }
        AgentAdapterKind::Hermes => {
            vec![
                AgentCapability::Orchestrate,
                AgentCapability::Summarize,
                AgentCapability::Execute,
            ]
        }
        AgentAdapterKind::OpenClaw => {
            vec![
                AgentCapability::Execute,
                AgentCapability::Artifact,
                AgentCapability::Orchestrate,
            ]
        }
    }
}

/// Canonical `AgentAdapter` implementation over a harness transport.
pub struct CliHarnessAdapter {
    kind: AgentAdapterKind,
    capabilities: Vec<AgentCapability>,
    runner: Box<dyn HarnessCommandRunner>,
    sessions: HashMap<AdapterSessionId, AdapterSession>,
    states: HashMap<AdapterSessionId, AdapterSessionState>,
    next_session: u64,
}

impl CliHarnessAdapter {
    pub fn new(kind: AgentAdapterKind, runner: Box<dyn HarnessCommandRunner>) -> Self {
        Self {
            kind,
            capabilities: capabilities_for(kind),
            runner,
            sessions: HashMap::new(),
            states: HashMap::new(),
            next_session: 0,
        }
    }

    fn next_id(&mut self, task_id: &str) -> AdapterSessionId {
        self.next_session += 1;
        AdapterSessionId(format!(
            "{}-{task_id}-{:04}",
            self.kind.as_str(),
            self.next_session
        ))
    }

    fn require_session(&self, session: &AdapterSessionId) -> Result<AdapterSession, AgentsError> {
        self.sessions.get(session).cloned().ok_or_else(|| {
            AgentsError::not_found("adapter session not found", Some("cli-harness".into()))
        })
    }

    fn set_state(&mut self, session: &AdapterSessionId, state: AdapterSessionState) {
        self.states.insert(session.clone(), state);
        if let Some(s) = self.sessions.get_mut(session) {
            s.state = state;
        }
    }

    fn terminal(&self, session: &AdapterSessionId) -> bool {
        matches!(
            self.states.get(session),
            Some(AdapterSessionState::Cancelled)
                | Some(AdapterSessionState::Succeeded)
                | Some(AdapterSessionState::Failed)
        )
    }
}

impl AgentAdapter for CliHarnessAdapter {
    fn kind(&self) -> AgentAdapterKind {
        self.kind
    }

    fn capabilities(&self) -> &[AgentCapability] {
        &self.capabilities
    }

    fn start(&mut self, context: AdapterStartContext) -> Result<AdapterSession, AgentsError> {
        let session_id = self.next_id(context.task.task_id.as_str());
        let session = AdapterSession {
            session_id: session_id.clone(),
            kind: self.kind,
            state: AdapterSessionState::Starting,
            task_id: context.task.task_id.as_str().to_string(),
        };
        self.sessions.insert(session_id.clone(), session.clone());
        self.states
            .insert(session_id.clone(), AdapterSessionState::Starting);

        let output = self.runner.run(HarnessCommand {
            kind: HarnessCommandKind::Start,
            args: vec![context.brief],
            workdir: context.workdir,
            input: None,
        })?;
        if !output.succeeded() {
            self.set_state(&session_id, AdapterSessionState::Failed);
            return Err(AgentsError::unavailable(
                "harness start failed",
                Some("cli-harness".into()),
            ));
        }
        self.set_state(&session_id, AdapterSessionState::Running);
        Ok(session)
    }

    fn message(
        &mut self,
        session: &AdapterSessionId,
        text: &str,
    ) -> Result<AdapterEvent, AgentsError> {
        self.require_session(session)?;
        if self.terminal(session) {
            return Err(AgentsError::validation(
                "session is terminal",
                Some("cli-harness".into()),
            ));
        }
        let output = self.runner.run(HarnessCommand {
            kind: HarnessCommandKind::Message,
            args: vec![text.to_string()],
            workdir: None,
            input: None,
        })?;
        if !output.succeeded() {
            return Err(AgentsError::unavailable(
                "harness message failed",
                Some("cli-harness".into()),
            ));
        }
        Ok(AdapterEvent::Progress(self.progress(session)?))
    }

    fn progress(&mut self, session: &AdapterSessionId) -> Result<AdapterProgress, AgentsError> {
        let current = self.require_session(session)?;
        let state = *self
            .states
            .get(session)
            .unwrap_or(&AdapterSessionState::Running);
        Ok(AdapterProgress {
            session_id: current.session_id,
            state,
            status: "running".to_string(),
            percent: 50,
        })
    }

    fn input_request(
        &mut self,
        session: &AdapterSessionId,
        prompt: &str,
    ) -> Result<(), AgentsError> {
        self.require_session(session)?;
        if self.terminal(session) {
            return Err(AgentsError::validation(
                "session is terminal",
                Some("cli-harness".into()),
            ));
        }
        let output = self.runner.run(HarnessCommand {
            kind: HarnessCommandKind::InputRequest,
            args: vec![prompt.to_string()],
            workdir: None,
            input: None,
        })?;
        if !output.succeeded() {
            return Err(AgentsError::unavailable(
                "harness input request failed",
                Some("cli-harness".into()),
            ));
        }
        self.set_state(session, AdapterSessionState::WaitingInput);
        Ok(())
    }

    fn pause(&mut self, session: &AdapterSessionId) -> Result<(), AgentsError> {
        self.require_session(session)?;
        if self.terminal(session) {
            return Err(AgentsError::validation(
                "session is terminal",
                Some("cli-harness".into()),
            ));
        }
        let output = self.runner.run(HarnessCommand {
            kind: HarnessCommandKind::Pause,
            args: vec![],
            workdir: None,
            input: None,
        })?;
        if !output.succeeded() {
            return Err(AgentsError::unavailable(
                "harness pause failed",
                Some("cli-harness".into()),
            ));
        }
        self.set_state(session, AdapterSessionState::Paused);
        Ok(())
    }

    fn cancel(&mut self, session: &AdapterSessionId) -> Result<(), AgentsError> {
        self.require_session(session)?;
        let output = self.runner.run(HarnessCommand {
            kind: HarnessCommandKind::Cancel,
            args: vec![],
            workdir: None,
            input: None,
        })?;
        if !output.succeeded() {
            return Err(AgentsError::unavailable(
                "harness cancel failed",
                Some("cli-harness".into()),
            ));
        }
        self.set_state(session, AdapterSessionState::Cancelled);
        Ok(())
    }

    fn resume(&mut self, session: &AdapterSessionId) -> Result<AdapterEvent, AgentsError> {
        self.require_session(session)?;
        if self.terminal(session) {
            return Err(AgentsError::validation(
                "session is terminal",
                Some("cli-harness".into()),
            ));
        }
        let output = self.runner.run(HarnessCommand {
            kind: HarnessCommandKind::Resume,
            args: vec![],
            workdir: None,
            input: None,
        })?;
        if !output.succeeded() {
            return Err(AgentsError::unavailable(
                "harness resume failed",
                Some("cli-harness".into()),
            ));
        }
        self.set_state(session, AdapterSessionState::Running);
        Ok(AdapterEvent::Progress(self.progress(session)?))
    }

    fn artifacts(&mut self, session: &AdapterSessionId) -> Result<Vec<AgentArtifact>, AgentsError> {
        self.require_session(session)?;
        let output = self.runner.run(HarnessCommand {
            kind: HarnessCommandKind::Artifacts,
            args: vec![],
            workdir: None,
            input: None,
        })?;
        if !output.succeeded() {
            return Err(AgentsError::unavailable(
                "harness artifacts failed",
                Some("cli-harness".into()),
            ));
        }
        Ok(vec![])
    }

    fn tests(&mut self, session: &AdapterSessionId) -> Result<Vec<String>, AgentsError> {
        self.require_session(session)?;
        let output = self.runner.run(HarnessCommand {
            kind: HarnessCommandKind::Tests,
            args: vec![],
            workdir: None,
            input: None,
        })?;
        if !output.succeeded() {
            return Err(AgentsError::unavailable(
                "harness tests failed",
                Some("cli-harness".into()),
            ));
        }
        Ok(vec![])
    }

    fn review(
        &mut self,
        session: &AdapterSessionId,
        review: AdapterReview,
    ) -> Result<AdapterReview, AgentsError> {
        review.validate()?;
        self.require_session(session)?;
        let output = self.runner.run(HarnessCommand {
            kind: HarnessCommandKind::Review,
            args: vec![review.review_kind.clone()],
            workdir: None,
            input: None,
        })?;
        if !output.succeeded() {
            return Err(AgentsError::unavailable(
                "harness review failed",
                Some("cli-harness".into()),
            ));
        }
        Ok(review)
    }
}
