//! EP-019 healing contracts (SPEC-018; ADR-026).
//!
//! Public, provider-neutral contracts for the self-healing engineering
//! loop. All interfaces use typed IDs, authenticated tenant and
//! principal context, canonical SPEC-006 errors, correlation, and
//! idempotency for retryable commands. A provider implementation may add
//! internal types but cannot alter the canonical contract.
//!
//! A model/agent may generate a diagnosis hypothesis, propose a patch,
//! suggest tests, and explain likely root cause. It may NOT make
//! authoritative claims that the defect is fixed, the patch is safe,
//! production is healthy, or rollback is unnecessary. Those claims come
//! only from real evidence produced by the validation/verification
//! boundaries.

use crate::error::{HealingError, HealingErrorCode};
use crate::vocabulary::{DiagnosisConfidence, IncidentState};
use nexus_domain::{
    ApprovalClass, CorrelationId, DiagnosisId, IncidentId, PatchId, Risk, TenantId,
};
use serde::{Deserialize, Serialize};

/// A real structured signal that may become an incident (SPEC-018
/// behavior 1). Controlled fixtures are acceptable for deterministic
/// failure tests; production behavior never generates fake incidents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncidentSignal {
    /// Canonical correlation id (never the raw error string).
    pub correlation_id: CorrelationId,
    /// Tenant boundary the signal belongs to.
    pub tenant_id: TenantId,
    /// Error class (canonical vocabulary, not free-form text).
    pub error_class: String,
    /// Affected service/component identifier.
    pub component: String,
    /// Deployment/version of the affected component where known.
    pub version: Option<String>,
    /// Workflow/task identifier where the signal arose.
    pub workflow_id: Option<String>,
    /// Signal kind (process failure, health failure, test failure,
    /// workflow failure, connector failure, security event, resource
    /// exhaustion, deployment regression).
    pub kind: IncidentSignalKind,
    /// First observed epoch millis.
    pub first_seen_epoch_ms: u64,
    /// Last observed epoch millis.
    pub last_seen_epoch_ms: u64,
}

/// Canonical incident signal kind (SPEC-018; ADR-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IncidentSignalKind {
    ProcessFailure,
    HealthFailure,
    TestFailure,
    WorkflowFailure,
    ConnectorFailure,
    SecurityEvent,
    ResourceExhaustion,
    DeploymentRegression,
}

impl IncidentSignalKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ProcessFailure => "PROCESS_FAILURE",
            Self::HealthFailure => "HEALTH_FAILURE",
            Self::TestFailure => "TEST_FAILURE",
            Self::WorkflowFailure => "WORKFLOW_FAILURE",
            Self::ConnectorFailure => "CONNECTOR_FAILURE",
            Self::SecurityEvent => "SECURITY_EVENT",
            Self::ResourceExhaustion => "RESOURCE_EXHAUSTION",
            Self::DeploymentRegression => "DEPLOYMENT_REGRESSION",
        }
    }
}

impl std::fmt::Display for IncidentSignalKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for IncidentSignalKind {
    type Err = HealingError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "PROCESS_FAILURE" => Ok(Self::ProcessFailure),
            "HEALTH_FAILURE" => Ok(Self::HealthFailure),
            "TEST_FAILURE" => Ok(Self::TestFailure),
            "WORKFLOW_FAILURE" => Ok(Self::WorkflowFailure),
            "CONNECTOR_FAILURE" => Ok(Self::ConnectorFailure),
            "SECURITY_EVENT" => Ok(Self::SecurityEvent),
            "RESOURCE_EXHAUSTION" => Ok(Self::ResourceExhaustion),
            "DEPLOYMENT_REGRESSION" => Ok(Self::DeploymentRegression),
            other => Err(HealingError::vocabulary("IncidentSignalKind", other)),
        }
    }
}

/// A correlated incident (SPEC-018 canonical term `Incident`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incident {
    pub incident_id: IncidentId,
    pub correlation_id: CorrelationId,
    pub tenant_id: TenantId,
    /// Canonical lifecycle state; never collapsed.
    pub state: IncidentState,
    /// Risk class of the affected behavior (R0..R4).
    pub risk: Risk,
    /// Affected components, correlated by canonical identifier.
    pub components: Vec<String>,
    /// First observed epoch millis.
    pub first_seen_epoch_ms: u64,
    /// Last observed epoch millis.
    pub last_seen_epoch_ms: u64,
    /// Deterministic deduplication key (canonical signature).
    pub dedup_key: String,
}

/// A diagnosis task (SPEC-018 canonical term `Diagnosis`). A diagnosis
/// ALWAYS starts as HYPOTHESIS; only reproducible evidence raises
/// confidence to VALIDATED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosisTask {
    pub diagnosis_id: DiagnosisId,
    pub incident_id: IncidentId,
    pub tenant_id: TenantId,
    pub correlation_id: CorrelationId,
    /// Model/agent-generated root-cause hypothesis.
    pub hypothesis: String,
    /// Confidence: HYPOTHESIS until reproducible evidence supports it.
    pub confidence: DiagnosisConfidence,
    /// Evidence references (logs, traces, metrics, reproduction) that
    /// support the hypothesis. Redacted metadata only.
    pub evidence_refs: Vec<String>,
    /// Bounded diagnosis attempts so far (retry bound).
    pub attempts: u32,
}

