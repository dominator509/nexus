//! EP-008 unit tests: construction, validation, serialization,
//! vocabulary rejection, and risk floors (SPEC-005/SPEC-006).

use std::str::FromStr;

use crate::approval::{ApprovalAssertion, ApprovalAssertionError, ApprovalDecision};
use crate::capability::{CapabilityGrant, CapabilityGrantError, GrantState};
use crate::error::{PolicyError, PolicyErrorCode};
use crate::gateway::{ActionRequest, ActionRequestError, DenialReason};
use crate::policy::{ContextPolicyEngine, PolicyDecision, PolicyInput};
use crate::receipt::{ActionReceipt, ReceiptError};
use crate::relationship::{RelationshipDecision, RelationshipError, RelationshipTuple};
use crate::risk::{RiskAssessmentInput, deterministic_risk_floor};
use crate::verification::{ExpectedState, VerificationPlan, VerificationResult};
use crate::vocabulary::{ActionLifecycleState, PolicyVocabularyError};
use nexus_auth::AuthenticationStrength;
use nexus_domain::{
    ApprovalClass, CapabilityClass, CorrelationId, NexusId, PrincipalType, Reversal, Risk, TenantId,
};
use nexus_identity::{Principal, TrustLevel};

fn tenant() -> TenantId {
    TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn nexus() -> NexusId {
    NexusId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
}

fn nexus2() -> NexusId {
    NexusId::new("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap()
}

fn corr() -> CorrelationId {
    CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000004").unwrap()
}

fn principal() -> Principal {
    Principal::new(nexus(), PrincipalType::Human, tenant())
}

#[test]
fn ep008_unit_relationship_tuple_constructs_and_roundtrips() {
    let t = RelationshipTuple::new(
        tenant(),
        principal(),
        "owner",
        "task",
        "018f0f6f-9c1e-7b6e-8000-000000000010",
    )
    .unwrap();
    let json = serde_json::to_string(&t).unwrap();
    let back: RelationshipTuple = serde_json::from_str(&json).unwrap();
    assert_eq!(t, back);
    assert_eq!(t.relation, "owner");
}

#[test]
fn ep008_unit_relationship_tuple_rejects_empty_relation() {
    let err = RelationshipTuple::new(
        tenant(),
        principal(),
        "",
        "task",
        "018f0f6f-9c1e-7b6e-8000-000000000010",
    )
    .unwrap_err();
    assert_eq!(err, RelationshipError::EmptyRelation);
}

#[test]
fn ep008_unit_relationship_tuple_rejects_bad_object_id() {
    let err =
        RelationshipTuple::new(tenant(), principal(), "owner", "task", "not-a-uuid").unwrap_err();
    assert_eq!(err, RelationshipError::InvalidObjectId);
}

#[test]
fn ep008_unit_relationship_decision_fails_closed() {
    let allowed = RelationshipDecision::Allowed;
    assert!(allowed.is_allowed());
    let denied = RelationshipDecision::Denied {
        reason: "no tuple".into(),
    };
    assert!(!denied.is_allowed());
}

#[test]
fn ep008_unit_policy_input_rejects_empty_object_type() {
    let err = PolicyInput::new(
        tenant(),
        principal(),
        CapabilityClass::Query,
        Risk::R0,
        AuthenticationStrength::MultiFactor,
        TrustLevel::Verified,
        "",
        "018f0f6f-9c1e-7b6e-8000-000000000010",
    )
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        crate::policy::PolicyInputError::EmptyObjectType.to_string()
    );
}

#[test]
fn ep008_unit_policy_decision_allow_deny_serde() {
    let allow = PolicyDecision::allow("policy/v1");
    assert!(allow.allowed);
    let deny = PolicyDecision::deny("policy/v1", "no read permission");
    assert!(!deny.allowed);
    let json = serde_json::to_string(&allow).unwrap();
    let back: PolicyDecision = serde_json::from_str(&json).unwrap();
    assert_eq!(allow, back);
}

#[test]
fn ep008_unit_risk_floor_query_is_r0() {
    let input = RiskAssessmentInput::new(
        CapabilityClass::Query,
        Reversal::None,
        false,
        AuthenticationStrength::SingleFactor,
    );
    assert_eq!(deterministic_risk_floor(&input).unwrap(), Risk::R0);
}

#[test]
fn ep008_unit_risk_floor_secret_query_is_r2() {
    let input = RiskAssessmentInput::new(
        CapabilityClass::Query,
        Reversal::None,
        true,
        AuthenticationStrength::SingleFactor,
    );
    assert_eq!(deterministic_risk_floor(&input).unwrap(), Risk::R2);
}

#[test]
fn ep008_unit_risk_floor_irreversible_command_is_r3() {
    let input = RiskAssessmentInput::new(
        CapabilityClass::Command,
        Reversal::Irreversible,
        false,
        AuthenticationStrength::MultiFactor,
    );
    assert_eq!(deterministic_risk_floor(&input).unwrap(), Risk::R3);
}

#[test]
fn ep008_unit_risk_floor_irreversible_admin_is_r4() {
    let input = RiskAssessmentInput::new(
        CapabilityClass::Administrative,
        Reversal::Irreversible,
        false,
        AuthenticationStrength::StepUp,
    );
    assert_eq!(deterministic_risk_floor(&input).unwrap(), Risk::R4);
}

#[test]
fn ep008_unit_capability_grant_scope_and_times() {
    let grant = CapabilityGrant::new(
        nexus(),
        tenant(),
        CapabilityClass::Command,
        nexus(),
        nexus2(),
        "task:complete",
        100,
        200,
    )
    .unwrap();
    assert!(grant.is_usable_at(150));
    assert!(!grant.is_usable_at(200));
    assert!(!grant.is_usable_at(300));
}

#[test]
fn ep008_unit_capability_grant_rejects_inverted_times() {
    let err = CapabilityGrant::new(
        nexus(),
        tenant(),
        CapabilityClass::Command,
        nexus(),
        nexus2(),
        "task:complete",
        200,
        100,
    )
    .unwrap_err();
    assert_eq!(err, CapabilityGrantError::InvertedTimes);
}

#[test]
fn ep008_unit_capability_grant_revoke_and_expire() {
    let mut grant = CapabilityGrant::new(
        nexus(),
        tenant(),
        CapabilityClass::Command,
        nexus(),
        nexus2(),
        "task:complete",
        100,
        200,
    )
    .unwrap();
    grant.revoke();
    assert_eq!(grant.state, GrantState::Revoked);
    assert!(!grant.is_usable_at(150));
}

#[test]
fn ep008_unit_approval_assertion_binds_to_digest() {
    let assertion = ApprovalAssertion::new(
        nexus(),
        corr(),
        "digest-abc",
        nexus2(),
        ApprovalClass::Human,
        AuthenticationStrength::MultiFactor,
        ApprovalDecision::Approved,
        100,
        200,
    )
    .unwrap();
    assert!(assertion.approves("digest-abc", 150));
    assert!(!assertion.approves("digest-other", 150));
    assert!(!assertion.approves("digest-abc", 250));
}

#[test]
fn ep008_unit_approval_assertion_rejects_empty_digest() {
    let err = ApprovalAssertion::new(
        nexus(),
        corr(),
        "",
        nexus2(),
        ApprovalClass::Human,
        AuthenticationStrength::MultiFactor,
        ApprovalDecision::Approved,
        100,
        200,
    )
    .unwrap_err();
    assert_eq!(err, ApprovalAssertionError::EmptyDigest);
}

#[test]
fn ep008_unit_action_request_constructs_and_rejects_empty() {
    let req = ActionRequest::new(
        nexus(),
        corr(),
        tenant(),
        "digest-abc",
        "task:complete",
        nexus2(),
        150,
    )
    .unwrap();
    assert_eq!(req.action, "task:complete");
    let err = ActionRequest::new(
        nexus(),
        corr(),
        tenant(),
        "",
        "task:complete",
        nexus2(),
        150,
    )
    .unwrap_err();
    assert_eq!(err, ActionRequestError::EmptyDigest);
}

#[test]
fn ep008_unit_receipt_from_decision_denied() {
    let decision = crate::gateway::ActionDecision::Denied {
        reason: DenialReason::Policy,
        message: "denied by policy".into(),
    };
    let receipt = ActionReceipt::from_decision(
        nexus(),
        corr(),
        nexus2(),
        &decision,
        "policy/v1",
        vec!["ref:audit-1".into()],
        160,
    )
    .unwrap();
    assert_eq!(receipt.lifecycle, ActionLifecycleState::Rejected);
    assert_eq!(receipt.denial_reason, Some(DenialReason::Policy));
    assert_eq!(receipt.policy_version, "policy/v1");
    assert_eq!(receipt.state.to_string(), "ISSUED");
}

#[test]
fn ep008_unit_receipt_rejects_empty_policy_version() {
    let decision = crate::gateway::ActionDecision::Denied {
        reason: DenialReason::Policy,
        message: "denied".into(),
    };
    let err = ActionReceipt::from_decision(nexus(), corr(), nexus2(), &decision, "", vec![], 160)
        .unwrap_err();
    assert_eq!(err, ReceiptError::EmptyPolicyVersion);
}

#[test]
fn ep008_unit_verification_plan_and_result() {
    let expected = ExpectedState::new(nexus2(), "task:completed").unwrap();
    let plan = VerificationPlan::new(expected, 30, 2).unwrap();
    assert_eq!(plan.timeout_seconds, 30);
    let result = VerificationResult::new(true, "task:completed", 200).unwrap();
    assert!(result.matched);
    let json = serde_json::to_string(&plan).unwrap();
    let back: VerificationPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(plan, back);
}

#[test]
fn ep008_unit_verification_plan_rejects_zero_timeout() {
    let expected = ExpectedState::new(nexus2(), "task:completed").unwrap();
    assert!(VerificationPlan::new(expected, 0, 1).is_err());
}

#[test]
fn ep008_unit_lifecycle_vocabulary_accepts_and_rejects() {
    assert_eq!(
        ActionLifecycleState::from_str("EXECUTING").unwrap(),
        ActionLifecycleState::Executing
    );
    assert_eq!(ActionLifecycleState::Compensating.as_str(), "COMPENSATING");
    assert!(ActionLifecycleState::from_str("COMPLETED").is_err());
    let err = ActionLifecycleState::from_str("COMPLETED").unwrap_err();
    assert!(matches!(err, PolicyVocabularyError(_)));
}

#[test]
fn ep008_unit_lifecycle_vocabulary_serde_roundtrip() {
    let json = serde_json::to_string(&ActionLifecycleState::AwaitingApproval).unwrap();
    assert_eq!(json, "\"AWAITING_APPROVAL\"");
    let back: ActionLifecycleState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, ActionLifecycleState::AwaitingApproval);
}

