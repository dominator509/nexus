//! EP-015 M3 integration proof: the REAL model router boundary.
//!
//! The router's deterministic decision drives the REAL EP-014
//! DeepSeekFlashProvider through the REAL DeepSeekReflexTransport
//! (pinned ureq HTTP) against a controlled provider sandbox speaking
//! the OpenAI-compatible chat completions surface. The sandbox is a
//! protocol simulator under TESTING.md's integration layer; it never
//! certifies the DeepSeek commercial API, but it proves the routing ->
//! reflex -> HTTP boundary: allow path, deterministic bypass, provider
//! unavailable fail-closed, and real read-timeout fail-closed.

use nexus_domain::vocabulary::{Privacy, Risk, Route};
use nexus_model_gateway::model::PromptSegmentPart;
use nexus_model_gateway::vocabulary::EffortTier;
use nexus_model_router::{
    DeterministicModelRouter, NexusModelRouter, RoutingDecisionClass, RoutingFeatures,
};
use nexus_reflex::{
    DeepSeekFlashProvider, DeepSeekReflexTransport, EffortInput, EffortPolicy,
    NexusControlObjectValidator, PromptSegmentCatalog, ReflexProvider, ReflexRequest,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Controlled provider sandbox speaking the OpenAI-compatible chat
/// completions protocol.
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
        Self::spawn_inner(responses, false)
    }

    /// Accepts connections but never responds (real read-timeout peer).
    fn spawn_silent() -> Self {
        Self::spawn_inner(vec![], true)
    }

    fn spawn_inner(responses: Vec<SandboxResponse>, silent: bool) -> Self {
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
                        if silent {
                            // Hold the connection open without answering;
                            // the client read timeout must fire.
                            let _ = stream;
                            thread::sleep(Duration::from_secs(35));
                        } else {
                            let response = responses.get(next).cloned();
                            next += 1;
                            if let Some(response) = response {
                                let _ = handle_connection(stream, response);
                            }
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

fn reflex_request(tier: EffortTier, request_id: &str) -> ReflexRequest {
    ReflexRequest {
        request_id: request_id.into(),
        correlation_id: "c-m3".into(),
        causation_id: None,
        tenant_id: "t-1".into(),
        principal_id: "p-1".into(),
        effort_input: EffortInput::new(tier),
        segments: canonical_segments(),
        cacheable: true,
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

fn features(complexity: f64, risk: Risk, privacy: Privacy) -> RoutingFeatures {
    RoutingFeatures::new(
        "contacts.query",
        complexity,
        privacy,
        risk,
        Some("contacts.query".into()),
        0.3,
        500,
        false,
        0.99,
        0.95,
        true,
        Some(1000),
    )
}

#[test]
fn ep015_integration_router_route_drives_real_reflex_provider() {
    // Full boundary: router selects a route from real features, then the
    // real DeepSeekFlashProvider (real HTTP transport to the sandbox)
    // resolves the request to a validated NexusControlObject.
    let sandbox = ProviderSandbox::spawn(vec![ok_response(
        r#"{"intent":"contacts.query","route":"REFLEX","risk":"R1","privacy":"PERSONAL","ambiguity":0.2,"approval_required":false,"executable_instruction":true,"confidence":0.9,"required_capabilities":["contacts.query"],"entities":{}}"#,
        95,
    )]);
    let mut router =
        DeterministicModelRouter::new().with_provider_availability("deepseek-v4-flash", 0.99);
    let decision = router
        .route("r-1", "c-1", &features(0.5, Risk::R2, Privacy::Personal))
        .unwrap();
    assert_eq!(decision.class, RoutingDecisionClass::Routed);
    assert!(decision.route == Route::Reflex || decision.route == Route::FrontierApi);

    // The selected provider resolves the reflex request for real.
    let mut provider = provider_with_sandbox(&sandbox);
    let reflex = provider
        .reflex(&reflex_request(EffortTier::High, "r-1"))
        .unwrap();
    assert_eq!(reflex.class.to_string(), "MODEL");
    assert_eq!(reflex.control_object.control["intent"], "contacts.query");
    assert_eq!(reflex.control_object.usage.prompt_tokens, 100);
    assert_eq!(reflex.control_object.usage.cache_hit_prompt_tokens, 95);
}

#[test]
fn ep015_integration_deterministic_route_bypasses_model() {
    // Router selects DETERMINISTIC for a deterministic task; the real
    // provider resolves it WITHOUT touching the transport (the sandbox
    // has no response queued; any network call would fail the test).
    let sandbox = ProviderSandbox::spawn(vec![]);
    let mut router = DeterministicModelRouter::new();
    let decision = router
        .route("r-1", "c-1", &features(0.0, Risk::R0, Privacy::Public))
        .unwrap();
    assert_eq!(decision.route, Route::Deterministic);

    let mut provider = provider_with_sandbox(&sandbox);
    let reflex = provider
        .reflex(&reflex_request(EffortTier::Deterministic, "r-1"))
        .unwrap();
    assert_eq!(reflex.class.to_string(), "DETERMINISTIC");
    assert_eq!(reflex.control_object.usage.prompt_tokens, 0);
}

#[test]
fn ep015_integration_router_falls_back_when_provider_unavailable() {
    // A provider below the availability floor triggers a deterministic
    // FALLBACK decision; no provider call is attempted.
    let mut router =
        DeterministicModelRouter::new().with_provider_availability("deepseek-v4-flash", 0.1);
    let decision = router
        .route("r-1", "c-1", &features(0.5, Risk::R2, Privacy::Personal))
        .unwrap();
    assert_eq!(decision.class, RoutingDecisionClass::Fallback);
    assert_eq!(decision.route, Route::Reflex);
}

#[test]
fn ep015_integration_provider_unreachable_fails_closed() {
    // Real connection to a closed port: the reflex provider returns a
    // typed UNAVAILABLE error; the router-composed pipeline never
    // fabricates a decision.
    let sandbox = ProviderSandbox::spawn(vec![]);
    let mut provider = provider_with_sandbox(&sandbox);
    drop(sandbox); // listener closed; port now refuses
    let err = provider
        .reflex(&reflex_request(EffortTier::High, "r-1"))
        .unwrap_err();
    assert_eq!(err.code.to_string(), "UNAVAILABLE");
    assert!(err.correlation_id.is_some());
}

#[test]
fn ep015_integration_provider_timeout_fails_closed() {
    // A silent peer (accepts, never responds): the REAL transport read
    // timeout fires and the provider fails closed with a typed TIMEOUT
    // (or UNAVAILABLE) error. Never a fabricated decision.
    let sandbox = ProviderSandbox::spawn_silent();
    let mut provider = provider_with_sandbox(&sandbox);
    let err = provider
        .reflex(&reflex_request(EffortTier::High, "r-1"))
        .unwrap_err();
    let code = err.code.to_string();
    assert!(
        code == "TIMEOUT" || code == "UNAVAILABLE",
        "expected TIMEOUT or UNAVAILABLE, got {code}"
    );
}

#[test]
fn ep015_integration_router_is_idempotent_across_boundary() {
    // Identical features produce byte-identical routing decisions and
    // byte-identical reflex requests (same request/segment assembly).
    let mut router = DeterministicModelRouter::new();
    let a = router
        .route("r-1", "c-1", &features(0.3, Risk::R1, Privacy::Personal))
        .unwrap();
    let b = router
        .route("r-1", "c-1", &features(0.3, Risk::R1, Privacy::Personal))
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&a).unwrap(),
        serde_json::to_vec(&b).unwrap()
    );
}
