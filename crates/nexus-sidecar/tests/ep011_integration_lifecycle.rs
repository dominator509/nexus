//! EP-011 M4 integration lifecycle suite (directives B/J/M/P/Q/R/S/T/
//! U/W/O/Y).
//!
//! Real sidecar process + real fixture provider over real loopback
//! HTTP. Covers the allow path, restart, concurrency bounds, slow
//! client/provider, webhook ingress, owned poller, credential canary,
//! observability events, and the authorization boundary.

mod common;

use common::*;

use std::fs;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------
// Directive B: allow path over the real chain
// ---------------------------------------------------------------------

#[test]
fn ep011_integration_sidecar_allow_path_query() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 200);
    assert!(value["output"]["contacts"].is_array());
}

#[test]
fn ep011_integration_sidecar_allow_path_command_and_idempotency() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);

    // First command succeeds.
    let body = command_envelope(
        "fixture.contacts.command",
        serde_json::json!({ "name": "alice" }),
        "op-allow-1",
    );
    let (status, value) = client.post("/v1/command", body, Some("1"));
    assert_eq!(status, 200);
    assert_eq!(value["output"]["id"], "c1");

    // Same idempotency key replays the same result (directive L:
    // in-process retry idempotency PASS).
    let body2 = command_envelope(
        "fixture.contacts.command",
        serde_json::json!({ "name": "alice" }),
        "op-allow-1",
    );
    let (status2, value2) = client.post("/v1/command", body2, Some("1"));
    assert_eq!(status2, 200);
    assert_eq!(value2["output"]["id"], "c1");
}

#[test]
fn ep011_integration_sidecar_health_probe_loopback_only() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let (status, value) = client.get("/v1/fixture/healthz");
    assert_eq!(status, 200);
    assert_eq!(value["status"], "ok");
}

// ---------------------------------------------------------------------
// Directive M: sidecar restart
// ---------------------------------------------------------------------

#[test]
fn ep011_integration_sidecar_restart_rebinds_cleanly() {
    let provider = spawn_provider();

    let sidecar1 = spawn_sidecar(&provider.base, &[]);
    let client1 = Client::new(&sidecar1.base);
    let (status, _) = client1.post(
        "/v1/query",
        query_envelope("fixture.contacts.query", serde_json::json!({})),
        Some("1"),
    );
    assert_eq!(status, 200);
    let port1 = sidecar1.port;
    drop(sidecar1);

    // Restart on a new ephemeral port; the old port must be released.
    let sidecar2 = spawn_sidecar(&provider.base, &[]);
    assert_ne!(sidecar2.port, port1);
    let client2 = Client::new(&sidecar2.base);
    let (status2, value2) = client2.post(
        "/v1/query",
        query_envelope("fixture.contacts.query", serde_json::json!({})),
        Some("1"),
    );
    assert_eq!(status2, 200);
    assert!(value2["output"]["contacts"].is_array());
}

unsafe extern "C" {
    #[link_name = "kill"]
    fn libc_kill(pid: i32, sig: i32) -> i32;
}

const SIGTERM: i32 = 15;

