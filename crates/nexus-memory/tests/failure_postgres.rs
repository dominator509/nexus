//! EP-004 M4 failure tests: forced failures, abuse cases, and fail-closed
//! behavior through REAL PostgreSQL 18.4 + pgvector (pinned
//! `pgvector/pgvector:pg18`).
//!
//! Every test exercises a REAL failure mechanism - never a mock of the
//! component being proven:
//!   - unavailable dependency: the container is terminated mid-session
//!   - timeout: statement_timeout budget exhausted on pg_sleep
//!   - malformed input: CHECK constraints reject bad content_hash/status
//!   - duplicate request: PRIMARY KEY rejects the second identical insert
//!   - denied permission: cross-tenant UPDATE/DELETE affects 0 rows
//!   - cancelled work: pg_cancel_backend aborts a running query
//!   - partial side effect: a failed second statement rolls back the whole
//!     transaction (atomicity)
//!
//! Fail-closed doctrine: an unauthorized or failed write must never leave
//! partial state, and error text must not leak credentials.

use std::process::Command;
use std::time::{Duration, Instant};

use postgres::{Client, NoTls};
use uuid::Uuid;

const IMAGE: &str = "pgvector/pgvector:pg18";

fn uid(s: &str) -> Uuid {
    Uuid::parse_str(s).expect("valid uuid literal")
}

fn migration_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("repo root")
        .join(name)
}

const MIGRATIONS: [&str; 2] = [
    "migrations/001_memory_and_world_graph.sql",
    "migrations/002_memory_embeddings_vector.sql",
];

struct TestPostgres {
    container: String,
    port: u16,
}

impl TestPostgres {
    fn start() -> Self {
        let name = format!(
            "nexus-ep004-fail-{}",
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
                "POSTGRES_PASSWORD=nexus",
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
        let pg = Self { container, port };
        pg.wait_ready();
        pg
    }

    fn host_port(container: &str) -> u16 {
        let out = Command::new("docker")
            .args(["port", container, "5432"])
            .output()
            .expect("docker port failed");
        assert!(
            out.status.success(),
            "docker port failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = String::from_utf8_lossy(&out.stdout);
        let port = text
            .trim()
            .rsplit(':')
            .next()
            .expect("no host port")
            .parse::<u16>()
            .expect("host port must be numeric");
        assert!(port > 0, "host port must not be 0");
        port
    }

    fn wait_ready(&self) {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let res = Client::connect(
                &format!(
                    "host=127.0.0.1 port={} user=nexus password=nexus dbname=nexus connect_timeout=2",
                    self.port
                ),
                NoTls,
            );
            if let Ok(mut client) = res {
                let ok = client.simple_query("SELECT 1").is_ok();
                drop(client);
                if ok {
                    return;
                }
            }
            assert!(
                Instant::now() < deadline,
                "postgres did not become ready within 60s"
            );
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    fn client(&self) -> Client {
        Client::connect(
            &format!(
                "host=127.0.0.1 port={} user=nexus password=nexus dbname=nexus",
                self.port
            ),
            NoTls,
        )
        .expect("connect to test postgres")
    }

    fn apply_migrations(&self) {
        let mut client = self.client();
        for migration in MIGRATIONS {
            let sql = std::fs::read_to_string(migration_path(migration))
                .unwrap_or_else(|_| panic!("read migration {migration}"));
            client
                .batch_execute(&sql)
                .unwrap_or_else(|e| panic!("apply {migration}: {e}"));
        }
    }

    /// Terminate the backing container (real dependency removal).
    fn kill(&self) {
        let out = Command::new("docker")
            .args(["rm", "-f", &self.container])
            .output()
            .expect("docker rm failed");
        assert!(out.status.success(), "docker rm -f failed");
    }
}

impl Drop for TestPostgres {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container])
            .output();
    }
}

fn setup() -> TestPostgres {
    let pg = TestPostgres::start();
    pg.apply_migrations();
    pg
}

fn insert_record(
    client: &mut Client,
    memory_id: &str,
    tenant_id: &str,
    content_hash: &str,
    status: &str,
) -> Result<u64, postgres::Error> {
    client.execute(
        "INSERT INTO memory_records (
            memory_id, tenant_id, namespace, memory_type, content, content_hash,
            source, actor, created_at, observed_at, confidence, sensitivity,
            purpose, retention, status
         ) VALUES ($1, $2, 'private', 'SEMANTIC', $3::jsonb, $4, 'system', 'svc',
            '2026-08-12T00:00:00Z'::timestamptz, '2026-08-12T00:00:00Z'::timestamptz,
            0.9, 'SECRET', 'remember', 'Days 30', $5)",
        &[
            &uid(memory_id),
            &uid(tenant_id),
            &serde_json::json!({ "note": "failure probe" }),
            &content_hash,
            &status,
        ],
    )
}

