//! EP-014 M3 integration proof: REAL DeepSeek V4 Flash reflex
//! transport across the boundary (SPEC-009).
//!
//! `DeepSeekReflexTransport` is production code wrapping EP-013's real
//! `OpenAiCompatibleTransport` (pinned ureq HTTP client). This suite
//! proves contract behavior across a controlled provider sandbox
//! (authorized by EP-014 M3 CONTENT item 3 and TESTING.md integration
//! layer): real HTTP request bytes, response normalization to the
//! canonical `NexusControlObject`, typed SPEC-006 error classification,
//! and the full `DeepSeekFlashProvider` -> `DeepSeekReflexTransport` ->
//! HTTP path with validated output continuing.
//!
//! The sandbox is a protocol simulator for the OpenAI-compatible chat
//! completions surface. It never certifies the DeepSeek commercial API;
//! it proves the reflex transport boundary.

use nexus_model_gateway::model::PromptSegmentPart;
use nexus_model_gateway::vocabulary::EffortTier;
use nexus_reflex::{
    CacheLedger, DeepSeekFlashProvider, DeepSeekReflexTransport, EffortInput, EffortPolicy,
    NexusControlObjectValidator, PromptSegmentCatalog, ReflexProvider, ReflexRequest,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// A controlled provider sandbox speaking the OpenAI-compatible
/// chat completions protocol. It records the request it received and
/// serves a scripted response.
struct ProviderSandbox {
    port: u16,
    shutdown: mpsc::Sender<()>,
    handle: Option<thread::JoinHandle<()>>,
}

#[derive(Clone)]
struct SandboxResponse {
    body: String,
    status: u16,
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

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.port)
    }
}

impl Drop for ProviderSandbox {
    fn drop(&mut self) {
        let _ = self.shutdown.send(());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn handle_connection(mut stream: TcpStream, response: SandboxResponse) -> std::io::Result<()> {
    // Read the request head (enough for the body to be drained).
    let mut buf = [0u8; 8192];
    let _ = stream.read(&mut buf)?;
    let status = if response.status == 200 {
        "200 OK"
    } else {
        "500 Internal Server Error"
    };
    let body = response.body;
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    stream.flush()?;
    Ok(())
}

fn ok_response(content: &str, cache_hit: u64) -> SandboxResponse {
    // Serialize via serde_json so the control-object content is properly
    // JSON-escaped inside the message content field.
    let body = serde_json::json!({
        "id": "chatcmpl-1",
        "model": "deepseek-v4-flash",
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 20,
            "prompt_cache_hit_tokens": cache_hit,
        },
        "choices": [{
            "message": {
                "role": "assistant",
                "content": content,
            }
        }],
    });
    SandboxResponse {
        status: 200,
        body: body.to_string(),
    }
}

fn malformed_response() -> SandboxResponse {
    SandboxResponse {
        status: 200,
        body: r#"{"id":"chatcmpl-2","model":"deepseek-v4-flash","choices":[{"message":{"role":"assistant","content":"x"}}]}"#
            .to_string(),
    }
}

fn error_response() -> SandboxResponse {
    SandboxResponse {
        status: 500,
        body: r#"{"error":{"message":"boom"}}"#.to_string(),
    }
}

fn canonical_segments() -> Vec<PromptSegmentPart> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/prompts/reflex");
    let catalog = PromptSegmentCatalog::from_canonical_dir(&dir).unwrap();
    catalog
        .ordered()
        .into_iter()
        .map(|s| PromptSegmentPart {
            segment: s.segment,
            content: s.content,
        })
        .collect()
}

fn reflex_request(tier: EffortTier, cacheable: bool) -> ReflexRequest {
    ReflexRequest {
        request_id: "r-m3-1".into(),
        correlation_id: "c-m3-1".into(),
        causation_id: None,
        tenant_id: "t-1".into(),
        principal_id: "p-1".into(),
        effort_input: EffortInput::new(tier),
        segments: canonical_segments(),
        cacheable,
        budget_ref: None,
        schema_version: "1.0.0".into(),
    }
}

fn provider_with_sandbox(sandbox: &ProviderSandbox) -> DeepSeekFlashProvider {
    let transport = DeepSeekReflexTransport::new(
        DeepSeekReflexTransport::deepseek_manifest(sandbox.url()),
        Some("test-credential".into()),
    )
    .unwrap();
    DeepSeekFlashProvider::new(
        "deepseek-v4-flash",
        EffortPolicy::new(),
        NexusControlObjectValidator::new("1.0.0"),
        nexus_model_gateway::health::ProviderHealth::new(
            "deepseek-v4-flash",
            nexus_model_gateway::vocabulary::ProviderHealthState::Healthy,
            None,
            "sandbox",
            "probe",
        ),
    )
    .with_transport(Box::new(transport))
}

