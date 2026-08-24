//! EP-040 M3 provider certification integration proofs against the REAL
//! digest-pinned PostgreSQL 18.4 container (COMPONENT_REGISTRY.yaml).
//!
//! Every proof in this file spawns a real ephemeral container through the
//! real docker CLI and executes real SQL through the published host port.
//! There are no mocks, no in-memory engines, and no scripted responders.

use nexus_provider_certification::certifier::{
    probe_live, EvidenceProvenance, RealProviderCertifier,
};
use nexus_provider_certification::transport::PostgresTransport;
use nexus_test_contract::error::TestingErrorCode;
use nexus_test_contract::model::ProviderCertificationSuite;
use nexus_test_contract::ProviderCertificationPort;

/// Real provider probe: the engine answers SELECT version() with the real
/// 18.4 server banner through the published host port.
#[test]
fn ep040_integration_provider_real_probe_observes_engine() {
    let pg = PostgresTransport::start().expect("start real postgres container");
    let probe = probe_live(&pg).expect("real probe");
    assert_eq!(probe.provider, "postgresql");
    assert_eq!(probe.interface, "sql-tcp-host-port");
    assert!(
        probe.version.contains("18.4"),
        "unexpected version: {}",
        probe.version
    );
    assert_eq!(probe.digest, nexus_provider_certification::POSTGRES_DIGEST);
    assert!(
        probe.roundtrip_ms < 30_000,
        "roundtrip implausible: {}",
        probe.roundtrip_ms
    );
}

/// Real round-trip: create, insert, select, count on the real engine.
#[test]
fn ep040_integration_provider_real_roundtrip() {
    let pg = PostgresTransport::start().expect("start real postgres container");
    let count = pg.roundtrip().expect("real roundtrip");
    assert!(count >= 1, "expected at least one row, got {count}");
}

/// Readiness is proven by a real connect + SELECT 1 through the published
/// host port (docker port-publish can lag pg_isready - EP-001 M5 flake fix).
#[test]
fn ep040_integration_provider_readiness_through_host_port() {
    let pg = PostgresTransport::start().expect("start real postgres container");
    let mut client = pg
        .connect_with_password(&pg.password)
        .expect("real connect with runtime credential");
    let ok = client
        .simple_query("SELECT 1")
        .expect("SELECT 1 must succeed on the real engine");
    assert!(
        !ok.is_empty(),
        "SELECT 1 must return at least one result message"
    );
}

/// Cancellation/timeout: statement_timeout kills a slow query on the real
/// engine and the connection recovers (fail-closed, not wedged).
#[test]
fn ep040_integration_provider_statement_timeout_cancels_slow_query() {
    let pg = PostgresTransport::start().expect("start real postgres container");
    let mut client = postgres::Client::connect(
        &format!(
            "host=127.0.0.1 port={} user={} password={} dbname={} options='-c statement_timeout=1000'",
            pg.port(),
            pg.user,
            pg.password,
            pg.dbname
        ),
        postgres::NoTls,
    )
    .expect("connect with statement_timeout");
    let started = std::time::Instant::now();
    let res = client.batch_execute("SELECT pg_sleep(30)");
    let elapsed = started.elapsed();
    assert!(res.is_err(), "statement_timeout must cancel the slow query");
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "statement_timeout must fail fast, took {elapsed:?}"
    );
    let recovered = client.simple_query("SELECT 1");
    assert!(
        recovered.is_ok(),
        "connection must recover after cancellation"
    );
}

/// Idempotency: a UNIQUE constraint rejects a duplicate insert on the real
/// engine (SPEC-006 idempotency semantics).
#[test]
fn ep040_integration_provider_idempotency_unique_constraint() {
    let pg = PostgresTransport::start().expect("start real postgres container");
    let mut client = pg
        .connect_with_password(&pg.password)
        .expect("real connect");
    client
        .batch_execute(
            "CREATE TABLE ep040_m3_idem (idempotency_key TEXT PRIMARY KEY, payload TEXT)",
        )
        .expect("create table");
    let first = client.execute(
        "INSERT INTO ep040_m3_idem (idempotency_key, payload) VALUES ($1, $2)",
        &[&"key-1", &"once"],
    );
    assert!(first.is_ok(), "first insert must succeed");
    let dup = client.execute(
        "INSERT INTO ep040_m3_idem (idempotency_key, payload) VALUES ($1, $2)",
        &[&"key-1", &"twice"],
    );
    assert!(dup.is_err(), "duplicate idempotency_key must be rejected");
}