#[test]
fn ep011_integration_sidecar_sigterm_clean_shutdown() {
    let provider = spawn_provider();
    let bin = env!("CARGO_BIN_EXE_nexus-sidecar");
    let mut cmd = Command::new(bin);
    cmd.env("NEXUS_SIDECAR_TENANT", TENANT_A)
        .env("NEXUS_SIDECAR_CONNECTOR", "fixture-connector")
        .env("NEXUS_SIDECAR_CAPABILITIES", "fixture.contacts.query:QUERY")
        .env("NEXUS_PROVIDER_URL", &provider.base)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child: Child = cmd.spawn().expect("spawn sidecar");
    let port = read_port(&mut child);
    let client = Client::new(&format!("http://127.0.0.1:{port}"));
    let (status, _) = client.post(
        "/v1/query",
        query_envelope("fixture.contacts.query", serde_json::json!({})),
        Some("1"),
    );
    assert_eq!(status, 200);

    // SIGTERM -> clean exit 0.
    let pid = child.id() as i32;
    unsafe {
        libc_kill(pid, SIGTERM);
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(Some(_)) = child.try_wait() {
            break;
        }
        if Instant::now() > deadline {
            panic!("sidecar did not exit after SIGTERM");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let status = child.wait().unwrap();
    assert!(status.success(), "SIGTERM shutdown must exit 0");
}

// ---------------------------------------------------------------------
// Directive D: mid-request shutdown is bounded, fail-closed, and
// releases every owned resource
// ---------------------------------------------------------------------

#[test]
fn ep011_integration_sidecar_mid_request_shutdown_bounded() {
    let provider = spawn_provider();
    let bin = env!("CARGO_BIN_EXE_nexus-sidecar");
    let mut cmd = Command::new(bin);
    cmd.env("NEXUS_SIDECAR_TENANT", TENANT_A)
        .env("NEXUS_SIDECAR_CONNECTOR", "fixture-connector")
        .env("NEXUS_SIDECAR_CAPABILITIES", "fixture.contacts.query:QUERY")
        .env("NEXUS_PROVIDER_URL", &provider.base)
        // Long provider timeout so the request stays genuinely
        // in-flight when SIGTERM arrives (provider armed slow below).
        .env("NEXUS_SIDECAR_PROVIDER_TIMEOUT_MS", "60000")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child: Child = cmd.spawn().expect("spawn sidecar");
    let port = read_port(&mut child);
    let base = format!("http://127.0.0.1:{port}");

    // Arm a very slow provider response so dispatch blocks in-flight.
    let arm = Client::new(&provider.base);
    arm.post(
        "/v1/fixture/arm",
        serde_json::json!({"kind": "query_slow", "value": 60}),
        Some("1"),
    );

    // Fire the request on a thread; it must still be in-flight when we
    // shut down (provider is slow; no response can arrive early).
    let base2 = base.clone();
    let in_flight = std::thread::spawn(move || {
        let url = format!("{base2}/v1/query");
        let client = reqwest::blocking::Client::new();
        client
            .post(&url)
            .json(&query_envelope(
                "fixture.contacts.query",
                serde_json::json!({}),
            ))
            .header("x-nexus-protocol-version", "1")
            .send()
    });

    // Wait until the request is provably in-flight: REQUEST_ACCEPTED is
    // emitted after validation and immediately before provider dispatch.
    // Block on the stderr pipe (no artificial sleep).
    {
        use std::io::Read;
        let stderr_pipe = child.stderr.as_mut().unwrap();
        let mut chunk = [0u8; 8192];
        let mut seen = String::new();
        while let Ok(n) = stderr_pipe.read(&mut chunk) {
            if n == 0 {
                break;
            }
            seen.push_str(&String::from_utf8_lossy(&chunk[..n]));
            if seen.contains("REQUEST_ACCEPTED") {
                break;
            }
        }
    }

    // Controlled shutdown while the request is in-flight.
    let started = Instant::now();
    unsafe {
        libc_kill(child.id() as i32, SIGTERM);
    }
    let status = child.wait().unwrap();
    let shutdown_ms = started.elapsed().as_millis();
    assert!(status.success(), "mid-request SIGTERM must exit 0");
    assert!(
        shutdown_ms < 10_000,
        "shutdown must be bounded, took {shutdown_ms}ms"
    );

    // The in-flight client must NOT receive a fabricated success. It
    // observes canonical termination semantics (connection closed by
    // the exiting process; the provider never answered).
    match in_flight.join().unwrap() {
        Ok(resp) => {
            let status = resp.status();
            assert!(
                status.is_server_error() || status.is_client_error(),
                "mid-request shutdown must not yield success, got {status}"
            );
        }
        Err(_) => {
            // Connection terminated: canonical failure/termination.
        }
    }

    // Listener released: the old port can be bound again immediately.
    let rebind = std::net::TcpListener::bind(("127.0.0.1", port));
    assert!(
        rebind.is_ok(),
        "listener must be released after mid-request shutdown"
    );
    // No orphan sidecar: the child was reaped by wait(); the provider
    // is test-owned and reaped by Drop.
    drop(child);
}

// ---------------------------------------------------------------------
// Directive T: concurrency bound / resource pressure
// ---------------------------------------------------------------------

#[test]
fn ep011_integration_sidecar_concurrency_bound_enforced() {
    let provider = spawn_provider();
    // Tight bound so saturation is observable.
    let sidecar = spawn_sidecar(&provider.base, &[("NEXUS_SIDECAR_MAX_CONCURRENCY", "2")]);
    let arm = Client::new(&provider.base);

    // Saturate: arm a slow provider response, then fire concurrent
    // requests. At least one must be accepted; the concurrency bound
    // prevents unbounded growth (no assertion of load capacity).
    arm.post(
        "/v1/fixture/arm",
        serde_json::json!({"kind": "query_slow", "value": 0.4}),
        Some("1"),
    );
    let handles: Vec<_> = (0..6)
        .map(|_| {
            let client = Client::new(&sidecar.base);
            std::thread::spawn(move || {
                client.post(
                    "/v1/query",
                    query_envelope("fixture.contacts.query", serde_json::json!({})),
                    Some("1"),
                )
            })
        })
        .collect();
    let mut ok = 0;
    for h in handles {
        let (status, value) = h.join().unwrap();
        if status == 200 {
            ok += 1;
        } else {
            // Typed overload (429 RATE_LIMIT) or timeout; never a
            // fabricated success.
            assert!(
                status == 429 || status == 504 || status == 502,
                "unexpected status {status}: {value}"
            );
        }
    }
    assert!(ok >= 1, "at least one concurrent request must succeed");
}

// ---------------------------------------------------------------------
// Directive U: slow client / slow provider (distinct phases)
// ---------------------------------------------------------------------

#[test]
fn ep011_integration_sidecar_slow_provider_typed_timeout() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(
        &provider.base,
        &[("NEXUS_SIDECAR_PROVIDER_TIMEOUT_MS", "500")],
    );
    let client = Client::new(&sidecar.base);
    let arm = Client::new(&provider.base);
    arm.post(
        "/v1/fixture/arm",
        serde_json::json!({"kind": "query_slow", "value": 5}),
        Some("1"),
    );
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 504);
    assert_eq!(value["code"], "TIMEOUT");
}

