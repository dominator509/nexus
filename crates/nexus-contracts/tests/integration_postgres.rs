//! EP-002 M3 integration tests: generated contracts through REAL PostgreSQL.
//!
//! Uses the pinned `postgres:18.4` image (COMPONENT_REGISTRY.yaml,
//! VERSIONS.lock.yaml) in a real ephemeral container - never an in-memory
//! substitute. Readiness is proven by connecting through the PUBLISHED HOST
//! PORT (docker's port-publish can lag pg_isready; the test consumes the host
//! port - EP-001 M5 flake fix). Host ports are dynamically allocated so
//! parallel runs never collide on a fixed port.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use nexus_contracts::{ActionRequest, InvocationContext, NexusControlObject};
use postgres::{Client, NoTls};

const IMAGE: &str = "postgres:18.4";

/// A running ephemeral postgres container with a dynamically published host port.
struct TestPostgres {
    container: String,
    port: u16,
}

impl TestPostgres {
    fn start() -> Self {
        let name = format!(
            "nexus-ep002-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let out = Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                &name,
                "-e",
                "POSTGRES_USER=nexus",
                "-e",
                "POSTGRES_PASSWORD=nexus-test",
                "-e",
                "POSTGRES_DB=nexus",
                "-p",
                "127.0.0.1::5432",
                IMAGE,
            ])
            .output()
            .expect("docker run failed");
        assert!(
            out.status.success(),
            "docker run failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let container = name;
        let port = Self::host_port(&container);
        Self::wait_ready(port);
        Self { container, port }
    }

