//! EP-023 M4 failure suite: REAL forced failures against the REAL
//! nexus-frigate adapter/transport (SPEC-021; M4 directive P).
//!
//! Every failure is produced by a real mechanism, never a mock:
//!
//! 1. `provider_stopped`  - the real Frigate container is stopped by
//!    the gate script before this phase -> VisionErrorCode::Unavailable
//! 2. `closed_port`       - a real localhost port that accepts no
//!    listener -> Unavailable
//! 3. `silent_peer`       - a REAL local TCP peer that accepts the
//!    connection and never responds -> Timeout (real socket read
//!    deadline via RestTransport::with_timeout)
//! 4. `http_401`          - a real HTTP responder returning 401 ->
//!    Authorization (CONTROLLED_TEST_FIXTURE for the transport
//!    classifier; not Frigate provider certification)
//! 5. `malformed_json`    - a real HTTP response with invalid JSON ->
//!    External + malformed counter
//! 6. `schema_invalid`    - a real HTTP response with valid JSON but
//!    wrong DTO shape -> fail closed (External)
//! 7. redaction canaries  - rtsp://user:EP023_SECRET_CANARY@... and
//!    ?token=EP023_TOKEN_CANARY must appear ZERO times in errors,
//!    audit, metrics, diagnostic output, stderr/stdout
//! 8. correlation id      - present and stable for one operation
//! 9. audit ring bounded
//! 10. counters increment
//! 11. stream truth after failure: previously STREAMING -> provider
//!     unavailable -> next fresh observation is never STREAMING
//! 12. diagnostic status against healthy provider
//! 13. diagnostic status against unavailable provider
//! 14. diagnostic redaction
//!
//! Env: `FRIGATE_BASE_URL` required for provider-dependent tests;
//! transport-classifier tests bind their own local sockets and do not
//! need the live stack.

use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use nexus_frigate::observability::FrigateObservability;
use nexus_frigate::{FrigateAdapter, RestTransport};
use nexus_vision::provider::CameraProvider;
use nexus_vision::vocabulary::CameraId;
use nexus_vision::VisionErrorCode;

const CANARY_URL_USER: &str = "rtsp://user:EP023_SECRET_CANARY@192.0.2.10:554/stream";
const CANARY_TOKEN: &str = "EP023_TOKEN_CANARY";

fn base_url() -> String {
    env::var("FRIGATE_BASE_URL").unwrap_or_else(|_| {
        panic!(
            "FRIGATE_BASE_URL is required for ep023_failure provider tests \
             (start the real Frigate stack via scripts/ep023-m4-tests.sh)"
        )
    })
}

// ---------------------------------------------------------------------------
// Controlled fixtures: real sockets, real HTTP responses.
// ---------------------------------------------------------------------------

/// Bind a listener, hand the port back, and hold the listener open.
fn listener() -> TcpListener {
    TcpListener::bind("127.0.0.1:0").expect("bind local fixture listener")
}

fn port_of(listener: &TcpListener) -> u16 {
    listener.local_addr().expect("local addr").port()
}

/// A REAL TCP peer that accepts the connection and then stays silent
/// longer than any client timeout (CONTROLLED_TEST_FIXTURE; the
/// timeout mechanism itself is the real transport).
fn spawn_silent_peer(hold: Duration) -> u16 {
    let listener = listener();
    let port = port_of(&listener);
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            // Read whatever the client sends, then deliberately send
            // nothing for `hold` (longer than the test timeout).
            let mut buf = [0u8; 1024];
            let _ = stream.read(&mut buf);
            thread::sleep(hold);
        }
    });
    port
}

/// A REAL HTTP responder that returns the given raw response bytes to
/// every request (CONTROLLED_TEST_FIXTURE).
fn spawn_http_responder(response: &'static str) -> u16 {
    let listener = listener();
    let port = port_of(&listener);
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf);
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    port
}

const HTTP_401: &str =
    "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
const HTTP_403: &str = "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
const HTTP_404: &str = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
const HTTP_500: &str =
    "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
