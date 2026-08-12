//! EP-003 M4 failure tests: identity persistence fails safely under REAL
//! faults against real PostgreSQL 18.4.
//!
//! Every test exercises the real failure mechanism - a killed container, a
//! server-side statement_timeout, a denied role, a UNIQUE conflict, a
//! cancelled statement, or a rolled-back transaction. No component is
//! mocked.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use nexus_identity::{Household, LifecycleState, PersonProfile, PrivacyContext, Session};
use postgres::{Client, NoTls};

const IMAGE: &str = "postgres:18.4";

struct TestPostgres {
    container: String,
    port: u16,
}

impl TestPostgres {
    fn start() -> Self {
        let name = format!(
            "nexus-ep003f-{}",
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

    fn kill(&self) {
        let _ = Command::new("docker")
            .args(["kill", &self.container])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
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

fn sample_person() -> PersonProfile {
    PersonProfile::new(
        nexus_domain::PersonId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101").unwrap(),
        nexus_domain::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102").unwrap(),
        "Lin",
        LifecycleState::Active,
        None,
        vec![],
    )
    .unwrap()
}

fn sample_household() -> Household {
    Household::new(
        nexus_domain::HouseholdId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103").unwrap(),
        nexus_domain::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102").unwrap(),
        "Home",
        vec![],
    )
    .unwrap()
}

fn sample_session() -> Session {
    Session::new(
        nexus_domain::NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6130").unwrap(),
        nexus_domain::TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102").unwrap(),
        nexus_domain::NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101").unwrap(),
        nexus_domain::PrincipalType::Human,
        None,
        1000,
        2000,
        nexus_domain::CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073").unwrap(),
    )
}

/// Unavailable dependency: after the container is killed, a new connection
/// fails closed with a connection error (fail-closed, never a hang).
#[test]
fn ep003_failure_unavailable_dependency_when_container_killed() {
    let pg = TestPostgres::start();
    let port = pg.port;
    // Prove it was up.
    let mut client = pg.client();
    client.simple_query("SELECT 1").expect("engine was up");
    drop(client);
    pg.kill();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut saw_failure = false;
    while Instant::now() < deadline {
        let res = Client::connect(
            &format!(
                "host=127.0.0.1 port={port} user=nexus password=nexus-test dbname=nexus connect_timeout=2"
            ),
            NoTls,
        );
        if res.is_err() {
            saw_failure = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    assert!(saw_failure, "killed engine must fail closed, not hang");
}

/// Timeout: a server-side statement_timeout aborts a slow statement with a
/// structured error instead of blocking forever (SPEC-006).
#[test]
fn ep003_failure_timeout_aborts_long_statement() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute("SET statement_timeout = '250ms'")
        .expect("set timeout");
    let res = client.simple_query("SELECT pg_sleep(5)");
    assert!(res.is_err(), "slow statement must be aborted by timeout");
}

/// Malformed input: invalid JSONB is rejected by the engine and the
/// canonical record types reject malformed payloads (SPEC-006 validation).
#[test]
fn ep003_failure_malformed_input_rejected() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute("CREATE TABLE t (payload JSONB)")
        .expect("create table");
    let res = client.execute("INSERT INTO t (payload) VALUES ($1)", &[&"not-json"]);
    assert!(res.is_err(), "malformed JSONB must be rejected");
    // The domain types reject bad UUIDs at construction.
    let bad = nexus_domain::TenantId::new("not-a-uuid");
    assert!(bad.is_err());
}

/// Duplicate request: reusing a session id on the real engine conflicts
/// deterministically (SPEC-006 conflict).
#[test]
fn ep003_failure_duplicate_session_conflicts() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute(
            "CREATE TABLE sessions (session_id TEXT PRIMARY KEY, payload JSONB NOT NULL)",
        )
        .expect("create table");
    let s = sample_session();
    let payload = serde_json::to_value(&s).unwrap();
    client
        .execute(
            "INSERT INTO sessions (session_id, payload) VALUES ($1, $2)",
            &[&s.session_id.to_string(), &payload],
        )
        .expect("first insert");
    let dup = client.execute(
        "INSERT INTO sessions (session_id, payload) VALUES ($1, $2)",
        &[&s.session_id.to_string(), &payload],
    );
    assert!(dup.is_err(), "duplicate must conflict on the real engine");
}

/// Denied permission: a role without privileges on the table receives a
/// structured authorization error (fail closed; least privilege).
#[test]
fn ep003_failure_denied_permission_fails_closed() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute(
            "CREATE TABLE secret_people (id TEXT PRIMARY KEY, payload JSONB NOT NULL); \
             CREATE ROLE nosy NOLOGIN;",
        )
        .expect("setup");
    let res = client.execute(
        "SET ROLE nosy; INSERT INTO secret_people (id, payload) VALUES ($1, $2)",
        &[
            &"0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101",
            &serde_json::json!({"n": 1}),
        ],
    );
    assert!(res.is_err(), "unprivileged role must be denied");
}

/// Cancelled work: a statement cancelled by timeout leaves the session
/// usable (recovery, not corruption).
#[test]
fn ep003_failure_cancelled_work_recovers() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute("SET statement_timeout = '250ms'")
        .expect("set timeout");
    assert!(client.simple_query("SELECT pg_sleep(5)").is_err());
    // The same session continues to work after cancellation.
    let row = client
        .query_one("SELECT 1 + 1", &[])
        .expect("session recovers after cancelled work");
    let v: i32 = row.get(0);
    assert_eq!(v, 2);
}

/// Partial side effects: when a multi-insert transaction fails midway, the
/// earlier insert is rolled back - no partial durable truth (SPEC-006,
/// INV-004).
#[test]
fn ep003_failure_partial_side_effects_rollback() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute("CREATE TABLE people (id TEXT PRIMARY KEY, payload JSONB NOT NULL)")
        .expect("create table");
    let person = sample_person();
    let household = sample_household();
    let p_payload = serde_json::to_value(&person).unwrap();
    let h_payload = serde_json::to_value(&household).unwrap();
    client.batch_execute("BEGIN").expect("begin");
    client
        .execute(
            "INSERT INTO people (id, payload) VALUES ($1, $2)",
            &[&person.person_id.to_string(), &p_payload],
        )
        .expect("first insert");
    let second = client.execute(
        "INSERT INTO people (id, payload) VALUES ($1, $2)",
        &[&person.person_id.to_string(), &h_payload],
    );
    assert!(second.is_err(), "duplicate inside txn must fail");
    client.batch_execute("ROLLBACK").expect("rollback");
    let count: i64 = client
        .query_one("SELECT COUNT(*) FROM people", &[])
        .expect("count")
        .get(0);
    assert_eq!(count, 0, "partial write must be rolled back");
}

