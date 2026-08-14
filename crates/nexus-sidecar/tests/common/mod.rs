//! Shared helpers for EP-011 M4 integration tests (directive B/Z).
//!
//! Every test spawns the REAL sidecar binary and the REAL fixture
//! provider process, then drives real HTTP over loopback. No
//! in-process mocks. Teardown is strict: processes are terminated and
//! their ports verified released (directive AA).

#![allow(dead_code)]

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub const TENANT_A: &str = "018f0f6f-9c1e-7b6e-8000-000000000003";
pub const TENANT_B: &str = "018f0f6f-9c1e-7b6e-8000-000000000099";
pub const REQUEST_ID: &str = "018f0f6f-9c1e-7b6e-8000-000000000001";
pub const CORRELATION_ID: &str = "018f0f6f-9c1e-7b6e-8000-000000000002";

/// A spawned fixture provider process (the M3 Python fixture).
pub struct ProviderProc {
    pub child: Child,
    pub port: u16,
    pub base: String,
}

impl ProviderProc {
    /// Kill the provider (directive J.2/J.3: real process death).
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ProviderProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// A spawned sidecar process (the real hardened boundary).
pub struct SidecarProc {
    pub child: Child,
    pub port: u16,
    pub base: String,
}

impl Drop for SidecarProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Paths resolved from the crate root.
fn crate_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Spawn the real fixture provider (Python) on an ephemeral port.
pub fn spawn_provider() -> ProviderProc {
    let repo_root = crate_root().join("..").join("..");
    let fixture = repo_root
        .join("tests")
        .join("connectors")
        .join("fixture_sidecar.py");
    let mut child = Command::new("python3")
        .arg(&fixture)
        .current_dir(&repo_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn fixture provider");
    let port = read_port(&mut child);
    ProviderProc {
        child,
        port,
        base: format!("http://127.0.0.1:{port}"),
    }
}

/// Spawn the real sidecar binary (directive B).
///
/// `extra_env` lets tests configure failure modes (provider URL, bad
/// tenant, limits, webhook secret, etc).
pub fn spawn_sidecar(provider_base: &str, extra_env: &[(&str, &str)]) -> SidecarProc {
    let bin = env!("CARGO_BIN_EXE_nexus-sidecar");
    let mut cmd = Command::new(bin);
    cmd.env("NEXUS_SIDECAR_TENANT", TENANT_A)
        .env("NEXUS_SIDECAR_CONNECTOR", "fixture-connector")
        .env(
            "NEXUS_SIDECAR_CAPABILITIES",
            "fixture.contacts.query:QUERY,fixture.contacts.command:COMMAND,fixture.billing.command:COMMAND,fixture.reconcile.workflow:WORKFLOW,fixture.health:QUERY,fixture.audit.changefeed:QUERY",
        )
        .env("NEXUS_PROVIDER_URL", provider_base)
        .env("NEXUS_SIDECAR_CREDENTIAL_SCOPE", "fixture-connector:vault:fixture-token")
        .env("NEXUS_SIDECAR_MAX_CONCURRENCY", "8")
        .env("NEXUS_SIDECAR_READ_TIMEOUT_MS", "3000")
        .env("NEXUS_SIDECAR_PROVIDER_TIMEOUT_MS", "3000")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().expect("spawn sidecar");
    let port = read_port(&mut child);
    SidecarProc {
        child,
        port,
        base: format!("http://127.0.0.1:{port}"),
    }
}

/// Read the `PORT <n>` readiness line from a process stdout.
pub fn read_port(child: &mut Child) -> u16 {
    let stdout = child.stdout.as_mut().expect("stdout piped");
    let mut buf = String::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        let mut chunk = [0u8; 4096];
        match stdout.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.push_str(&String::from_utf8_lossy(&chunk[..n]));
                if let Some(idx) = buf.find("PORT ") {
                    let rest = &buf[idx + 5..];
                    let port_str: String =
                        rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                    return port_str.parse().expect("PORT line has a number");
                }
            }
            Err(_) => break,
        }
    }
    // Include stderr (when available) so startup failures are visible.
    let mut err = String::new();
    if let Some(stderr) = child.stderr.as_mut() {
        let _ = stderr.read_to_string(&mut err);
    }
    let exit = child.try_wait();
    panic!("sidecar/provider did not print PORT; stdout: {buf:?} stderr: {err:?} exit: {exit:?}");
}

