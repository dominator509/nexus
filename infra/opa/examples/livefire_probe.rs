//! EP-008 M5 live-fire probe: the FULL authorization chain as ONE
//! system.
//!
//! Wires the REAL deterministic M2 gateway (`DeterministicGateway` in
//! crates/nexus-action-gateway) to the REAL OpenFGA relationship
//! adapter (`OpenFgaAuthorizer`, M3) and the REAL OPA contextual policy
//! adapter (`OpaAuthorizer`, M4) against REAL ephemeral containers.
//! This is the crown-jewel composition proof: relationship -> contextual
//! policy -> risk floor -> R3/R4 human approval -> capability grant ->
//! action gateway authorization -> canonical ActionReceipt and
//! VerificationPlan, with deterministic fail-closed behavior at every
//! boundary.
//!
//! No fake RelationshipAuthorizer and no fake ContextPolicyEngine are
//! used anywhere in this probe. `model_recommendation` and
//! `presented_receipt` are accepted on the input surface ONLY to prove
//! directives M and D: model output never grants authority, and an
//! ActionReceipt is audit evidence, not a reusable bearer credential.
//! The gateway never consults either field.
//!
//! Input (stdin, JSON): `{ "openfga_base_url": "...",
//! "openfga_store_id": "...", "openfga_model_id": "...",
//! "opa_base_url": "...", "opa_policy_version": "nexus-policy-v1",
//! "opa_context": {...}, "request": {...}, "actor": {...},
//! "capability": "ADMINISTRATIVE", "reversal": "NONE",
//! "touches_secret": false, "grant": {...}|null, "approval": {...}|null,
//! "now_unix_s": N, "receipt_id": "...", "model_recommendation":
//! "ALLOW"|null, "presented_receipt": {...}|null }`.
//!
//! Output (stdout, JSON): `{ "decision": "ALLOWED"|"DENIED"|"ERROR",
//! "reason": "...", "risk": "R3", "policy_version": "...", "stages":
//! [...], "receipt": {...}|null, "verification_plan": {...}|null,
//! "relationship_event": {...}|null, "policy_event": {...}|null,
//! "error": "..." }`.

use std::io::Read;