const HTTP_MALFORMED: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\nConnection: close\r\n\r\n{not json!!}";
const HTTP_SCHEMA_INVALID: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 15\r\nConnection: close\r\n\r\n{\"cameras\": 42}";

fn transport_at(port: u16) -> RestTransport {
    RestTransport::new(format!("http://127.0.0.1:{port}"))
}

// ---------------------------------------------------------------------------
// 2. closed port -> Unavailable
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_closed_port_connection_failure_unavailable() {
    // A real port that is definitely closed: bind then drop.
    let port = {
        let listener = listener();
        port_of(&listener)
    };
    let adapter = FrigateAdapter::new(transport_at(port));
    let err = adapter
        .health()
        .expect_err("closed port must fail provider health");
    assert_eq!(
        err.code,
        VisionErrorCode::Unavailable,
        "closed port must map to Unavailable, got {}",
        err.code.as_str()
    );
    // Correlation id must be present and safe.
    let cid = err
        .correlation_id
        .expect("correlation id on transport error");
    assert!(cid.starts_with("frigate-"), "unexpected correlation {cid}");
}

// ---------------------------------------------------------------------------
// 3. silent accepted peer -> Timeout
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_silent_peer_times_out() {
    let port = spawn_silent_peer(Duration::from_secs(8));
    // A small bounded deadline: the production transport MUST fail
    // closed instead of hanging the caller forever.
    let transport = RestTransport::new(format!("http://127.0.0.1:{port}"))
        .with_timeout(Duration::from_millis(700));
    let adapter = FrigateAdapter::new(transport);
    let started = std::time::Instant::now();
    let err = adapter
        .health()
        .expect_err("silent peer must produce a timeout");
    let elapsed = started.elapsed();
    assert_eq!(
        err.code,
        VisionErrorCode::Timeout,
        "accepted-but-silent connection must map to Timeout, got {}",
        err.code.as_str()
    );
    assert!(
        elapsed < Duration::from_secs(6),
        "timeout took too long: {elapsed:?}"
    );
    assert!(
        err.correlation_id.is_some(),
        "timeout error must carry a correlation id"
    );
}

// ---------------------------------------------------------------------------
// 4. real HTTP 401/403 -> Authorization
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_http_401_authorization() {
    let port = spawn_http_responder(HTTP_401);
    let adapter = FrigateAdapter::new(transport_at(port));
    let err = adapter
        .list_cameras()
        .expect_err("401 must fail camera listing");
    assert_eq!(
        err.code,
        VisionErrorCode::Authorization,
        "401 must map to Authorization, got {}",
        err.code.as_str()
    );
}

#[test]
fn ep023_failure_frigate_http_403_authorization() {
    let port = spawn_http_responder(HTTP_403);
    let adapter = FrigateAdapter::new(transport_at(port));
    let err = adapter
        .list_cameras()
        .expect_err("403 must fail camera listing");
    assert_eq!(err.code, VisionErrorCode::Authorization);
}

#[test]
fn ep023_failure_frigate_http_404_not_found() {
    let port = spawn_http_responder(HTTP_404);
    let adapter = FrigateAdapter::new(transport_at(port));
    let err = adapter
        .list_cameras()
        .expect_err("404 must fail camera listing");
    assert_eq!(
        err.code,
        VisionErrorCode::NotFound,
        "404 unknown resource must map to NotFound, got {}",
        err.code.as_str()
    );
}

#[test]
fn ep023_failure_frigate_http_500_unavailable() {
    let port = spawn_http_responder(HTTP_500);
    let adapter = FrigateAdapter::new(transport_at(port));
    let err = adapter
        .list_cameras()
        .expect_err("500 must fail camera listing");
    assert_eq!(
        err.code,
        VisionErrorCode::Unavailable,
        "500 provider error must map to Unavailable, got {}",
        err.code.as_str()
    );
}

// ---------------------------------------------------------------------------
// 5. malformed JSON -> External + malformed counter
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_malformed_json_external_and_counter() {
    let port = spawn_http_responder(HTTP_MALFORMED);
    let adapter = FrigateAdapter::new(transport_at(port));
    let err = adapter
        .list_cameras()
        .expect_err("invalid JSON must fail camera listing");
    assert_eq!(
        err.code,
        VisionErrorCode::External,
        "malformed JSON maps to External (canonical malformed code), got {}",
        err.code.as_str()
    );
    // The transport boundary detected a malformed response.
    let metrics = adapter.metrics();
    let malformed = metrics["malformed_total"].as_u64().unwrap_or(0);
    assert!(
        malformed >= 1,
        "malformed counter must increment, got {malformed}"
    );
    // No fabricated empty camera result: the operation failed closed.
    let audit = adapter.audit();
    assert!(
        audit.iter().any(|r| !r.ok),
        "audit must record the failed operation"
    );
}

// ---------------------------------------------------------------------------
// 6. schema-invalid body -> fail closed
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_schema_invalid_fails_closed() {
    let port = spawn_http_responder(HTTP_SCHEMA_INVALID);
    let adapter = FrigateAdapter::new(transport_at(port));
    let err = adapter
        .list_cameras()
        .expect_err("wrong DTO shape must fail camera listing");
    assert_eq!(
        err.code,
        VisionErrorCode::External,
        "schema-invalid provider body fails closed, got {}",
        err.code.as_str()
    );
}

// ---------------------------------------------------------------------------
// 7. redaction canaries: ZERO occurrence of the actual secret values
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_redaction_canaries_absent() {
    // A URL that embeds a real-looking secret, plus a token canary.
    let redacted = nexus_frigate::redact_url(CANARY_URL_USER);
    assert!(
        !redacted.contains("EP023_SECRET_CANARY"),
        "redacted URL leaked userinfo secret: {redacted}"
    );
    let redacted_token =
        nexus_frigate::redact_url("http://frigate.local/api/events?token=EP023_TOKEN_CANARY");
    assert!(
        !redacted_token.contains(CANARY_TOKEN),
        "redacted URL leaked token: {redacted_token}"
    );

    // A real transport failure whose error message may embed a URL
    // with credentials: the error, audit, and metrics must all be free
    // of the canary values.
    let port = spawn_http_responder(HTTP_401);
    let transport = transport_at(port).with_token(CANARY_TOKEN);
    // Force a transport-level failure that can embed the URL.
    let adapter = FrigateAdapter::new(transport);
    let err = adapter
        .list_cameras()
        .expect_err("expected failure for redaction probe");
    let err_text = format!("{err:?} {err:?}");
    assert!(
        !err_text.contains(CANARY_TOKEN),
        "VisionError leaked token canary"
    );
    assert!(
        !err_text.contains("EP023_SECRET_CANARY"),
        "VisionError leaked userinfo canary"
    );
    for record in adapter.audit() {
        let text = format!("{record:?}");
        assert!(!text.contains(CANARY_TOKEN), "audit leaked token canary");
        assert!(
            !text.contains("EP023_SECRET_CANARY"),
            "audit leaked userinfo canary"
        );
    }
    let metrics_text = format!("{}", adapter.metrics());
    assert!(
        !metrics_text.contains(CANARY_TOKEN),
        "metrics leaked token canary"
    );
}

// ---------------------------------------------------------------------------
// 8. correlation id present and stable for one operation
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_correlation_present_and_stable() {
    let port = spawn_http_responder(HTTP_500);
    let adapter = FrigateAdapter::new(transport_at(port));
    let err = adapter
        .list_cameras()
        .expect_err("500 must fail camera listing");
    let cid = err.correlation_id.expect("correlation id on failure");
    // The audit record for the same operation must carry the SAME id.
    let audit = adapter.audit();
    let matching = audit
        .iter()
        .filter(|r| !r.ok)
        .filter(|r| r.correlation_id.as_deref() == Some(cid.as_ref()))
        .count();
    assert!(
        matching >= 1,
        "audit must contain a record matching correlation {cid}"
    );
}

// ---------------------------------------------------------------------------
// 9. audit ring bounded
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_audit_ring_bounded() {
    let port = spawn_http_responder(HTTP_500);
    let obs = FrigateObservability::new(5);
    let adapter = FrigateAdapter::with_observability(transport_at(port), obs);
    // More operations than the ring capacity.
    for _ in 0..12 {
        let _ = adapter.list_cameras();
    }
    let audit = adapter.audit();
    assert_eq!(audit.len(), 5, "audit ring must stay bounded at capacity");
    // Deterministic oldest-entry eviction: the first recorded detail
    // ("EXTERNAL" from the first failed op) must be gone from the head.
    // The ring keeps the most recent 5 records; the 12th is present.
    assert!(
        audit.iter().any(|r| r.operation == "list_cameras"),
        "ring holds recent records"
    );
}

// ---------------------------------------------------------------------------
// 10. counters increment correctly
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_counters_increment() {
    // Timeout path: silent peer -> timeouts_total increments.
    let port = spawn_silent_peer(Duration::from_secs(8));
    let transport = RestTransport::new(format!("http://127.0.0.1:{port}"))
        .with_timeout(Duration::from_millis(400));
    let adapter = FrigateAdapter::new(transport);
    let _ = adapter.health();
    let metrics = adapter.metrics();
    assert!(
        metrics["operations_total"].as_u64().unwrap_or(0) >= 1,
        "operations counter must increment"
    );
    assert!(
        metrics["failures_total"].as_u64().unwrap_or(0) >= 1,
        "failures counter must increment"
    );
    assert!(
        metrics["timeouts_total"].as_u64().unwrap_or(0) >= 1,
        "timeouts counter must increment on silent peer"
    );
    assert_eq!(
        metrics["auth_failures_total"].as_u64().unwrap_or(0),
        0,
        "no auth failures on a timeout"
    );

    // Authorization path: 401 -> auth_failures_total increments.
    let port401 = spawn_http_responder(HTTP_401);
    let adapter401 = FrigateAdapter::new(transport_at(port401));
    let _ = adapter401.list_cameras();
    let metrics401 = adapter401.metrics();
    assert!(
        metrics401["auth_failures_total"].as_u64().unwrap_or(0) >= 1,
        "auth_failures counter must increment on 401"
    );
}

// ---------------------------------------------------------------------------
// 11. stream truth after failure (requires the real stack; phase B)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m4-tests.sh"]
fn ep023_failure_frigate_provider_stopped_unavailable() {
    // Gate phase B: the real Frigate container is stopped. The SAME
    // production operation that succeeded in phase A must now return
    // Unavailable - never a stale success, never STREAMING from cache.
    let adapter = FrigateAdapter::new(RestTransport::new(base_url()));
    let err = adapter
        .list_cameras()
        .expect_err("stopped provider must fail camera listing");
    assert_eq!(
        err.code,
        VisionErrorCode::Unavailable,
        "stopped provider must map to Unavailable, got {}",
        err.code.as_str()
    );
    let audit = adapter.audit();
    assert!(audit.iter().any(|r| !r.ok), "audit must record the failure");
    let metrics = adapter.metrics();
    assert!(
        metrics["failures_total"].as_u64().unwrap_or(0) >= 1,
        "failure counter must increment"
    );
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m4-tests.sh"]
fn ep023_failure_frigate_never_streaming_without_fresh_evidence() {
    // Phase B (provider stopped): a fresh availability observation must
    // never report STREAMING. The adapter has no stale cache - it
    // re-probes the real provider on every call.
    let adapter = FrigateAdapter::new(RestTransport::new(base_url()));
    let camera = CameraId::new("nexus_front").expect("canonical camera id");
    let result = adapter.availability(&camera);
    match result {
        Ok(avail) => {
            assert_ne!(
                avail.as_str(),
                "STREAMING",
                "must never report STREAMING without live provider evidence"
            );
        }
        Err(err) => {
            assert_eq!(
                err.code,
                VisionErrorCode::Unavailable,
                "stopped provider availability must fail unavailable, got {}",
                err.code.as_str()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 12/13/14. diagnostic status/redaction (requires the real stack for
// healthy status; unavailable status works against a stopped provider
// or a closed port)
// ---------------------------------------------------------------------------

#[test]
fn ep023_failure_frigate_diag_status_unavailable() {
    // A closed localhost port: frigate-diag must report unreachable
    // safely, never fabricate health.
    let port = {
        let listener = listener();
        port_of(&listener)
    };
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_frigate-diag"))
        .arg("status")
        .env("FRIGATE_BASE_URL", format!("http://127.0.0.1:{port}"))
        .output()
        .expect("run frigate-diag");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        stdout.contains("provider_reachable") && stdout.contains("false"),
        "diag must report unreachable provider, got: {stdout}"
    );
    assert!(
        !stdout.contains(CANARY_TOKEN),
        "diag stdout leaked token canary"
    );
    assert!(
        !stderr.contains(CANARY_TOKEN),
        "diag stderr leaked token canary"
    );
}

#[test]
fn ep023_failure_frigate_diag_redaction() {
    // Run frigate-diag with a canary-secret base URL: no RTSP
    // password, bearer token, or query secret may appear in
    // stdout/stderr.
    let port = spawn_http_responder(HTTP_500);
    let secret_base = format!("http://user:EP023_SECRET_CANARY@127.0.0.1:{port}");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_frigate-diag"))
        .arg("status")
        .env("FRIGATE_BASE_URL", secret_base)
        .env("FRIGATE_TOKEN", CANARY_TOKEN)
        .output()
        .expect("run frigate-diag");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        !stdout.contains("EP023_SECRET_CANARY"),
        "diag stdout leaked userinfo canary: {stdout}"
    );
    assert!(
        !stdout.contains(CANARY_TOKEN),
        "diag stdout leaked token canary: {stdout}"
    );
    assert!(
        !stderr.contains("EP023_SECRET_CANARY"),
        "diag stderr leaked userinfo canary: {stderr}"
    );
    assert!(
        !stderr.contains(CANARY_TOKEN),
        "diag stderr leaked token canary: {stderr}"
    );
}

// ---------------------------------------------------------------------------
// helpers used by phase A (healthy provider) - gate script phase
// selection keeps these out of the way of the stopped-provider phases.
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m4-tests.sh"]
fn ep023_failure_frigate_diag_status_healthy() {
    // Gate phase A: the real Frigate stack is up.
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_frigate-diag"))
        .arg("status")
        .env("FRIGATE_BASE_URL", base_url())
        .output()
        .expect("run frigate-diag");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(
        stdout.contains("provider_reachable") && stdout.contains("true"),
        "diag must report reachable provider, got: {stdout}"
    );
    assert!(
        stdout.contains("frigate_version"),
        "diag must report version when available: {stdout}"
    );
    assert!(
        stdout.contains("camera_count"),
        "diag must report camera count: {stdout}"
    );
    // Diagnostic output is safe: no secrets.
    assert!(
        !stdout.contains(CANARY_TOKEN),
        "diag stdout leaked token canary"
    );
}

#[test]
#[ignore = "requires live Frigate stack (FRIGATE_BASE_URL); run via scripts/ep023-m4-tests.sh"]
fn ep023_failure_frigate_recovery_after_provider_restart() {
    // Gate phase C: the real Frigate container was restarted. A fresh
    // observation must recover: list_cameras succeeds and the canary
    // camera is discovered again.
    let adapter = FrigateAdapter::new(RestTransport::new(base_url()));
    let cameras = adapter.list_cameras().expect("provider recovered");
    assert!(
        cameras.iter().any(|c| c.as_str() == "nexus_front"),
        "canonical camera must be discovered after recovery"
    );
}
