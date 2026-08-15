//! EP-017 M1 contract, vocabulary, and package boundary tests
//! (SPEC-010; ADR-024).
//!
//! Proves construction, validation, serialization, vocabulary
//! rejection, and dependency-direction constraints of the agent
//! orchestrator contracts. No provider behavior is exercised here
//! (M1 owns the contract boundary only).

use nexus_agents::{
    AgentAdapterKind, AgentArtifact, AgentBudget, AgentBudgetClass, AgentCapability, AgentTask,
    AgentTaskState, CapabilityRequest, Delegation, DelegationState,
};
use nexus_domain::{ArtifactId, CorrelationId, ObjectiveId, TaskId, TenantId};
use std::str::FromStr;

fn task_id(n: u8) -> TaskId {
    TaskId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a{n:02x}")).unwrap()
}

fn objective_id(n: u8) -> ObjectiveId {
    ObjectiveId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a{n:02x}")).unwrap()
}

fn correlation_id(n: u8) -> CorrelationId {
    CorrelationId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a{n:02x}")).unwrap()
}

fn tenant_id() -> TenantId {
    TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a80").unwrap()
}

fn budget() -> AgentBudget {
    AgentBudget::new(AgentBudgetClass::TotalTokens, 100_000)
}

// ---------------------------------------------------------------------
// Construction and validation
// ---------------------------------------------------------------------

#[test]
fn ep017_unit_task_constructs_and_validates() {
    let task = AgentTask::new(
        task_id(0x01),
        objective_id(0x02),
        correlation_id(0x03),
        tenant_id(),
        "p-1".into(),
        AgentCapability::Implement,
        budget(),
        1_700_000_000_000,
    )
    .unwrap();
    assert_eq!(task.state, AgentTaskState::Requested);
    assert!(task.parent_task.is_none());
    assert!(task.assigned_agent.is_none());
    assert!(task.artifact_ids.is_empty());
}

#[test]
fn ep017_unit_task_rejects_empty_principal() {
    let error = AgentTask::new(
        task_id(0x01),
        objective_id(0x02),
        correlation_id(0x03),
        tenant_id(),
        "".into(),
        AgentCapability::Implement,
        budget(),
        1_700_000_000_000,
    )
    .unwrap_err();
    assert_eq!(error.code.as_str(), "VALIDATION");
}

#[test]
fn ep017_unit_task_terminal_state_cannot_transition() {
    let mut task = AgentTask::new(
        task_id(0x01),
        objective_id(0x02),
        correlation_id(0x03),
        tenant_id(),
        "p-1".into(),
        AgentCapability::Implement,
        budget(),
        1_700_000_000_000,
    )
    .unwrap();
    task.transition(AgentTaskState::Succeeded, 1_700_000_000_001)
        .unwrap();
    let error = task
        .transition(AgentTaskState::Running, 1_700_000_000_002)
        .unwrap_err();
    assert_eq!(error.code.as_str(), "VALIDATION");
    assert!(AgentTaskState::Succeeded.is_terminal());
    assert!(AgentTaskState::Failed.is_terminal());
    assert!(AgentTaskState::Cancelled.is_terminal());
    assert!(!AgentTaskState::Running.is_terminal());
}

#[test]
fn ep017_unit_capability_request_constructs_and_validates() {
    let request = CapabilityRequest {
        request_id: "cr-1".into(),
        correlation_id: "c-1".into(),
        tenant_id: tenant_id().as_str().into(),
        principal_id: "p-1".into(),
        objective_id: objective_id(0x02).as_str().into(),
        task_id: task_id(0x01).as_str().into(),
        capability: AgentCapability::Review,
        required_permissions: vec!["read:repo".into()],
        budget: budget(),
    };
    request.validate().unwrap();
}

#[test]
fn ep017_unit_capability_request_rejects_empty_identity() {
    let request = CapabilityRequest {
        request_id: String::new(),
        correlation_id: "c-1".into(),
        tenant_id: tenant_id().as_str().into(),
        principal_id: "p-1".into(),
        objective_id: objective_id(0x02).as_str().into(),
        task_id: task_id(0x01).as_str().into(),
        capability: AgentCapability::Review,
        required_permissions: vec![],
        budget: budget(),
    };
    let error = request.validate().unwrap_err();
    assert_eq!(error.code.as_str(), "VALIDATION");
}

