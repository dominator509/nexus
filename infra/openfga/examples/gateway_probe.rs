//! EP-008 M3 gateway probe: real ActionGateway composition.
//!
//! Wires the REAL deterministic M2 gateway (`DeterministicGateway` in
//! crates/nexus-action-gateway) to the REAL OpenFGA adapter
//! (`OpenFgaAuthorizer`) against a REAL ephemeral OpenFGA container.
//! The Python integration suite builds and invokes this binary to prove
//! directive E: relationship deny stops the gateway; a valid
//! relationship path continues to the next authorization stage.
//!
//! Input (stdin, JSON): `{ "base_url": "...", "store_id": "...",
//! "model_id": "...", "policy_version": "...", "request": {...},
//! "actor": {"principal_id": "...", "principal_type": "HUMAN",
//! "tenant_id": "..."}, "capability": "COMMAND", "reversal": "NONE",
//! "touches_secret": false, "grant": {...}|null, "approval": {...}|null,
//! "now_unix_s": 0 }`.
//!
//! Output (stdout, JSON): `{ "decision": "ALLOWED"|"DENIED", "reason":
//! "...", "risk": "R0", "policy_version": "..." }` or `{ "error": "..." }`.

use std::io::Read;

use nexus_action_gateway::engine::{
    DecisionInput, DeterministicGateway, GatewayConfig, GatewayOutcome,
};
use nexus_auth::AuthenticationStrength;
use nexus_domain::{
    ApprovalClass, CapabilityClass, CorrelationId, NexusId, PrincipalType, Reversal, TenantId,
};
use nexus_identity::Principal;
use nexus_openfga::mapping::{OpenFgaAuthorizer, OpenFgaConfig};
use nexus_openfga::telemetry::RecordingSink;
use nexus_policy::approval::{ApprovalAssertion, ApprovalDecision};
use nexus_policy::capability::CapabilityGrant;
use nexus_policy::error::PolicyError;
use nexus_policy::gateway::{ActionDecision, ActionRequest};
use nexus_policy::policy::{ContextPolicyEngine, PolicyDecision, PolicyInput};
use serde::{Deserialize, Serialize};

/// Deterministic allow-all contextual policy engine (probe-only; the
/// real contextual engine is OPA in M4). The probe's purpose is the
/// relationship stage - the policy stage must not block the valid path.
#[derive(Debug, Clone, Copy)]
struct AllowPolicy;

impl ContextPolicyEngine for AllowPolicy {
    fn evaluate(&self, _input: &PolicyInput) -> Result<PolicyDecision, PolicyError> {
        Ok(PolicyDecision::allow("probe-allow"))
    }
}

#[derive(Debug, Deserialize)]
struct ProbeInput {
    base_url: String,
    store_id: String,
    model_id: String,
    policy_version: String,
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

fn run(input: ProbeInput) -> Result<ProbeOutput, String> {
    // Adapter against the real container.
    let config = OpenFgaConfig::new(&input.base_url, &input.store_id, &input.model_id)
        .map_err(|e| e.to_string())?
        .with_correlation(
            CorrelationId::new(&input.request.correlation)
                .map_err(|e| format!("invalid correlation: {e}"))?,
        );
    let sink = RecordingSink::default();
    let relationships = OpenFgaAuthorizer::with_sink(config, sink.clone());

    // Real M2 gateway.
    let gw_config = GatewayConfig::new(&input.policy_version).map_err(|e| e.to_string())?;
    let gateway = DeterministicGateway::new(gw_config, relationships, AllowPolicy);

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
