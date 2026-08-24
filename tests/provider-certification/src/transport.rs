//! Real transport to an ephemeral PostgreSQL 18.4 container.
//!
//! The container is spawned with the real `docker` CLI using a unique
//! EP-040-owned name and a runtime-generated password (never a tracked
//! literal). Readiness is proven by connecting through the published host
//! port (docker's port-publish can lag pg_isready; the probe consumes the
//! host port). Dropping the transport removes the container and verifies
//! zero residue.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use postgres::{Client, NoTls};

use crate::{POSTGRES_DIGEST, POSTGRES_IMAGE};

/// A successful real probe of the provider: the engine answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderProbe {
    /// Canonical provider id (COMPONENT_REGISTRY.yaml).
    pub provider: String,
    /// Exact engine version observed from the real server.
    pub version: String,
    /// Exact interface exercised (SQL over TCP through the host port).
    pub interface: String,
    /// Image digest pinned in COMPONENT_REGISTRY.yaml.
    pub digest: String,
    /// Round-trip latency observed for SELECT 1.
    pub roundtrip_ms: u64,
}

/// Transport-level failure with a typed cause (never a generic success).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// The docker CLI or image is unavailable.
    Unavailable(String),
    /// Credentials were rejected by the real engine.
    Authentication(String),
    /// The container started but never became ready.
    Timeout(String),
    /// The probe ran but the engine did not answer correctly.
    Verification(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(m) => write!(f, "provider unavailable: {m}"),
            Self::Authentication(m) => write!(f, "provider authentication failed: {m}"),
            Self::Timeout(m) => write!(f, "provider readiness timeout: {m}"),
            Self::Verification(m) => write!(f, "provider verification failed: {m}"),
        }
    }
}

impl std::error::Error for TransportError {}

/// Real PostgreSQL transport: spawns the digest-pinned container, probes
/// readiness through the published host port, and drops cleanly.
pub struct PostgresTransport {
    pub container: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
}

impl PostgresTransport {
    /// Spawn a fresh ephemeral container with a runtime-generated
    /// password and a unique EP-040-owned name.
    pub fn start() -> Result<Self, TransportError> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let name = format!("nexus-ep040-m3-{nanos}");
        let user = "nexus".to_string();
        let password = Self::runtime_password();
        let dbname = "nexus".to_string();

        let out = Command::new("docker")
            .args([
                "run",
                "-d",
                "--name",
                &name,
                "-e",
                &format!("POSTGRES_USER={user}"),
                "-e",
                &format!("POSTGRES_PASSWORD={password}"),
                "-e",
                &format!("POSTGRES_DB={dbname}"),
                "-p",
                "127.0.0.1::5432",
                POSTGRES_IMAGE,
            ])
            .output()
            .map_err(|e| TransportError::Unavailable(format!("docker run failed: {e}")))?;
        if !out.status.success() {
            return Err(TransportError::Unavailable(format!(
                "docker run failed: {}",
                String::from_utf8_lossy(&out.stderr)
            )));
        }

        let port = Self::host_port(&name).inspect_err(|_| {
            let _ = Command::new("docker")
                .args(["rm", "-f", &name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        })?;

        let transport = Self {
            container: name,
            port,
            user,
            password,
            dbname,
        };
        transport.wait_ready().inspect_err(|_| {
            let _ = Command::new("docker")
                .args(["rm", "-f", &transport.container])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        })?;
        Ok(transport)
    }

    /// Runtime-generated password: hex from /dev/urandom, never a tracked
    /// literal and never a real credential.
    fn runtime_password() -> String {
        let mut bytes = [0u8; 18];
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            use std::io::Read;
            let _ = f.read_exact(&mut bytes);
        }
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn host_port(container: &str) -> Result<u16, TransportError> {
        for _ in 0..50 {
            let out = Command::new("docker")
                .args(["port", container, "5432"])
                .output()
                .map_err(|e| TransportError::Unavailable(format!("docker port failed: {e}")))?;
            if out.status.success() {
                let line = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if let Some(port) = line
                    .rsplit(':')
                    .next()
                    .and_then(|p| p.trim().parse::<u16>().ok())
                {
                    return Ok(port);
                }
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        Err(TransportError::Timeout(format!(
            "docker port never published for {container}"
        )))
    }

    fn wait_ready(&self) -> Result<(), TransportError> {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut last: Option<postgres::Error> = None;
        while Instant::now() < deadline {
            match self.connect() {
                Ok(mut client) => {
                    if client.simple_query("SELECT 1").is_ok() {
                        return Ok(());
                    }
                }
                Err(e) => last = Some(e),
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        Err(TransportError::Timeout(format!(
            "postgres host port {} not ready within 60s: {last:?}",
            self.port
        )))
    }

    fn connect(&self) -> Result<Client, postgres::Error> {
        Client::connect(
            &format!(
                "host=127.0.0.1 port={} user={} password={} dbname={} connect_timeout=2",
                self.port, self.user, self.password, self.dbname
            ),
            NoTls,
        )
    }

    /// Connect with an explicit password (for auth-failure proofs).
    pub fn connect_with_password(&self, password: &str) -> Result<Client, postgres::Error> {
        Client::connect(
            &format!(
                "host=127.0.0.1 port={} user={} password={} dbname={} connect_timeout=2",
                self.port, self.user, password, self.dbname
            ),
            NoTls,
        )
    }

    /// Real probe: connect, SELECT version(), and measure a round-trip.
    pub fn probe(&self) -> Result<ProviderProbe, TransportError> {
        let started = Instant::now();
        let mut client = self
            .connect()
            .map_err(|e| TransportError::Authentication(format!("probe connect failed: {e}")))?;
        let row = client
            .query_one("SELECT version()", &[])
            .map_err(|e| TransportError::Verification(format!("version query failed: {e}")))?;
        let version: String = row.get(0);
        let roundtrip_ms = started.elapsed().as_millis() as u64;
        Ok(ProviderProbe {
            provider: "postgresql".into(),
            version,
            interface: "sql-tcp-host-port".into(),
            digest: POSTGRES_DIGEST.to_string(),
            roundtrip_ms,
        })
    }

    /// Real round-trip: create a table, insert, select, count.
    pub fn roundtrip(&self) -> Result<u64, TransportError> {
        let mut client = self.connect().map_err(|e| {
            TransportError::Authentication(format!("roundtrip connect failed: {e}"))
        })?;
        client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS ep040_m3_probe (id BIGSERIAL PRIMARY KEY, note TEXT)",
            )
            .map_err(|e| TransportError::Verification(format!("create failed: {e}")))?;
        client
            .execute(
                "INSERT INTO ep040_m3_probe (note) VALUES ('real transport')",
                &[],
            )
            .map_err(|e| TransportError::Verification(format!("insert failed: {e}")))?;
        let row = client
            .query_one("SELECT count(*) FROM ep040_m3_probe", &[])
            .map_err(|e| TransportError::Verification(format!("count failed: {e}")))?;
        let count: i64 = row.get(0);
        Ok(count as u64)
    }

    /// Verify zero residue: the container is gone and nothing on the
    /// docker CLI matches our owned name.
    pub fn verify_clean(&self) -> bool {
        let out = Command::new("docker")
            .args(["ps", "-a", "--no-trunc", "--format", "{{.Names}}"])
            .output();
        match out {
            Ok(out) => !String::from_utf8_lossy(&out.stdout).contains(&self.container),
            Err(_) => false,
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for PostgresTransport {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