// ---------------------------------------------------------------------
// Directive P/Q: webhook ingress
// ---------------------------------------------------------------------

fn webhook_secret() -> (String, String) {
    // secret bytes -> hex; fingerprint is the configured label.
    let secret = b"webhook-test-secret";
    let secret_hex: String = secret.iter().map(|b| format!("{b:02x}")).collect();
    (secret_hex, "fp-webhook-test".to_string())
}

fn hmac_hex(secret: &[u8], payload: &[u8]) -> String {
    use hmac::KeyInit;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key");
    mac.update(payload);
    let digest = mac.finalize().into_bytes();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn webhook_envelope(
    capability_id: &str,
    provider_event_id: &str,
    provider_event_type: &str,
    signature: Option<&str>,
    key_fingerprint: Option<&str>,
    raw_payload: serde_json::Value,
) -> serde_json::Value {
    let mut input = serde_json::json!({
        "provider_event_id": provider_event_id,
        "provider_event_type": provider_event_type,
        "raw_payload": raw_payload,
    });
    if let Some(sig) = signature {
        input
            .as_object_mut()
            .unwrap()
            .insert("signature".to_string(), serde_json::json!(sig));
    }
    if let Some(fp) = key_fingerprint {
        input
            .as_object_mut()
            .unwrap()
            .insert("key_fingerprint".to_string(), serde_json::json!(fp));
    }
    serde_json::json!({
        "protocol_version": "1",
        "correlation_id": CORRELATION_ID,
        "request_id": REQUEST_ID,
        "tenant_id": TENANT_A,
        "connector_id": "fixture-connector",
        "capability_id": capability_id,
        "operation": "WEBHOOK",
        "transport": "WEBHOOK",
        "schema_version": "1.0",
        "input": input,
    })
}

fn spawn_sidecar_with_webhook(provider_base: &str) -> SidecarProc {
    let (secret_hex, fp) = webhook_secret();
    spawn_sidecar(
        provider_base,
        &[
            ("NEXUS_SIDECAR_WEBHOOK_SECRET_HEX", &secret_hex),
            ("NEXUS_SIDECAR_WEBHOOK_FINGERPRINT", &fp),
        ],
    )
}

#[test]
fn ep011_integration_sidecar_webhook_valid_signature_accepted() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar_with_webhook(&provider.base);
    let client = Client::new(&sidecar.base);
    let (secret_hex, _fp) = webhook_secret();
    let secret = hex_decode(&secret_hex);
    let payload = serde_json::json!({
        "provider_event_id": "evt-1",
        "provider_event_type": "invoice.created",
        "raw_payload": { "amount": 100 },
    })
    .to_string();
    let sig = hmac_hex(&secret, payload.as_bytes());
    let body = webhook_envelope(
        "fixture.audit.changefeed",
        "evt-1",
        "invoice.created",
        Some(&sig),
        Some("fp-webhook-test"),
        serde_json::json!({ "amount": 100 }),
    );
    let (status, value) = client.post("/v1/webhook/normalize", body, Some("1"));
    assert_eq!(status, 200);
    assert_eq!(value["verification"], "VALID");
    assert_eq!(value["executable"], false);
    assert_eq!(value["event"]["event_type"], "invoice.created");
}

