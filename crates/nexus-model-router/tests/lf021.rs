//! LF-021 model-provider-failover live-fire proof (EP-015 M5; ADR-022).
//!
//! The production failover surface (`DeterministicModelRouter::route_with_failover` with
//! `ProviderFailoverPolicy`, config-driven from the canonical `config/models/router/policy.json`
//! failover section) is exercised through the REAL EP-014 `DeepSeekFlashProvider` and the REAL
//! `DeepSeekReflexTransport` (pinned ureq) against real controlled HTTP endpoints speaking the
//! OpenAI-compatible chat completions surface.
//!
//! Certified stages (LF-021):
//! - Primary baseline: router selects the primary route; the real primary provider resolves a valid NexusControlObject.
//! - Primary attempted BEFORE failover; the failure is a real typed transport failure (connection-refused -> UNAVAILABLE, silent peer -> TIMEOUT/UNAVAILABLE).
//! - The real router observes the failure and the failover policy decides; the proof never calls the secondary directly.
//! - Trace/correlation id preserved across every stage.
//! - Budgets carry forward (never reset); bounded attempts.
//! - Same canonical NexusControlObject validation for every result; malformed/contract-invalid responses fail closed.
//! - Security policy dominates availability (prohibited secondary is never used).
//! - Secondary failure fails closed; no fabricated control object.
//! - Redacted audit chain through the real RouteAuditRecord/AuditSink.
//!
//! External provider certification boundary: no external DeepSeek
//! account is required by this node and no DeepSeek credential is
//! present in the environment; the transport is exercised against real
//! controlled HTTP endpoints. External DeepSeek/secondary vendor
//! certification: NOT ASSERTED.

use nexus_domain::vocabulary::{Privacy, Risk, Route};
use nexus_model_gateway::model::PromptSegmentPart;
use nexus_model_gateway::vocabulary::EffortTier;
use nexus_model_router::microbrain::DisabledMicrobrain;
use nexus_model_router::{
    AuditSink, DeterministicModelRouter, FailoverStage, ProviderFailureClass, RouteAuditRecord,
    RouterStrategyClass, RoutingDecisionClass,
};
use nexus_reflex::{
    DeepSeekFlashProvider, DeepSeekReflexTransport, EffortInput, EffortPolicy,
    NexusControlObjectValidator, PromptSegmentCatalog, ReflexRequest,
};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Controlled provider endpoint speaking the OpenAI-compatible chat
/// completions protocol (TESTING.md integration layer; never certifies
/// the DeepSeek commercial API).
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
        correlation_id: "c-lf021".into(),
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

fn provider_with_sandbox(provider_id: &str, sandbox: &ProviderSandbox) -> DeepSeekFlashProvider {
    let transport = DeepSeekReflexTransport::new(
        DeepSeekReflexTransport::deepseek_manifest(sandbox.url()),
        Some("test-credential".into()),
    )
    .unwrap();
    DeepSeekFlashProvider::new(
        provider_id,
        EffortPolicy::new(),
        NexusControlObjectValidator::new("1.0.0"),
        nexus_model_gateway::health::ProviderHealth::new(
            provider_id,
            nexus_model_gateway::vocabulary::ProviderHealthState::Healthy,
            None,
            "sandbox",
            "probe",
        ),
    )
    .with_transport(Box::new(transport))
}

fn features(
    complexity: f64,
    risk: Risk,
    privacy: Privacy,
    cost: f64,
    latency_ms: u64,
    budget: Option<u64>,
) -> nexus_model_router::RoutingFeatures {
    nexus_model_router::RoutingFeatures::new(
        "contacts.query",
        complexity,
        privacy,
        risk,
        Some("contacts.query".into()),
        cost,
        latency_ms,
        false,
        0.99,
        0.95,
        true,
        budget,
    )
}

/// Capture sink: the REAL AuditSink contract; records the real
/// RouteAuditRecord chain.
#[derive(Debug)]
struct CaptureSink(Arc<Mutex<Vec<RouteAuditRecord>>>);

impl AuditSink for CaptureSink {
    fn record(&mut self, record: &RouteAuditRecord) {
        self.0.lock().unwrap().push(record.clone());
    }
}

fn canonical_control() -> &'static str {
    r#"{"intent":"contacts.query","route":"REFLEX","risk":"R1","privacy":"PERSONAL","ambiguity":0.2,"approval_required":false,"executable_instruction":true,"confidence":0.9,"required_capabilities":["contacts.query"],"entities":{}}"#
}