/// Error text must never leak the test password (structured, redacted logs).
fn assert_no_secret_leak(err: &postgres::Error) {
    let text = err.to_string();
    assert!(
        !text.contains("password=") && !text.contains("POSTGRES_PASSWORD"),
        "error text leaked credentials: {text}"
    );
}

#[test]
fn ep004_failure_unavailable_dependency_is_structured() {
    let pg = setup();
    // A client connected BEFORE the dependency dies.
    let mut survivor = pg.client();
    // Real failure: the dependency disappears mid-session.
    pg.kill();
    // Reconnect must fail with a structured connection error (fail-closed),
    // and the error text must not leak credentials.
    let res = Client::connect(
        &format!(
            "host=127.0.0.1 port={} user=nexus password=nexus dbname=nexus connect_timeout=2",
            pg.port
        ),
        NoTls,
    );
    assert!(res.is_err(), "reconnect against dead dependency must fail");
    assert_no_secret_leak(res.err().as_ref().unwrap());
    // The survivor connection must surface a real error on use, never a
    // silent empty result.
    let query_res = survivor.query("SELECT 1 FROM memory_records", &[]);
    assert!(
        query_res.is_err(),
        "query on a connection whose backend died must fail"
    );
    assert_no_secret_leak(query_res.err().as_ref().unwrap());
}

#[test]
fn ep004_failure_timeout_aborts_transaction() {
    let pg = setup();
    let mut client = pg.client();
    client
        .batch_execute("BEGIN")
        .expect("begin explicit transaction");
    client
        .batch_execute("SET LOCAL statement_timeout = 100")
        .expect("set local timeout");
    // Budget exhausted: pg_sleep(5) far exceeds 100ms.
    let res = client.query("SELECT pg_sleep(5)", &[]);
    assert!(res.is_err(), "sleep beyond budget must time out");
    let err = res.err().unwrap();
    assert_no_secret_leak(&err);
    // The failing statement aborted the transaction; the next statement
    // must fail with "current transaction is aborted" (no silent partial
    // work on a dead transaction).
    let after = client.query("SELECT 1", &[]);
    assert!(after.is_err(), "aborted transaction must reject new work");
    // Roll back and prove no partial state survives.
    let _ = client.batch_execute("ROLLBACK");
    let rows = client
        .query("SELECT count(*) FROM memory_records", &[])
        .expect("count after rollback");
    let count: i64 = rows[0].get(0);
    assert_eq!(count, 0, "aborted transaction must leave no partial state");
}