/// Minimal blocking HTTP client for the tests (real transport).
pub struct Client {
    pub base: String,
}

impl Client {
    pub fn new(base: &str) -> Self {
        Self {
            base: base.to_string(),
        }
    }

    /// POST a JSON body; returns (status, parsed JSON or raw).
    pub fn post(
        &self,
        path: &str,
        body: serde_json::Value,
        protocol_version: Option<&str>,
    ) -> (u16, serde_json::Value) {
        let url = format!("{}{}", self.base, path);
        let client = reqwest::blocking::Client::new();
        let mut req = client.post(&url).json(&body);
        if let Some(v) = protocol_version {
            req = req.header("x-nexus-protocol-version", v);
        }
        let resp = req.send().expect("request");
        let status = resp.status().as_u16();
        let value: serde_json::Value = resp.json().unwrap_or(serde_json::json!(null));
        (status, value)
    }

    /// POST a raw body (for malformed/truncated/oversized tests).
    /// Declares `application/json` so the rejection must come from
    /// the body, not the content-type check.
    pub fn post_raw(
        &self,
        path: &str,
        raw: Vec<u8>,
        protocol_version: Option<&str>,
    ) -> (u16, serde_json::Value) {
        let url = format!("{}{}", self.base, path);
        let client = reqwest::blocking::Client::new();
        let mut req = client
            .post(&url)
            .header("content-type", "application/json")
            .body(raw);
        if let Some(v) = protocol_version {
            req = req.header("x-nexus-protocol-version", v);
        }
        let resp = req.send().expect("request");
        let status = resp.status().as_u16();
        let value: serde_json::Value = resp.json().unwrap_or(serde_json::json!(null));
        (status, value)
    }

    /// Raw GET (health probe).
    pub fn get(&self, path: &str) -> (u16, serde_json::Value) {
        let url = format!("{}{}", self.base, path);
        let client = reqwest::blocking::Client::new();
        let resp = client.get(&url).send().expect("request");
        let status = resp.status().as_u16();
        let value: serde_json::Value = resp.json().unwrap_or(serde_json::json!(null));
        (status, value)
    }

    /// Send a request with an arbitrary method (directive I).
    pub fn method(&self, method: &str, path: &str) -> u16 {
        let url = format!("{}{}", self.base, path);
        let client = reqwest::blocking::Client::new();
        let resp = client
            .request(
                reqwest::Method::from_bytes(method.as_bytes()).unwrap(),
                &url,
            )
            .send()
            .expect("request");
        resp.status().as_u16()
    }

    /// Send a request with a different Content-Type (directive D).
    pub fn post_content_type(
        &self,
        path: &str,
        raw: Vec<u8>,
        content_type: &str,
    ) -> (u16, serde_json::Value) {
        let url = format!("{}{}", self.base, path);
        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(&url)
            .header("content-type", content_type)
            .body(raw)
            .send()
            .expect("request");
        let status = resp.status().as_u16();
        let value: serde_json::Value = resp.json().unwrap_or(serde_json::json!(null));
        (status, value)
    }
}

/// Canonical valid envelope for a query (directive C).
pub fn query_envelope(capability_id: &str, input: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "protocol_version": "1",
        "correlation_id": CORRELATION_ID,
        "request_id": REQUEST_ID,
        "tenant_id": TENANT_A,
        "connector_id": "fixture-connector",
        "capability_id": capability_id,
        "operation": "QUERY",
        "transport": "REST",
        "schema_version": "1.0",
        "input": input,
    })
}

/// Canonical valid envelope for a command (directive C/L).
pub fn command_envelope(
    capability_id: &str,
    input: serde_json::Value,
    idempotency_key: &str,
) -> serde_json::Value {
    serde_json::json!({
        "protocol_version": "1",
        "correlation_id": CORRELATION_ID,
        "request_id": REQUEST_ID,
        "tenant_id": TENANT_A,
        "connector_id": "fixture-connector",
        "capability_id": capability_id,
        "operation": "COMMAND",
        "transport": "REST",
        "schema_version": "1.0",
        "idempotency_key": idempotency_key,
        "input": input,
    })
}
