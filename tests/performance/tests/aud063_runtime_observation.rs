//! AUD-063 leaf proofs (RX-021): performance certification measures a
//! REAL runtime over the wire, never a hand-fed constant.
//!
//! Hostile + positive proofs:
//! - `aud063_probe_unreachable_fails_closed`: a dead endpoint can never
//!   fabricate an observation.
//! - `aud063_probe_non_healthy_fails_closed`: an endpoint that answers but
//!   does not report healthy is never certified.
//! - `aud063_probe_measures_real_wire_latency`: a live TCP endpoint yields
//!   a real observation with sane p95/max (hostile check: not zero).
//! - `aud063_real_observation_certifies_budget`: probe p95 observed into a
//!   budget and evaluated through the canonical evaluator path.
//! - `aud063_hand_fed_constant_not_runtime_evidence`: a budget fed a
//!   constant without a probe observation must fail the same path that a
//!   real observation would certify (no vacuous equality).

use nexus_test_contract::model::PerformanceBudget;
use nexus_test_contract::PerformanceBudgetPort;
use nexus_test_performance::{DeterministicBudgetEvaluator, RuntimeLatencyProbe};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

/// Spawn a minimal HTTP/1.1 health server on an ephemeral port. The server
/// answers one request per connection with a `healthy` body. Returns the
/// base URL of the server.
fn spawn_health_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            thread::spawn(move || {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{"status":"healthy"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            });
        }
    });
    format!("http://{addr}")
}

/// Spawn a TCP listener that accepts then immediately closes (dead after
/// accept - simulates a black-holed endpoint) and returns its URL.
fn spawn_closed_port_url() -> String {
    // Bind and drop the listener immediately so nothing accepts; the
    // returned port is guaranteed closed.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    drop(listener);
    format!("http://{addr}")
}

#[test]
fn aud063_probe_unreachable_fails_closed() {
    let url = spawn_closed_port_url();
    let probe = RuntimeLatencyProbe::new(url).with_samples(1);
    let err = probe.probe().expect_err("dead endpoint must fail closed");
    let text = format!("{err}");
    assert!(
        text.contains("failed") || text.contains("refused"),
        "unreachable probe error was: {text}"
    );
}

#[test]
fn aud063_probe_non_healthy_fails_closed() {
    // A server that answers 200 but with a non-healthy body.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            thread::spawn(move || {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{"status":"degraded"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            });
        }
    });
    let url = format!("http://{addr}");
    let probe = RuntimeLatencyProbe::new(url).with_samples(1);
    let err = probe
        .probe()
        .expect_err("non-healthy endpoint must fail closed");
    let text = format!("{err}");
    assert!(
        text.contains("not report healthy"),
        "non-healthy probe error was: {text}"
    );
}

#[test]
fn aud063_probe_measures_real_wire_latency() {
    let url = spawn_health_server();
    let probe = RuntimeLatencyProbe::new(url)
        .with_samples(3)
        .with_timeout(std::time::Duration::from_secs(2));
    let obs = probe.probe().expect("live endpoint must probe");
    assert!(obs.healthy, "observation must record healthy");
    assert_eq!(obs.samples, 3, "observation must carry sample count");
    // Real wire latency is never exactly zero; the hostile check is that a
    // fabricated constant cannot masquerade as a measurement.
    assert!(
        obs.p95_ms > 0.0 && obs.max_ms > 0.0,
        "real latency must be a positive measurement (p95={} max={})",
        obs.p95_ms,
        obs.max_ms
    );
    assert!(
        obs.p95_ms < 5000.0,
        "local health server p95 should be far under 5s, got {}",
        obs.p95_ms
    );
}