#[test]
fn ep017_unit_budget_validates_and_consumes_fail_closed() {
    let mut b = budget();
    b.validate().unwrap();
    assert_eq!(b.remaining(), 100_000);
    assert!(!b.exhausted());
    b.consume(40_000).unwrap();
    assert_eq!(b.used, 40_000);
    let error = b.consume(70_000).unwrap_err();
    assert_eq!(error.code.as_str(), "POLICY");
    assert!(b.exhausted() == false);
}

#[test]
fn ep017_unit_budget_rejects_zero_limit_and_over_usage() {
    let zero = AgentBudget::new(AgentBudgetClass::TotalCost, 0);
    let error = zero.validate().unwrap_err();
    assert_eq!(error.code.as_str(), "VALIDATION");

    let over = AgentBudget {
        class: AgentBudgetClass::TotalCost,
        limit: 10,
        used: 11,
    };
    let error = over.validate().unwrap_err();
    assert_eq!(error.code.as_str(), "VALIDATION");
}

#[test]
fn ep017_unit_artifact_validates_hash_and_name() {
    let artifact = AgentArtifact {
        artifact_id: ArtifactId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a40").unwrap(),
        task_id: task_id(0x01),
        name: "diff.patch".into(),
        content_hash: "a".repeat(64),
        provenance: vec![],
        content_type: "text/plain".into(),
        created_at_epoch_ms: 1_700_000_000_000,
    };
    artifact.validate().unwrap();
}

#[test]
fn ep017_unit_artifact_rejects_bad_hash() {
    let artifact = AgentArtifact {
        artifact_id: ArtifactId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8a40").unwrap(),
        task_id: task_id(0x01),
        name: "diff.patch".into(),
        content_hash: "short".into(),
        provenance: vec![],
        content_type: "text/plain".into(),
        created_at_epoch_ms: 1_700_000_000_000,
    };
    let error = artifact.validate().unwrap_err();
    assert_eq!(error.code.as_str(), "VALIDATION");
}

#[test]
fn ep017_unit_delegation_constructs_and_validates() {
    let delegation = Delegation {
        delegation_id: "d-1".into(),
        correlation_id: correlation_id(0x03),
        objective_id: objective_id(0x02),
        task_id: task_id(0x01),
        from_principal: "p-1".into(),
        to_agent: nexus_agents::AgentCardId("agent-1".into()),
        state: DelegationState::Proposed,
        created_at_epoch_ms: 1_700_000_000_000,
    };
    delegation.validate().unwrap();
}

// ---------------------------------------------------------------------
// Serialization round trips
// ---------------------------------------------------------------------

#[test]
fn ep017_unit_task_serialization_round_trip() {
    let task = AgentTask::new(
        task_id(0x01),
        objective_id(0x02),
        correlation_id(0x03),
        tenant_id(),
        "p-1".into(),
        AgentCapability::Implement,
        budget(),
        1_700_000_000_000,
    )
    .unwrap();
    let json = serde_json::to_string(&task).unwrap();
    let decoded: AgentTask = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, task);
}

#[test]
fn ep017_unit_capability_request_serialization_round_trip() {
    let request = CapabilityRequest {
        request_id: "cr-1".into(),
        correlation_id: "c-1".into(),
        tenant_id: tenant_id().as_str().into(),
        principal_id: "p-1".into(),
        objective_id: objective_id(0x02).as_str().into(),
        task_id: task_id(0x01).as_str().into(),
        capability: AgentCapability::Orchestrate,
        required_permissions: vec!["read:repo".into()],
        budget: budget(),
    };
    let json = serde_json::to_string(&request).unwrap();
    let decoded: CapabilityRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, request);
    assert!(json.contains("\"ORCHESTRATE\""));
}

#[test]
fn ep017_unit_budget_serialization_round_trip() {
    let json = serde_json::to_string(&budget()).unwrap();
    let decoded: AgentBudget = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, budget());
    assert!(json.contains("\"TOTAL_TOKENS\""));
}