/// Event emission: NOTIFY/LISTEN is a real engine event mechanism and the
/// listener observes the emitted payload across a real connection.
#[test]
fn ep040_integration_provider_event_emission_notify_listen() {
    let pg = PostgresTransport::start().expect("start real postgres container");
    let mut listener = pg
        .connect_with_password(&pg.password)
        .expect("listener connect");
    listener
        .simple_query("LISTEN ep040_m3_events")
        .expect("listen");
    let mut notifier = pg
        .connect_with_password(&pg.password)
        .expect("notifier connect");
    notifier
        .simple_query("NOTIFY ep040_m3_events, 'real-event-payload'")
        .expect("notify");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut observed = false;
    while std::time::Instant::now() < deadline {
        use postgres::fallible_iterator::FallibleIterator;
        let mut notifications = listener.notifications();
        let mut pending = notifications.timeout_iter(std::time::Duration::from_millis(200));
        if let Some(notification) = pending.next().expect("notification read") {
            if notification.payload() == "real-event-payload" {
                observed = true;
                break;
            }
        }
    }
    assert!(
        observed,
        "NOTIFY payload must be observed by the real listener"
    );
}

/// Cleanup: dropping the transport removes the container and zero residue
/// is verified through the real docker CLI.
#[test]
fn ep040_integration_provider_cleanup_zero_residue() {
    let pg = PostgresTransport::start().expect("start real postgres container");
    let container = pg.container.clone();
    drop(pg);
    // Give docker a moment to remove.
    std::thread::sleep(std::time::Duration::from_millis(500));
    let out = std::process::Command::new("docker")
        .args(["ps", "-a", "--no-trunc", "--format", "{{.Names}}"])
        .output()
        .expect("docker ps");
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains(&container),
        "container {container} must be removed after drop"
    );
}

/// Real provider evidence certifies the suite for the exact provider/
/// version/interface exercised.
#[test]
fn ep040_integration_provider_real_evidence_certifies() {
    let pg = PostgresTransport::start().expect("start real postgres container");
    let probe = probe_live(&pg).expect("real probe");
    let certifier = RealProviderCertifier::new(probe, "run-1", "abc1234");
    let suite = ProviderCertificationSuite::new("postgresql", "core")
        .certify(vec![
            "evidence://ep040-m3/probe-1".into(),
            "evidence://ep040-m3/roundtrip-1".into(),
        ])
        .expect("suite certify");
    let certified = certifier.certify(suite).expect("real certifier");
    assert_eq!(
        certified.status.as_str(),
        "CERTIFIED",
        "real evidence must certify"
    );
    assert_eq!(certified.provider, "postgresql");
}

/// Provider unavailable fails closed as Unavailable - never a silent skip.
#[test]
fn ep040_integration_provider_auth_failure_fails_closed() {
    let pg = PostgresTransport::start().expect("start real postgres container");
    let res = pg.connect_with_password("definitely-wrong-password");
    assert!(
        res.is_err(),
        "wrong password must be rejected by the real engine"
    );
}

/// Provider unavailable fails closed as Unavailable - never a silent skip.
#[test]
fn ep040_unit_provider_unavailable_fails_closed() {
    // A transport that was never started cannot probe; the error is
    // Unavailable (typed), never a generic success or skip.
    let err = probe_missing().expect_err("missing provider must fail");
    assert_eq!(err.code, TestingErrorCode::Unavailable);
}

fn probe_missing(
) -> nexus_test_contract::error::TestingResult<nexus_provider_certification::transport::ProviderProbe>
{
    let pg = PostgresTransport {
        container: "nexus-ep040-m3-nonexistent".into(),
        port: 1,
        user: "nexus".into(),
        password: "missing".into(),
        dbname: "nexus".into(),
    };
    probe_live(&pg)
}

/// Mock/simulated evidence can never certify (MOCK PASSED != PRODUCTION
/// PATH VERIFIED).
#[test]
fn ep040_unit_provider_mock_evidence_never_certifies() {
    let suite = ProviderCertificationSuite::new("postgresql", "core")
        .certify(vec!["evidence://mock/1".into()])
        .expect("suite certify");
    let res = nexus_provider_certification::certifier::behavior::certify_with_provenance(
        suite,
        EvidenceProvenance::Mock,
    );
    let err = res.expect_err("mock evidence must be rejected");
    assert_eq!(err.code, TestingErrorCode::MockOnlyCertification);
}

/// Simulated evidence can never certify either.
#[test]
fn ep040_unit_provider_simulated_evidence_never_certifies() {
    let suite = ProviderCertificationSuite::new("postgresql", "core")
        .certify(vec!["evidence://simulated/1".into()])
        .expect("suite certify");
    let res = nexus_provider_certification::certifier::behavior::certify_with_provenance(
        suite,
        EvidenceProvenance::Simulated,
    );
    let err = res.expect_err("simulated evidence must be rejected");
    assert_eq!(err.code, TestingErrorCode::MockOnlyCertification);
}