#[test]
fn ep011_failure_sidecar_webhook_invalid_signature_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar_with_webhook(&provider.base);
    let client = Client::new(&sidecar.base);
    let body = webhook_envelope(
        "fixture.audit.changefeed",
        "evt-2",
        "invoice.created",
        Some("deadbeef"),
        Some("fp-webhook-test"),
        serde_json::json!({}),
    );
    let (status, value) = client.post("/v1/webhook/normalize", body, Some("1"));
    assert_eq!(status, 401);
    assert_eq!(value["code"], "VERIFICATION");
}

#[test]
fn ep011_failure_sidecar_webhook_missing_signature_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar_with_webhook(&provider.base);
    let client = Client::new(&sidecar.base);
    let body = webhook_envelope(
        "fixture.audit.changefeed",
        "evt-3",
        "invoice.created",
        None,
        Some("fp-webhook-test"),
        serde_json::json!({}),
    );
    let (status, value) = client.post("/v1/webhook/normalize", body, Some("1"));
    assert_eq!(status, 401);
    assert_eq!(value["code"], "VERIFICATION");
}

#[test]
fn ep011_failure_sidecar_webhook_wrong_fingerprint_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar_with_webhook(&provider.base);
    let client = Client::new(&sidecar.base);
    let (secret_hex, _fp) = webhook_secret();
    let secret = hex_decode(&secret_hex);
    let payload = serde_json::json!({ "provider_event_id": "evt-4" }).to_string();
    let sig = hmac_hex(&secret, payload.as_bytes());
    let body = webhook_envelope(
        "fixture.audit.changefeed",
        "evt-4",
        "invoice.created",
        Some(&sig),
        Some("fp-other"),
        serde_json::json!({}),
    );
    let (status, value) = client.post("/v1/webhook/normalize", body, Some("1"));
    assert_eq!(status, 401);
    assert_eq!(value["code"], "VERIFICATION");
}

#[test]
fn ep011_failure_sidecar_webhook_replay_rejected() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar_with_webhook(&provider.base);
    let client = Client::new(&sidecar.base);
    let (secret_hex, _fp) = webhook_secret();
    let secret = hex_decode(&secret_hex);
    let payload = serde_json::json!({
        "provider_event_id": "evt-5",
        "provider_event_type": "invoice.created",
        "raw_payload": {},
    })
    .to_string();
    let sig = hmac_hex(&secret, payload.as_bytes());
    let body = webhook_envelope(
        "fixture.audit.changefeed",
        "evt-5",
        "invoice.created",
        Some(&sig),
        Some("fp-webhook-test"),
        serde_json::json!({}),
    );
    let (status, _) = client.post("/v1/webhook/normalize", body.clone(), Some("1"));
    assert_eq!(status, 200);
    // Replay with the same provider event id must be rejected.
    let (status2, value2) = client.post("/v1/webhook/normalize", body, Some("1"));
    assert_eq!(status2, 401);
    assert_eq!(value2["code"], "VERIFICATION");
}

