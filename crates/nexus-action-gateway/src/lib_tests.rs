//! EP-008 M2 unit tests: the deterministic gateway engine.
//!
//! Proves the acceptance obligations with real provider ports (simple
//! in-test implementations of the nexus-policy traits - test-double
//! zone per TESTING.md): fail closed on relationship/policy/capability/
//! approval, R3/R4 approval requirements, digest binding, model-approval
//! rejection, determinism, and grant scoping.

use nexus_auth::AuthenticationStrength;
use nexus_domain::{
    ApprovalClass, CapabilityClass, CorrelationId, NexusId, PrincipalType, Reversal, Risk, TenantId,
};
use nexus_identity::Principal;
use nexus_policy::approval::{ApprovalAssertion, ApprovalDecision};
use nexus_policy::capability::CapabilityGrant;
use nexus_policy::error::{PolicyError, PolicyErrorCode};
use nexus_policy::gateway::{ActionDecision, ActionGateway, ActionRequest, DenialReason};
use nexus_policy::policy::{ContextPolicyEngine, PolicyDecision, PolicyInput};
use nexus_policy::relationship::{RelationshipAuthorizer, RelationshipDecision, RelationshipTuple};

use crate::engine::{DecisionInput, DeterministicGateway, GatewayConfig, GatewayOutcome};

fn tenant() -> TenantId {
    TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn id(n: u32) -> NexusId {
    NexusId::new(format!("018f0f6f-9c1e-7b6e-8000-{n:012}")).unwrap()
}

fn corr() -> CorrelationId {
    CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000004").unwrap()
}

fn actor() -> Principal {
    Principal::new(id(2), PrincipalType::Human, tenant())
}

fn request(digest: &str, action: &str, target: u32) -> ActionRequest {
    ActionRequest::new(id(1), corr(), tenant(), digest, action, id(target), 100).unwrap()
}

fn grant(actor: NexusId, target: NexusId, scope: &str, exp: i64) -> CapabilityGrant {
    CapabilityGrant::new(
        id(5),
        tenant(),
        CapabilityClass::Command,
        actor,
        target,
        scope,
        90,
        exp,
    )
    .unwrap()
}

fn approval(digest: &str, class: ApprovalClass, exp: i64) -> ApprovalAssertion {
    ApprovalAssertion::new(
        id(6),
        corr(),
        digest,
        id(2),
        class,
        AuthenticationStrength::MultiFactor,
        ApprovalDecision::Approved,
        90,
        exp,
    )
    .unwrap()
}

// ---- test-double provider ports (TESTING.md test zone) ----

#[derive(Clone)]
struct AllowRelationships;

impl RelationshipAuthorizer for AllowRelationships {
    fn check(&self, _tuple: &RelationshipTuple) -> Result<RelationshipDecision, PolicyError> {
        Ok(RelationshipDecision::Allowed)
    }
}

#[derive(Clone)]
struct DenyRelationships;

impl RelationshipAuthorizer for DenyRelationships {
    fn check(&self, _tuple: &RelationshipTuple) -> Result<RelationshipDecision, PolicyError> {
        Ok(RelationshipDecision::Denied {
            reason: "no tuple".into(),
        })
    }
}

#[derive(Clone)]
struct FailRelationships;

impl RelationshipAuthorizer for FailRelationships {
    fn check(&self, _tuple: &RelationshipTuple) -> Result<RelationshipDecision, PolicyError> {
        Err(PolicyError::new(
            PolicyErrorCode::ExternalProvider,
            "provider unavailable",
            None,
        ))
    }
}

#[derive(Clone)]
struct AllowPolicy;

impl ContextPolicyEngine for AllowPolicy {
    fn evaluate(&self, _input: &PolicyInput) -> Result<PolicyDecision, PolicyError> {
        Ok(PolicyDecision::allow("policy/v1"))
    }
}

#[derive(Clone)]
struct DenyPolicy;

impl ContextPolicyEngine for DenyPolicy {
    fn evaluate(&self, _input: &PolicyInput) -> Result<PolicyDecision, PolicyError> {
        Ok(PolicyDecision::deny("policy/v1", "no read permission"))
    }
}

// ---- helpers ----

fn gateway_with(
    rel: impl RelationshipAuthorizer + Clone + 'static,
    pol: impl ContextPolicyEngine + Clone + 'static,
) -> DeterministicGateway<impl RelationshipAuthorizer + Clone, impl ContextPolicyEngine + Clone> {
    let config = GatewayConfig::new("policy/v1").unwrap();
    DeterministicGateway::new(config, rel, pol)
}

/// A standard R2-scoped input (Command, compensating, not secret).
fn input_r2(
    req: ActionRequest,
    grant: Option<CapabilityGrant>,
    approval: Option<ApprovalAssertion>,
) -> DecisionInput {
    DecisionInput::new(
        req,
        actor(),
        CapabilityClass::Command,
        Reversal::Compensating,
        false,
        grant,
        approval,
        150,
    )
}

/// An R4-scoped input (Administrative, irreversible, not secret).
fn input_r4(
    req: ActionRequest,
    grant: Option<CapabilityGrant>,
    approval: Option<ApprovalAssertion>,
) -> DecisionInput {
    DecisionInput::new(
        req,
        actor(),
        CapabilityClass::Administrative,
        Reversal::Irreversible,
        false,
        grant,
        approval,
        150,
    )
}

fn outcome_of(input: &DecisionInput) -> GatewayOutcome {
    let gw = gateway_with(AllowRelationships, AllowPolicy);
    gw.evaluate_input(input).unwrap()
}

// ---- tests ----

#[test]
fn ep008_unit_gateway_allows_covered_request() {
    let req = request("digest-abc", "task:complete", 10);
    let input = input_r2(req, Some(grant(id(2), id(10), "task:complete", 300)), None);
    let out = outcome_of(&input);
    assert!(out.decision.is_allowed());
    assert_eq!(out.policy_version, "policy/v1");
    assert_eq!(out.risk, Risk::R2);
}

#[test]
fn ep008_unit_gateway_denies_when_relationship_missing() {
    let req = request("digest-abc", "task:complete", 10);
    let input = input_r2(req, Some(grant(id(2), id(10), "task:complete", 300)), None);
    let gw = gateway_with(DenyRelationships, AllowPolicy);
    let out = gw.evaluate_input(&input).unwrap();
    assert!(!out.decision.is_allowed());
    assert_eq!(
        out.decision,
        ActionDecision::Denied {
            reason: DenialReason::Relationship,
            message: "no tuple".into(),
        }
    );
}

#[test]
fn ep008_unit_gateway_fails_closed_on_provider_error() {
    let req = request("digest-abc", "task:complete", 10);
    let input = input_r2(req, Some(grant(id(2), id(10), "task:complete", 300)), None);
    let gw = gateway_with(FailRelationships, AllowPolicy);
    let res = gw.evaluate_input(&input);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.code, PolicyErrorCode::ExternalProvider);
}

