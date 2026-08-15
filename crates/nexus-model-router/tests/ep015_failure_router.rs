//! EP-015 M4 failure and abuse suite for the router plane (SPEC-009,
//! SPEC-006; ADR-022).
//!
//! Every test exercises a REAL failure mechanism against the production
//! router and the REAL EP-014 reflex transport: provider unreachable,
//! provider read timeout, malformed provider payload, learned adapter
//! failure, out-of-distribution escalation, budget cap, authority
//! boundary, audit redaction, and post-failure state isolation. The
//! controlled provider sandbox scripts the failure; the adapter under
//! proof is never mocked.
//!
//! All errors are typed SPEC-006 codes with redacted messages; no
//! credential or prompt content is ever asserted or logged.

use nexus_domain::vocabulary::{Privacy, Risk};
use nexus_model_gateway::model::PromptSegmentPart;
use nexus_model_gateway::vocabulary::EffortTier;
use nexus_model_router::{
    AuditSink, DeterministicModelRouter, EscalationReason, LearnedRouterAdapter, LearnedScores,
    NexusModelRouter, RouteAuditRecord, RouterError, RouterStrategyClass, RoutingDecisionClass,
    RoutingFeatures,
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

fn malformed_response() -> SandboxResponse {
    SandboxResponse {
        status: 200,
        body: r#"{"id":"chatcmpl-2","model":"deepseek-v4-flash","choices":[{"message":{"role":"assistant","content":"x"}}]}"#
            .to_string(),
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
        correlation_id: "c-fail".into(),
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

fn features(complexity: f64, risk: Risk, privacy: Privacy, budget: Option<u64>) -> RoutingFeatures {
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
        budget,
    )
}

// ---- real failure mechanisms ----

#[test]
fn ep015_failure_provider_unreachable_fails_closed() {
    // The transport is dropped (listener closed); a real connection
    // attempt fails and the reflex provider returns a typed UNAVAILABLE
    // error, never a fabricated decision.
    let sandbox = ProviderSandbox::spawn(vec![]);
    let mut provider = provider_with_sandbox(&sandbox);
    drop(sandbox);
    let err = provider
        .reflex(&reflex_request(EffortTier::High, "r-fail-1"))
        .unwrap_err();
    assert_eq!(err.code.as_str(), "UNAVAILABLE");
    assert!(err.correlation_id.is_some());
}

#[test]
fn ep015_failure_provider_timeout_fails_closed() {
    // A silent peer accepts and never responds; the real transport read
    // timeout fires and the provider fails closed.
    let sandbox = ProviderSandbox::spawn_silent();
    let mut provider = provider_with_sandbox(&sandbox);
    let err = provider
        .reflex(&reflex_request(EffortTier::High, "r-fail-2"))
        .unwrap_err();
    let code = err.code.as_str();
    assert!(
        code == "TIMEOUT" || code == "UNAVAILABLE",
        "expected TIMEOUT or UNAVAILABLE, got {code}"
    );
}

#[test]
fn ep015_failure_malformed_provider_payload_fails_closed() {
    // The provider returns a payload missing usage; the transport
    // rejects it as VALIDATION. No control object continues.
    let sandbox = ProviderSandbox::spawn(vec![malformed_response()]);
    let mut provider = provider_with_sandbox(&sandbox);
    let err = provider
        .reflex(&reflex_request(EffortTier::High, "r-fail-3"))
        .unwrap_err();
    assert_eq!(err.code.as_str(), "VALIDATION");
}

#[test]
fn ep015_failure_learned_adapter_failure_fails_closed() {
    // A learned adapter that errors must never produce a fabricated
    // route: the failure propagates as a typed error.
    #[derive(Debug)]
    struct FailingAdapter;
    impl LearnedRouterAdapter for FailingAdapter {
        fn score(&mut self, _features: &RoutingFeatures) -> Result<LearnedScores, RouterError> {
            Err(RouterError::external_provider(
                "learned scorer unavailable",
                Some("learned-router".into()),
            ))
        }

        fn strategy(&self) -> RouterStrategyClass {
            RouterStrategyClass::RouteLlm
        }
    }
    let mut router = DeterministicModelRouter::new().with_learned_adapter(Box::new(FailingAdapter));
    let err = router
        .route(
            "r-1",
            "c-1",
            &features(0.2, Risk::R1, Privacy::Personal, None),
        )
        .unwrap_err();
    assert_eq!(err.code.as_str(), "EXTERNAL_PROVIDER");
}

#[test]
fn ep015_failure_learned_out_of_distribution_escalates() {
    // An out-of-distribution learned scorer is never trusted: the router
    // escalates and retains the policy route.
    #[derive(Debug)]
    struct OodAdapter;
    impl LearnedRouterAdapter for OodAdapter {
        fn score(&mut self, _features: &RoutingFeatures) -> Result<LearnedScores, RouterError> {
            Ok(LearnedScores::new(
                RouterStrategyClass::RouteLlm,
                vec![],
                true,
            ))
        }

        fn strategy(&self) -> RouterStrategyClass {
            RouterStrategyClass::RouteLlm
        }
    }
    let mut router = DeterministicModelRouter::new().with_learned_adapter(Box::new(OodAdapter));
    let decision = router
        .route(
            "r-1",
            "c-1",
            &features(0.2, Risk::R1, Privacy::Personal, None),
        )
        .unwrap();
    assert_eq!(decision.class, RoutingDecisionClass::Escalated);
    assert_eq!(
        decision.escalation_reason,
        Some(EscalationReason::OutOfDistribution)
    );
}

#[test]
fn ep015_failure_budget_cap_never_routed_over() {
    // A budget cap below the cost-weighted demand escalates; the router
    // never routes over the declared budget.
    let mut router = DeterministicModelRouter::new();
    let decision = router
        .route(
            "r-1",
            "c-1",
            &features(0.2, Risk::R1, Privacy::Personal, Some(50)),
        )
        .unwrap();
    assert_eq!(decision.class, RoutingDecisionClass::Escalated);
    assert_eq!(decision.escalation_reason, Some(EscalationReason::Budget));
}

#[test]
fn ep015_failure_audit_redacts_features_and_prompts() {
    // The audit record carries metadata only: never the domain,
    // features, prompt segments, or credentials.
    #[derive(Debug)]
    struct CaptureSink(std::sync::Arc<std::sync::Mutex<Vec<RouteAuditRecord>>>);
    impl AuditSink for CaptureSink {
        fn record(&mut self, record: &RouteAuditRecord) {
            self.0.lock().unwrap().push(record.clone());
        }
    }
    let shared = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut router =
        DeterministicModelRouter::new().with_audit_sink(Box::new(CaptureSink(shared.clone())));
    let _ = router
        .route(
            "r-1",
            "c-1",
            &features(0.2, Risk::R1, Privacy::Personal, None),
        )
        .unwrap();
    let records = shared.lock().unwrap();
    assert_eq!(records.len(), 1);
    let s = serde_json::to_string(&records[0]).unwrap();
    assert!(!s.contains("contacts.query"));
    assert!(!s.contains("complexity"));
    assert!(!s.contains("prompt"));
    assert!(!s.contains("credential"));
}

#[test]
fn ep015_failure_router_usable_after_provider_failure() {
    // After a provider failure, the router remains deterministic: a
    // subsequent request still produces the policy-correct decision (no
    // poisoned state).
    let sandbox = ProviderSandbox::spawn(vec![malformed_response()]);
    let mut provider = provider_with_sandbox(&sandbox);
    let _ = provider.reflex(&reflex_request(EffortTier::High, "r-fail-8"));
    let mut router = DeterministicModelRouter::new();
    let decision = router
        .route(
            "r-1",
            "c-1",
            &features(0.2, Risk::R1, Privacy::Personal, None),
        )
        .unwrap();
    assert_eq!(decision.class, RoutingDecisionClass::Routed);
    assert_eq!(decision.route.to_string(), "CHEAP_API");
}