#[test]
fn ep014_integration_real_transport_normalizes_control_object() {
    // Full allow path: DeepSeekFlashProvider -> real HTTP sandbox ->
    // canonical NexusControlObject -> validated and returned.
    let sandbox = ProviderSandbox::spawn(vec![ok_response(
        r#"{"intent":"contacts.query","route":"REFLEX","risk":"R1","privacy":"PERSONAL","ambiguity":0.2,"approval_required":false,"executable_instruction":true,"confidence":0.9,"required_capabilities":["contacts.query"],"entities":{}}"#,
        95,
    )]);
    let mut provider = provider_with_sandbox(&sandbox);
    let decision = provider
        .reflex(&reflex_request(EffortTier::High, true))
        .unwrap();
    assert_eq!(decision.class.to_string(), "MODEL");
    assert_eq!(decision.control_object.provider, "deepseek-v4-flash");
    assert_eq!(decision.control_object.control["intent"], "contacts.query");
    assert_eq!(decision.control_object.usage.prompt_tokens, 100);
    assert_eq!(decision.control_object.usage.cache_hit_prompt_tokens, 95);
}

#[test]
fn ep014_integration_real_transport_rejects_malformed_provider_response() {
    // Malformed provider response (missing usage) fails closed with a
    // typed VALIDATION error; no control object continues.
    let sandbox = ProviderSandbox::spawn(vec![malformed_response()]);
    let mut provider = provider_with_sandbox(&sandbox);
    let err = provider
        .reflex(&reflex_request(EffortTier::High, true))
        .unwrap_err();
    assert_eq!(err.code.to_string(), "VALIDATION");
}

#[test]
fn ep014_integration_real_transport_classifies_provider_error() {
    // HTTP 500 from the provider surfaces as EXTERNAL_PROVIDER.
    let sandbox = ProviderSandbox::spawn(vec![error_response()]);
    let mut provider = provider_with_sandbox(&sandbox);
    let err = provider
        .reflex(&reflex_request(EffortTier::High, true))
        .unwrap_err();
    assert_eq!(err.code.to_string(), "EXTERNAL_PROVIDER");
}

#[test]
fn ep014_integration_real_transport_classifies_unreachable() {
    // A real connection to a closed port classifies as UNAVAILABLE.
    let sandbox = ProviderSandbox::spawn(vec![]);
    let mut provider = provider_with_sandbox(&sandbox);
    let port = sandbox.port;
    drop(sandbox); // close the listener; the port is now refused
    let req = ReflexRequest {
        request_id: "r-m3-2".into(),
        correlation_id: "c-m3-2".into(),
        causation_id: None,
        tenant_id: "t-1".into(),
        principal_id: "p-1".into(),
        effort_input: EffortInput::new(EffortTier::High),
        segments: canonical_segments(),
        cacheable: true,
        budget_ref: None,
        schema_version: "1.0.0".into(),
    };
    let err = provider.reflex(&req).unwrap_err();
    assert_eq!(err.code.to_string(), "UNAVAILABLE");
    let _ = port;
}

#[test]
fn ep014_integration_deterministic_task_bypasses_real_transport() {
    // Even with a real transport configured, a deterministic request
    // never touches the network: the model is bypassed.
    let sandbox = ProviderSandbox::spawn(vec![]);
    let mut provider = provider_with_sandbox(&sandbox);
    let decision = provider
        .reflex(&reflex_request(EffortTier::Deterministic, true))
        .unwrap();
    assert_eq!(decision.class.to_string(), "DETERMINISTIC");
    assert_eq!(decision.control_object.usage.prompt_tokens, 0);
}

#[test]
fn ep014_integration_cache_ledger_records_real_usage() {
    let sandbox = ProviderSandbox::spawn(vec![
        ok_response(
            r#"{"intent":"contacts.query","route":"REFLEX","risk":"R1","privacy":"PERSONAL","ambiguity":0.2,"approval_required":false,"executable_instruction":true,"confidence":0.9,"required_capabilities":["contacts.query"],"entities":{}}"#,
            98,
        ),
        ok_response(
            r#"{"intent":"contacts.query","route":"REFLEX","risk":"R1","privacy":"PERSONAL","ambiguity":0.2,"approval_required":false,"executable_instruction":true,"confidence":0.9,"required_capabilities":["contacts.query"],"entities":{}}"#,
            98,
        ),
    ]);
    let mut provider = provider_with_sandbox(&sandbox);
    let mut ledger = CacheLedger::new(8);
    for _ in 0..2 {
        let decision = provider
            .reflex(&reflex_request(EffortTier::High, true))
            .unwrap();
        ledger.record(
            decision.control_object.usage.prompt_tokens,
            decision.control_object.usage.cache_hit_prompt_tokens,
        );
    }
    // 196/200 hit ratio >= 0.97 target (98 hits of 100 prompt tokens).
    assert!(ledger.meets_cache_target());
}