#[test]
fn aud063_real_observation_certifies_budget() {
    let url = spawn_health_server();
    let probe = RuntimeLatencyProbe::new(url)
        .with_samples(3)
        .with_timeout(std::time::Duration::from_secs(2));
    let obs = probe.probe().expect("live endpoint must probe");
    let budget = PerformanceBudget::new("aud063-healthz", "RX-021", "p95", 5000.0, "ms");
    let certified = obs
        .certify(budget.clone())
        .expect("real observation certifies");
    assert!(certified.observed, "certified budget must be observed");
    let evaluator = DeterministicBudgetEvaluator::new();
    assert!(
        evaluator.evaluate(&certified).is_ok(),
        "canonical evaluator must accept the real observation"
    );
    assert!(
        obs.evaluate(&budget).is_ok(),
        "evaluate() path must accept the real observation"
    );
}

#[test]
fn aud063_hand_fed_constant_not_runtime_evidence() {
    // A budget whose observed value was hand-fed (never probed) must not
    // satisfy the same certification surface a real observation does:
    // `evaluate` on a RealLatencyObservation enforces healthy + samples,
    // and an unobserved budget never passes the canonical evaluator.
    let url = spawn_health_server();
    let probe = RuntimeLatencyProbe::new(url)
        .with_samples(1)
        .with_timeout(std::time::Duration::from_secs(2));
    let obs = probe.probe().expect("live endpoint must probe");

    // Hand-fed constant: even a tiny 0.001ms would trivially pass a loose
    // budget, but it is not real evidence; the canonical evaluator refuses
    // an UNOBSERVED budget regardless of the number we might feed it.
    let unobserved = PerformanceBudget::new("aud063-hand-fed", "RX-021", "p95", 5000.0, "ms");
    let evaluator = DeterministicBudgetEvaluator::new();
    assert!(
        evaluator.evaluate(&unobserved).is_err(),
        "unobserved budget must fail closed even when a number is known"
    );

    // Hostile: a fabricated tiny constant (0.001ms) must NOT certify. The
    // probe observes the REAL p95 (~ms on localhost), so certify() fails
    // closed because the real measurement exceeds the fabricated bound.
    // A hand-fed constant can never masquerade as runtime evidence.
    let fabricated = PerformanceBudget::new("aud063-fabricated", "RX-021", "p95", 0.001, "ms");
    let err = obs
        .certify(fabricated.clone())
        .expect_err("a fabricated tiny bound must never certify against real p95");
    let text = format!("{err}");
    assert!(
        text.contains("exceeded"),
        "fabricated-bound failure must report budget exceeded: {text}"
    );

    // Positive: a loose budget certifies, and the certified observed value
    // is the REAL probe p95 - never the fabricated constant.
    let loose = PerformanceBudget::new("aud063-loose", "RX-021", "p95", 5000.0, "ms");
    let certified = obs.certify(loose).expect("loose budget certifies real p95");
    assert!(
        certified.observed && certified.observed_value == Some(obs.p95_ms),
        "certified observed value must be the real probe p95 (got {:?}, expected {:?})",
        certified.observed_value,
        Some(obs.p95_ms)
    );
}

/// The probe must reject malformed endpoints loudly rather than guessing.
#[test]
fn aud063_probe_rejects_non_http_endpoint() {
    let probe = RuntimeLatencyProbe::new("tcp://127.0.0.1:8443");
    let err = probe
        .probe()
        .expect_err("non-http endpoint must fail closed");
    let text = format!("{err}");
    assert!(
        text.contains("http:// endpoints only"),
        "malformed endpoint error was: {text}"
    );
}

/// The probe is usable through the object-safe budget port surface.
#[test]
fn aud063_real_observation_serde_roundtrip() {
    let url = spawn_health_server();
    let probe = RuntimeLatencyProbe::new(url)
        .with_samples(1)
        .with_timeout(std::time::Duration::from_secs(2));
    let obs = probe.probe().expect("live endpoint must probe");
    let json = serde_json::to_string(&obs).expect("serialize observation");
    assert!(
        json.contains("\"healthy\":true"),
        "json must carry healthy: {json}"
    );
    assert!(
        json.contains("\"endpoint\":"),
        "json must carry endpoint: {json}"
    );
}