use nexus_action_gateway::engine::{
    DecisionInput, DeterministicGateway, GatewayConfig, GatewayOutcome,
};
use nexus_auth::AuthenticationStrength;
use nexus_domain::{
    ApprovalClass, CapabilityClass, CorrelationId, NexusId, PrincipalType, Reversal, Risk, TenantId,
};
use nexus_identity::Principal;
use nexus_opa::mapping::{OpaAuthorizer, OpaConfig, OpaContext};
use nexus_opa::telemetry::RecordingSink as OpaSink;
use nexus_openfga::mapping::{OpenFgaAuthorizer, OpenFgaConfig};
use nexus_openfga::telemetry::RecordingSink as OpenFgaSink;
use nexus_policy::approval::{ApprovalAssertion, ApprovalDecision};
use nexus_policy::capability::CapabilityGrant;
use nexus_policy::gateway::{ActionDecision, ActionRequest, DenialReason};
use nexus_policy::receipt::ActionReceipt;
use nexus_policy::verification::{ExpectedState, VerificationPlan};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct ProbeInput {
    openfga_base_url: String,
    openfga_store_id: String,
    openfga_model_id: String,
    opa_base_url: String,
    opa_policy_version: String,
    opa_context: ProbeContext,
    request: ProbeRequest,
    actor: ProbeActor,
    capability: String,
    reversal: String,
    touches_secret: bool,
    grant: Option<ProbeGrant>,
    approval: Option<ProbeApproval>,
    now_unix_s: i64,
    receipt_id: Option<String>,
    /// Advisory-only (directive M): the gateway has no model input.
    model_recommendation: Option<String>,
    /// Advisory-only (directive D): receipts are audit evidence.
    presented_receipt: Option<serde_json::Value>,
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

/// Redacted provider event for evidence (never raw payloads).
#[derive(Debug, Serialize)]
struct ProviderEvent {
    provider: String,
    allowed: bool,
    fingerprint: String,
    error_class: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProbeOutput {
    decision: String,
    reason: Option<String>,
    risk: Option<String>,
    policy_version: Option<String>,
    stages: Vec<String>,
    receipt: Option<serde_json::Value>,
    verification_plan: Option<serde_json::Value>,
    relationship_event: Option<ProviderEvent>,
    policy_event: Option<ProviderEvent>,
    /// Advisory model recommendation received but NEVER consulted
    /// (directive M: models never grant authority).
    model_recommendation_received: bool,
    /// A presented receipt received but NEVER consulted (directive D:
    /// receipts are audit evidence, not bearer credentials).
    presented_receipt_received: bool,
    error: Option<String>,
}

fn parse<T: std::str::FromStr>(s: &str, what: &str) -> Result<T, String> {
    s.parse::<T>().map_err(|_| format!("invalid {what}: {s}"))
}

fn run(input: ProbeInput) -> Result<ProbeOutput, String> {
    let correlation = CorrelationId::new(&input.request.correlation)
        .map_err(|e| format!("invalid correlation: {e}"))?;

    // REAL OpenFGA relationship adapter (M3), with telemetry recording.
    let openfga_config = OpenFgaConfig::new(
        &input.openfga_base_url,
        &input.openfga_store_id,
        &input.openfga_model_id,
    )
    .map_err(|e| e.to_string())?
    .with_correlation(correlation.clone());
    let openfga_sink = OpenFgaSink::default();
    let relationships = OpenFgaAuthorizer::with_sink(openfga_config, openfga_sink.clone());

    // REAL OPA contextual policy adapter (M4), with telemetry recording.
    let ctx = OpaContext::new(
        input.opa_context.location.clone(),
        input.opa_context.network_trust.clone(),
        input.opa_context.maintenance,
        input.opa_context.emergency,
        input.opa_context.device_state.clone(),
        input.opa_context.sensitivity.clone(),
    );
    let opa_config = OpaConfig::new(&input.opa_base_url, &input.opa_policy_version)
        .map_err(|e| e.to_string())?
        .with_context(ctx)
        .with_correlation(correlation.clone());
    let opa_sink = OpaSink::default();
    let policy = OpaAuthorizer::with_sink(opa_config, opa_sink.clone());

    // REAL deterministic M2 gateway over both REAL adapters.
    let gw_config = GatewayConfig::new(&input.opa_policy_version).map_err(|e| e.to_string())?;
    let gateway = DeterministicGateway::new(gw_config, relationships, policy);

    let request = ActionRequest::new(
        NexusId::new(&input.request.request_id).map_err(|e| format!("invalid request id: {e}"))?,
        correlation.clone(),
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
        request.clone(),
        actor,
        capability,
        reversal,
        input.touches_secret,
        grant.clone(),
        approval.clone(),
        input.now_unix_s,
    );

    let result = gateway.evaluate_input(&decision_input);
    match result {
        Ok(GatewayOutcome {
            decision,
            risk,
            policy_version,
            ..
        }) => {
            let (decision_str, reason) = match &decision {
                ActionDecision::Allowed { .. } => ("ALLOWED".to_string(), None),
                ActionDecision::Denied { reason: r, message } => {
                    ("DENIED".to_string(), Some(format!("{r}: {message}")))
                }
            };

            let stages = derive_stages(&decision, risk);

            // Canonical ActionReceipt (nexus-policy contract) with
            // evidence references only - never secrets or raw provider
            // payloads.
            let receipt = build_receipt(
                &input,
                &request,
                &decision,
                &policy_version,
                &openfga_sink,
                &opa_sink,
                risk,
            )?;

            // Canonical VerificationPlan (deterministic for identical
            // DecisionInput). The expected state records the
            // AUTHORIZATION outcome; EP-008 owns authorization only, so
            // no execution or verification success is claimed
            // (AUTHORIZED != EXECUTED != VERIFIED).
            let verification_plan = if decision.is_allowed() {
                let expected =
                    ExpectedState::new(request.target_id.clone(), "authorization:approved")
                        .map_err(|e| e.to_string())?;
                let plan = VerificationPlan::new(expected, 30, 3).map_err(|e| e.to_string())?;
                Some(serde_json::to_value(&plan).map_err(|e| e.to_string())?)
            } else {
                None
            };

            Ok(ProbeOutput {
                decision: decision_str,
                reason,
                risk: Some(risk.as_str().to_string()),
                policy_version: Some(policy_version),
                stages,
                receipt: Some(serde_json::to_value(&receipt).map_err(|e| e.to_string())?),
                verification_plan,
                relationship_event: relationship_event(&openfga_sink),
                policy_event: policy_event(&opa_sink),
                model_recommendation_received: input.model_recommendation.is_some(),
                presented_receipt_received: input.presented_receipt.is_some(),
                error: None,
            })
        }
        Err(err) => {
            // Typed provider failure: the gateway NEVER returns ALLOW
            // on provider error. Attribute the typed cause by the
            // telemetry error class recorded by each real adapter.
            // Note: OPA's version check (data.nexus.policy_version)
            // fails BEFORE any evaluation event is emitted, so an empty
            // OPA sink with a live relationship check is still an OPA
            // provider failure.
            let rel_events = openfga_sink.events();
            let pol_events = opa_sink.events();
            let rel_err = rel_events.iter().rev().find(|e| e.error_class.is_some());
            let pol_err = pol_events.iter().rev().find(|e| e.error_class.is_some());
            let (provider, cause) = match (rel_err, pol_err) {
                (Some(e), _) => ("OPENFGA", format!("{:?}", e.error_class)),
                (None, Some(e)) => ("OPA", format!("{:?}", e.error_class)),
                (None, None) if pol_events.is_empty() => {
                    ("OPA", "version check failed before evaluation".to_string())
                }
                (None, None) => ("UNKNOWN", "no provider telemetry".to_string()),
            };
            Ok(ProbeOutput {
                decision: "ERROR".to_string(),
                reason: Some(format!("{provider} provider failure ({cause}): {err}")),
                risk: None,
                policy_version: Some(input.opa_policy_version),
                stages: vec![format!("ERROR_{provider}")],
                receipt: None,
                verification_plan: None,
                relationship_event: relationship_event(&openfga_sink),
                policy_event: policy_event(&opa_sink),
                model_recommendation_received: input.model_recommendation.is_some(),
                presented_receipt_received: input.presented_receipt.is_some(),
                error: Some(format!("{provider}: {err}")),
            })
        }
    }
}

/// Observable deterministic stage progression (SPEC-005/SPEC-006 order).
fn derive_stages(decision: &ActionDecision, risk: Risk) -> Vec<String> {
    match decision {
        ActionDecision::Allowed { .. } => vec![
            "RELATIONSHIP_PASS".to_string(),
            "POLICY_PASS".to_string(),
            format!("RISK_{}", risk.as_str()),
            "APPROVAL_PASS".to_string(),
            "CAPABILITY_PASS".to_string(),
            "ALLOWED".to_string(),
        ],
        ActionDecision::Denied {
            reason: DenialReason::Relationship,
            ..
        } => vec!["RELATIONSHIP_DENY".to_string()],
        ActionDecision::Denied {
            reason: DenialReason::Policy,
            ..
        } => vec!["RELATIONSHIP_PASS".to_string(), "POLICY_DENY".to_string()],
        ActionDecision::Denied {
            reason: DenialReason::MissingApproval,
            ..
        } => vec![
            "RELATIONSHIP_PASS".to_string(),
            "POLICY_PASS".to_string(),
            format!("RISK_{}", risk.as_str()),
            "APPROVAL_DENY".to_string(),
        ],
        ActionDecision::Denied {
            reason: DenialReason::NoCapability,
            ..
        } => vec![
            "RELATIONSHIP_PASS".to_string(),
            "POLICY_PASS".to_string(),
            format!("RISK_{}", risk.as_str()),
            "APPROVAL_PASS".to_string(),
            "CAPABILITY_DENY".to_string(),
        ],
        ActionDecision::Denied { reason, .. } => vec![format!("DENIED_{}", reason.as_str())],
    }
}

/// Canonical redacted ActionReceipt. Evidence refs are fingerprints and
/// references only (directive C: no secrets, no raw provider payloads).
fn build_receipt(
    input: &ProbeInput,
    request: &ActionRequest,
    decision: &ActionDecision,
    policy_version: &str,
    openfga_sink: &OpenFgaSink,
    opa_sink: &OpaSink,
    risk: Risk,
) -> Result<ActionReceipt, String> {
    let receipt_id = match &input.receipt_id {
        Some(id) => NexusId::new(id).map_err(|e| format!("invalid receipt id: {e}"))?,
        None => request.request_id.clone(),
    };
    let rel_fp = openfga_sink
        .events()
        .first()
        .map(|e| format!("relationship:{}", e.fingerprint))
        .unwrap_or_else(|| "relationship:not-recorded".to_string());
    let pol_fp = opa_sink
        .events()
        .first()
        .map(|e| format!("policy:{}", e.version))
        .unwrap_or_else(|| "policy:not-recorded".to_string());
    let approval_ref = input
        .approval
        .as_ref()
        .map(|a| format!("approval:{}", a.assertion_id))
        .unwrap_or_else(|| "approval:none".to_string());
    let grant_ref = input
        .grant
        .as_ref()
        .map(|g| format!("grant:{}", g.grant_id))
        .unwrap_or_else(|| "grant:none".to_string());
    let evidence_refs = vec![
        rel_fp,
        pol_fp,
        approval_ref,
        grant_ref,
        format!("risk:{}", risk.as_str()),
        format!("digest:{}", request.action_digest),
    ];
    ActionReceipt::from_decision(
        receipt_id,
        request.correlation.clone(),
        request.request_id.clone(),
        decision,
        policy_version,
        evidence_refs,
        input.now_unix_s,
    )
    .map_err(|e| e.to_string())
}

fn relationship_event(sink: &OpenFgaSink) -> Option<ProviderEvent> {
    sink.events().first().map(|e| ProviderEvent {
        provider: "openfga".to_string(),
        allowed: e.allowed,
        fingerprint: e.fingerprint.clone(),
        error_class: e.error_class.map(|c| format!("{c:?}")),
    })
}

fn policy_event(sink: &OpaSink) -> Option<ProviderEvent> {
    sink.events().first().map(|e| ProviderEvent {
        provider: "opa".to_string(),
        allowed: e.allowed,
        fingerprint: e.fingerprint.clone(),
        error_class: e.error_class.map(|c| format!("{c:?}")),
    })
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
                risk: None,
                policy_version: None,
                stages: vec!["ERROR".to_string()],
                receipt: None,
                verification_plan: None,
                relationship_event: None,
                policy_event: None,
                model_recommendation_received: false,
                presented_receipt_received: false,
                error: Some(msg),
            };
            println!("{}", serde_json::to_string(&out).unwrap());
        }
    }
}