#[test]
fn ep011_failure_sidecar_unknown_webhook_event_not_executable() {
    // Directive Q: an arbitrary event name must never become a
    // capability id.
    let provider = spawn_provider();
    let sidecar = spawn_sidecar_with_webhook(&provider.base);
    let client = Client::new(&sidecar.base);
    let (secret_hex, _fp) = webhook_secret();
    let secret = hex_decode(&secret_hex);
    let payload = serde_json::json!({
        "provider_event_id": "evt-6",
        "provider_event_type": "fixture.contacts.command",
        "raw_payload": { "name": "evil" },
    })
    .to_string();
    let sig = hmac_hex(&secret, payload.as_bytes());
    let body = webhook_envelope(
        "fixture.audit.changefeed",
        "evt-6",
        "fixture.contacts.command",
        Some(&sig),
        Some("fp-webhook-test"),
        serde_json::json!({ "name": "evil" }),
    );
    let (status, value) = client.post("/v1/webhook/normalize", body, Some("1"));
    assert_eq!(status, 200);
    // Normalized event preserved, but never executed as a command.
    assert_eq!(value["event"]["event_type"], "fixture.contacts.command");
    assert_eq!(value["executable"], false);
    // Provider must show zero command executions for that name.
    let q = client.post(
        "/v1/query",
        query_envelope("fixture.contacts.query", serde_json::json!({})),
        Some("1"),
    );
    let contacts = q.1["output"]["contacts"].as_array().unwrap();
    assert!(
        !contacts.iter().any(|c| c["name"] == "evil"),
        "webhook event must not execute a command"
    );
}

// ---------------------------------------------------------------------
// Directive R/S: owned poller boundary
// ---------------------------------------------------------------------

fn spawn_sidecar_with_poller(provider_base: &str, state_dir: &std::path::Path) -> SidecarProc {
    spawn_sidecar(
        provider_base,
        &[
            ("NEXUS_SIDECAR_STATE_DIR", state_dir.to_str().unwrap()),
            ("NEXUS_SIDECAR_SOURCE", "source.jsonl"),
            ("NEXUS_SIDECAR_CHECKPOINT", "checkpoint.ckpt"),
        ],
    )
}

fn poll_envelope(capability_id: &str) -> serde_json::Value {
    serde_json::json!({
        "protocol_version": "1",
        "correlation_id": CORRELATION_ID,
        "request_id": REQUEST_ID,
        "tenant_id": TENANT_A,
        "connector_id": "fixture-connector",
        "capability_id": capability_id,
        "operation": "POLL",
        "transport": "REST",
        "schema_version": "1.0",
        "input": {},
    })
}

#[test]
fn ep011_integration_sidecar_poller_reads_real_source_and_checkpoints() {
    let provider = spawn_provider();
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("source.jsonl"),
        "{\"row\":1}\n{\"row\":2}\n",
    )
    .unwrap();
    let sidecar = spawn_sidecar_with_poller(&provider.base, dir.path());
    let client = Client::new(&sidecar.base);

    let (status, value) = client.post(
        "/v1/poll",
        poll_envelope("fixture.audit.changefeed"),
        Some("1"),
    );
    assert_eq!(status, 200);
    assert_eq!(value["events"].as_array().unwrap().len(), 2);
    assert_eq!(value["next_cursor"], "2");

    // Unchanged poll emits no fabricated changes.
    let (status2, value2) = client.post(
        "/v1/poll",
        poll_envelope("fixture.audit.changefeed"),
        Some("1"),
    );
    assert_eq!(status2, 200);
    assert_eq!(value2["events"].as_array().unwrap().len(), 0);

    // Source mutation produces only the expected canonical change.
    fs::write(
        dir.path().join("source.jsonl"),
        "{\"row\":1}\n{\"row\":2}\n{\"row\":3}\n",
    )
    .unwrap();
    let (status3, value3) = client.post(
        "/v1/poll",
        poll_envelope("fixture.audit.changefeed"),
        Some("1"),
    );
    assert_eq!(status3, 200);
    let events = value3["events"].as_array().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event_type"], "legacy.record.created");
}

#[test]
fn ep011_failure_sidecar_poller_corrupt_checkpoint_detected() {
    let provider = spawn_provider();
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("source.jsonl"), "{\"row\":1}\n").unwrap();
    fs::write(dir.path().join("checkpoint.ckpt"), "not-a-number\n").unwrap();
    let sidecar = spawn_sidecar_with_poller(&provider.base, dir.path());
    let client = Client::new(&sidecar.base);
    let (status, value) = client.post(
        "/v1/poll",
        poll_envelope("fixture.audit.changefeed"),
        Some("1"),
    );
    assert_eq!(status, 500);
    assert!(value["message"].as_str().unwrap().contains("corrupt"));
}