fn audit_stages(records: &[RouteAuditRecord]) -> Vec<FailoverStage> {
    records.iter().filter_map(|r| r.stage).collect::<Vec<_>>()
}

#[test]
fn ep015_livefire_primary_baseline_success() {
    // D: known-good baseline before any failover. Router -> primary ->
    // real transport -> valid NexusControlObject -> canonical validator
    // (provider-owned) PASS.
    let sandbox = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut router = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_audit_sink(Box::new(CaptureSink(shared.clone())));
    let mut primary = provider_with_sandbox("deepseek-v4-flash", &sandbox);
    let mut secondary = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox);

    let outcome = router
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.5, Risk::R2, Privacy::Personal, 0.3, 500, Some(1000)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary,
            &mut secondary,
            Route::Reflex,
        )
        .unwrap();

    assert!(outcome.failure.is_none(), "baseline must succeed");
    let reflex = outcome.final_reflex.as_ref().expect("validated object");
    assert_eq!(reflex.class.to_string(), "MODEL");
    assert_eq!(reflex.control_object.schema_version, "1.0.0");
    assert_eq!(reflex.control_object.control["intent"], "contacts.query");
    assert_eq!(reflex.control_object.usage.prompt_tokens, 100);
    // Model output has no authority (N): no authorization/grant fields.
    let control = serde_json::to_string(&reflex.control_object.control).unwrap();
    assert!(!control.contains("authorization"));
    assert!(!control.contains("grant"));
    // Bounded: one attempt on the primary only.
    assert_eq!(outcome.provider_attempts, 1);
    assert_eq!(outcome.max_provider_attempts, 2);
    assert_eq!(outcome.remaining_budget, Some(900));
    assert_eq!(outcome.remaining_latency_ms, 400);
    assert!(outcome.secondary_received_budget.is_none());

    // Audit chain: decision record (stage None) then PRIMARY_SELECTED,
    // PRIMARY_ATTEMPTED, ROUTE_COMPLETED.
    let records = shared.lock().unwrap();
    assert!(records[0].stage.is_none(), "decision record first");
    assert_eq!(
        audit_stages(&records),
        vec![
            FailoverStage::PrimarySelected,
            FailoverStage::PrimaryAttempted,
            FailoverStage::RouteCompleted,
        ]
    );
    for rec in records.iter() {
        assert_eq!(rec.correlation_id, "c-lf021");
    }
}

#[test]
fn ep015_livefire_failover_unavailable_to_secondary() {
    // E/F/G/H/I/O: the REAL primary transport fails with a typed
    // UNAVAILABLE (connection refused - the primary endpoint is
    // stopped), the router's failover policy selects the configured
    // secondary, the secondary's real transport returns a valid
    // NexusControlObject, schema/trace/budget are preserved, and the
    // real audit chain records every stage.
    let sandbox_a = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let sandbox_b = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut router = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_audit_sink(Box::new(CaptureSink(shared.clone())));
    let mut primary = provider_with_sandbox("deepseek-v4-flash", &sandbox_a);
    let mut secondary = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox_b);
    drop(sandbox_a); // real mechanism: primary endpoint stopped

    let outcome = router
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.5, Risk::R2, Privacy::Personal, 0.3, 500, Some(1000)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary,
            &mut secondary,
            Route::Reflex,
        )
        .unwrap();

    // Failover succeeded through the secondary.
    assert!(outcome.failure.is_none(), "failover must succeed");
    let reflex = outcome.final_reflex.as_ref().expect("validated object");
    assert_eq!(reflex.class.to_string(), "MODEL");
    assert_eq!(reflex.control_object.schema_version, "1.0.0");
    assert_eq!(reflex.control_object.control["intent"], "contacts.query");
    assert_eq!(reflex.control_object.usage.prompt_tokens, 100);

    // Bounded attempts: exactly 2 (primary + secondary), never more.
    assert_eq!(outcome.provider_attempts, 2);
    assert_eq!(outcome.max_provider_attempts, 2);

    // H: budgets carry forward, never reset. Primary consumed 100
    // milli-cost + 100 ms; the secondary received the remaining budget.
    assert_eq!(outcome.secondary_received_budget, Some(900));
    assert!(outcome.secondary_received_budget < Some(1000));
    assert_eq!(outcome.remaining_budget, Some(800));
    assert_eq!(outcome.remaining_latency_ms, 300);

    // G: one logical trace id across every stage.
    assert_eq!(outcome.correlation_id, "c-lf021");
    assert_eq!(reflex.correlation_id, "c-lf021");

    // O: ordered, typed, redacted audit chain.
    let records = shared.lock().unwrap();
    assert!(records[0].stage.is_none(), "decision record first");
    assert_eq!(
        audit_stages(&records),
        vec![
            FailoverStage::PrimarySelected,
            FailoverStage::PrimaryAttempted,
            FailoverStage::PrimaryFailed,
            FailoverStage::FailoverEligible,
            FailoverStage::SecondarySelected,
            FailoverStage::SecondaryAttempted,
            FailoverStage::SecondaryValidated,
            FailoverStage::RouteCompleted,
        ]
    );
    let primary_failed = records
        .iter()
        .find(|r| r.stage == Some(FailoverStage::PrimaryFailed))
        .expect("primary failed stage");
    assert_eq!(
        primary_failed.failure_class,
        Some(ProviderFailureClass::Unavailable)
    );
    let completed = records
        .iter()
        .find(|r| r.stage == Some(FailoverStage::RouteCompleted))
        .expect("route completed stage");
    assert_eq!(
        completed.provider_id.as_deref(),
        Some("deepseek-v4-flash-secondary")
    );
    for rec in records.iter() {
        assert_eq!(rec.correlation_id, "c-lf021");
    }
    // P: audit records are redacted - no credentials, no prompt bodies,
    // no feature domain.
    let serialized = serde_json::to_string(&*records).unwrap();
    assert!(!serialized.contains("test-credential"));
    assert!(!serialized.contains("contacts.query"));
}