    fn host_port(container: &str) -> u16 {
        for _ in 0..50 {
            let out = Command::new("docker")
                .args(["port", container, "5432"])
                .output()
                .expect("docker port failed");
            if out.status.success() {
                let line = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Some(port) = line
                    .rsplit(':')
                    .next()
                    .and_then(|p| p.trim().parse::<u16>().ok())
                {
                    return port;
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        panic!("docker port never published for {container}");
    }

    fn wait_ready(port: u16) {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut last: Option<postgres::Error> = None;
        while Instant::now() < deadline {
            let res = Client::connect(
                &format!(
                    "host=127.0.0.1 port={port} user=nexus password=nexus-test dbname=nexus connect_timeout=2"
                ),
                NoTls,
            );
            match res {
                Ok(mut client) => {
                    let _ = client.simple_query("SELECT 1");
                    return;
                }
                Err(e) => {
                    last = Some(e);
                    std::thread::sleep(Duration::from_millis(500));
                }
            }
        }
        panic!("postgres host port {port} not ready within 60s: {last:?}");
    }

    fn client(&self) -> Client {
        Client::connect(
            &format!(
                "host=127.0.0.1 port={} user=nexus password=nexus-test dbname=nexus",
                self.port
            ),
            NoTls,
        )
        .expect("connect to test postgres")
    }
}

impl Drop for TestPostgres {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn sample_control_object() -> NexusControlObject {
    NexusControlObject {
        schema_version: "1.0.0".into(),
        intent: "home.lights.set".into(),
        route: "DETERMINISTIC".into(),
        risk: "R0".into(),
        privacy: "HOUSEHOLD".into(),
        ambiguity: 0.0,
        approval_required: false,
        executable_instruction: true,
        confidence: 0.99,
        required_capabilities: vec!["home.lights.set".into()],
        entities: serde_json::json!({"device": "living-room-1"}),
        escalation_reason: None,
        workflow: None,
    }
}

fn sample_action_request() -> ActionRequest {
    ActionRequest {
        action_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6071".into(),
        tenant_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072".into(),
        principal_id: "user_1".into(),
        capability_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6075".into(),
        idempotency_key: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6076".into(),
        risk: "R3".into(),
        approval_class: "HUMAN".into(),
        reversal: "COMPENSATING".into(),
        arguments: serde_json::json!({"door": "front"}),
        expected_state: serde_json::json!({"locked": true}),
        invocation: InvocationContext {
            request_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073".into(),
            correlation_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6074".into(),
            origin_system: "voice".into(),
            external_actor_id: "user_1".into(),
            external_actor_type: "PERSON".into(),
            channel: Some(Some("voice".into())),
            causation_id: None,
            approval_id: None,
            device_id: None,
            objective_id: None,
            room_id: None,
            task_id: None,
        },
    }
}

/// The generated contracts survive a real SQL round-trip through PostgreSQL.
#[test]
fn ep002_integration_contracts_roundtrip_real_postgres() {
    let pg = TestPostgres::start();
    let mut client = pg.client();

    client
        .batch_execute(
            "CREATE TABLE control_objects (
                id BIGSERIAL PRIMARY KEY,
                intent TEXT NOT NULL,
                payload JSONB NOT NULL
            )",
        )
        .expect("create table");

    let obj = sample_control_object();
    let payload = serde_json::to_value(&obj).expect("serialize control object");
    client
        .execute(
            "INSERT INTO control_objects (intent, payload) VALUES ($1, $2)",
            &[&obj.intent, &payload],
        )
        .expect("insert");

    let row = client
        .query_one(
            "SELECT payload FROM control_objects WHERE intent = $1",
            &[&obj.intent],
        )
        .expect("select");
    let back: NexusControlObject = serde_json::from_value(row.get(0)).expect("deserialize");
    assert_eq!(obj, back);
}

/// Idempotency: a UNIQUE idempotency_key rejects a duplicate ActionRequest
/// insert on the real engine (SPEC-006).
#[test]
fn ep002_integration_idempotency_key_is_unique_in_postgres() {
    let pg = TestPostgres::start();
    let mut client = pg.client();

    client
        .batch_execute(
            "CREATE TABLE action_requests (
                idempotency_key TEXT PRIMARY KEY,
                payload JSONB NOT NULL
            )",
        )
        .expect("create table");

    let req = sample_action_request();
    let payload = serde_json::to_value(&req).expect("serialize");
    let first = client.execute(
        "INSERT INTO action_requests (idempotency_key, payload) VALUES ($1, $2)",
        &[&req.idempotency_key, &payload],
    );
    assert!(first.is_ok(), "first insert must succeed");
    let dup = client.execute(
        "INSERT INTO action_requests (idempotency_key, payload) VALUES ($1, $2)",
        &[&req.idempotency_key, &payload],
    );
    assert!(dup.is_err(), "duplicate idempotency_key must be rejected");
}

/// Cancellation/timeout: a deliberately slow statement is killed by the
/// server's statement_timeout (real timeout mechanism), and the connection
/// remains usable (fail-closed, not wedged).
#[test]
fn ep002_integration_slow_query_cancel_and_recovery() {
    let pg = TestPostgres::start();
    // statement_timeout=1000ms: the server cancels the query after 1s.
    let mut client = Client::connect(
        &format!(
            "host=127.0.0.1 port={} user=nexus password=nexus-test dbname=nexus options='-c statement_timeout=1000'",
            pg.port
        ),
        NoTls,
    )
    .expect("connect with statement_timeout");

    let started = Instant::now();
    let res = client.batch_execute("SELECT pg_sleep(30)");
    let elapsed = started.elapsed();
    assert!(res.is_err(), "statement_timeout must cancel the slow query");
    assert!(
        elapsed < Duration::from_secs(10),
        "statement_timeout must fail fast, took {elapsed:?}"
    );
    // Connection must recover and run a fresh statement (fail-closed, not wedged).
    let recovered = client.simple_query("SELECT 1");
    assert!(
        recovered.is_ok(),
        "connection must recover after cancellation"
    );
}

/// Cleanup: dropping the container removes the resource (docker rm -f in Drop),
/// and a fresh container is fully independent.
#[test]
fn ep002_integration_ephemeral_container_isolation_and_cleanup() {
    let a = TestPostgres::start();
    let mut ca = a.client();
    ca.batch_execute("CREATE TABLE isolated (id INT)")
        .expect("create in a");
    ca.execute("INSERT INTO isolated VALUES (1)", &[])
        .expect("insert in a");
    let n: i64 = ca
        .query_one("SELECT count(*) FROM isolated", &[])
        .expect("count in a")
        .get(0);
    assert_eq!(n, 1);

    let b = TestPostgres::start();
    let mut cb = b.client();
    // Schema isolation: table from container A must not exist in B.
    let err = cb.query_one("SELECT count(*) FROM isolated", &[]);
    assert!(err.is_err(), "containers must be isolated");
    drop(a);
    drop(b);
}