#[test]
fn ep011_failure_sidecar_poller_path_traversal_rejected() {
    let provider = spawn_provider();
    // A source path with `..` must be rejected at provisioning: the
    // sidecar refuses to start (exit 2, fail-closed config error).
    let dir = tempfile::tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_nexus-sidecar");
    let mut cmd = Command::new(bin);
    cmd.env("NEXUS_SIDECAR_TENANT", TENANT_A)
        .env("NEXUS_SIDECAR_CONNECTOR", "fixture-connector")
        .env(
            "NEXUS_SIDECAR_CAPABILITIES",
            "fixture.audit.changefeed:QUERY",
        )
        .env("NEXUS_PROVIDER_URL", &provider.base)
        .env("NEXUS_SIDECAR_STATE_DIR", dir.path().to_str().unwrap())
        .env("NEXUS_SIDECAR_SOURCE", "../escape.jsonl")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = cmd.output().expect("sidecar must exit");
    assert!(!output.status.success(), "sidecar must fail closed");
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(
        err.contains("path traversal"),
        "config error must name path traversal: {err}"
    );
}

#[test]
fn ep011_failure_sidecar_poller_unprovisioned_fails_closed() {
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let (status, value) = client.post(
        "/v1/poll",
        poll_envelope("fixture.audit.changefeed"),
        Some("1"),
    );
    assert_eq!(status, 503);
    assert_eq!(value["code"], "UNAVAILABLE");
}

// ---------------------------------------------------------------------
// Directive O: credential canary scan
// ---------------------------------------------------------------------

#[test]
fn ep011_integration_sidecar_credential_canary_never_leaks() {
    // Directive L: a real canary value must never appear in the HTTP
    // response, an HTTP error body, the sidecar stdout (PORT contract),
    // or the sidecar stderr (redacted telemetry).
    let provider = spawn_provider();
    let bin = env!("CARGO_BIN_EXE_nexus-sidecar");
    let mut cmd = Command::new(bin);
    cmd.env("NEXUS_SIDECAR_TENANT", TENANT_A)
        .env("NEXUS_SIDECAR_CONNECTOR", "fixture-connector")
        .env(
            "NEXUS_SIDECAR_CAPABILITIES",
            "fixture.contacts.query:QUERY,fixture.contacts.command:COMMAND",
        )
        .env("NEXUS_PROVIDER_URL", &provider.base)
        .env(
            "NEXUS_SIDECAR_CREDENTIAL_SCOPE",
            "fixture-connector:vault:fixture-token",
        )
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child: Child = cmd.spawn().expect("spawn sidecar");
    let port = read_port(&mut child);
    let base = format!("http://127.0.0.1:{port}");
    let client = Client::new(&base);

    const CANARY: &str = "fixture-secret-value";

    // 1. Permitted credential reference on a command: the value must
    //    not appear in the success response (fingerprint only).
    let body = command_envelope(
        "fixture.contacts.command",
        serde_json::json!({
            "name": "cred-test",
            "credential_reference": "vault:fixture-token",
        }),
        "op-cred-1",
    );
    let (status, value) = client.post("/v1/command", body, Some("1"));
    assert_eq!(status, 200);
    let body_str = value.to_string();
    assert!(
        !body_str.contains(CANARY),
        "credential value leaked in response: {body_str}"
    );

    // 2. Out-of-scope reference: the typed denial body must not carry
    //    the canary either.
    let mut bad = query_envelope("fixture.contacts.query", serde_json::json!({}));
    bad.as_object_mut().unwrap().insert(
        "input".to_string(),
        serde_json::json!({ "credential_reference": "vault:other-secret" }),
    );
    let (status2, value2) = client.post("/v1/query", bad, Some("1"));
    assert_eq!(status2, 403);
    assert_eq!(value2["code"], "AUTHORIZATION");
    let err_str = value2.to_string();
    assert!(
        !err_str.contains(CANARY),
        "credential value leaked in error body: {err_str}"
    );

    // 3. Controlled shutdown, then drain stdout + stderr fully: the
    //    canary must appear nowhere (readiness is PORT-only, telemetry
    //    is redacted).
    unsafe {
        libc_kill(child.id() as i32, SIGTERM);
    }
    let _ = child.wait();
    let mut out = String::new();
    let mut err = String::new();
    {
        use std::io::Read;
        if let Some(so) = child.stdout.as_mut() {
            let _ = so.read_to_string(&mut out);
        }
        if let Some(se) = child.stderr.as_mut() {
            let _ = se.read_to_string(&mut err);
        }
    }
    assert!(
        !out.contains(CANARY),
        "credential value leaked in sidecar stdout: {out}"
    );
    assert!(
        !err.contains(CANARY),
        "credential value leaked in sidecar stderr: {err}"
    );
    // Redaction sanity: raw tenant id still absent from telemetry.
    assert!(!err.contains(TENANT_A), "raw tenant id leaked: {err}");
    drop(child);
}

