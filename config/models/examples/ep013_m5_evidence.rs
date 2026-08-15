//! EP-013 M5 live-fire evidence binary (SPEC-009; EP-013).
//!
//! Composes the REAL `nexus-model-transport` HTTP adapter and the
//! REAL `nexus-bifrost` gateway against a controlled provider
//! sandbox and writes deterministic machine-readable evidence to
//! `.agent/state/evidence/ep013-m5/ep013-m5-live-fire.json`.
//!
//! Evidence is redacted: correlation ids, provider ids, typed codes,
//! and fingerprints only. No credential, prompt, or model output
//! content is persisted.
//!
//! EP-013 owns no standalone LF registry entry; its live-fire proof
//! is this composed real-dependency proof (ExecPlan Section 9).

use nexus_bifrost::{BifrostConfig, gateway::BifrostGatewayBuilder};
use nexus_model_gateway::{
    ModelBudget, ModelGateway,
    budget::{BudgetDecision, BudgetLedger},
    model::{ModelRequest, PromptSegment, PromptSegmentPart, UsageReport},
    vocabulary::EffortTier,
};
use nexus_model_transport::{
    OpenAiCompatibleTransportBuilder,
    config::{ManifestProviderKind, ProviderManifest},
};
use serde_json::json;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const EVIDENCE_DIR: &str = ".agent/state/evidence/ep013-m5";
const EVIDENCE_FILE: &str = ".agent/state/evidence/ep013-m5/ep013-m5-live-fire.json";

#[derive(Clone)]
struct SandboxResponse {
    status: u16,
    body: String,
}

struct ProviderSandbox {
    port: u16,
    shutdown: mpsc::Sender<()>,
    handle: Option<thread::JoinHandle<()>>,
}