#[test]
fn ep015_livefire_failover_timeout_to_secondary() {
    // J: a REAL read-timeout (silent peer) is a failover-eligible typed
    // failure (TIMEOUT or UNAVAILABLE per the locked policy), and the
    // failover chain completes through the secondary.
    let sandbox_a = ProviderSandbox::spawn_silent();
    let sandbox_b = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut router = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_audit_sink(Box::new(CaptureSink(shared.clone())));
    let mut primary = provider_with_sandbox("deepseek-v4-flash", &sandbox_a);
    let mut secondary = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox_b);

    let outcome = router
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.5, Risk::R2, Privacy::Personal, 0.3, 500, Some(1000)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary,
            &mut secondary,
            Route::Reflex,
        )
        .unwrap();

    assert!(outcome.failure.is_none(), "timeout failover must succeed");
    assert_eq!(outcome.provider_attempts, 2);
    assert!(outcome.final_reflex.is_some());
    let records = shared.lock().unwrap();
    let primary_failed = records
        .iter()
        .find(|r| r.stage == Some(FailoverStage::PrimaryFailed))
        .expect("primary failed stage");
    let class = primary_failed.failure_class.expect("typed class");
    assert!(
        class == ProviderFailureClass::Timeout || class == ProviderFailureClass::Unavailable,
        "expected TIMEOUT or UNAVAILABLE, got {class:?}"
    );
    assert!(audit_stages(&records).contains(&FailoverStage::SecondaryValidated));
}

#[test]
fn ep015_livefire_validation_failure_does_not_failover() {
    // J: a contract-invalid provider payload is NOT failover-eligible.
    // The real transport types a raw malformed envelope as
    // EXTERNAL_PROVIDER (envelope parse failure) and the provider types
    // a schema-invalid control object as VALIDATION; both classify to
    // non-failover classes (Contract/External). The router fails closed
    // and never attempts the secondary.
    let sandbox_a = ProviderSandbox::spawn(vec![SandboxResponse {
        body: "not-json".into(),
        status: 200,
    }]);
    let sandbox_b = ProviderSandbox::spawn(vec![]); // no response queued
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut router = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_audit_sink(Box::new(CaptureSink(shared.clone())));
    let mut primary = provider_with_sandbox("deepseek-v4-flash", &sandbox_a);
    let mut secondary = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox_b);

    let outcome = router
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.5, Risk::R2, Privacy::Personal, 0.3, 500, Some(1000)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary,
            &mut secondary,
            Route::Reflex,
        )
        .unwrap();

    let failure = outcome.failure.as_ref().expect("fail closed");
    assert!(
        matches!(
            failure.class,
            ProviderFailureClass::Contract | ProviderFailureClass::External
        ),
        "expected a non-failover class, got {:?}",
        failure.class
    );
    assert!(outcome.final_reflex.is_none());
    assert_eq!(outcome.provider_attempts, 1);
    let records = shared.lock().unwrap();
    let stages = audit_stages(&records);
    assert!(
        !stages.contains(&FailoverStage::SecondarySelected),
        "secondary must never be selected for a contract failure"
    );
    assert!(stages.contains(&FailoverStage::FailedClosed));
}

