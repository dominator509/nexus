//! Deterministic action gateway engine (EP-008 M2).
//!
//! The engine is pure: no wall clock, no randomness, no network, no
//! database. All external inputs (relationship results, policy
//! decisions, risk, grants, approvals, the authenticated actor, and the
//! current time) are passed in through `DecisionInput` and the injected
//! provider ports. The same inputs always produce the same decision,
//! which makes the engine replayable and testable under Temporal.

use nexus_auth::AuthenticationStrength;
use nexus_domain::{ApprovalClass, CapabilityClass, Reversal, Risk};
use nexus_identity::{Principal, TrustLevel};
use nexus_policy::approval::{ApprovalAssertion, ApprovalDecision};
use nexus_policy::capability::CapabilityGrant;
use nexus_policy::error::PolicyError;
use nexus_policy::gateway::{ActionDecision, ActionGateway, ActionRequest, DenialReason};
use nexus_policy::policy::{ContextPolicyEngine, PolicyDecision, PolicyInput};
use nexus_policy::relationship::{RelationshipAuthorizer, RelationshipDecision, RelationshipTuple};
use nexus_policy::risk::{RiskAssessmentInput, deterministic_risk_floor, risk_at_least};

/// At or above which risk an approval assertion is mandatory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApprovalRequirement {
    /// Risk class threshold: actions at or above this risk require a
    /// valid approval assertion or a cryptographic step-up.
    pub risk_threshold: Risk,
}

impl Default for ApprovalRequirement {
    fn default() -> Self {
        Self {
            risk_threshold: Risk::R3,
        }
    }
}

/// Static gateway configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayConfig {
    /// Policy version the gateway claims for receipts.
    pub policy_version: String,
    /// Approval requirement.
    pub approval: ApprovalRequirement,
}

impl GatewayConfig {
    /// Construct a gateway config; rejects empty policy version.
    pub fn new(policy_version: impl Into<String>) -> Result<Self, GatewayError> {
        let policy_version = policy_version.into();
        if policy_version.trim().is_empty() {
            return Err(GatewayError::EmptyPolicyVersion);
        }
        Ok(Self {
            policy_version,
            approval: ApprovalRequirement::default(),
        })
    }

    /// Set a non-default approval threshold.
    pub fn with_approval_threshold(mut self, risk_threshold: Risk) -> Self {
        self.approval.risk_threshold = risk_threshold;
        self
    }
}

/// All external inputs the gateway needs for one evaluation.
#[derive(Debug, Clone)]
pub struct DecisionInput {
    /// The action request being evaluated.
    pub request: ActionRequest,
    /// The authenticated actor (resolved by the adapter boundary).
    pub actor: Principal,
    /// Capability class of the action (risk input).
    pub capability: CapabilityClass,
    /// Reversal class of the action (risk input).
    pub reversal: Reversal,
    /// Whether the action touches secret/security-class data (risk input).
    pub touches_secret: bool,
    /// The candidate capability grant, if any.
    pub grant: Option<CapabilityGrant>,
    /// The approval assertion, if any, bound to the action digest.
    pub approval: Option<ApprovalAssertion>,
    /// Current time, unix seconds (injected; never read from wall clock).
    pub now_unix_s: i64,
}

impl DecisionInput {
    /// Construct an evaluation input.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request: ActionRequest,
        actor: Principal,
        capability: CapabilityClass,
        reversal: Reversal,
        touches_secret: bool,
        grant: Option<CapabilityGrant>,
        approval: Option<ApprovalAssertion>,
        now_unix_s: i64,
    ) -> Self {
        Self {
            request,
            actor,
            capability,
            reversal,
            touches_secret,
            grant,
            approval,
            now_unix_s,
        }
    }
}

/// The deterministic gateway engine.
///
/// `RA` is the relationship authorizer, `CE` the contextual policy
/// engine. Risk classification uses the deterministic floor from
/// nexus-policy (SPEC-005/SPEC-006), so the engine's decision is
/// fully deterministic given its inputs. Provider failures are mapped
/// to fail-closed denials.
pub struct DeterministicGateway<RA, CE> {
    config: GatewayConfig,
    relationships: RA,
    policy: CE,
}