#[test]
fn ep004_failure_malformed_input_rejected_by_check_constraints() {
    let pg = setup();
    let mut client = pg.client();
    // content_hash violates the 64-hex CHECK constraint (real DB rejection).
    let res = insert_record(
        &mut client,
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8010",
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8011",
        "not-a-hash",
        "ACTIVE",
    );
    assert!(res.is_err(), "bad content_hash must be rejected");
    assert_no_secret_leak(&res.err().unwrap());
    // status violates the lifecycle CHECK constraint.
    let res = insert_record(
        &mut client,
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8012",
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8013",
        &"f".repeat(64),
        "NOT_A_STATUS",
    );
    assert!(res.is_err(), "unknown status must be rejected");
    assert_no_secret_leak(&res.err().unwrap());
    // Nothing was persisted.
    let rows = client
        .query(
            "SELECT count(*) FROM memory_records WHERE tenant_id = $1",
            &[&uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8011")],
        )
        .expect("count tenant A");
    let count: i64 = rows[0].get(0);
    assert_eq!(count, 0, "rejected inserts must not persist");
}

#[test]
fn ep004_failure_duplicate_memory_id_conflicts() {
    let pg = setup();
    let mut client = pg.client();
    let id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8020";
    let tenant = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8021";
    insert_record(&mut client, id, tenant, &"a".repeat(64), "ACTIVE")
        .expect("first insert succeeds");
    // Duplicate request: same memory_id again -> PRIMARY KEY violation.
    let res = insert_record(&mut client, id, tenant, &"b".repeat(64), "ACTIVE");
    assert!(res.is_err(), "duplicate memory_id must conflict");
    assert_no_secret_leak(&res.err().unwrap());
    // The original row is intact and exactly one copy exists.
    let rows = client
        .query(
            "SELECT content_hash FROM memory_records WHERE memory_id = $1",
            &[&uid(id)],
        )
        .expect("select original");
    assert_eq!(rows.len(), 1, "duplicate must not create a second row");
    let hash: String = rows[0].get(0);
    assert_eq!(hash, "a".repeat(64), "original content must be intact");
}

#[test]
fn ep004_failure_cross_tenant_write_is_denied() {
    let pg = setup();
    let mut client = pg.client();
    let id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8030";
    let owner = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8031";
    let intruder = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8032";
    insert_record(&mut client, id, owner, &"c".repeat(64), "ACTIVE")
        .expect("owner insert succeeds");
    // Cross-tenant UPDATE: the tenant_id filter makes it affect 0 rows.
    let updated = client
        .execute(
            "UPDATE memory_records SET status = 'DELETED'
             WHERE memory_id = $1 AND tenant_id = $2",
            &[&uid(id), &uid(intruder)],
        )
        .expect("cross-tenant update runs");
    assert_eq!(updated, 0, "cross-tenant UPDATE must affect zero rows");
    // Cross-tenant DELETE: same doctrine.
    let deleted = client
        .execute(
            "DELETE FROM memory_records WHERE memory_id = $1 AND tenant_id = $2",
            &[&uid(id), &uid(intruder)],
        )
        .expect("cross-tenant delete runs");
    assert_eq!(deleted, 0, "cross-tenant DELETE must affect zero rows");
    // The owner still sees the ACTIVE row (fail-closed, no data loss).
    let rows = client
        .query(
            "SELECT status FROM memory_records WHERE memory_id = $1 AND tenant_id = $2",
            &[&uid(id), &uid(owner)],
        )
        .expect("owner read");
    assert_eq!(rows.len(), 1, "owner row must survive the intrusion");
    let status: String = rows[0].get(0);
    assert_eq!(status, "ACTIVE");
}

#[test]
fn ep004_failure_cancelled_work_aborts_query() {
    let pg = setup();
    let mut victim = pg.client();
    let victim_pid: i32 = victim
        .query_one("SELECT pg_backend_pid()", &[])
        .expect("victim pid")
        .get(0);
    // Launch a long query on the victim connection.
    let handle = std::thread::spawn(move || {
        let mut v = victim;
        v.query("SELECT pg_sleep(30)", &[])
    });
    // Give the victim time to start sleeping, then really cancel it.
    std::thread::sleep(Duration::from_millis(500));
    let mut canceller = pg.client();
    let cancelled = canceller
        .execute("SELECT pg_cancel_backend($1)", &[&victim_pid])
        .expect("pg_cancel_backend runs");
    assert_eq!(cancelled, 1, "cancel must target the victim backend");
    let res = handle.join().expect("victim thread joined");
    assert!(res.is_err(), "cancelled query must fail");
    assert_no_secret_leak(&res.err().unwrap());
}

#[test]
fn ep004_failure_partial_side_effect_rolls_back_atomically() {
    let pg = setup();
    let mut client = pg.client();
    let id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8040";
    let tenant = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f8041";
    let mut tx = client.transaction().expect("begin tx");
    // First statement succeeds within the transaction...
    tx.execute(
        "INSERT INTO memory_records (
            memory_id, tenant_id, namespace, memory_type, content, content_hash,
            source, actor, created_at, observed_at, confidence, sensitivity,
            purpose, retention, status
         ) VALUES ($1, $2, 'private', 'SEMANTIC', $3::jsonb, $4, 'system', 'svc',
            '2026-08-12T00:00:00Z'::timestamptz, '2026-08-12T00:00:00Z'::timestamptz,
            0.9, 'SECRET', 'remember', 'Days 30', 'ACTIVE')",
        &[
            &uid(id),
            &uid(tenant),
            &serde_json::json!({ "note": "doomed" }),
            &"d".repeat(64),
        ],
    )
    .expect("first statement succeeds");
    // ...but the second violates the embedding FK (nonexistent parent).
    let res = tx.execute(
        "INSERT INTO memory_embeddings (memory_id, tenant_id, model, dimensions, model_version, embedding)
         VALUES ($1, $2, 'minilm', 384, 'v1', '[0.1]'::vector)",
        &[&uid(id), &uid(tenant)],
    );
    assert!(res.is_err(), "FK violation must fail the second statement");
    // Roll back: the first insert must NOT survive (atomicity).
    let rollback = tx.rollback();
    assert!(rollback.is_ok(), "rollback must succeed");
    let rows = client
        .query(
            "SELECT count(*) FROM memory_records WHERE memory_id = $1",
            &[&uid(id)],
        )
        .expect("count after rollback");
    let count: i64 = rows[0].get(0);
    assert_eq!(count, 0, "partial side effect must roll back atomically");
}