#[test]
fn ep008_unit_gateway_denies_when_policy_denies() {
    let req = request("digest-abc", "task:complete", 10);
    let input = input_r2(req, Some(grant(id(2), id(10), "task:complete", 300)), None);
    let gw = gateway_with(AllowRelationships, DenyPolicy);
    let out = gw.evaluate_input(&input).unwrap();
    assert!(!out.decision.is_allowed());
    assert_eq!(
        out.decision,
        ActionDecision::Denied {
            reason: DenialReason::Policy,
            message: "policy denied: no read permission".into(),
        }
    );
}

#[test]
fn ep008_unit_gateway_denies_without_grant() {
    let req = request("digest-abc", "task:complete", 10);
    let input = input_r2(req, None, None);
    let out = outcome_of(&input);
    assert_eq!(
        out.decision,
        ActionDecision::Denied {
            reason: DenialReason::NoCapability,
            message: "no capability grant covers the request".into(),
        }
    );
}

#[test]
fn ep008_unit_gateway_denies_expired_grant() {
    let req = request("digest-abc", "task:complete", 10);
    let input = input_r2(req, Some(grant(id(2), id(10), "task:complete", 100)), None);
    let out = outcome_of(&input);
    assert!(!out.decision.is_allowed());
    assert_eq!(
        out.decision,
        ActionDecision::Denied {
            reason: DenialReason::NoCapability,
            message: "capability grant does not cover the request".into(),
        }
    );
}

#[test]
fn ep008_unit_gateway_denies_scope_mismatch() {
    let req = request("digest-abc", "task:delete", 10);
    let input = input_r2(req, Some(grant(id(2), id(10), "task:complete", 300)), None);
    let out = outcome_of(&input);
    assert!(!out.decision.is_allowed());
    assert_eq!(
        out.decision,
        ActionDecision::Denied {
            reason: DenialReason::NoCapability,
            message: "capability grant does not cover the request".into(),
        }
    );
}

#[test]
fn ep008_unit_gateway_denies_actor_mismatch() {
    let req = request("digest-abc", "task:complete", 10);
    // Grant bound to a different actor (id 3, not the acting id 2).
    let input = input_r2(req, Some(grant(id(3), id(10), "task:complete", 300)), None);
    let out = outcome_of(&input);
    assert!(!out.decision.is_allowed());
}

