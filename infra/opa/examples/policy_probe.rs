//! EP-008 M4 gateway probe: real ActionGateway + OPA composition.
//!
//! Wires the REAL deterministic M2 gateway (`DeterministicGateway` in
//! crates/nexus-action-gateway) to the REAL OPA adapter
//! (`OpaAuthorizer`) against a REAL ephemeral OPA container. The
//! Python failure suite builds and invokes this binary to prove
//! directive G ordering: relationship allow -> policy deny -> STOP at
//! POLICY; relationship allow -> policy allow -> continue to risk
//! floor; and OPA unavailable -> policy provider failure -> no
//! risk/approval/capability stage can manufacture ALLOW.
//!
//! Input (stdin, JSON): `{ "base_url": "...", "policy_version": "...",
//! "relationship": "ALLOW"|"DENY"|"FAIL", "context": {...}, "request":
//! {...}, "actor": {...}, "capability": "COMMAND", "reversal": "NONE",
//! "touches_secret": false, "grant": {...}|null, "approval": {...}|null,
//! "now_unix_s": 0 }`.
//!
//! Output (stdout, JSON): `{ "decision": "ALLOWED"|"DENIED"|"ERROR",
//! "reason": "...", "risk": "R0", "policy_version": "..." }`.

use std::io::Read;

use nexus_action_gateway::engine::{
    DecisionInput, DeterministicGateway, GatewayConfig, GatewayOutcome,
};
use nexus_auth::AuthenticationStrength;
use nexus_domain::{
    ApprovalClass, CapabilityClass, CorrelationId, NexusId, PrincipalType, Reversal, TenantId,
};
use nexus_identity::{Principal, TrustLevel};
use nexus_opa::mapping::{OpaAuthorizer, OpaConfig, OpaContext};
use nexus_opa::telemetry::RecordingSink;
use nexus_policy::approval::{ApprovalAssertion, ApprovalDecision};
use nexus_policy::capability::CapabilityGrant;
use nexus_policy::error::{PolicyError, PolicyErrorCode};
use nexus_policy::gateway::{ActionDecision, ActionRequest};
use nexus_policy::relationship::{RelationshipAuthorizer, RelationshipDecision, RelationshipTuple};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ProbeInput {
    base_url: String,
    policy_version: String,
    relationship: String,
    context: ProbeContext,
    request: ProbeRequest,
    actor: ProbeActor,
    capability: String,
    reversal: String,
    touches_secret: bool,
    grant: Option<ProbeGrant>,
    approval: Option<ProbeApproval>,
    now_unix_s: i64,
}

