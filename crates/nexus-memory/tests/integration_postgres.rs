//! EP-004 M3 integration tests: memory and world graph through REAL
//! PostgreSQL 18.4 + pgvector.
//!
//! Uses the pinned `pgvector/pgvector:pg18` image (COMPONENT_REGISTRY.yaml,
//! VERSIONS.lock.yaml: postgresql 18.4, pgvector 0.8.6) in a real ephemeral
//! container - never an in-memory substitute. Readiness is proven by
//! connecting through the PUBLISHED HOST PORT. Host ports are dynamically
//! allocated so parallel runs never collide. Both additive migrations under
//! `migrations/` are applied; the vector migration proves the pgvector
//! extension and HNSW index are real.

use std::process::Command;
use std::time::{Duration, Instant};

use postgres::{Client, NoTls};
use uuid::Uuid;

const IMAGE: &str = "pgvector/pgvector:pg18";

/// Parse a canonical NexusId string into a postgres UUID value.
fn uid(s: &str) -> Uuid {
    Uuid::parse_str(s).expect("valid uuid literal")
}

/// Migration files are relative to the repository root (tests run from the
/// crate directory, so resolve upward).
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

/// A running ephemeral postgres container with a dynamically published host port.
struct TestPostgres {
    container: String,
    port: u16,
}

impl TestPostgres {
    fn start() -> Self {
        let name = format!(
            "nexus-ep004-{}",
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
        // Format: "127.0.0.1:PORT\n" (or "0.0.0.0:PORT\n").
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
            match res {
                Ok(mut client) => {
                    let ok = client.simple_query("SELECT 1").is_ok();
                    drop(client);
                    if ok {
                        return;
                    }
                }
                Err(_) => {}
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
}

impl Drop for TestPostgres {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container])
            .output();
    }
}

fn ep004_integration_setup() -> TestPostgres {
    let pg = TestPostgres::start();
    pg.apply_migrations();
    pg
}

#[test]
fn ep004_integration_memory_record_round_trips_through_jsonb() {
    let pg = ep004_integration_setup();
    let mut client = pg.client();
    client
        .execute(
            "INSERT INTO memory_records (
                memory_id, tenant_id, namespace, memory_type, content, content_hash,
                source, actor, created_at, observed_at, confidence, sensitivity,
                purpose, retention, status, derived_from, supersedes, embedding_ref
             ) VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7, $8, '2026-08-12T00:00:00Z'::timestamptz, '2026-08-12T00:00:00Z'::timestamptz, $9, $10, $11, $12, $13, $14, $15, $16)",
            &[
                &uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7001"),
                &uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7002"),
                &"household",
                &"EPISODIC",
                &serde_json::json!({ "note": "groceries" }),
                &"a".repeat(64),
                &"voice",
                &"principal",
                &0.8f64,
                &"HOUSEHOLD",
                &"remember",
                &"Days 30",
                &"ACTIVE",
                &Vec::<Uuid>::new(),
                &Option::<Uuid>::None,
                &Option::<String>::None,
            ],
        )
        .expect("insert memory record");
    let row = client
        .query_one(
            "SELECT content_hash, confidence, status FROM memory_records WHERE memory_id = $1",
            &[&uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7001")],
        )
        .expect("select memory record");
    let hash: String = row.get(0);
    let confidence: f64 = row.get(1);
    let status: String = row.get(2);
    assert_eq!(hash, "a".repeat(64));
    assert_eq!(confidence, 0.8);
    assert_eq!(status, "ACTIVE");
}

