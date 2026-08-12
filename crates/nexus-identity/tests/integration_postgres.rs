//! EP-003 M3 integration tests: identity contracts through REAL PostgreSQL.
//!
//! Uses the pinned `postgres:18.4` image (COMPONENT_REGISTRY.yaml,
//! VERSIONS.lock.yaml) in a real ephemeral container - never an in-memory
//! substitute. Readiness is proven by connecting through the PUBLISHED HOST
//! PORT (docker's port-publish can lag pg_isready; the test consumes the
//! host port - EP-001 M5 flake fix). Host ports are dynamically allocated
//! so parallel runs never collide on a fixed port.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use nexus_domain::{CorrelationId, DeviceId, NexusId, PersonId, PrincipalType, TenantId};
use nexus_identity::{
    EvidenceKind, Household, LifecycleState, PersonProfile, PresenceEvidence, Principal,
    PrivacyContext, Session,
};
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
            "nexus-ep003-{}",
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

fn sample_person() -> PersonProfile {
    PersonProfile::new(
        PersonId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101").unwrap(),
        TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102").unwrap(),
        "Lin",
        LifecycleState::Active,
        Some(nexus_domain::HouseholdId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103").unwrap()),
        vec![nexus_domain::BusinessId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6104").unwrap()],
    )
    .unwrap()
}

fn sample_household() -> Household {
    Household::new(
        nexus_domain::HouseholdId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6103").unwrap(),
        TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102").unwrap(),
        "The Lin Household",
        vec![PersonId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101").unwrap()],
    )
    .unwrap()
}

fn sample_session() -> Session {
    Session::new(
        NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6130").unwrap(),
        TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102").unwrap(),
        NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101").unwrap(),
        PrincipalType::Human,
        Some(DeviceId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105").unwrap()),
        1000,
        2000,
        CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073").unwrap(),
    )
}

fn sample_principal() -> Principal {
    Principal::new(
        NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6140").unwrap(),
        PrincipalType::Human,
        TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6102").unwrap(),
    )
}

/// The identity domain records survive a real SQL round-trip through
/// PostgreSQL as JSONB (INV-004: PostgreSQL is the initial durable truth).
#[test]
fn ep003_integration_identity_records_roundtrip_real_postgres() {
    let pg = TestPostgres::start();
    let mut client = pg.client();

    client
        .batch_execute(
            "CREATE TABLE identity_records (
                id TEXT PRIMARY KEY,
                kind TEXT NOT NULL,
                tenant_id TEXT NOT NULL,
                payload JSONB NOT NULL
            )",
        )
        .expect("create table");

    // PersonProfile
    let person = sample_person();
    let payload = serde_json::to_value(&person).expect("serialize person");
    client
        .execute(
            "INSERT INTO identity_records (id, kind, tenant_id, payload) VALUES ($1, $2, $3, $4)",
            &[
                &person.person_id.to_string(),
                &"person",
                &person.tenant_id.to_string(),
                &payload,
            ],
        )
        .expect("insert person");
    let row = client
        .query_one(
            "SELECT payload FROM identity_records WHERE id = $1",
            &[&person.person_id.to_string()],
        )
        .expect("select person");
    let back: PersonProfile = serde_json::from_value(row.get(0)).expect("deserialize person");
    assert_eq!(person, back);

    // Household
    let household = sample_household();
    let payload = serde_json::to_value(&household).expect("serialize household");
    client
        .execute(
            "INSERT INTO identity_records (id, kind, tenant_id, payload) VALUES ($1, $2, $3, $4)",
            &[
                &household.household_id.to_string(),
                &"household",
                &household.tenant_id.to_string(),
                &payload,
            ],
        )
        .expect("insert household");
    let row = client
        .query_one(
            "SELECT payload FROM identity_records WHERE id = $1",
            &[&household.household_id.to_string()],
        )
        .expect("select household");
    let back: Household = serde_json::from_value(row.get(0)).expect("deserialize household");
    assert_eq!(household, back);

    // Session
    let session = sample_session();
    let payload = serde_json::to_value(&session).expect("serialize session");
    client
        .execute(
            "INSERT INTO identity_records (id, kind, tenant_id, payload) VALUES ($1, $2, $3, $4)",
            &[
                &session.session_id.to_string(),
                &"session",
                &session.tenant_id.to_string(),
                &payload,
            ],
        )
        .expect("insert session");
    let row = client
        .query_one(
            "SELECT payload FROM identity_records WHERE id = $1",
            &[&session.session_id.to_string()],
        )
        .expect("select session");
    let back: Session = serde_json::from_value(row.get(0)).expect("deserialize session");
    assert_eq!(session, back);

    // Principal
    let principal = sample_principal();
    let payload = serde_json::to_value(&principal).expect("serialize principal");
    client
        .execute(
            "INSERT INTO identity_records (id, kind, tenant_id, payload) VALUES ($1, $2, $3, $4)",
            &[
                &principal.principal_id.to_string(),
                &"principal",
                &principal.tenant_id.to_string(),
                &payload,
            ],
        )
        .expect("insert principal");
    let row = client
        .query_one(
            "SELECT payload FROM identity_records WHERE id = $1",
            &[&principal.principal_id.to_string()],
        )
        .expect("select principal");
    let back: Principal = serde_json::from_value(row.get(0)).expect("deserialize principal");
    assert_eq!(principal, back);
}

/// Session uniqueness: the same session_id cannot be inserted twice on the
/// real engine; a duplicate request must be rejected deterministically
/// (SPEC-006 conflict semantics).
#[test]
fn ep003_integration_session_id_is_unique_in_postgres() {
    let pg = TestPostgres::start();
    let mut client = pg.client();

    client
        .batch_execute(
            "CREATE TABLE sessions (
                session_id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                payload JSONB NOT NULL
            )",
        )
        .expect("create table");

    let session = sample_session();
    let payload = serde_json::to_value(&session).expect("serialize session");
    client
        .execute(
            "INSERT INTO sessions (session_id, tenant_id, payload) VALUES ($1, $2, $3)",
            &[
                &session.session_id.to_string(),
                &session.tenant_id.to_string(),
                &payload,
            ],
        )
        .expect("first insert");
    let dup = client.execute(
        "INSERT INTO sessions (session_id, tenant_id, payload) VALUES ($1, $2, $3)",
        &[
            &session.session_id.to_string(),
            &session.tenant_id.to_string(),
            &payload,
        ],
    );
    assert!(
        dup.is_err(),
        "duplicate session_id must be rejected by the engine"
    );
}

/// Presence evidence survives a real round-trip and the canonical enums keep
/// their exact wire values through JSONB.
#[test]
fn ep003_integration_presence_evidence_roundtrip_real_postgres() {
    let pg = TestPostgres::start();
    let mut client = pg.client();

    client
        .batch_execute(
            "CREATE TABLE evidence (
                id BIGSERIAL PRIMARY KEY,
                payload JSONB NOT NULL
            )",
        )
        .expect("create table");

    let evidence = PresenceEvidence::new(
        EvidenceKind::Voice,
        Some(DeviceId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105").unwrap()),
        0.9,
        1_700_000_000,
    )
    .expect("evidence");
    let payload = serde_json::to_value(&evidence).expect("serialize evidence");
    client
        .execute("INSERT INTO evidence (payload) VALUES ($1)", &[&payload])
        .expect("insert evidence");
    let row = client
        .query_one("SELECT payload FROM evidence ORDER BY id DESC LIMIT 1", &[])
        .expect("select evidence");
    let back: PresenceEvidence = serde_json::from_value(row.get(0)).expect("deserialize");
    assert_eq!(evidence, back);
    let raw: serde_json::Value = row.get(0);
    assert_eq!(raw["kind"], "VOICE");
    assert_eq!(raw["confidence"], 0.9);
}

/// Container cleanup: after the test the ephemeral container is removed.
#[test]
fn ep003_integration_ephemeral_container_is_cleaned_up() {
    let pg = TestPostgres::start();
    let container = pg.container.clone();
    let out = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", &container])
        .output()
        .expect("docker inspect failed");
    assert!(out.status.success(), "container must exist while test runs");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "true");
    drop(pg);
    let out = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", &container])
        .output()
        .expect("docker inspect failed");
    assert!(
        !out.status.success(),
        "container must be removed after Drop"
    );
}