/// Stale evidence (run_id/git_commit mismatch) is rejected.
#[test]
fn ep040_unit_provider_stale_evidence_rejected() {
    let probe = nexus_provider_certification::transport::ProviderProbe {
        provider: "postgresql".into(),
        version: "18.4".into(),
        interface: "sql-tcp-host-port".into(),
        digest: nexus_provider_certification::POSTGRES_DIGEST.to_string(),
        roundtrip_ms: 1,
    };
    let certifier = RealProviderCertifier::new(probe, "run-current", "deadbeef");
    let err = certifier
        .reject_stale("run-stale", "deadbeef")
        .expect_err("stale run_id must be rejected");
    assert_eq!(err.code, TestingErrorCode::Verification);
    let err = certifier
        .reject_stale("run-current", "0000000")
        .expect_err("stale git_commit must be rejected");
    assert_eq!(err.code, TestingErrorCode::Verification);
    certifier
        .reject_stale("run-current", "deadbeef")
        .expect("current evidence must pass");
}

/// Missing evidence cannot certify.
#[test]
fn ep040_unit_provider_missing_evidence_rejected() {
    let probe = nexus_provider_certification::transport::ProviderProbe {
        provider: "postgresql".into(),
        version: "18.4".into(),
        interface: "sql-tcp-host-port".into(),
        digest: nexus_provider_certification::POSTGRES_DIGEST.to_string(),
        roundtrip_ms: 1,
    };
    let certifier = RealProviderCertifier::new(probe, "run-1", "abc1234");
    let suite = ProviderCertificationSuite::new("postgresql", "core");
    let err = certifier
        .certify(suite)
        .expect_err("missing evidence must be rejected");
    assert_eq!(err.code, TestingErrorCode::MissingEvidence);
}

/// Suite provider must match the probed provider (identity binding).
#[test]
fn ep040_unit_provider_identity_binding() {
    let probe = nexus_provider_certification::transport::ProviderProbe {
        provider: "postgresql".into(),
        version: "18.4".into(),
        interface: "sql-tcp-host-port".into(),
        digest: nexus_provider_certification::POSTGRES_DIGEST.to_string(),
        roundtrip_ms: 1,
    };
    let certifier = RealProviderCertifier::new(probe, "run-1", "abc1234");
    let suite = ProviderCertificationSuite::new("minio", "core")
        .certify(vec!["evidence://ep040-m3/probe-1".into()])
        .expect("suite certify");
    let err = certifier
        .certify(suite)
        .expect_err("provider mismatch must be rejected");
    assert_eq!(err.code, TestingErrorCode::Verification);
}

/// Redaction: secret-shaped evidence is rejected. The M1 contract suite
/// itself enforces redaction at certify() time; the real certifier adds
/// a second check (defense in depth) for evidence set by any path.
#[test]
fn ep040_unit_provider_evidence_redaction_enforced() {
    let leaked = format!("evidence://{}/1", secret_shaped_canary());
    // Layer 1: the M1 contract suite rejects secret-shaped evidence.
    let suite_res = ProviderCertificationSuite::new("postgresql", "core").certify(vec![leaked]);
    let suite_err = suite_res.expect_err("suite must reject secret-shaped evidence");
    assert_eq!(suite_err.code, TestingErrorCode::Validation);

    // Layer 2: the real certifier rejects evidence that carries a
    // secret-shaped value even when constructed without the suite check.
    let probe = nexus_provider_certification::transport::ProviderProbe {
        provider: "postgresql".into(),
        version: "18.4".into(),
        interface: "sql-tcp-host-port".into(),
        digest: nexus_provider_certification::POSTGRES_DIGEST.to_string(),
        roundtrip_ms: 1,
    };
    let certifier = RealProviderCertifier::new(probe, "run-1", "abc1234");
    let suite = ProviderCertificationSuite {
        provider: "postgresql".into(),
        profile: "core".into(),
        evidence: vec![secret_shaped_canary()],
        status: nexus_test_contract::vocabulary::CertificationStatus::NotAsserted,
    };
    let err = certifier
        .certify(suite)
        .expect_err("secret-shaped evidence must be rejected");
    assert_eq!(err.code, TestingErrorCode::Validation);
}

/// Runtime-constructed secret canary: never a tracked source literal.
fn secret_shaped_canary() -> String {
    let mut s = String::from("sk-");
    s.push_str(&format!("{:x}", 0xdead_beef_u64));
    s
}