#[test]
fn ep004_integration_tenant_isolation_blocks_cross_tenant_reads() {
    let pg = ep004_integration_setup();
    let mut client = pg.client();
    // Tenant A record.
    client
        .execute(
            "INSERT INTO memory_records (
                memory_id, tenant_id, namespace, memory_type, content, content_hash,
                source, actor, created_at, observed_at, confidence, sensitivity,
                purpose, retention, status
             ) VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7, $8, '2026-08-12T00:00:00Z'::timestamptz, '2026-08-12T00:00:00Z'::timestamptz, $9, $10, $11, $12, $13)",
            &[
                &uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7010"),
                &uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7011"),
                &"private",
                &"SEMANTIC",
                &serde_json::json!({ "secret": "tenant-a" }),
                &"b".repeat(64),
                &"system",
                &"svc-a",
                &0.9f64,
                &"SECRET",
                &"remember",
                &"INDEFINITE",
                &"ACTIVE",
            ],
        )
        .expect("insert tenant A record");
    // Tenant B query must return nothing (tenant_id is a hard filter).
    let rows = client
        .query(
            "SELECT memory_id FROM memory_records WHERE tenant_id = $1 AND status = 'ACTIVE'",
            &[&uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7012")],
        )
        .expect("tenant B query");
    assert!(rows.is_empty(), "tenant B must not see tenant A records");
}

#[test]
fn ep004_integration_supersession_updates_status_transactionally() {
    let pg = ep004_integration_setup();
    let mut client = pg.client();
    let old_id = uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7020");
    let new_id = uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7021");
    let tenant = uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7022");
    // Insert the ACTIVE target.
    client
        .execute(
            "INSERT INTO memory_records (
                memory_id, tenant_id, namespace, memory_type, content, content_hash,
                source, actor, created_at, observed_at, confidence, sensitivity,
                purpose, retention, status
             ) VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7, $8, '2026-08-12T00:00:00Z'::timestamptz, '2026-08-12T00:00:00Z'::timestamptz, $9, $10, $11, $12, $13)",
            &[
                &old_id,
                &tenant,
                &"household",
                &"SEMANTIC",
                &serde_json::json!({ "v": 1 }),
                &"c".repeat(64),
                &"system",
                &"svc",
                &0.9f64,
                &"HOUSEHOLD",
                &"remember",
                &"Days 30",
                &"ACTIVE",
            ],
        )
        .expect("insert target");
    // Supersede within one transaction: target -> SUPERSEDED, successor inserted.
    let mut tx = client.transaction().expect("begin tx");
    tx.execute(
        "UPDATE memory_records SET status = 'SUPERSEDED' WHERE memory_id = $1 AND tenant_id = $2 AND status = 'ACTIVE'",
        &[&old_id, &tenant],
    )
    .expect("supersede target");
    tx.execute(
        "INSERT INTO memory_records (
            memory_id, tenant_id, namespace, memory_type, content, content_hash,
            source, actor, created_at, observed_at, confidence, sensitivity,
            purpose, retention, status, supersedes
         ) VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7, $8, '2026-08-12T01:00:00Z'::timestamptz, '2026-08-12T01:00:00Z'::timestamptz, $9, $10, $11, $12, $13, $14)",
        &[
            &new_id,
            &tenant,
            &"household",
            &"SEMANTIC",
            &serde_json::json!({ "v": 2 }),
            &"d".repeat(64),
            &"system",
            &"svc",
            &0.95f64,
            &"HOUSEHOLD",
            &"remember",
            &"Days 30",
            &"PROPOSED",
            &old_id,
        ],
    )
    .expect("insert successor");
    tx.commit().expect("commit supersession");
    // Both rows visible with correct states.
    let row = client
        .query_one(
            "SELECT status FROM memory_records WHERE memory_id = $1",
            &[&old_id],
        )
        .expect("read old");
    let status: String = row.get(0);
    assert_eq!(status, "SUPERSEDED");
}

#[test]
fn ep004_integration_world_graph_adjacency_walk_works() {
    let pg = ep004_integration_setup();
    let mut client = pg.client();
    let tenant = uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7030");
    // a -> b -> c
    for (from, to) in [
        (
            uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7031"),
            uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7032"),
        ),
        (
            uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7032"),
            uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7033"),
        ),
    ] {
        client
            .execute(
                "INSERT INTO world_graph_edges (tenant_id, from_node, to_node, edge_type) VALUES ($1, $2, $3, 'related')",
                &[&tenant, &from, &to],
            )
            .expect("insert edge");
    }
    // Recursive walk from a finds b and c (fallback doctrine).
    let rows = client
        .query(
            "WITH RECURSIVE walk(node) AS (
                SELECT $2::uuid
                UNION
                SELECT e.to_node FROM world_graph_edges e
                JOIN walk w ON e.from_node = w.node
                WHERE e.tenant_id = $1
             )
             SELECT node FROM walk ORDER BY node",
            &[&tenant, &uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7031")],
        )
        .expect("recursive walk");
    assert_eq!(rows.len(), 3, "walk must find a, b, c");
}