#[derive(Debug, Deserialize)]
struct ProbeContext {
    location: Option<String>,
    network_trust: Option<String>,
    maintenance: Option<bool>,
    emergency: Option<bool>,
    device_state: Option<String>,
    sensitivity: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeRequest {
    request_id: String,
    correlation: String,
    tenant_id: String,
    action_digest: String,
    action: String,
    target_id: String,
    requested_at_unix_s: i64,
}

#[derive(Debug, Deserialize)]
struct ProbeActor {
    principal_id: String,
    principal_type: String,
    tenant_id: String,
}

#[derive(Debug, Deserialize)]
struct ProbeGrant {
    grant_id: String,
    tenant_id: String,
    capability: String,
    actor: String,
    target_id: String,
    scope: String,
    issued_at_unix_s: i64,
    expires_at_unix_s: i64,
}

#[derive(Debug, Deserialize)]
struct ProbeApproval {
    assertion_id: String,
    correlation: String,
    action_digest: String,
    approver: String,
    approval_class: String,
    strength: String,
    decision: String,
    issued_at_unix_s: i64,
    expires_at_unix_s: i64,
}

#[derive(Debug, Serialize)]
struct ProbeOutput {
    decision: String,
    reason: Option<String>,
    risk: Option<String>,
    policy_version: Option<String>,
    error: Option<String>,
}

fn parse<T: std::str::FromStr>(s: &str, what: &str) -> Result<T, String> {
    s.parse::<T>().map_err(|_| format!("invalid {what}: {s}"))
}

/// Relationship port driven by the probe input (test harness only;
/// the REAL relationship provider is OpenFGA, proven in M3 and in the
/// ordering tests below via a real gateway path).
#[derive(Clone)]
struct ProbeRelationships {
    mode: String,
}

impl RelationshipAuthorizer for ProbeRelationships {
    fn check(&self, _tuple: &RelationshipTuple) -> Result<RelationshipDecision, PolicyError> {
        match self.mode.as_str() {
            "ALLOW" => Ok(RelationshipDecision::Allowed),
            "DENY" => Ok(RelationshipDecision::Denied {
                reason: "no tuple".to_string(),
            }),
            _ => Err(PolicyError::new(
                PolicyErrorCode::ExternalProvider,
                "relationship provider unavailable",
                None,
            )),
        }
    }
}

fn run(input: ProbeInput) -> Result<ProbeOutput, String> {
    // REAL OPA adapter against the real container.
    let mut ctx = OpaContext::new(
        input.context.location.clone(),
        input.context.network_trust.clone(),
        input.context.maintenance,
        input.context.emergency,
        input.context.device_state.clone(),
        input.context.sensitivity.clone(),
    );
    ctx.device_state = input.context.device_state.clone();
    let config = OpaConfig::new(&input.base_url, &input.policy_version)
        .map_err(|e| e.to_string())?
        .with_context(ctx)
        .with_correlation(
            CorrelationId::new(&input.request.correlation)
                .map_err(|e| format!("invalid correlation: {e}"))?,
        );
    let sink = RecordingSink::default();
    let policy = OpaAuthorizer::with_sink(config, sink.clone());

    // Real M2 gateway with the probe relationship port.
    let gw_config = GatewayConfig::new(&input.policy_version).map_err(|e| e.to_string())?;
    let gateway = DeterministicGateway::new(
        gw_config,
        ProbeRelationships {
            mode: input.relationship,
        },
        policy,
    );

    let request = ActionRequest::new(
        NexusId::new(&input.request.request_id).map_err(|e| format!("invalid request id: {e}"))?,
        CorrelationId::new(&input.request.correlation)
            .map_err(|e| format!("invalid correlation: {e}"))?,
        TenantId::new(&input.request.tenant_id).map_err(|e| format!("invalid tenant: {e}"))?,
        &input.request.action_digest,
        &input.request.action,
        NexusId::new(&input.request.target_id).map_err(|e| format!("invalid target: {e}"))?,
        input.request.requested_at_unix_s,
    )
    .map_err(|e| e.to_string())?;

    let actor = Principal::new(
        NexusId::new(&input.actor.principal_id).map_err(|e| format!("invalid actor: {e}"))?,
        parse::<PrincipalType>(&input.actor.principal_type, "principal_type")?,
        TenantId::new(&input.actor.tenant_id).map_err(|e| format!("invalid actor tenant: {e}"))?,
    );

    let capability = parse::<CapabilityClass>(&input.capability, "capability")?;
    let reversal = parse::<Reversal>(&input.reversal, "reversal")?;

    let grant = match &input.grant {
        Some(g) => Some(
            CapabilityGrant::new(
                NexusId::new(&g.grant_id).map_err(|e| format!("invalid grant id: {e}"))?,
                TenantId::new(&g.tenant_id).map_err(|e| format!("invalid grant tenant: {e}"))?,
                parse::<CapabilityClass>(&g.capability, "grant.capability")?,
                NexusId::new(&g.actor).map_err(|e| format!("invalid grant actor: {e}"))?,
                NexusId::new(&g.target_id).map_err(|e| format!("invalid grant target: {e}"))?,
                &g.scope,
                g.issued_at_unix_s,
                g.expires_at_unix_s,
            )
            .map_err(|e| e.to_string())?,
        ),
        None => None,
    };

    let approval = match &input.approval {
        Some(a) => Some(
            ApprovalAssertion::new(
                NexusId::new(&a.assertion_id).map_err(|e| format!("invalid assertion id: {e}"))?,
                CorrelationId::new(&a.correlation)
                    .map_err(|e| format!("invalid assertion correlation: {e}"))?,
                &a.action_digest,
                NexusId::new(&a.approver).map_err(|e| format!("invalid approver: {e}"))?,
                parse::<ApprovalClass>(&a.approval_class, "approval_class")?,
                parse::<AuthenticationStrength>(&a.strength, "strength")?,
                serde_json::from_str::<ApprovalDecision>(&format!("\"{}\"", a.decision))
                    .map_err(|_| format!("invalid decision: {}", a.decision))?,
                a.issued_at_unix_s,
                a.expires_at_unix_s,
            )
            .map_err(|e| e.to_string())?,
        ),
        None => None,
    };

    let decision_input = DecisionInput::new(
        request,
        actor,
        capability,
        reversal,
        input.touches_secret,
        grant,
        approval,
        input.now_unix_s,
    );

    match gateway.evaluate_input(&decision_input) {
        Ok(GatewayOutcome {
            decision,
            risk,
            policy_version,
            ..
        }) => {
            let (decision, reason) = match decision {
                ActionDecision::Allowed { .. } => ("ALLOWED".to_string(), None),
                ActionDecision::Denied { reason: r, message } => {
                    ("DENIED".to_string(), Some(format!("{r}: {message}")))
                }
            };
            Ok(ProbeOutput {
                decision,
                reason,
                risk: Some(risk.as_str().to_string()),
                policy_version: Some(policy_version),
                error: None,
            })
        }
        Err(err) => Ok(ProbeOutput {
            decision: "ERROR".to_string(),
            reason: Some(err.to_string()),
            risk: None,
            policy_version: None,
            error: Some(err.to_string()),
        }),
    }
}

fn main() {
    let mut buf = String::new();
    if std::io::stdin().read_to_string(&mut buf).is_err() {
        eprintln!("failed to read stdin");
        std::process::exit(2);
    }
    let input: ProbeInput = match serde_json::from_str(&buf) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("invalid input JSON: {e}");
            std::process::exit(2);
        }
    };
    match run(input) {
        Ok(out) => println!("{}", serde_json::to_string(&out).unwrap()),
        Err(msg) => {
            let out = ProbeOutput {
                decision: "ERROR".to_string(),
                reason: Some(msg.clone()),
                policy_version: None,
                risk: None,
                error: Some(msg),
            };
            println!("{}", serde_json::to_string(&out).unwrap());
        }
    }
}

#[allow(dead_code)]
fn _unused(_: TrustLevel) {}