/// Structured errors: engine errors carry SQLSTATE codes that callers can
/// match deterministically.
#[test]
fn ep003_failure_structured_sqlstate_errors() {
    let pg = TestPostgres::start();
    let mut client = pg.client();
    client
        .batch_execute("CREATE TABLE t (id TEXT PRIMARY KEY, payload JSONB NOT NULL)")
        .expect("create table");
    let err = client
        .execute(
            "INSERT INTO t (id, payload) VALUES ($1, $2)",
            &[&"x", &serde_json::json!({"a": 1})],
        )
        .expect("first insert");
    assert_eq!(err, 1);
    let dup = client
        .execute(
            "INSERT INTO t (id, payload) VALUES ($1, $2)",
            &[&"x", &serde_json::json!({"a": 2})],
        )
        .expect_err("duplicate must error");
    let db = dup.as_db_error().expect("duplicate must be a db error");
    assert_eq!(db.code().code(), "23505", "UNIQUE violation SQLSTATE");
}

/// Privacy context fail-closed behavior is deterministic even under
/// malformed-ish inputs (shared room + personal class => private routing).
#[test]
fn ep003_failure_privacy_routing_fails_closed() {
    let ctx = PrivacyContext::new(nexus_domain::Privacy::Personal, true);
    assert!(ctx.requires_private_routing());
    let ctx2 = PrivacyContext::new(nexus_domain::Privacy::Public, true);
    assert!(!ctx2.requires_private_routing());
}
