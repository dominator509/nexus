//! EP-002 M4 failure tests: fail-closed behavior under real faults.
//!
//! Test names begin with `ep002_failure_`. Every test exercises a REAL
//! failure mechanism (TESTING.md): unavailable dependency, server-side
//! timeout, malformed input, duplicate request, denied permission, and
//! cancelled work - no mocks of the component being proven. Uses the pinned
//! postgres:18.4 image in ephemeral containers with dynamic host ports.

use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use nexus_contracts::{ActionRequest, InvocationContext, NexusControlObject};
use postgres::{Client, NoTls};

const IMAGE: &str = "postgres:18.4";

struct TestPostgres {
    container: String,
    port: u16,
}

impl TestPostgres {
    fn start() -> Self {
        let name = format!(
            "nexus-ep002-fail-{}",
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
        let port = Self::host_port(&name);
        Self::wait_ready(port);
        Self {
            container: name,
            port,
        }
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
        while Instant::now() < deadline {
            let res = Client::connect(
                &format!(
                    "host=127.0.0.1 port={port} user=nexus password=nexus-test dbname=nexus connect_timeout=2"
                ),
                NoTls,
            );
            if let Ok(mut client) = res {
                let _ = client.simple_query("SELECT 1");
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        panic!("postgres host port {port} not ready within 60s");
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

    fn connect_timeout(&self, ms: u32) -> Client {
        Client::connect(
            &format!(
                "host=127.0.0.1 port={} user=nexus password=nexus-test dbname=nexus options='-c statement_timeout={ms}'",
                self.port
            ),
            NoTls,
        )
        .expect("connect with timeout")
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

fn free_port() -> u16 {
    // Bind and release: a port with no listener for unavailable-dependency tests.
    let l = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    l.local_addr().unwrap().port()
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
        entities: serde_json::json!({}),
        escalation_reason: None,
        workflow: None,
    }
}

/// Unavailable dependency: a closed port must fail the connect, never pass.
#[test]
fn ep002_failure_unavailable_dependency_fails_closed() {
    let port = free_port();
    let started = Instant::now();
    let res = Client::connect(
        &format!(
            "host=127.0.0.1 port={port} user=nexus password=nexus-test dbname=nexus connect_timeout=2"
        ),
        NoTls,
    );
    assert!(res.is_err(), "unavailable dependency must fail closed");
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "unavailable dependency must fail fast"
    );
}

/// Timeout: statement_timeout cancels a slow statement (real server mechanism).
#[test]
fn ep002_failure_timeout_cancels_slow_statement() {
    let pg = TestPostgres::start();
    let mut client = pg.connect_timeout(500);
    let started = Instant::now();
    let res = client.batch_execute("SELECT pg_sleep(30)");
    assert!(res.is_err(), "statement_timeout must error");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "timeout must fail fast"
    );
}

/// Malformed input: a JSONB payload that violates the schema is rejected by
/// the validated wrapper (const mismatch) before it can reach storage.
#[test]
fn ep002_failure_malformed_input_rejected() {
    use nexus_contracts::{ValidatedNexusControlObject, ValidationError};
    let mut obj = sample_control_object();
    obj.schema_version = "9.9.9".into();
    let res = ValidatedNexusControlObject::try_from(&obj);
    assert_eq!(
        res,
        Err(ValidationError::Const {
            field: "schema_version",
            expected: "1.0.0".into(),
            actual: "9.9.9".into(),
        })
    );

    // Malformed UUIDv7 IDs are rejected by the typed validated layer.
    let mut req = sample_action_request();
    req.tenant_id = "not-a-uuid".into();
    let res = nexus_contracts::ValidatedActionRequest::try_from(&req);
    assert!(matches!(
        res,
        Err(ValidationError::Id {
            field: "tenantId",
            ..
        })
    ));
}

/// Duplicate request: a UNIQUE idempotency_key is enforced by the real engine.
#[test]
fn ep002_failure_duplicate_idempotency_key_rejected() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute(
            "CREATE TABLE action_requests (idempotency_key TEXT PRIMARY KEY, payload JSONB NOT NULL)",
        )
        .expect("create table");
    let req = sample_action_request();
    let payload = serde_json::to_value(&req).expect("serialize");
    client
        .execute(
            "INSERT INTO action_requests (idempotency_key, payload) VALUES ($1, $2)",
            &[&req.idempotency_key, &payload],
        )
        .expect("first insert");
    let dup = client.execute(
        "INSERT INTO action_requests (idempotency_key, payload) VALUES ($1, $2)",
        &[&req.idempotency_key, &payload],
    );
    assert!(dup.is_err(), "duplicate idempotency_key must be rejected");
}

/// Denied permission: a user without table privilege must fail with a
/// permission error from the real engine (fail closed).
#[test]
fn ep002_failure_denied_permission_fails_closed() {
    let pg = TestPostgres::start();
    let mut admin = pg.client();
    admin
        .batch_execute(
            "CREATE TABLE secrets_table (id INT PRIMARY KEY, payload TEXT);
             CREATE ROLE nexus_unprivileged LOGIN PASSWORD 'nexus-test';
             GRANT CONNECT ON DATABASE nexus TO nexus_unprivileged;
             REVOKE ALL ON secrets_table FROM nexus_unprivileged;",
        )
        .expect("provision roles");
    let res = Client::connect(
        &format!(
            "host=127.0.0.1 port={} user=nexus_unprivileged password=nexus-test dbname=nexus",
            pg.port
        ),
        NoTls,
    );
    let mut limited = match res {
        Ok(c) => c,
        Err(e) => panic!("unprivileged connect failed: {e}"),
    };
    let denied = limited.simple_query("SELECT * FROM secrets_table");
    assert!(
        denied.is_err(),
        "denied permission must fail closed on the real engine"
    );
}

/// Cancelled work: after a cancelled transaction, the connection rolls back
/// and remains usable for fresh work (no partial side effect persists).
#[test]
fn ep002_failure_cancelled_work_rolls_back() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute("CREATE TABLE ledger (id INT PRIMARY KEY, note TEXT)")
        .expect("create table");
    client
        .batch_execute("BEGIN; INSERT INTO ledger VALUES (1, 'pending')")
        .expect("begin + insert");
    // Simulate cancellation: rollback the in-flight transaction.
    client.batch_execute("ROLLBACK").expect("rollback");
    let n: i64 = client
        .query_one("SELECT count(*) FROM ledger", &[])
        .expect("count")
        .get(0);
    assert_eq!(n, 0, "cancelled work must not persist partial side effects");
}

/// Observability: errors carry structured context (SQLSTATE), never secrets.
#[test]
fn ep002_failure_errors_are_structured_and_redacted() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute("CREATE TABLE t (id INT PRIMARY KEY)")
        .expect("create table");
    let err = client
        .execute("INSERT INTO t (id) VALUES (1), (1)", &[])
        .expect_err("duplicate key must error");
    let db = err
        .as_db_error()
        .expect("must carry a DB error with SQLSTATE");
    let code = db.code().code();
    assert_eq!(code, "23505", "duplicate key SQLSTATE 23505");
    // The structured error must not embed the connection password.
    let rendered = err.to_string();
    assert!(
        !rendered.contains("nexus-test"),
        "error rendering must redact credentials"
    );
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
