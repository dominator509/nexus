//! Focused diagnostic for the propose serialization failure.

use std::process::Command;
use std::time::{Duration, Instant};

use postgres::{Client, NoTls};
use uuid::Uuid;

const IMAGE: &str = "pgvector/pgvector:pg18";

#[test]
fn diag_propose_serialization() {
    let name = format!(
        "nexus-diag-{}",
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
        .expect("docker run");
    assert!(out.status.success());
    let port_out = Command::new("docker")
        .args(["port", &name, "5432"])
        .output()
        .expect("docker port");
    let text = String::from_utf8_lossy(&port_out.stdout);
    let port: u16 = text.trim().rsplit(':').next().unwrap().parse().unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut client = loop {
        let mut connected = Client::connect(
            &format!("host=127.0.0.1 port={port} user=nexus password=nexus dbname=nexus connect_timeout=2"),
            NoTls,
        )
        .ok();
        let ready = connected
            .as_mut()
            .is_some_and(|c| c.simple_query("SELECT 1").is_ok());
        if ready {
            break connected.expect("ready client");
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(500));
    };
    for m in [
        "migrations/001_memory_and_world_graph.sql",
        "migrations/002_memory_embeddings_vector.sql",
        "migrations/003_tenant_isolation_rls.sql",
    ] {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(m);
        let sql = std::fs::read_to_string(p).unwrap();
        client
            .batch_execute(&sql)
            .unwrap_or_else(|e| panic!("apply {m}: {e}"));
    }
    // Now replicate the propose INSERT parameter list exactly, inside a
    // real transaction with the tenant claim set - the same conditions the
    // adapter runs under. Assert the outcome; a silent print is not a test.
    let mid = Uuid::parse_str("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7001").unwrap();
    let tenant = Uuid::parse_str("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7002").unwrap();
    let content = serde_json::json!({ "note": "groceries" });
    let derived: Vec<Uuid> = vec![];
    let supersedes: Option<Uuid> = None;
    let embedding_ref: Option<String> = None;

    client.simple_query("BEGIN").expect("begin transaction");
    client
        .execute(
            "SELECT set_config('app.tenant_id', $1, true)",
            &[&tenant.to_string()],
        )
        .expect("set tenant claim");

    let params: &[&(dyn postgres::types::ToSql + Sync)] = &[
        &mid,
        &tenant,
        &"household".to_string(),
        &"SEMANTIC",
        &content,
        &"a".repeat(64),
        &"test".to_string(),
        &"principal".to_string(),
        &"2026-08-12T00:00:00Z".to_string(),
        &"2026-08-12T00:00:00Z".to_string(),
        &0.8f64,
        &"HOUSEHOLD",
        &"remember".to_string(),
        &"Days 30".to_string(),
        &"PROPOSED",
        &derived,
        &supersedes,
        &embedding_ref,
    ];
    let n = client
        .execute(
            "INSERT INTO memory_records (
                memory_id, tenant_id, namespace, memory_type, content, content_hash,
                source, actor, created_at, observed_at, confidence, sensitivity,
                purpose, retention, status, derived_from, supersedes, embedding_ref
             ) VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7, $8, $9::text::timestamptz,
                $10::text::timestamptz, $11, $12, $13, $14, $15, $16::uuid[], $17::uuid,
                $18)",
            params,
        )
        .expect("propose INSERT with RFC3339 text timestamps must serialize");
    assert_eq!(n, 1, "exactly one row inserted");
    let _ = Command::new("docker").args(["rm", "-f", &name]).output();
    println!("DIAG OK: RFC3339 text timestamps serialize against timestamptz params");
}