impl ProviderSandbox {
    fn spawn(responses: Vec<SandboxResponse>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind sandbox");
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel::<()>();
        let handle = thread::spawn(move || {
            let mut next = 0usize;
            let mut running = true;
            while running {
                listener.set_nonblocking(true).ok();
                match listener.accept() {
                    Ok((stream, _)) => {
                        let response = responses.get(next).cloned();
                        next += 1;
                        if let Some(response) = response {
                            let _ = handle_connection(stream, response);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                        match rx.try_recv() {
                            Ok(()) | Err(mpsc::TryRecvError::Disconnected) => running = false,
                            Err(mpsc::TryRecvError::Empty) => {}
                        }
                    }
                    Err(_) => {}
                }
            }
        });
        Self {
            port: addr.port(),
            shutdown: tx,
            handle: Some(handle),
        }
    }

    fn stop(mut self) {
        let _ = self.shutdown.send(());
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for ProviderSandbox {
    fn drop(&mut self) {
        let _ = self.shutdown.send(());
    }
}

fn handle_connection(mut stream: TcpStream, response: SandboxResponse) -> std::io::Result<()> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                if buf.len() > 1 << 16 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let reason = if response.status == 200 {
        "OK"
    } else {
        "ERROR"
    };
    let body = response.body.as_bytes();
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.status,
        reason,
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn chat_completion_json(model: &str) -> String {
    format!(
        r#"{{"id":"chatcmpl-1","model":"{model}","usage":{{"prompt_tokens":12,"completion_tokens":6,"prompt_cache_hit_tokens":10}},"choices":[{{"message":{{"role":"assistant","content":"reply"}}}}]}}"#
    )
}

fn request(correlation: &str) -> ModelRequest {
    ModelRequest {
        request_id: format!("r-{correlation}"),
        correlation_id: correlation.to_string(),
        causation_id: None,
        tenant_id: "018f0f6f-9c1e-7b6e-8000-000000000001".into(),
        principal_id: "018f0f6f-9c1e-7b6e-8000-00000000000a".into(),
        effort_tier: EffortTier::Deterministic,
        segments: vec![PromptSegmentPart {
            segment: PromptSegment::Constitution,
            content: "constitution".into(),
        }],
        budget_ref: None,
        schema_version: "1.0".into(),
    }
}

fn transport_manifest(port: u16) -> ProviderManifest {
    ProviderManifest::new(
        "bifrost",
        ManifestProviderKind::Bifrost,
        format!("http://127.0.0.1:{port}/v1"),
        "0.1.0",
        "Apache-2.0",
        "COMPONENT_REGISTRY.yaml id=bifrost",
        "ModelGateway contract; Bifrost preferred but replaceable",
    )
}

struct LedgerBudget {
    ledger: BudgetLedger,
}

impl ModelBudget for LedgerBudget {
    fn check(
        &self,
        _request: &ModelRequest,
    ) -> Result<BudgetDecision, nexus_model_gateway::ModelGatewayError> {
        Ok(self.ledger.check(18))
    }

    fn record(
        &mut self,
        _request: &ModelRequest,
        usage: &UsageReport,
    ) -> Result<(), nexus_model_gateway::ModelGatewayError> {
        self.ledger.record(usage.total_tokens())
    }
}

fn main() {
    let _ = std::fs::create_dir_all(EVIDENCE_DIR);
    let correlation = "ep013-m5-0001";

    // ALLOW PATH: real transport + real gateway + real budget over a
    // controlled provider sandbox.
    let sandbox = ProviderSandbox::spawn(vec![SandboxResponse {
        status: 200,
        body: chat_completion_json("bifrost"),
    }]);
    let manifest = transport_manifest(sandbox.port);
    let provider = OpenAiCompatibleTransportBuilder::new(manifest)
        .with_credential("test-key")
        .build();
    let config = BifrostConfig::new("gw-m5", "bifrost", vec![]);
    let mut gateway = BifrostGatewayBuilder::new(config, nexus_bifrost::gateway::SystemTimeSource)
        .with_provider(Box::new(provider))
        .with_budget(LedgerBudget {
            ledger: BudgetLedger::new("b-m5", 1000),
        })
        .build()
        .unwrap();

    let allow_result = gateway.generate(&request(correlation));
    let allow = match &allow_result {
        Ok(resp) => json!({
            "decision": "ALLOWED",
            "provider": resp.control_object.provider,
            "model": resp.control_object.model,
            "prompt_tokens": resp.control_object.usage.prompt_tokens,
            "completion_tokens": resp.control_object.usage.completion_tokens,
            "cache_hit_prompt_tokens": resp.control_object.usage.cache_hit_prompt_tokens,
            "schema_version": resp.control_object.schema_version,
        }),
        Err(e) => json!({
            "decision": "DENIED",
            "code": e.code.as_str(),
            "message": e.message,
        }),
    };
    sandbox.stop();

    // DENIED PATH: budget exhausted (17 < 18 cost) fails closed.
    let sandbox2 = ProviderSandbox::spawn(vec![]);
    let manifest2 = transport_manifest(sandbox2.port);
    let provider2 = OpenAiCompatibleTransportBuilder::new(manifest2)
        .with_credential("test-key")
        .build();
    let config2 = BifrostConfig::new("gw-m5-denied", "bifrost", vec![]);
    let mut gateway2 =
        BifrostGatewayBuilder::new(config2, nexus_bifrost::gateway::SystemTimeSource)
            .with_provider(Box::new(provider2))
            .with_budget(LedgerBudget {
                ledger: BudgetLedger::new("b-m5-denied", 17),
            })
            .build()
            .unwrap();
    let denied_result = gateway2.generate(&request("ep013-m5-0002"));
    let denied = match &denied_result {
        Ok(_) => json!({"decision": "ALLOWED"}),
        Err(e) => json!({
            "decision": "DENIED",
            "code": e.code.as_str(),
            "message": e.message,
        }),
    };
    sandbox2.stop();

    // DENIED PATH: rate limit exceeded.
    let sandbox3 = ProviderSandbox::spawn(vec![
        SandboxResponse {
            status: 200,
            body: chat_completion_json("bifrost"),
        },
        SandboxResponse {
            status: 200,
            body: chat_completion_json("bifrost"),
        },
    ]);
    let manifest3 = transport_manifest(sandbox3.port);
    let provider3 = OpenAiCompatibleTransportBuilder::new(manifest3)
        .with_credential("test-key")
        .build();
    let config3 = BifrostConfig::new("gw-m5-rl", "bifrost", vec![])
        .with_rate_limit(nexus_bifrost::config::RateLimitPolicy::new(2, 60));
    let mut gateway3 =
        BifrostGatewayBuilder::new(config3, nexus_bifrost::gateway::SystemTimeSource)
            .with_provider(Box::new(provider3))
            .with_budget(LedgerBudget {
                ledger: BudgetLedger::new("b-m5-rl", 1000),
            })
            .build()
            .unwrap();
    let _ = gateway3.generate(&request("ep013-m5-0003"));
    let _ = gateway3.generate(&request("ep013-m5-0003"));
    let rl_result = gateway3.generate(&request("ep013-m5-0003"));
    let rate_limited = match &rl_result {
        Ok(_) => json!({"decision": "ALLOWED"}),
        Err(e) => json!({
            "decision": "DENIED",
            "code": e.code.as_str(),
            "message": e.message,
        }),
    };
    sandbox3.stop();

    let evidence = json!({
        "node": "EP-013",
        "milestone": "M5",
        "title": "model gateway live-fire composed proof",
        "correlation_id": correlation,
        "components": {
            "nexus-model-transport": {
                "transport": "OpenAiCompatibleTransport",
                "http_client": "ureq 2.12.1 (default-features=false, json)",
                "protocol": "OpenAI-compatible chat completions",
                "provider_manifest": "config/models/providers/providers.json",
                "preferred": "bifrost",
                "fallback": "deepseek-v4-flash"
            },
            "nexus-bifrost": {
                "gateway": "BifrostGateway",
                "router": "BifrostRouter (Bifrost preferred when healthy+certified)",
                "budget": "BudgetLedger check-before-route",
                "retry": "deterministic backoff",
                "rate_limit": "fixed window",
                "usage_accounting": "record after success"
            },
            "contract": "nexus-model-gateway ModelGateway/ModelProvider/ModelBudget (SPEC-009)"
        },
        "allow_path": allow,
        "denied_budget_exhausted": denied,
        "denied_rate_limited": rate_limited,
        "telemetry": {
            "redacted": true,
            "classes": [
                "ROUTE_SELECTED",
                "BUDGET_DENIED",
                "RATE_LIMITED",
                "PROVIDER_UNAVAILABLE",
                "PROVIDER_TIMEOUT",
                "PROVIDER_ERROR",
                "FALLBACK",
                "RETRY",
                "ALLOWED",
                "DENIED"
            ]
        },
        "authority_boundary": "model output is advisory only; models never grant authority (SPEC-009 behavior 10)",
        "evidence_file": EVIDENCE_FILE
    });

    let text = serde_json::to_string_pretty(&evidence).unwrap();
    std::fs::write(EVIDENCE_FILE, format!("{text}\n")).expect("write evidence");
    println!("EP-013 M5 evidence written: {EVIDENCE_FILE}");
    println!("allow decision: {}", evidence["allow_path"]["decision"]);
    println!(
        "denied budget decision: {}",
        evidence["denied_budget_exhausted"]["decision"]
    );
    println!(
        "denied rate-limit decision: {}",
        evidence["denied_rate_limited"]["decision"]
    );
}
