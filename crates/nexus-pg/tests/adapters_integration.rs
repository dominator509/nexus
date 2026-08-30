//! RX-005 AUD-007 integration tests: production PostgreSQL adapters.
//!
//! These tests exercise the CONCRETE `nexus-pg` adapters (`PgUnitOfWork`,
//! `PgMemoryRepository`, `PgWorldGraphRepository`, `PgVectorRepository`,
//! `PgRepositorySet`) against REAL PostgreSQL 18.4 + pgvector
//! (`pgvector/pgvector:pg18`, pinned in VERSIONS.lock.yaml) in an ephemeral
//! container - never a mock, never raw SQL bypassing the production ports.
//!
//! The old EP-004 integration tests drove raw SQL; AUD-007's root cause was
//! precisely that the "real PostgreSQL integration test" bypassed the
//! production abstractions. This suite proves the production path:
//! repository operations flow through the ports, tenant isolation is
//! enforced by RLS at the database boundary, and the composite FK binds an
//! embedding to its parent record's tenant.

use std::process::Command;
use std::time::{Duration, Instant};

use nexus_data::{
    MemoryProposal, MemoryQuery, MemoryRecord, MemoryRepository, MemoryStatus, RepositorySet,
    RetentionPolicy, RetentionUnit, Sensitivity, UnitOfWork, VectorRepository,
    WorldGraphRepository,
};
use nexus_domain::{CorrelationId, EventId, MemoryType, NexusId, TenantId};
use nexus_events::{
    EventDataClass, EventEnvelope, EventType, InboxRepository, InboxStatus, OutboxRepository,
    OutboxStatus,
};
use nexus_pg::{
    PgInboxRepository, PgMemoryRepository, PgOutboxRepository, PgRepositorySet, PgUnitOfWork,
    PgVectorRepository, PgWorldGraphRepository,
};
use postgres::{Client, NoTls};
use uuid::Uuid;

const IMAGE: &str = "pgvector/pgvector:pg18";

fn uid(s: &str) -> Uuid {
    Uuid::parse_str(s).expect("valid uuid literal")
}

fn nid(s: &str) -> NexusId {
    NexusId::new(s).expect("valid nexus id literal")
}

fn tid(s: &str) -> TenantId {
    TenantId::new(s).expect("valid tenant id literal")
}

fn migration_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate parent")
        .parent()
        .expect("repo root")
        .join(name)
}

const MIGRATIONS: [&str; 4] = [
    "migrations/001_memory_and_world_graph.sql",
    "migrations/002_memory_embeddings_vector.sql",
    "migrations/003_tenant_isolation_rls.sql",
    "migrations/004_outbox_inbox.sql",
];

/// A running ephemeral postgres container with a dynamically published host port.
struct TestPostgres {
    container: String,
    port: u16,
}