impl<RA, CE> DeterministicGateway<RA, CE>
where
    RA: RelationshipAuthorizer,
    CE: ContextPolicyEngine,
{
    /// Construct the gateway from its providers.
    pub fn new(config: GatewayConfig, relationships: RA, policy: CE) -> Self {
        Self {
            config,
            relationships,
            policy,
        }
    }

    /// Evaluate one action request with all external inputs.
    ///
    /// Deterministic order (SPEC-005/SPEC-006):
    /// 1. Relationship check (fail closed).
    /// 2. Contextual policy (fail closed).
    /// 3. Risk classification (deterministic floor).
    /// 4. Approval requirement for R3/R4.
    /// 5. Capability grant (usable, scoped, actor, target).
    /// 6. Allow.
    pub fn evaluate_input(&self, input: &DecisionInput) -> Result<GatewayOutcome, PolicyError> {
        let request = &input.request;
        let now = input.now_unix_s;

        // 1. Relationship: actor holds the action relation on the target.
        let tuple = RelationshipTuple::new(
            request.tenant_id.clone(),
            input.actor.clone(),
            "actor",
            "action",
            request.target_id.as_str(),
        )
        .map_err(|_| PolicyError::validation("invalid relationship tuple"))?;
        let rel = self.relationships.check(&tuple)?;
        match rel {
            RelationshipDecision::Allowed => {}
            RelationshipDecision::Denied { reason } => {
                return Ok(outcome(
                    deny(DenialReason::Relationship, reason),
                    Risk::R0,
                    None,
                    &self.config.policy_version,
                ));
            }
        }

        // 2. Contextual policy.
        let policy_input = PolicyInput::new(
            request.tenant_id.clone(),
            input.actor.clone(),
            CapabilityClass::Command,
            Risk::R0,
            AuthenticationStrength::SingleFactor,
            TrustLevel::Unverified,
            "action",
            request.target_id.as_str(),
        )
        .map_err(|_| PolicyError::validation("invalid policy input"))?;
        let policy_decision = self.policy.evaluate(&policy_input)?;
        if !policy_decision.allowed {
            return Ok(outcome(
                deny(
                    DenialReason::Policy,
                    format!("policy denied: {}", policy_decision.reason),
                ),
                Risk::R0,
                Some(policy_decision),
                &self.config.policy_version,
            ));
        }

        // 3. Risk classification from the deterministic floor.
        let risk_input = RiskAssessmentInput::new(
            input.capability,
            input.reversal,
            input.touches_secret,
            input
                .approval
                .as_ref()
                .map(|a| a.strength)
                .unwrap_or(AuthenticationStrength::SingleFactor),
        );
        let risk = deterministic_risk_floor(&risk_input).unwrap_or(Risk::R0);

        // 4. Approval requirement for R3/R4 (SPEC-005 behavior 4).
        let needs_approval = risk_at_least(risk, self.config.approval.risk_threshold);
        if needs_approval {
            let ok = match &input.approval {
                Some(a) => {
                    a.decision == ApprovalDecision::Approved
                        && a.approves(&request.action_digest, now)
                        && !is_model_approval(a.approval_class)
                        && a.strength >= AuthenticationStrength::MultiFactor
                }
                None => false,
            };
            if !ok {
                return Ok(outcome(
                    deny(
                        DenialReason::MissingApproval,
                        "R3/R4 action requires a valid human approval assertion",
                    ),
                    risk,
                    Some(policy_decision),
                    &self.config.policy_version,
                ));
            }
        }

        // 5. Capability grant: usable, actor-bound, target-bound, scope.
        let grant = match &input.grant {
            Some(g) => g,
            None => {
                return Ok(outcome(
                    deny(
                        DenialReason::NoCapability,
                        "no capability grant covers the request",
                    ),
                    risk,
                    Some(policy_decision),
                    &self.config.policy_version,
                ));
            }
        };
        if !grant.is_usable_at(now)
            || grant.actor != input.actor.principal_id
            || grant.target_id != request.target_id
            || !grant
                .scope
                .split(',')
                .any(|s| request.action.starts_with(s.trim()))
        {
            return Ok(outcome(
                deny(
                    DenialReason::NoCapability,
                    "capability grant does not cover the request",
                ),
                risk,
                Some(policy_decision),
                &self.config.policy_version,
            ));
        }

        // 6. Allow.
        Ok(outcome(
            ActionDecision::Allowed {
                grant: grant.clone(),
            },
            risk,
            Some(policy_decision),
            &self.config.policy_version,
        ))
    }
}

impl<RA, CE> ActionGateway for DeterministicGateway<RA, CE>
where
    RA: RelationshipAuthorizer,
    CE: ContextPolicyEngine,
{
    /// The port entry point; the engine requires explicit
    /// `DecisionInput` (including the authenticated actor and current
    /// time), so the port method returns a fail-closed internal error
    /// unless the adapter uses `evaluate_input`.
    fn evaluate(&self, _request: &ActionRequest) -> Result<ActionDecision, PolicyError> {
        Err(PolicyError::internal(
            "gateway requires DecisionInput; use evaluate_input",
        ))
    }
}

/// The result of a gateway evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayOutcome {
    /// The final decision.
    pub decision: ActionDecision,
    /// Risk class assigned to the action.
    pub risk: Risk,
    /// Contextual policy decision that contributed (redacted).
    pub policy_decision: Option<PolicyDecision>,
    /// Policy version claimed by the gateway.
    pub policy_version: String,
}

/// Gateway construction/evaluation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayError {
    /// Policy version was empty/whitespace.
    EmptyPolicyVersion,
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::EmptyPolicyVersion => "policy version must not be empty",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for GatewayError {}

/// Whether an approval class is a model/agent approval (SPEC-005
/// behavior 4: R4 never accepts model approval).
fn is_model_approval(class: ApprovalClass) -> bool {
    matches!(class, ApprovalClass::Policy | ApprovalClass::None)
}

fn deny(reason: DenialReason, message: impl Into<String>) -> ActionDecision {
    ActionDecision::Denied {
        reason,
        message: message.into(),
    }
}

fn outcome(
    decision: ActionDecision,
    risk: Risk,
    policy_decision: Option<PolicyDecision>,
    policy_version: &str,
) -> GatewayOutcome {
    GatewayOutcome {
        decision,
        risk,
        policy_decision,
        policy_version: policy_version.to_string(),
    }
}