#[test]
fn ep015_livefire_secondary_failure_fails_closed() {
    // K/I: after a failover-eligible primary failure, a failing
    // secondary fails closed - typed failure, bounded attempts (2),
    // never a fabricated control object. Two real secondary failure
    // mechanisms: endpoint stopped (UNAVAILABLE) and malformed payload
    // (contract-invalid).
    let sandbox_a = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let sandbox_b = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut router = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_audit_sink(Box::new(CaptureSink(shared.clone())));
    let mut primary = provider_with_sandbox("deepseek-v4-flash", &sandbox_a);
    let mut secondary = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox_b);
    drop(sandbox_a);
    drop(sandbox_b); // both real endpoints stopped

    let outcome = router
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.5, Risk::R2, Privacy::Personal, 0.3, 500, Some(1000)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary,
            &mut secondary,
            Route::Reflex,
        )
        .unwrap();

    let failure = outcome.failure.as_ref().expect("typed failure");
    assert_eq!(failure.class, ProviderFailureClass::Unavailable);
    assert!(outcome.final_reflex.is_none(), "never fabricate an object");
    assert_eq!(outcome.provider_attempts, 2, "bounded attempts");
    let records = shared.lock().unwrap();
    assert!(audit_stages(&records).contains(&FailoverStage::FailedClosed));

    // Secondary returns a contract-invalid payload: fail closed with a
    // non-failover typed class; never a fabricated object.
    let sandbox_c = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let sandbox_d = ProviderSandbox::spawn(vec![SandboxResponse {
        body: "not-json".into(),
        status: 200,
    }]);
    let shared2 = Arc::new(Mutex::new(Vec::new()));
    let mut router2 = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_audit_sink(Box::new(CaptureSink(shared2.clone())));
    let mut primary2 = provider_with_sandbox("deepseek-v4-flash", &sandbox_c);
    let mut secondary2 = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox_d);
    drop(sandbox_c); // primary unavailable -> failover-eligible

    let outcome2 = router2
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.5, Risk::R2, Privacy::Personal, 0.3, 500, Some(1000)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary2,
            &mut secondary2,
            Route::Reflex,
        )
        .unwrap();

    let failure2 = outcome2.failure.as_ref().expect("typed failure");
    assert!(
        matches!(
            failure2.class,
            ProviderFailureClass::Contract | ProviderFailureClass::External
        ),
        "expected a non-failover class, got {:?}",
        failure2.class
    );
    assert!(outcome2.final_reflex.is_none(), "never fabricate an object");
    assert_eq!(outcome2.provider_attempts, 2, "bounded attempts");
    let records2 = shared2.lock().unwrap();
    assert!(audit_stages(&records2).contains(&FailoverStage::FailedClosed));
}

#[test]
fn ep015_livefire_security_override_blocks_prohibited_secondary() {
    // L: availability never outranks security. SECRET privacy prohibits
    // a CHEAP_API secondary; the router must fail closed instead of
    // using the prohibited provider.
    let sandbox_a = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let sandbox_b = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut router = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_audit_sink(Box::new(CaptureSink(shared.clone())));
    let mut primary = provider_with_sandbox("deepseek-v4-flash", &sandbox_a);
    let mut secondary = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox_b);
    drop(sandbox_a);

    let outcome = router
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.5, Risk::R1, Privacy::Secret, 0.3, 500, Some(1000)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary,
            &mut secondary,
            Route::CheapApi, // prohibited tier for SECRET privacy
        )
        .unwrap();

    let failure = outcome.failure.as_ref().expect("fail closed");
    assert_eq!(failure.class, ProviderFailureClass::SecurityDenied);
    assert!(outcome.final_reflex.is_none());
    assert_eq!(outcome.provider_attempts, 1);
    let records = shared.lock().unwrap();
    let stages = audit_stages(&records);
    assert!(
        !stages.contains(&FailoverStage::SecondarySelected),
        "prohibited secondary must never be used"
    );
    assert!(stages.contains(&FailoverStage::FailedClosed));
}