impl TestPostgres {
    fn start() -> Self {
        let name = format!(
            "nexus-rx005-{}",
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
        let port = Self::host_port(&name);
        let pg = Self {
            container: name,
            port,
        };
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

fn record(id: &str, tenant: &str, content: serde_json::Value, status: MemoryStatus) -> MemoryRecord {
    MemoryRecord {
        memory_id: nid(id),
        tenant_id: tid(tenant),
        namespace: "household".to_string(),
        memory_type: MemoryType::Semantic,
        content,
        content_hash: "a".repeat(64),
        source: "test".to_string(),
        actor: "principal".to_string(),
        created_at: "2026-08-12T00:00:00Z".to_string(),
        observed_at: "2026-08-12T00:00:00Z".to_string(),
        confidence: 0.8,
        sensitivity: Sensitivity::Household,
        purpose: "remember".to_string(),
        retention: RetentionPolicy::for_duration(RetentionUnit::Days, 30),
        status,
        derived_from: vec![],
        supersedes: None,
        embedding_ref: None,
    }
}

fn envelope(seed: u8) -> EventEnvelope {
    EventEnvelope {
        event_id: EventId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc0{seed:02x}")).unwrap(),
        event_type: EventType::new("memory.record.created").unwrap(),
        schema_version: "1.0.0".to_string(),
        source: "integration".to_string(),
        subject: "ignored".to_string(),
        time: "2026-08-12T00:00:00Z".to_string(),
        tenant_id: tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc002"),
        actor: "principal".to_string(),
        correlation_id: CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5fc010").unwrap(),
        causation_id: None,
        data_class: EventDataClass::Household,
        payload: serde_json::json!({ "seed": seed }),
    }
}

#[test]
fn rx005_production_memory_repository_propose_activate_get_round_trip() {
    let pg = setup();
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
    let tenant = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7002");
    let mem_id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7001";
    let mut repo = PgMemoryRepository::new(&uow);

    repo.propose(
        tenant.clone(),
        MemoryProposal {
            record: record(mem_id, tenant.as_str(), serde_json::json!({ "note": "groceries" }), MemoryStatus::Proposed),
        },
    )
    .expect("propose through adapter");

    repo.activate(tenant.clone(), nid(mem_id)).expect("activate");
    let got = repo.get(tenant.clone(), nid(mem_id)).expect("get");
    assert_eq!(got.status, MemoryStatus::Active);
    assert_eq!(got.content, serde_json::json!({ "note": "groceries" }));
    assert_eq!(got.retention, RetentionPolicy::for_duration(RetentionUnit::Days, 30));
    uow.commit().expect("commit");
}

#[test]
fn rx005_production_unit_of_work_rollback_discards_writes() {
    let pg = setup();
    let tenant = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7012");
    let mem_id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7011";
    {
        let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
        let mut repo = PgMemoryRepository::new(&uow);
        repo.propose(
            tenant.clone(),
            MemoryProposal {
                record: record(mem_id, tenant.as_str(), serde_json::json!({ "v": 1 }), MemoryStatus::Proposed),
            },
        )
        .expect("propose");
        uow.rollback().expect("rollback");
    }
    // A fresh unit of work must not see the rolled-back write.
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow 2");
    let mut repo = PgMemoryRepository::new(&uow);
    let err = repo.get(tenant.clone(), nid(mem_id)).unwrap_err();
    assert_eq!(err.code(), nexus_data::DataErrorCode::Conflict);
    uow.commit().expect("commit 2");
}

#[test]
fn rx005_production_world_graph_walk_through_adapter() {
    let pg = setup();
    let tenant = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7030");
    let a = nid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7031");
    let b = nid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7032");
    let c = nid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7033");

    // Seed edges (a -> b -> c) before opening the unit of work so the
    // adapter's transaction sees them.
    let mut seeder = pg.client();
    seeder
        .execute(
            "INSERT INTO world_graph_edges (tenant_id, from_node, to_node, edge_type) VALUES ($1, $2, $3, 'related')",
            &[&uid(tenant.as_str()), &uid(a.as_str()), &uid(b.as_str())],
        )
        .expect("seed a->b");
    seeder
        .execute(
            "INSERT INTO world_graph_edges (tenant_id, from_node, to_node, edge_type) VALUES ($1, $2, $3, 'related')",
            &[&uid(tenant.as_str()), &uid(b.as_str()), &uid(c.as_str())],
        )
        .expect("seed b->c");
    drop(seeder);

    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
    let mut repo = PgWorldGraphRepository::new(&uow);

    let walked = repo.walk(tenant.clone(), a.clone(), 2).expect("walk");
    assert!(walked.contains(&a));
    assert!(walked.contains(&b));
    assert!(walked.contains(&c));
    assert!(repo.follow(tenant.clone(), a.clone(), b.clone()).expect("follow a->b"));
    assert!(!repo.follow(tenant.clone(), c.clone(), a.clone()).expect("follow c->a is absent"));
    uow.commit().expect("commit");
}

#[test]
fn rx005_production_vector_repository_upsert_and_nearest() {
    let pg = setup();
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
    let tenant = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7041");
    let mem_id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7040";

    // Parent memory record must exist (composite FK is real).
    {
        let mut repo = PgMemoryRepository::new(&uow);
        repo.propose(
            tenant.clone(),
            MemoryProposal {
                record: record(mem_id, tenant.as_str(), serde_json::json!({ "note": "vector parent" }), MemoryStatus::Proposed),
            },
        )
        .expect("propose parent");
        repo.activate(tenant.clone(), nid(mem_id)).expect("activate parent");
    }

    let dims = vec![0.1f32; 384];
    let mut vrepo = PgVectorRepository::new(&uow);
    vrepo
        .upsert_vector(tenant.clone(), nid(mem_id), dims.clone())
        .expect("upsert vector");

    let near = vrepo
        .nearest(tenant.clone(), &dims, 1)
        .expect("nearest");
    assert_eq!(near.len(), 1);
    assert_eq!(near[0].record.memory_id, nid(mem_id));
    assert!((near[0].score - 1.0).abs() < 1e-9, "identical vectors cosine ~1");
    uow.commit().expect("commit");
}

#[test]
fn rx005_production_repository_set_binds_all_repos_to_one_uow() {
    let pg = setup();
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
    let tenant = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7052");
    let mem_id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7051";
    let mut set = PgRepositorySet::new(&uow, tenant.clone());
    assert_eq!(set.tenant(), tenant);

    {
        let repo = set.memory().expect("memory accessor");
        repo.propose(
            tenant.clone(),
            MemoryProposal {
                record: record(mem_id, tenant.as_str(), serde_json::json!({ "note": "set" }), MemoryStatus::Proposed),
            },
        )
        .expect("propose through set");
        repo.activate(tenant.clone(), nid(mem_id)).expect("activate through set");
    }
    {
        let vrepo = set.vector().expect("vector accessor");
        vrepo
            .upsert_vector(tenant.clone(), nid(mem_id), vec![0.2f32; 384])
            .expect("upsert through set");
    }
    {
        let grepo = set.world_graph().expect("world graph accessor");
        // No edges seeded; walk returns just the start node.
        let walked = grepo.walk(tenant.clone(), nid(mem_id), 1).expect("walk through set");
        assert!(walked.contains(&nid(mem_id)));
    }
    uow.commit().expect("commit");
}

#[test]
fn rx005_rls_blocks_cross_tenant_access_at_database_boundary() {
    let pg = setup();
    // Seed a record for tenant A as the nexus superuser (bypasses RLS) via
    // the production adapter, then prove tenant B cannot see it even with
    // raw SQL - the RLS policy, not the application, is the enforcement.
    {
        let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
        let tenant_a = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7061");
        let mem_id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7060";
        let mut repo = PgMemoryRepository::new(&uow);
        repo.propose(
            tenant_a.clone(),
            MemoryProposal {
                record: record(mem_id, tenant_a.as_str(), serde_json::json!({ "secret": "tenant-a" }), MemoryStatus::Proposed),
            },
        )
        .expect("propose tenant A");
        repo.activate(tenant_a.clone(), nid(mem_id)).expect("activate tenant A");
        uow.commit().expect("commit");
    }

    // A non-superuser role (RLS subject) with the tenant B claim must not
    // see tenant A's row, even via a raw SELECT that omits the tenant
    // filter - the database boundary enforces isolation.
    let mut admin = pg.client();
    admin
        .simple_query("CREATE ROLE nexus_rls_test LOGIN PASSWORD 'rls'")
        .expect("create role");
    admin
        .simple_query("GRANT SELECT, INSERT, UPDATE, DELETE ON memory_records, world_graph_edges, memory_embeddings TO nexus_rls_test")
        .expect("grant");
    drop(admin);

    let mut rls_client = Client::connect(
        &format!(
            "host=127.0.0.1 port={} user=nexus_rls_test password=rls dbname=nexus",
            pg.port
        ),
        NoTls,
    )
    .expect("connect as rls role");
    // Claim tenant B (not the owner of the seeded row) inside an explicit
    // transaction - the same usage as the production adapters, whose
    // set_config('app.tenant_id', $1, true) is transaction-local.
    rls_client.simple_query("BEGIN").expect("begin tenant B tx");
    rls_client
        .execute(
            "SELECT set_config('app.tenant_id', $1, true)",
            &[&"0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7062"],
        )
        .expect("set tenant B claim");
    let rows = rls_client
        .query(
            "SELECT memory_id FROM memory_records WHERE status = 'ACTIVE'",
            &[],
        )
        .expect("tenant B raw query");
    assert!(
        rows.is_empty(),
        "tenant B must not see tenant A records through RLS"
    );
    rls_client.simple_query("COMMIT").expect("commit tenant B tx");

    // Claim tenant A: the same raw query now sees the row.
    rls_client.simple_query("BEGIN").expect("begin tenant A tx");
    rls_client
        .execute(
            "SELECT set_config('app.tenant_id', $1, true)",
            &[&"0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7061"],
        )
        .expect("set tenant A claim");
    let rows = rls_client
        .query(
            "SELECT memory_id FROM memory_records WHERE status = 'ACTIVE'",
            &[],
        )
        .expect("tenant A raw query");
    assert_eq!(rows.len(), 1, "tenant A sees its own record through RLS");
    rls_client.simple_query("COMMIT").expect("commit tenant A tx");

    // Fail-closed probes (regression guard for the NULLIF hardening in
    // migration 003): a session that never claimed a tenant, and a session
    // whose transaction-local claim has expired ('' placeholder), must see
    // zero rows and must NOT raise E22P02. An unguarded
    // current_setting(...)::uuid would error on '' instead of denying.
    // Probes run as the non-superuser RLS subject - superusers bypass RLS.
    let rls_conn = format!(
        "host=127.0.0.1 port={} user=nexus_rls_test password=rls dbname=nexus",
        pg.port
    );
    let mut unset = Client::connect(&rls_conn, NoTls).expect("connect as rls role (unset)");
    let rows = unset
        .query(
            "SELECT memory_id FROM memory_records WHERE status = 'ACTIVE'",
            &[],
        )
        .expect("unset claim must not error");
    assert!(rows.is_empty(), "unset claim denies all rows");
    drop(unset);

    let mut expired = Client::connect(&rls_conn, NoTls).expect("connect as rls role (expired)");
    expired
        .execute(
            "SELECT set_config('app.tenant_id', $1, true)",
            &[&"0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7061"],
        )
        .expect("set claim in implicit transaction");
    let rows = expired
        .query(
            "SELECT memory_id FROM memory_records WHERE status = 'ACTIVE'",
            &[],
        )
        .expect("expired claim must not error");
    assert!(rows.is_empty(), "expired claim denies all rows");
}

#[test]
fn rx005_composite_fk_binds_embedding_to_parent_tenant() {
    let pg = setup();
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
    let tenant_a = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7071");
    let mem_id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7070";
    {
        let mut repo = PgMemoryRepository::new(&uow);
        repo.propose(
            tenant_a.clone(),
            MemoryProposal {
                record: record(mem_id, tenant_a.as_str(), serde_json::json!({ "note": "parent" }), MemoryStatus::Proposed),
            },
        )
        .expect("propose parent");
        repo.activate(tenant_a.clone(), nid(mem_id)).expect("activate");
    }
    // The composite FK is (tenant_id, memory_id) -> memory_records. An
    // embedding row for tenant B referencing tenant A's memory_id must be
    // rejected by the database - application-level checks are no longer the
    // only protection.
    let tenant_b = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7072");
    let dims = format!("[{}]", vec!["0.1"; 384].join(","));
    let mut fk_client = pg.client();
    let res = fk_client.execute(
        &format!(
            "INSERT INTO memory_embeddings (memory_id, tenant_id, model, dimensions, model_version, embedding)
             VALUES ($1, $2, 'minilm', 384, 'v1', '{dims}'::vector)"
        ),
        &[&uid(mem_id), &uid(tenant_b.as_str())],
    );
    assert!(
        res.is_err(),
        "cross-tenant embedding insert must be rejected by composite FK"
    );
    uow.commit().expect("commit");
}

#[test]
fn rx005_migrations_are_idempotent_with_rls() {
    let pg = setup();
    // Applying all three migrations a second time must succeed (additive,
    // IF NOT EXISTS / DO-block guarded).
    pg.apply_migrations();
    let mut client = pg.client();
    let row = client
        .query_one(
            "SELECT count(*) FROM pg_policies WHERE schemaname = 'public' AND tablename IN ('memory_records', 'world_graph_edges', 'memory_embeddings')",
            &[],
        )
        .expect("policy count");
    let count: i64 = row.get(0);
    assert_eq!(count, 3, "three RLS policies must exist");
    let row = client
        .query_one(
            "SELECT count(*) FROM pg_constraint WHERE conname = 'memory_embeddings_tenant_memory_fkey'",
            &[],
        )
        .expect("fk count");
    let fk_count: i64 = row.get(0);
    assert_eq!(fk_count, 1, "composite tenant FK must exist");
    // Migration 004's ledger tables must also exist after double apply.
    let row = client
        .query_one(
            "SELECT count(*) FROM pg_tables
             WHERE schemaname = 'public' AND tablename IN ('outbox', 'inbox')",
            &[],
        )
        .expect("ledger table count");
    let table_count: i64 = row.get(0);
    assert_eq!(table_count, 2, "outbox and inbox tables must exist");
}

#[test]
fn rx005_production_query_honors_namespace_and_sensitivity_ceiling() {
    let pg = setup();
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
    let tenant = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7081");
    let mut repo = PgMemoryRepository::new(&uow);

    // One HOUSEHOLD record in namespace household, one SECRET record.
    let mut r1 = record(
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7080",
        tenant.as_str(),
        serde_json::json!({ "note": "public note" }),
        MemoryStatus::Active,
    );
    r1.namespace = "household".to_string();
    repo.propose(
        tenant.clone(),
        MemoryProposal { record: r1.clone() },
    )
    .expect("propose r1");
    repo.activate(tenant.clone(), r1.memory_id.clone()).expect("activate r1");

    let mut r2 = record(
        "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7083",
        tenant.as_str(),
        serde_json::json!({ "note": "classified" }),
        MemoryStatus::Active,
    );
    r2.namespace = "business".to_string();
    r2.sensitivity = Sensitivity::Secret;
    repo.propose(tenant.clone(), MemoryProposal { record: r2.clone() })
        .expect("propose r2");
    repo.activate(tenant.clone(), r2.memory_id.clone()).expect("activate r2");

    // Sensitivity ceiling HOUSEHOLD excludes the SECRET record even though
    // both are ACTIVE.
    let q = MemoryQuery {
        max_sensitivity: Some(Sensitivity::Household),
        ..Default::default()
    };
    let candidates = repo.query(tenant.clone(), &q).expect("query");
    assert_eq!(candidates.len(), 1, "sensitivity ceiling filters SECRET");
    assert_eq!(candidates[0].record.memory_id, r1.memory_id);

    uow.commit().expect("commit");
}

#[test]
fn rx005_production_supersede_delete_and_vector_remove_lifecycle() {
    let pg = setup();
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
    let tenant = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7092");
    let old_id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7090";
    let new_id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7091";
    let mut repo = PgMemoryRepository::new(&uow);

    // Propose + activate the old record.
    repo.propose(
        tenant.clone(),
        MemoryProposal {
            record: record(
                old_id,
                tenant.as_str(),
                serde_json::json!({ "note": "old" }),
                MemoryStatus::Proposed,
            ),
        },
    )
    .expect("propose old");
    repo.activate(tenant.clone(), nid(old_id))
        .expect("activate old");

    // Supersede it with a new record; the new one carries supersedes=old
    // and the old one transitions to SUPERSEDED.
    let mut new_record = record(
        new_id,
        tenant.as_str(),
        serde_json::json!({ "note": "new" }),
        MemoryStatus::Active,
    );
    new_record.supersedes = Some(nid(old_id));
    repo.supersede(tenant.clone(), nid(old_id), new_record.clone())
        .expect("supersede");

    let old = repo.get(tenant.clone(), nid(old_id)).expect("get old");
    assert_eq!(old.status, MemoryStatus::Superseded, "old record is SUPERSEDED");
    let new = repo.get(tenant.clone(), nid(new_id)).expect("get new");
    assert_eq!(new.status, MemoryStatus::Active);
    assert_eq!(new.supersedes, Some(nid(old_id)));

    // Attach an embedding to the new record, prove nearest retrieves it,
    // then remove it and prove nearest no longer returns it.
    let dims = vec![0.3f32; 384];
    let mut vrepo = PgVectorRepository::new(&uow);
    vrepo
        .upsert_vector(tenant.clone(), nid(new_id), dims.clone())
        .expect("upsert vector");
    let near = vrepo.nearest(tenant.clone(), &dims, 5).expect("nearest");
    assert!(
        near.iter().any(|c| c.record.memory_id == nid(new_id)),
        "embedding retrievable after upsert"
    );
    vrepo.remove(tenant.clone(), nid(new_id)).expect("remove vector");
    let near = vrepo
        .nearest(tenant.clone(), &dims, 5)
        .expect("nearest after remove");
    assert!(near.is_empty(), "removed embedding no longer retrieved");

    // Soft-delete the new record; status becomes DELETED.
    repo.delete(tenant.clone(), nid(new_id)).expect("delete");
    let deleted = repo.get(tenant.clone(), nid(new_id)).expect("get deleted");
    assert_eq!(deleted.status, MemoryStatus::Deleted);

    uow.commit().expect("commit");
}

#[test]
fn rx005_outbox_append_is_atomic_with_domain_write() {
    let pg = setup();
    let tenant = tid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f70a2");
    let mem_id = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f70a1";

    // Commit path: the domain write and the outbox append land together.
    {
        let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
        let mut mrepo = PgMemoryRepository::new(&uow);
        mrepo.propose(
            tenant.clone(),
            MemoryProposal {
                record: record(
                    mem_id,
                    tenant.as_str(),
                    serde_json::json!({ "note": "atomic" }),
                    MemoryStatus::Proposed,
                ),
            },
        )
        .expect("propose");
        let orepo = PgOutboxRepository::new(&uow);
        orepo.append(&envelope(1)).expect("append");
        uow.commit().expect("commit");
    }
    // Both visible to a fresh unit of work.
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow 2");
    let mut mrepo = PgMemoryRepository::new(&uow);
    assert_eq!(
        mrepo.get(tenant.clone(), nid(mem_id))
            .expect("get")
            .status,
        MemoryStatus::Proposed
    );
    let orepo = PgOutboxRepository::new(&uow);
    assert_eq!(orepo.fetch_pending(10).expect("fetch").len(), 1);
    uow.commit().expect("commit 2");

    // Rollback path: neither the write nor the append survives.
    {
        let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow 3");
        let mut mrepo = PgMemoryRepository::new(&uow);
        mrepo.propose(
            tenant.clone(),
            MemoryProposal {
                record: record(
                    "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f70a3",
                    tenant.as_str(),
                    serde_json::json!({ "note": "rollback" }),
                    MemoryStatus::Proposed,
                ),
            },
        )
        .expect("propose");
        let orepo = PgOutboxRepository::new(&uow);
        orepo.append(&envelope(2)).expect("append");
        uow.rollback().expect("rollback");
    }
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow 4");
    let mut mrepo = PgMemoryRepository::new(&uow);
    assert_eq!(
        mrepo.get(tenant.clone(), nid("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f70a3"))
            .unwrap_err()
            .code(),
        nexus_data::DataErrorCode::Conflict
    );
    let orepo = PgOutboxRepository::new(&uow);
    assert_eq!(
        orepo.fetch_pending(10).expect("fetch").len(),
        1,
        "only the committed outbox row remains"
    );
    uow.commit().expect("commit 4");
}

#[test]
fn rx005_outbox_publisher_lifecycle_and_bounded_retry() {
    let pg = setup();
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
    let orepo = PgOutboxRepository::new(&uow);

    let a = orepo.append(&envelope(10)).expect("append a");
    assert_eq!(a.status, OutboxStatus::Pending);
    let b = orepo.append(&envelope(11)).expect("append b");

    let pending = orepo.fetch_pending(10).expect("fetch");
    assert_eq!(pending.len(), 2);

    // Mark a in-flight: fetch_pending excludes it; b remains.
    orepo.mark_publishing(&a.outbox_id).expect("mark publishing");
    let pending = orepo.fetch_pending(10).expect("fetch");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].outbox_id, b.outbox_id);

    // Publish succeeds: the row leaves the pending set.
    orepo.mark_published(&a.outbox_id).expect("mark published");
    let pending = orepo.fetch_pending(10).expect("fetch");
    assert_eq!(pending.len(), 1, "published row leaves the pending set");

    // Publish b fails: FAILED with attempts incremented, still retried.
    orepo
        .mark_failed(&b.outbox_id, "nats timeout")
        .expect("mark failed");
    let pending = orepo.fetch_pending(10).expect("fetch");
    assert_eq!(pending.len(), 1, "failed row is retried");
    assert_eq!(pending[0].status, OutboxStatus::Failed);
    assert_eq!(pending[0].attempts, 1);
    assert!(
        pending[0].last_error.as_deref().unwrap().contains("nats"),
        "redacted failure reason is stored"
    );

    // Marking a missing id fails closed with Conflict.
    let err = orepo.mark_published("missing").unwrap_err();
    assert_eq!(err.code(), nexus_events::EventErrorCode::Conflict);

    uow.commit().expect("commit");
}

#[test]
fn rx005_inbox_deduplicates_and_lifecycle() {
    let pg = setup();
    let mut uow = PgUnitOfWork::begin(pg.client()).expect("begin uow");
    let irepo = PgInboxRepository::new(&uow);

    // First sighting records; replay deduplicates.
    assert!(irepo.record_delivery("indexer", "evt-1").expect("first"));
    assert!(!irepo.record_delivery("indexer", "evt-1").expect("replay"));
    assert!(irepo.record_delivery("indexer", "evt-2").expect("second"));

    // Consumers are isolated.
    assert!(irepo.record_delivery("other", "evt-1").expect("other consumer"));

    let new = irepo.fetch_new("indexer", 10).expect("fetch");
    assert_eq!(new.len(), 2);

    // Done rows leave the pending set.
    irepo.mark_done("indexer", "evt-1").expect("done");
    let new = irepo.fetch_new("indexer", 10).expect("fetch");
    assert_eq!(new.len(), 1);
    assert_eq!(new[0].event_id, "evt-2");

    // Failed rows are retried with attempts incremented.
    irepo
        .mark_failed("indexer", "evt-2", "handler error")
        .expect("failed");
    let new = irepo.fetch_new("indexer", 10).expect("fetch");
    assert_eq!(new.len(), 1);
    assert_eq!(new[0].status, InboxStatus::Failed);
    assert_eq!(new[0].attempts, 1);

    // A redelivered DONE event stays deduplicated.
    assert!(!irepo
        .record_delivery("indexer", "evt-1")
        .expect("redeliver done"));

    // Marking a missing delivery fails closed.
    let err = irepo.mark_done("indexer", "evt-nope").unwrap_err();
    assert_eq!(err.code(), nexus_events::EventErrorCode::Conflict);

    uow.commit().expect("commit");
}