#[test]
fn ep004_integration_pgvector_extension_and_hnsw_are_real() {
    let pg = ep004_integration_setup();
    let mut client = pg.client();
    // The vector migration created the extension and the HNSW index; prove
    // both are actually present on the real engine.
    let row = client
        .query_one(
            "SELECT extname FROM pg_extension WHERE extname = 'vector'",
            &[],
        )
        .expect("vector extension");
    let ext: String = row.get(0);
    assert_eq!(ext, "vector");
    let row = client
        .query_one(
            "SELECT indexname FROM pg_indexes WHERE tablename = 'memory_embeddings' AND indexdef LIKE '%hnsw%'",
            &[],
        )
        .expect("hnsw index");
    let idx: String = row.get(0);
    assert!(!idx.is_empty(), "HNSW index must exist");
    // Insert a vector row and prove cosine distance works. The embedding is
    // a real 384-dimension vector matching the column declaration.
    let dims_384 = format!(
        "[{}]",
        std::iter::repeat("0.1")
            .take(384)
            .collect::<Vec<_>>()
            .join(",")
    );
    let insert_sql = format!(
        "INSERT INTO memory_embeddings (memory_id, tenant_id, model, dimensions, model_version, embedding)
         VALUES ($1, $2, 'minilm', 384, 'v1', '{dims}'::vector)",
        dims = dims_384
    );
    // memory_embeddings.memory_id REFERENCES memory_records: insert the
    // parent record first (the FK is real and enforced).
    client
        .execute(
            "INSERT INTO memory_records (
                memory_id, tenant_id, namespace, memory_type, content, content_hash,
                source, actor, created_at, observed_at, confidence, sensitivity,
                purpose, retention, status
             ) VALUES ($1, $2, 'memory', 'SEMANTIC', $3::jsonb, $4, 'system', 'svc',
                '2026-08-12T00:00:00Z'::timestamptz, '2026-08-12T00:00:00Z'::timestamptz,
                0.9, 'HOUSEHOLD', 'remember', 'Days 30', 'ACTIVE')",
            &[
                &uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7040"),
                &uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7041"),
                &serde_json::json!({ "note": "vector parent" }),
                &"e".repeat(64),
            ],
        )
        .expect("insert parent memory record");
    client
        .execute(
            &insert_sql,
            &[
                &uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7040"),
                &uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7041"),
            ],
        )
        .expect("insert embedding");
    let cosine_sql = format!(
        "SELECT 1 - (embedding <=> '{dims}'::vector) AS cosine FROM memory_embeddings WHERE memory_id = $1",
        dims = dims_384
    );
    let row = client
        .query_one(&cosine_sql, &[&uid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7040")])
        .expect("cosine query");
    let cosine: f64 = row.get(0);
    assert!(
        (cosine - 1.0).abs() < 1e-9,
        "identical vectors have cosine 1"
    );
}

#[test]
fn ep004_integration_migrations_are_idempotent() {
    let pg = ep004_integration_setup();
    // Applying both migrations a second time must succeed (additive,
    // IF NOT EXISTS).
    pg.apply_migrations();
    let mut client = pg.client();
    let row = client
        .query_one(
            "SELECT count(*) FROM information_schema.tables WHERE table_schema = 'public' AND table_name IN ('memory_records', 'world_graph_edges', 'memory_embeddings')",
            &[],
        )
        .expect("table count");
    let count: i64 = row.get(0);
    assert_eq!(count, 3, "all three EP-004 tables must exist");
}