/// A patch candidate (SPEC-018 canonical term `PatchCandidate`). A
/// proposal/artifact, never automatically applied production state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProposal {
    pub patch_id: PatchId,
    pub incident_id: IncidentId,
    pub tenant_id: TenantId,
    pub correlation_id: CorrelationId,
    /// Exact files changed (scope; unexpected expansion fails validation).
    pub files_changed: Vec<String>,
    /// The diff (patch artifact).
    pub diff: String,
    /// Rationale for the patch.
    pub rationale: String,
    /// Tests added/changed by the patch.
    pub tests_changed: Vec<String>,
    /// Risk estimate (R0..R4).
    pub risk: Risk,
    /// Dependency changes introduced by the patch (empty if none).
    pub dependency_changes: Vec<String>,
    /// Migration impact (empty if none).
    pub migration_impact: String,
    /// Rollback plan reference (RollbackPlan digest/id).
    pub rollback_plan_ref: String,
    /// Canonical patch digest binding approvals and validation to the
    /// exact artifact.
    pub patch_digest: String,
}

/// Independent reviewer verdict (SPEC-018 behavior 4). An independent
/// reviewer examines root cause, diff, tests, security, compatibility,
/// and rollback; a model/agent cannot be its own reviewer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewVerdict {
    pub reviewer: String,
    pub decision: ReviewDecision,
    pub comments: String,
    /// Binds to the exact patch digest; cannot authorize a different patch.
    pub patch_digest: String,
}

/// Review decision (SPEC-018; ADR-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewDecision {
    Approve,
    Reject,
    RequestChanges,
}

impl ReviewDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "APPROVE",
            Self::Reject => "REJECT",
            Self::RequestChanges => "REQUEST_CHANGES",
        }
    }
}

/// Sandbox validation result (SPEC-018 behavior 3; directive section 9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxVerdict {
    pub pass: bool,
    /// Patch applies cleanly, build succeeds, targeted reproduction
    /// FAIL->PASS, affected tests pass, regression tests pass, no
    /// forbidden placeholders/stubs, scope remains allowed.
    pub checks: Vec<String>,
    pub evidence_ref: String,
}

/// Security validation result (SPEC-018 behavior 5; directive section 10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityVerdict {
    pub pass: bool,
    /// Security checks, dependency audit, license gate, reality gate,
    /// static analysis, secret scanning, authorization invariants.
    pub checks: Vec<String>,
    pub evidence_ref: String,
}

/// HITL/policy approval bound to an exact patch digest (SPEC-018
/// behavior 5; directive section 11). Approval of patch A can never
/// authorize patch B.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationApproval {
    pub approval_id: nexus_domain::ApprovalId,
    pub incident_id: IncidentId,
    pub tenant_id: TenantId,
    pub correlation_id: CorrelationId,
    /// The exact patch digest this approval authorizes.
    pub patch_digest: String,
    /// Approval class required by policy (HUMAN/STRONG_HUMAN/FOUR_EYES).
    pub approval_class: ApprovalClass,
    /// Principal that granted the approval (distinct human where class
    /// requires it).
    pub approver: String,
    pub granted_at_epoch_ms: u64,
}

/// The incident engine port: the canonical self-healing lifecycle.
///
/// Implementations are provider-neutral and versioned. Every transition
/// is explicit; states are never collapsed; a model/agent can never
/// declare its own fix successful.
pub trait IncidentEngine {
    /// Observe a real structured signal; returns the correlated incident
    /// (deduplicated by canonical signature).
    fn observe(&self, signal: IncidentSignal) -> Result<Incident, HealingError>;

    /// Raise an incident to the given state if the transition is
    /// canonical and allowed.
    fn transition(&self, incident: &mut Incident, to: IncidentState) -> Result<(), HealingError>;

    /// Create a diagnosis task (HYPOTHESIS confidence).
    fn create_diagnosis(
        &self,
        incident: &Incident,
        hypothesis: String,
    ) -> Result<DiagnosisTask, HealingError>;

    /// Record that a diagnosis is now supported/reproduced/validated by
    /// evidence. Only real evidence can raise confidence to VALIDATED.
    fn update_diagnosis_confidence(
        &self,
        diagnosis: &mut DiagnosisTask,
        confidence: DiagnosisConfidence,
        evidence_ref: String,
    ) -> Result<(), HealingError>;

    /// Register a patch proposal bound to the incident.
    fn propose_patch(
        &self,
        incident: &Incident,
        proposal: PatchProposal,
    ) -> Result<(), HealingError>;

    /// Record a sandbox validation verdict for a patch.
    fn record_sandbox_validation(
        &self,
        incident: &mut Incident,
        verdict: &SandboxVerdict,
    ) -> Result<(), HealingError>;

    /// Record a security validation verdict for a patch.
    fn record_security_validation(
        &self,
        incident: &mut Incident,
        verdict: &SecurityVerdict,
    ) -> Result<(), HealingError>;

    /// Bind a human/policy approval to the exact patch digest.
    fn record_approval(
        &self,
        incident: &mut Incident,
        approval: &RemediationApproval,
    ) -> Result<(), HealingError>;

    /// Record post-deploy verification outcome for the original
    /// reproduction. Only real observed verification may close an
    /// incident.
    fn record_post_deploy_verification(
        &self,
        incident: &mut Incident,
        verified: bool,
    ) -> Result<(), HealingError>;

    /// Verify the error code contract (SPEC-006) for a result.
    fn code(&self) -> HealingErrorCode;
}
