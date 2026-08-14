//! EP-012 M5 composed fabric live-fire evidence writer.
//!
//! Runs the REAL composed gateway (the real MCP engine, the real A2A
//! gateway, and a real hash-bound artifact store) through the full
//! SPEC-003 chain and writes machine-readable evidence under
//! `.agent/state/evidence/ep012-m5/`.
//!
//! The probe is deterministic: identical DecisionInput produces
//! identical evidence, so a committed-state re-verify does not churn
//! the file.
//!
//! This binary is evidence tooling only. It is never an authorization
//! oracle and never grants authority.

use nexus_auth::vocabulary::AuthenticationStrength;
use nexus_domain::{NexusId, PrincipalType, TenantId};
use nexus_gateway::{ComposedGateway, ComposedGatewayConfig};
use nexus_mcp::session::SessionBinding;
use std::path::PathBuf;

const TENANT: &str = "018f0f6f-9c1e-7b6e-8000-000000000001";
const PRINCIPAL: &str = "018f0f6f-9c1e-7b6e-8000-00000000000a";
const REQUEST_ID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01";
const CORRELATION_ID: &str = "corr-0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01";

fn binding() -> SessionBinding {
    SessionBinding {
        principal_id: NexusId::new(PRINCIPAL).unwrap(),
        principal_type: PrincipalType::Human,
        tenant_id: TenantId::new(TENANT).unwrap(),
        authentication_strength: AuthenticationStrength::StepUp,
    }
}

fn main() {
    let mut gateway = ComposedGateway::new(ComposedGatewayConfig::default());
    let outcome = gateway
        .run_probe(
            REQUEST_ID,
            CORRELATION_ID,
            binding(),
            Some(serde_json::json!({"recommendation": "ALLOW"})),
            Some(serde_json::json!({"receipt": "stale"})),
        )
        .expect("crown-jewel composed probe must pass");

    let value = serde_json::to_value(&outcome).expect("outcome serializes");
    let json = serde_json::to_string_pretty(&value).expect("pretty json");
    let md = render_markdown(&outcome);

    let evidence_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.agent/state/evidence/ep012-m5");
    std::fs::create_dir_all(&evidence_dir).expect("create evidence dir");
    std::fs::write(evidence_dir.join("ep012-m5-composed-gateway.json"), &json)
        .expect("write json evidence");
    std::fs::write(evidence_dir.join("EP-012-M5-composed-gateway.md"), &md)
        .expect("write markdown evidence");

    println!("EP-012 M5 composed gateway evidence written");
    println!("stages: {}", outcome.stages.join(" -> "));
    println!("final lifecycle: {}", outcome.final_lifecycle);
    println!("artifact digest: {}", outcome.artifact_digest);
}

fn render_markdown(o: &nexus_gateway::GatewayProbeOutcome) -> String {
    let mut out = String::new();
    out.push_str("# EP-012 M5 - Composed Fabric Gateway Live-Fire\n\n");
    out.push_str("Real engines: nexus-mcp McpEngine + nexus-a2a A2AGatewayImpl + hash-bound artifact store.\n\n");
    out.push_str(&format!("- request_id: `{}`\n", o.request_id));
    out.push_str(&format!("- correlation_id: `{}`\n", o.correlation_id));
    out.push_str(&format!("- principal_id: `{}`\n", o.principal_id));
    out.push_str(&format!("- tenant_id: `{}`\n", o.tenant_id));
    out.push_str(&format!("- mcp_protocol: `{}`\n", o.mcp_protocol));
    out.push_str(&format!("- a2a_protocol: `{}`\n", o.a2a_protocol));
    out.push_str("\n## Canonical ordering\n\n");
    out.push_str(&format!("```\n{}\n```\n\n", o.stages.join("\n")));
    out.push_str(&format!("- tool_count: {}\n", o.tool_count));
    out.push_str(&format!("- called_tool: `{}`\n", o.called_tool));
    out.push_str(&format!(
        "- idempotent_replay_identical: {}\n",
        o.idempotent_replay_identical
    ));
    out.push_str(&format!(
        "- cancelled_never_completes: {}\n",
        o.cancelled_never_completes
    ));
    out.push_str(&format!("- a2a_task_id: `{}`\n", o.a2a_task_id));
    out.push_str(&format!(
        "- stream_states: {}\n",
        o.stream_states.join(", ")
    ));
    out.push_str(&format!("- artifact_digest: `{}`\n", o.artifact_digest));
    out.push_str(&format!("- artifact_attached: {}\n", o.artifact_attached));
    out.push_str(&format!("- final_lifecycle: `{}`\n", o.final_lifecycle));
    out.push_str(&format!(
        "- cross_tenant_denied: {}\n",
        o.cross_tenant_denied
    ));
    out.push_str("\n## Authority boundaries\n\n");
    out.push_str(&format!(
        "- model_recommendation_never_consulted: {}\n",
        o.model_recommendation_never_consulted
    ));
    out.push_str(&format!(
        "- receipt_never_reusable: {}\n",
        o.receipt_never_reusable
    ));
    out.push_str(&format!(
        "- authorization_not_implied: {}\n",
        o.authorization_not_implied
    ));
    out.push_str("\nMCP acceptance != execution authorization (EP-008 owns authorization).\n");
    out.push_str("A2A task identity/tenant scope != capability grant.\n");
    out.push_str("Artifact integrity (hash binding) != execution authority.\n");
    out.push_str("Protocol acceptance != execution permission.\n");
    out.push_str("\n## Verification plan\n\n");
    for v in &o.verification_plan {
        out.push_str(&format!("- {v}\n"));
    }
    out.push('\n');
    out
}