#[test]
fn ep008_unit_policy_error_codes_are_stable() {
    assert_eq!(
        PolicyErrorCode::ExternalProvider.as_str(),
        "external_provider"
    );
    assert_eq!(PolicyErrorCode::Verification.as_str(), "verification");
    let err = PolicyError::new(PolicyErrorCode::Policy, "denied", Some(corr()));
    assert_eq!(err.code, PolicyErrorCode::Policy);
    assert!(err.message.contains("denied"));
}

#[test]
fn ep008_unit_denial_reasons_are_stable() {
    assert_eq!(
        DenialReason::InsufficientStrength.as_str(),
        "INSUFFICIENT_STRENGTH"
    );
    assert_eq!(DenialReason::MissingApproval.as_str(), "MISSING_APPROVAL");
    assert_eq!(
        DenialReason::VerificationFailed.as_str(),
        "VERIFICATION_FAILED"
    );
}

// A minimal real ContextPolicyEngine implementation used only to prove
// the port contract is implementable and fail-closed.
struct FloorPolicyEngine {
    version: &'static str,
}

impl ContextPolicyEngine for FloorPolicyEngine {
    fn evaluate(&self, input: &PolicyInput) -> Result<PolicyDecision, PolicyError> {
        // Deterministic floor: secret-touching queries and R3+ actions
        // require STEP_UP; otherwise allow.
        let strong_enough = match input.risk {
            Risk::R0 | Risk::R1 | Risk::R2 => true,
            Risk::R3 | Risk::R4 => input.strength == AuthenticationStrength::StepUp,
        };
        if strong_enough {
            Ok(PolicyDecision::allow(self.version))
        } else {
            Ok(PolicyDecision::deny(self.version, "step-up required"))
        }
    }
}

#[test]
fn ep008_unit_context_policy_engine_port_is_fail_closed() {
    let engine = FloorPolicyEngine {
        version: "policy/v1",
    };
    let ok = PolicyInput::new(
        tenant(),
        principal(),
        CapabilityClass::Query,
        Risk::R0,
        AuthenticationStrength::SingleFactor,
        TrustLevel::Verified,
        "task",
        "018f0f6f-9c1e-7b6e-8000-000000000010",
    )
    .unwrap();
    assert!(engine.evaluate(&ok).unwrap().allowed);

    let r4 = PolicyInput::new(
        tenant(),
        principal(),
        CapabilityClass::Administrative,
        Risk::R4,
        AuthenticationStrength::SingleFactor,
        TrustLevel::Verified,
        "task",
        "018f0f6f-9c1e-7b6e-8000-000000000010",
    )
    .unwrap();
    let denied = engine.evaluate(&r4).unwrap();
    assert!(!denied.allowed);
    assert_eq!(denied.reason, "step-up required");
}