// ---------------------------------------------------------------------
// Directive Y: authorization boundary
// ---------------------------------------------------------------------

#[test]
fn ep011_integration_sidecar_acceptance_is_not_authorization() {
    // A request passing sidecar validation is structurally acceptable
    // only. The sidecar never claims authorization; EP-008 remains
    // the final authorization authority (NOT ASSERTED here).
    let provider = spawn_provider();
    let sidecar = spawn_sidecar(&provider.base, &[]);
    let client = Client::new(&sidecar.base);
    let body = query_envelope("fixture.contacts.query", serde_json::json!({}));
    let (status, value) = client.post("/v1/query", body, Some("1"));
    assert_eq!(status, 200);
    // The response must not contain any authorization claim.
    let body_str = value.to_string();
    assert!(
        !body_str.to_lowercase().contains("authorized"),
        "sidecar must not claim authorization"
    );
}

// ---------------------------------------------------------------------
// Directive W: observability events are emitted and redacted
// ---------------------------------------------------------------------

#[test]
fn ep011_integration_sidecar_observability_events_emitted() {
    let provider = spawn_provider();
    let bin = env!("CARGO_BIN_EXE_nexus-sidecar");
    let mut cmd = Command::new(bin);
    cmd.env("NEXUS_SIDECAR_TENANT", TENANT_A)
        .env("NEXUS_SIDECAR_CONNECTOR", "fixture-connector")
        .env("NEXUS_SIDECAR_CAPABILITIES", "fixture.contacts.query:QUERY")
        .env("NEXUS_PROVIDER_URL", &provider.base)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child: Child = cmd.spawn().expect("spawn sidecar");
    let port = read_port(&mut child);
    let client = Client::new(&format!("http://127.0.0.1:{port}"));
    let _ = client.post(
        "/v1/query",
        query_envelope("fixture.contacts.query", serde_json::json!({})),
        Some("1"),
    );

    // Read available stderr telemetry WITHOUT blocking on EOF (the
    // child keeps running), then SIGTERM it for a clean exit
    // (directive M/W). The pipe read blocks until the flushed
    // lifecycle events arrive; no artificial sleep is needed.
    let mut stderr = String::new();
    {
        use std::io::Read;
        let stderr_pipe = child.stderr.as_mut().unwrap();
        let mut chunk = [0u8; 8192];
        while let Ok(n) = stderr_pipe.read(&mut chunk) {
            if n == 0 {
                break;
            }
            stderr.push_str(&String::from_utf8_lossy(&chunk[..n]));
            if stderr.contains("DISPATCH_COMPLETED") {
                break;
            }
        }
    }
    unsafe {
        libc_kill(child.id() as i32, SIGTERM);
    }
    let _ = child.wait();
    // Drain whatever the clean shutdown flushed (SIDECAR_STOPPED).
    {
        use std::io::Read;
        let stderr_pipe = child.stderr.as_mut().unwrap();
        let mut chunk = [0u8; 8192];
        while let Ok(n) = stderr_pipe.read(&mut chunk) {
            if n == 0 {
                break;
            }
            stderr.push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
    }
    assert!(stderr.contains("SIDECAR_STARTED"), "stderr: {stderr}");
    assert!(stderr.contains("SIDECAR_READY"), "stderr: {stderr}");
    assert!(stderr.contains("REQUEST_ACCEPTED"), "stderr: {stderr}");
    assert!(stderr.contains("DISPATCH_COMPLETED"), "stderr: {stderr}");
    assert!(stderr.contains("SIDECAR_STOPPED"), "stderr: {stderr}");
    // Redaction: raw tenant id must never appear.
    assert!(
        !stderr.contains(TENANT_A),
        "raw tenant id leaked into telemetry: {stderr}"
    );
    drop(child);
}

// ---------------------------------------------------------------------
// Hex helpers (test side)
// ---------------------------------------------------------------------

fn hex_decode(value: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(value.len() / 2);
    let bytes = value.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = (chunk[0] as char).to_digit(16).unwrap() as u8;
        let lo = (chunk[1] as char).to_digit(16).unwrap() as u8;
        out.push((hi << 4) | lo);
    }
    out
}