#[test]
fn ep015_livefire_budget_exhausted_blocks_failover() {
    // H/J: the failed primary attempt consumes per-attempt budget; when
    // the remaining budget cannot afford the secondary, the router
    // fails closed (budgets are never reset for failover).
    let sandbox_a = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let sandbox_b = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut router = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_audit_sink(Box::new(CaptureSink(shared.clone())));
    let mut primary = provider_with_sandbox("deepseek-v4-flash", &sandbox_a);
    let mut secondary = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox_b);
    drop(sandbox_a);

    // cost 0.05 (50 milli) routes within budget 120; the primary
    // attempt consumes 100 milli, leaving 20 - less than the 100 milli
    // a secondary attempt costs. Fail closed; never a fresh budget.
    let outcome = router
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.2, Risk::R1, Privacy::Personal, 0.05, 500, Some(120)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary,
            &mut secondary,
            Route::Reflex,
        )
        .unwrap();

    let failure = outcome.failure.as_ref().expect("fail closed");
    assert_eq!(failure.class, ProviderFailureClass::BudgetExhausted);
    assert!(outcome.final_reflex.is_none());
    assert_eq!(outcome.provider_attempts, 1);
    assert_eq!(outcome.secondary_received_budget, Some(20));
    assert_eq!(outcome.remaining_budget, Some(20));
    let records = shared.lock().unwrap();
    assert!(
        !audit_stages(&records).contains(&FailoverStage::SecondaryAttempted),
        "secondary must not be attempted without budget"
    );
}

#[test]
fn ep015_livefire_policy_denial_and_disabled_microbrain() {
    // J/M: a deterministic policy denial (R4) never triggers a provider
    // attempt, and the DisabledMicrobrain is never selected as a route
    // strategy (SPEC-025 shadow-before-promotion; V1 stays disabled).

    // R4 denial: no provider attempt at all.
    let sandbox_a = ProviderSandbox::spawn(vec![]);
    let sandbox_b = ProviderSandbox::spawn(vec![]);
    let shared = Arc::new(Mutex::new(Vec::new()));
    let mut router = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_audit_sink(Box::new(CaptureSink(shared.clone())));
    let mut primary = provider_with_sandbox("deepseek-v4-flash", &sandbox_a);
    let mut secondary = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox_b);

    let outcome = router
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.5, Risk::R4, Privacy::Personal, 0.3, 500, Some(1000)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary,
            &mut secondary,
            Route::Reflex,
        )
        .unwrap();

    assert_eq!(outcome.decision.class, RoutingDecisionClass::Rejected);
    let failure = outcome.failure.as_ref().expect("typed denial");
    assert_eq!(failure.class, ProviderFailureClass::Rejected);
    assert_eq!(outcome.provider_attempts, 0);
    let records = shared.lock().unwrap();
    assert!(
        !audit_stages(&records).contains(&FailoverStage::PrimaryAttempted),
        "policy denial must never attempt a provider"
    );

    // DisabledMicrobrain: routing stays deterministic policy; the
    // microbrain is NOT selected.
    let sandbox_ok = ProviderSandbox::spawn(vec![ok_response(canonical_control(), 95)]);
    let shared2 = Arc::new(Mutex::new(Vec::new()));
    let mut router2 = DeterministicModelRouter::new()
        .with_provider_availability("deepseek-v4-flash", 0.99)
        .with_microbrain(Box::new(DisabledMicrobrain))
        .with_audit_sink(Box::new(CaptureSink(shared2.clone())));
    let mut primary2 = provider_with_sandbox("deepseek-v4-flash", &sandbox_ok);
    let mut secondary2 = provider_with_sandbox("deepseek-v4-flash-secondary", &sandbox_ok);

    let outcome2 = router2
        .route_with_failover(
            "r-lf021",
            "c-lf021",
            &features(0.5, Risk::R2, Privacy::Personal, 0.3, 500, Some(1000)),
            &reflex_request(EffortTier::High, "r-lf021"),
            &mut primary2,
            &mut secondary2,
            Route::Reflex,
        )
        .unwrap();

    assert!(outcome2.failure.is_none());
    assert_eq!(outcome2.decision.strategy, RouterStrategyClass::Policy);
    let records2 = shared2.lock().unwrap();
    assert!(
        records2
            .iter()
            .all(|r| r.strategy == RouterStrategyClass::Policy),
        "disabled microbrain must never be selected"
    );
}
