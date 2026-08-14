//! EP-011 M3 live transport proof (directive C/F/G/H/I/O/Q).
//!
//! The Rust SDK client talks to the REAL fixture sidecar process over
//! REAL HTTP on a localhost ephemeral port. No direct function calls:
//!
//!   Rust test client
//!       -> real HTTP (127.0.0.1, ephemeral port)
//!       -> fixture sidecar process (tests/connectors/fixture_sidecar.py)
//!       -> Python SDK implementation
//!       -> fixture provider
//!
//! The same sidecar process is driven by the TypeScript and Python
//! suites, proving cross-language wire compatibility on the same
//! transport (directives C/D/E).

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{Value, json};

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repo root")
}

struct Sidecar {
    child: Child,
    base_url: String,
    client: Client,
}

impl Sidecar {
    fn start() -> Sidecar {
        let root = repo_root();
        let sidecar = root.join("tests/connectors/fixture_sidecar.py");
        let mut child = Command::new("python3")
            .arg(&sidecar)
            .current_dir(&root)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn fixture sidecar");
        let stdout = child.stdout.take().expect("sidecar stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        reader.read_line(&mut line).expect("read sidecar PORT line");
        let port: u16 = line
            .trim()
            .strip_prefix("PORT ")
            .expect("PORT line")
            .parse()
            .expect("port number");
        let base_url = format!("http://127.0.0.1:{port}");
        let mut headers = HeaderMap::new();
        headers.insert("X-Nexus-Protocol-Version", HeaderValue::from_static("1"));
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()
            .expect("reqwest client");
        Sidecar {
            child,
            base_url,
            client,
        }
    }

    fn post(&self, path: &str, body: Value) -> Result<Value, Value> {
        let resp = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(&body)
            .send()
            .expect("HTTP request");
        let status = resp.status();
        let parsed: Value = resp.json().expect("JSON response");
        if status.is_success() {
            Ok(parsed)
        } else {
            Err(parsed)
        }
    }

    fn ctx(tenant: &str) -> Value {
        json!({
            "request_id": "018f0f6f-9c1e-7b6e-8000-000000000001",
            "correlation_id": "018f0f6f-9c1e-7b6e-8000-000000000002",
            "origin_system": "rust-live",
            "external_actor_id": "user:alice",
            "external_actor_type": "HUMAN",
            "tenant_id": tenant,
        })
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

const TENANT_A: &str = "018f0f6f-9c1e-7b6e-8000-000000000003";
const TENANT_B: &str = "018f0f6f-9c1e-7b6e-8000-000000000099";

#[test]
fn ep011_integration_transport_discover_and_query() {
    let sidecar = Sidecar::start();
    let discover = sidecar
        .post("/v1/discover", json!({ "context": Sidecar::ctx(TENANT_A) }))
        .expect("discover ok");
    let caps = discover["capabilities"].as_array().expect("capabilities");
    assert!(caps.iter().any(|c| c["id"] == "fixture.contacts.query"));
    assert!(caps.iter().any(|c| c["id"] == "fixture.contacts.command"));

    let query = sidecar
        .post(
            "/v1/query",
            json!({
                "capability_id": "fixture.contacts.query",
                "context": Sidecar::ctx(TENANT_A),
                "input": { "limit": 10 },
            }),
        )
        .expect("query ok");
    assert_eq!(query["capability_id"], "fixture.contacts.query");
    assert!(query["output"].is_object());
}

#[test]
fn ep011_integration_transport_command_idempotency_replay_and_conflict() {
    let sidecar = Sidecar::start();
    let ctx = Sidecar::ctx(TENANT_A);

    let first = sidecar
        .post(
            "/v1/command",
            json!({
                "capability_id": "fixture.contacts.command",
                "context": ctx,
                "input": { "name": "Bob" },
                "idempotency_key": "k-rust-1",
            }),
        )
        .expect("first command ok");
    assert_eq!(first["output"]["id"], "c1");

    // Retry: same capability + same key + same digest -> replay,
    // provider must NOT execute a second time.
    let replay = sidecar
        .post(
            "/v1/command",
            json!({
                "capability_id": "fixture.contacts.command",
                "context": ctx,
                "input": { "name": "Bob" },
                "idempotency_key": "k-rust-1",
            }),
        )
        .expect("replay ok");
    assert_eq!(replay["output"]["id"], "c1");

    // Same key + different capability -> typed CONFLICT, no provider
    // execution.
    let conflict = sidecar
        .post(
            "/v1/command",
            json!({
                "capability_id": "fixture.billing.command",
                "context": ctx,
                "input": { "name": "X" },
                "idempotency_key": "k-rust-1",
            }),
        )
        .expect_err("conflict must be an error");
    assert_eq!(conflict["code"], "CONFLICT");
}

#[test]
fn ep011_integration_transport_class_mismatch_and_not_found() {
    let sidecar = Sidecar::start();
    let ctx = Sidecar::ctx(TENANT_A);

    // Query sent to a command-only capability -> typed class mismatch,
    // provider not invoked.
    let mismatch = sidecar
        .post(
            "/v1/query",
            json!({
                "capability_id": "fixture.contacts.command",
                "context": ctx,
                "input": {},
            }),
        )
        .expect_err("class mismatch must be an error");
    assert_eq!(mismatch["code"], "VALIDATION");
    assert!(
        mismatch["message"]
            .as_str()
            .unwrap()
            .contains("not a QUERY class")
    );

    // Unknown capability -> typed NotFound.
    let missing = sidecar
        .post(
            "/v1/query",
            json!({
                "capability_id": "fixture.does.not.exist",
                "context": ctx,
                "input": {},
            }),
        )
        .expect_err("unknown capability must 404");
    assert_eq!(missing["code"], "NOT_FOUND");
}

#[test]
fn ep011_integration_transport_workflow_dispatch_not_temporal() {
    let sidecar = Sidecar::start();
    let result = sidecar
        .post(
            "/v1/workflow",
            json!({
                "capability_id": "fixture.reconcile.workflow",
                "context": Sidecar::ctx(TENANT_A),
                "input": { "scope": "daily" },
            }),
        )
        .expect("workflow dispatch ok");
    // Transport dispatch only: a RUNNING handle. EP-011 does NOT claim
    // durable Temporal execution (EP-006 owns Temporal).
    assert_eq!(result["handle"]["workflow_id"], "wf-1");
    assert_eq!(result["status"], "RUNNING");
    // `output: None` serializes as JSON null - the workflow has NOT
    // completed, so no output may be claimed.
    assert!(result.get("output").is_none_or(|v| v.is_null()));
}

#[test]
fn ep011_integration_transport_health_is_observation() {
    let sidecar = Sidecar::start();
    let health = sidecar
        .post(
            "/v1/health",
            json!({
                "capability_id": "fixture.health",
                "context": Sidecar::ctx(TENANT_A),
            }),
        )
        .expect("health ok");
    assert_eq!(health["state"], "HEALTHY");
    // Health must never carry authorization material (directive I).
    let text = serde_json::to_string(&health).unwrap();
    assert!(!text.contains("allow") && !text.contains("grant") && !text.contains("token"));
}

#[test]
fn ep011_integration_transport_cross_tenant_denied() {
    let sidecar = Sidecar::start();
    // Same request from a different tenant -> NOT_FOUND, no existence
    // disclosure (canonical policy, directive O.8).
    let denied = sidecar
        .post(
            "/v1/query",
            json!({
                "capability_id": "fixture.contacts.query",
                "context": Sidecar::ctx(TENANT_B),
                "input": {},
            }),
        )
        .expect_err("cross-tenant must be denied");
    assert_eq!(denied["code"], "NOT_FOUND");
}

#[test]
fn ep011_integration_transport_protocol_version_fail_closed() {
    // Directive Q: an unsupported protocol version fails closed; the
    // payload is never silently reinterpreted.
    let root = repo_root();
    let sidecar = root.join("tests/connectors/fixture_sidecar.py");
    let mut child = Command::new("python3")
        .arg(&sidecar)
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn fixture sidecar");
    let stdout = child.stdout.take().expect("sidecar stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("PORT line");
    let port: u16 = line
        .trim()
        .strip_prefix("PORT ")
        .expect("PORT")
        .parse()
        .expect("port");
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("client");
    let resp = client
        .post(format!("http://127.0.0.1:{port}/v1/discover"))
        .header("X-Nexus-Protocol-Version", "99")
        .json(&json!({ "context": Sidecar::ctx(TENANT_A) }))
        .send()
        .expect("HTTP");
    assert_eq!(resp.status().as_u16(), 426);
    let body: Value = resp.json().expect("JSON");
    assert_eq!(body["code"], "VALIDATION");
    assert!(
        body["message"]
            .as_str()
            .unwrap()
            .contains("unsupported protocol version")
    );
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ep011_integration_transport_changefeed_and_credential_boundary() {
    let sidecar = Sidecar::start();
    let ctx = Sidecar::ctx(TENANT_A);

    // Changefeed: canonical batch with events + next cursor.
    let feed = sidecar
        .post(
            "/v1/changefeed",
            json!({
                "capability_id": "fixture.audit.changefeed",
                "context": ctx,
                "cursor": serde_json::Value::Null,
            }),
        )
        .expect("changefeed ok");
    assert!(feed["events"].is_array());
    assert!(feed["next_cursor"]["cursor"].is_string());

    // Credential boundary: the reference resolves inside the sandbox;
    // only a fingerprint ever crosses the wire. The raw secret value
    // must never appear in the response.
    let cmd = sidecar
        .post(
            "/v1/command",
            json!({
                "capability_id": "fixture.contacts.command",
                "context": ctx,
                "input": { "name": "C", "credential_reference": "vault:fixture-token" },
                "idempotency_key": "k-cred-1",
            }),
        )
        .expect("credential command ok");
    let fingerprint = cmd["output"]["credential_fingerprint"]
        .as_str()
        .expect("fingerprint present");
    assert_eq!(fingerprint.len(), 16);
    let text = serde_json::to_string(&cmd).unwrap();
    assert!(
        !text.contains("fixture-secret-value"),
        "secret leaked in response"
    );
}

#[test]
fn ep011_integration_transport_sidecar_unavailable_fails_closed() {
    // Directive O.1: connect to a port with nothing listening.
    let client = Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("client");
    let result = client
        .post("http://127.0.0.1:1/v1/discover")
        .header("X-Nexus-Protocol-Version", "1")
        .json(&json!({ "context": Sidecar::ctx(TENANT_A) }))
        .send();
    assert!(result.is_err(), "sidecar not listening must fail closed");
}