#[test]
fn ep008_unit_gateway_denies_target_mismatch() {
    let req = request("digest-abc", "task:complete", 10);
    let input = input_r2(req, Some(grant(id(2), id(11), "task:complete", 300)), None);
    let out = outcome_of(&input);
    assert!(!out.decision.is_allowed());
}

#[test]
fn ep008_unit_gateway_requires_approval_for_r3_r4() {
    // Administrative + irreversible = R4 per the deterministic floor.
    // R4 requires a human approval assertion; none is provided.
    let req = request("digest-abc", "admin:purge", 10);
    let input = input_r4(req, Some(grant(id(2), id(10), "admin:purge", 300)), None);
    let out = outcome_of(&input);
    assert!(!out.decision.is_allowed());
    assert_eq!(
        out.decision,
        ActionDecision::Denied {
            reason: DenialReason::MissingApproval,
            message: "R3/R4 action requires a valid human approval assertion".into(),
        }
    );
}

#[test]
fn ep008_unit_gateway_accepts_approval_with_matching_digest() {
    let req = request("digest-abc", "admin:purge", 10);
    let input = input_r4(
        req,
        Some(grant(id(2), id(10), "admin:purge", 300)),
        Some(approval("digest-abc", ApprovalClass::StrongHuman, 300)),
    );
    let out = outcome_of(&input);
    assert!(out.decision.is_allowed());
}

#[test]
fn ep008_unit_gateway_rejects_approval_with_wrong_digest() {
    let req = request("digest-abc", "admin:purge", 10);
    let input = input_r4(
        req,
        Some(grant(id(2), id(10), "admin:purge", 300)),
        Some(approval("digest-other", ApprovalClass::StrongHuman, 300)),
    );
    let out = outcome_of(&input);
    assert!(!out.decision.is_allowed());
    assert_eq!(
        out.decision,
        ActionDecision::Denied {
            reason: DenialReason::MissingApproval,
            message: "R3/R4 action requires a valid human approval assertion".into(),
        }
    );
}

#[test]
fn ep008_unit_gateway_rejects_expired_approval() {
    let req = request("digest-abc", "admin:purge", 10);
    let input = input_r4(
        req,
        Some(grant(id(2), id(10), "admin:purge", 300)),
        Some(approval("digest-abc", ApprovalClass::StrongHuman, 100)),
    );
    let out = outcome_of(&input);
    assert!(!out.decision.is_allowed());
}

#[test]
fn ep008_unit_gateway_rejects_model_approval_for_r4() {
    // SPEC-005 behavior 4: R4 never accepts model approval. A POLICY
    // class assertion is a model/agent approval and must be rejected.
    let req = request("digest-abc", "admin:purge", 10);
    let input = input_r4(
        req,
        Some(grant(id(2), id(10), "admin:purge", 300)),
        Some(approval("digest-abc", ApprovalClass::Policy, 300)),
    );
    let out = outcome_of(&input);
    assert!(!out.decision.is_allowed());
    assert_eq!(
        out.decision,
        ActionDecision::Denied {
            reason: DenialReason::MissingApproval,
            message: "R3/R4 action requires a valid human approval assertion".into(),
        }
    );
}

#[test]
fn ep008_unit_gateway_is_deterministic() {
    let req = request("digest-abc", "task:complete", 10);
    let input = input_r2(req, Some(grant(id(2), id(10), "task:complete", 300)), None);
    let a = outcome_of(&input);
    let b = outcome_of(&input);
    assert_eq!(a, b);
}

#[test]
fn ep008_unit_gateway_config_rejects_empty_policy_version() {
    assert!(GatewayConfig::new("").is_err());
}

#[test]
fn ep008_unit_gateway_port_method_fails_closed_without_input() {
    let gw = gateway_with(AllowRelationships, AllowPolicy);
    let req = request("digest-abc", "task:complete", 10);
    let res = ActionGateway::evaluate(&gw, &req);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert_eq!(err.code, PolicyErrorCode::InternalInvariant);
}

#[test]
fn ep008_unit_gateway_uses_risk_floor_ordering() {
    use nexus_policy::risk::{risk_at_least, risk_rank};
    assert_eq!(risk_rank(Risk::R0), 0);
    assert_eq!(risk_rank(Risk::R4), 4);
    assert!(risk_at_least(Risk::R4, Risk::R3));
    assert!(risk_at_least(Risk::R3, Risk::R3));
    assert!(!risk_at_least(Risk::R2, Risk::R3));
}