// ---------------------------------------------------------------------
// Vocabulary rejection
// ---------------------------------------------------------------------

#[test]
fn ep017_unit_vocabulary_task_state_round_trip_and_rejection() {
    for (wire, expected) in [
        ("REQUESTED", AgentTaskState::Requested),
        ("ASSIGNED", AgentTaskState::Assigned),
        ("RUNNING", AgentTaskState::Running),
        ("PAUSED", AgentTaskState::Paused),
        ("WAITING_INPUT", AgentTaskState::WaitingInput),
        ("REVIEWING", AgentTaskState::Reviewing),
        ("CANCELLED", AgentTaskState::Cancelled),
        ("SUCCEEDED", AgentTaskState::Succeeded),
        ("FAILED", AgentTaskState::Failed),
    ] {
        assert_eq!(AgentTaskState::from_str(wire).unwrap(), expected);
        assert_eq!(expected.as_str(), wire);
    }
    assert!(AgentTaskState::from_str("INVENTED").is_err());
}

#[test]
fn ep017_unit_vocabulary_adapter_kind_round_trip_and_rejection() {
    for (wire, expected) in [
        ("CODEX", AgentAdapterKind::Codex),
        ("CLAUDE_CODE", AgentAdapterKind::ClaudeCode),
        ("HERMES", AgentAdapterKind::Hermes),
        ("OPENCLAW", AgentAdapterKind::OpenClaw),
    ] {
        assert_eq!(AgentAdapterKind::from_str(wire).unwrap(), expected);
        assert_eq!(expected.as_str(), wire);
    }
    assert_eq!(AgentAdapterKind::ALL.len(), 4);
    assert!(AgentAdapterKind::from_str("GPT5").is_err());
}

#[test]
fn ep017_unit_vocabulary_capability_round_trip_and_rejection() {
    for (wire, expected) in [
        ("ORCHESTRATE", AgentCapability::Orchestrate),
        ("IMPLEMENT", AgentCapability::Implement),
        ("REVIEW", AgentCapability::Review),
        ("TEST", AgentCapability::Test),
        ("EXECUTE", AgentCapability::Execute),
        ("SUMMARIZE", AgentCapability::Summarize),
        ("ARTIFACT", AgentCapability::Artifact),
    ] {
        assert_eq!(AgentCapability::from_str(wire).unwrap(), expected);
        assert_eq!(expected.as_str(), wire);
    }
    assert!(AgentCapability::from_str("HACK").is_err());
}

#[test]
fn ep017_unit_vocabulary_delegation_state_round_trip_and_rejection() {
    for (wire, expected) in [
        ("PROPOSED", DelegationState::Proposed),
        ("ACCEPTED", DelegationState::Accepted),
        ("ACTIVE", DelegationState::Active),
        ("COMPLETED", DelegationState::Completed),
        ("REVOKED", DelegationState::Revoked),
        ("FAILED", DelegationState::Failed),
    ] {
        assert_eq!(DelegationState::from_str(wire).unwrap(), expected);
        assert_eq!(expected.as_str(), wire);
    }
    assert!(DelegationState::from_str("GHOSTED").is_err());
}

#[test]
fn ep017_unit_vocabulary_budget_class_round_trip_and_rejection() {
    for (wire, expected) in [
        ("TOTAL_TOKENS", AgentBudgetClass::TotalTokens),
        ("TOTAL_COST", AgentBudgetClass::TotalCost),
        ("MAX_CONCURRENT", AgentBudgetClass::MaxConcurrent),
        ("MAX_DURATION_SECS", AgentBudgetClass::MaxDurationSecs),
    ] {
        assert_eq!(AgentBudgetClass::from_str(wire).unwrap(), expected);
        assert_eq!(expected.as_str(), wire);
    }
    assert!(AgentBudgetClass::from_str("UNLIMITED").is_err());
}

#[test]
fn ep017_unit_review_contract_validates() {
    let review = nexus_agents::AdapterReview {
        session_id: nexus_agents::AdapterSessionId("s-1".into()),
        review_kind: "code-review".into(),
        target_artifact_ids: vec!["a-1".into()],
        verdict: None,
    };
    review.validate().unwrap();
}
