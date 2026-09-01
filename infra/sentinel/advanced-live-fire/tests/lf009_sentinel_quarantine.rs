//! LF-009 sentinel-quarantine live-fire (EP-031 M5).
//!
//! Proves a REAL advanced-detection quarantine journey over REAL
//! std::net sockets against controlled local fixtures emitting REAL
//! Zeek-shaped, Suricata EVE-shaped, CrowdSec LAPI-shaped,
//! Wazuh-shaped, osquery-shaped, and OPNsense-shaped responses. The
//! PRODUCTION connectors (nexus-zeek-connector,
//! nexus-suricata-connector, nexus-crowdsec-connector,
//! nexus-wazuh-connector, nexus-osquery-connector,
//! nexus-opnsense-connector) are composed behind the
//! nexus-sentinel-advanced contract; production transports are never
//! mocked - only the peer is controlled.
//!
//! The journey distinguishes EVERY state (SPEC-013 acceptance):
//!   RAW SENSOR EVENT   observed provider signal (notice / decision /
//!                      alert / query row)
//!   SECURITY EVENT     normalized observation with provenance
//!   CORRELATED         incident over compatible observed facts
//!   TRIAGED            bounded priority case
//!   RECOMMENDED        quarantine proposal (DATA, zero mutation)
//!   AUTHORIZED         policy permits (approved, reversible)
//!   EXECUTED           real provider mutation (OPNsense addRule+apply)
//!   VERIFIED           independent exact-target readback
//!   REVOKED            reversible rollback
//!
//! A raw sensor event can never become an executed response without
//! explicit approval; a destructive response is never preauthorized.
//! Current-run machine-readable evidence is written to
//! `.agent/state/evidence/LF-009-ep031-m5.json` embedding
//! `EP031_M5_RUN_ID` (stale evidence never satisfies the gate).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use nexus_crowdsec_connector::{CrowdSecThreatIntelProvider, HttpCrowdSecTransport};
use nexus_domain::{ApprovalClass, TenantId};
use nexus_opnsense_connector::{HttpOpnsenseTransport, OpnsenseFirewallProvider};
use nexus_osquery_connector::{
    DistributedQuery, HttpOsqueryEndpoint, OsqueryEndpointTelemetryProvider,
};
use nexus_sentinel::{
    FirewallProvider, NetworkDevice, NetworkDeviceId, NetworkSegment, QuarantineProposal,
    QuarantineState, SentinelCapabilityKind, TrustClass,
};
use nexus_sentinel_advanced::{
    CorrelationConfidence, EndpointTelemetryProvider, IncidentCorrelationId,
    NetworkDetectionProvider, ResponseKind, ResponsePlanId, ResponsePlanner, SecurityInvestigator,
    SecurityTriage, SecurityVerifier, ThreatIntelProvider, TriageCaseId, VerificationRecordId,
};
use nexus_sentinel_advanced_live_fire::{
    SentinelInvestigationService, SentinelResponsePlanner, SentinelTriageService,
    SentinelVerificationService,
};
use nexus_suricata_connector::{JsonLinesSuricataTransport, SuricataNetworkDetectionProvider};
use nexus_wazuh_connector::{HttpWazuhTransport, WazuhEndpointTelemetryProvider};
use nexus_zeek_connector::{JsonLinesZeekTransport, ZeekNetworkDetectionProvider};

const TENANT: &str = "018f0f6f-9c1e-7b6e-8000-000000000001";
const SCANNER: &str = "192.168.40.77"; // unknown device (network-lab scan source)
const TARGET: &str = "192.168.40.1"; // scanned host (srv-lab-1)
const CANARY_OPN_KEY: &str = "EP031_M5_CANARY_OPNSENSE_KEY";
const CANARY_OPN_SECRET: &str = "EP031_M5_CANARY_OPNSENSE_SECRET";
const CANARY_CS_ID: &str = "EP031_M5_CANARY_CROWDSEC_ID";
const CANARY_CS_PASS: &str = "EP031_M5_CANARY_CROWDSEC_PASS";
const CANARY_WZ_USER: &str = "EP031_M5_CANARY_WAZUH_USER";
const CANARY_WZ_PASS: &str = "EP031_M5_CANARY_WAZUH_PASS";
const CANARY_OSQ_SECRET: &str = "EP031_M5_CANARY_OSQUERY_SECRET";

fn tenant() -> TenantId {
    TenantId::from_str(TENANT).expect("tenant")
}

fn run_id() -> String {
    std::env::var("EP031_M5_RUN_ID").unwrap_or_else(|_| "local-run".to_string())
}

fn evidence_path() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    loop {
        let marker = dir.join("Cargo.toml");
        if marker.exists()
            && std::fs::read_to_string(&marker)
                .map(|s| s.contains("[workspace]"))
                .unwrap_or(false)
        {
            break;
        }
        if !dir.pop() {
            panic!("workspace root not found");
        }
    }
    dir.join(".agent/state/evidence/LF-009-ep031-m5.json")
}

fn write_evidence(doc: serde_json::Value) {
    let path = evidence_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create evidence dir");
    }
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&doc).expect("evidence json"),
    )
    .expect("write evidence");
}

// ---------------------------------------------------------------------------
// Fixture plumbing (REAL sockets, controlled peers)
// ---------------------------------------------------------------------------

fn read_until_blank_line(stream: &mut TcpStream) -> String {
    let mut buf = [0u8; 8192];
    let mut acc = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !acc.windows(4).any(|w| w == b"\r\n\r\n") {
        if Instant::now() > deadline {
            break;
        }
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                acc.extend_from_slice(&buf[..n]);
                if acc.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
        }
    }
    String::from_utf8_lossy(&acc).to_string()
}

fn parse_request(text: &str) -> (String, String, String) {
    let mut lines = text.lines();
    let req = lines.next().unwrap_or("");
    let mut parts = req.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let body_start = text.find("\r\n\r\n").map(|i| i + 4).unwrap_or(text.len());
    let body = text[body_start..].to_string();
    (method, path, body)
}

/// Spawn a fixture that answers up to `n` sequential HTTP requests on
/// one real socket each, dispatching to `handler(method, path, body)`.
fn spawn_http_fixture(
    n: usize,
    handler: impl Fn(&str, &str, &str) -> (u16, String, String) + Send + 'static,
) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        for _ in 0..n {
            listener.set_nonblocking(true).expect("nonblocking");
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(c) => break c,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if Instant::now() > deadline {
                            return;
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => return,
                }
            };
            let head = read_until_blank_line(&mut stream);
            let (method, path, body) = parse_request(&head);
            let (status, content_type, resp_body) = handler(&method, &path, &body);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{resp_body}",
                resp_body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (port, handle)
}

// ---------------------------------------------------------------------------
// Zeek fixture (REAL documented notice.log JSON surface)
// ---------------------------------------------------------------------------

/// Spawn a fixture that writes REAL Zeek notice.log JSON lines to the
/// first accepted connection, then closes.
fn spawn_zeek_fixture(lines: Vec<String>) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking");
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(c) => break c,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        for line in lines {
            let _ = stream.write_all(line.as_bytes());
            let _ = stream.write_all(b"\n");
        }
        let _ = stream.flush();
    });
    (port, handle)
}

// ---------------------------------------------------------------------------
// Suricata fixture (REAL documented EVE JSON surface)
// ---------------------------------------------------------------------------

/// Spawn a fixture that writes REAL Suricata eve.json alert lines to
/// the first accepted connection, then closes. AUD-030: the Enhanced
/// profile sensor must actually operate in the live-fire journey.
fn spawn_suricata_fixture(lines: Vec<String>) -> (u16, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).expect("nonblocking");
        let deadline = Instant::now() + Duration::from_secs(15);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(c) => break c,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return,
            }
        };
        for line in lines {
            let _ = stream.write_all(line.as_bytes());
            let _ = stream.write_all(b"\n");
        }
        let _ = stream.flush();
    });
    (port, handle)
}

// ---------------------------------------------------------------------------
// CrowdSec fixture (REAL documented LAPI surface)
// ---------------------------------------------------------------------------

fn spawn_crowdsec_fixture() -> (u16, thread::JoinHandle<()>) {
    let decisions_body = format!(
        r#"{{"decisions":[{{"id":1,"origin":"cscli","type":"ban","scope":"Ip","value":"{SCANNER}","duration":"4h0m0s","scenario":"crowdsecurity/port-scan","action":"ban","created_at":"2026-08-20T00:00:00Z"}}]}}"#
    );
    spawn_http_fixture(2, move |method, path, _body| {
        if method == "POST" && path == "/v1/watchers/login" {
            (
                200,
                "application/json".into(),
                r#"{"code":200,"token":"lf009-crowdsec-jwt"}"#.into(),
            )
        } else if method == "GET" && path.starts_with("/v1/decisions") {
            (200, "application/json".into(), decisions_body.clone())
        } else {
            (404, "application/json".into(), "{}".into())
        }
    })
}

// ---------------------------------------------------------------------------
// Wazuh fixture (REAL documented server API surface)
// ---------------------------------------------------------------------------

fn spawn_wazuh_fixture() -> (u16, thread::JoinHandle<()>) {
    let alerts_body = format!(
        r#"{{"data":{{"affected_items":[{{"id":"alert-1","timestamp":"2026-08-20T00:01:00Z","rule":{{"level":12,"description":"Host brute force attack"}},"agent":{{"id":"001","name":"srv-lab-1","ip":"{TARGET}"}}}}],"total_affected_items":1,"total_failed_items":0,"failed_items":[]}},"message":"ok","error":0}}"#
    );
    spawn_http_fixture(2, move |method, path, _body| {
        if method == "POST" && path == "/security/user/authenticate" {
            (
                200,
                "application/json".into(),
                r#"{"data":{"token":"lf009-wazuh-jwt"}}"#.into(),
            )
        } else if method == "GET" && path.starts_with("/alerts") {
            (200, "application/json".into(), alerts_body.clone())
        } else {
            (404, "application/json".into(), "{}".into())
        }
    })
}

// ---------------------------------------------------------------------------
// OPNsense fixture (REAL documented firewall automation API surface)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct OpnRule {
    uuid: String,
    description: String,
    enabled: bool,
    action: String,
    source_net: Option<String>,
}

fn spawn_opnsense_fixture(
    initial: Vec<OpnRule>,
) -> (u16, Arc<Mutex<Vec<OpnRule>>>, thread::JoinHandle<()>) {
    let state: Arc<Mutex<Vec<OpnRule>>> = Arc::new(Mutex::new(initial));
    let state2 = state.clone();
    let (port, handle) = spawn_http_fixture(8, move |method, path, body| {
        let mut st = state2.lock().unwrap();
        if method == "GET" && path.contains("/api/firewall/filter/searchRule") {
            let phrase = path.split("searchPhrase=").nth(1).unwrap_or("").to_string();
            let rows: Vec<serde_json::Value> = st
                .iter()
                .filter(|r| r.description.contains(&phrase))
                .map(|r| {
                    serde_json::json!({
                        "uuid": r.uuid,
                        "description": r.description,
                        "enabled": if r.enabled { "1" } else { "0" },
                        "action": r.action,
                        "source_net": r.source_net.clone().unwrap_or_default(),
                    })
                })
                .collect();
            (
                200,
                "application/json".into(),
                serde_json::json!({ "total": rows.len(), "rowCount": 100, "current": 1, "rows": rows })
                    .to_string(),
            )
        } else if method == "POST" && path.contains("/api/firewall/filter/addRule") {
            let parsed: serde_json::Value =
                serde_json::from_str(body).unwrap_or(serde_json::Value::Null);
            let rule = parsed.get("rule").cloned().unwrap_or_default();
            let uuid = format!("rule-{}", st.len() + 1);
            st.push(OpnRule {
                uuid: uuid.clone(),
                description: rule
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                enabled: true,
                action: rule
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("block")
                    .to_string(),
                source_net: rule
                    .get("source_net")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
            });
            (
                200,
                "application/json".into(),
                serde_json::json!({ "uuid": uuid }).to_string(),
            )
        } else if method == "POST" && path.contains("/api/firewall/filter/toggleRule/") {
            let rest = path
                .split("/api/firewall/filter/toggleRule/")
                .nth(1)
                .unwrap_or("");
            let mut parts = rest.split('/');
            let uuid = parts.next().unwrap_or("").to_string();
            let enabled = parts.next().unwrap_or("0") == "1";
            if let Some(r) = st.iter_mut().find(|r| r.uuid == uuid) {
                r.enabled = enabled;
            }
            (200, "application/json".into(), "{}".into())
        } else if method == "POST" && path.contains("/api/firewall/filter/apply") {
            (200, "application/json".into(), "{}".into())
        } else {
            (404, "application/json".into(), "{}".into())
        }
    });
    (port, state, handle)
}

// ---------------------------------------------------------------------------
// Devices and proposal helpers
// ---------------------------------------------------------------------------

fn unknown_device() -> NetworkDevice {
    NetworkDevice::new(
        NetworkDeviceId::new("unknown-scan-1").expect("device id"),
        tenant(),
        NetworkSegment::Iot,
        TrustClass::Unknown,
        "unknown-scan-device",
        "opnsense",
        "2026-08-20T00:00:00Z",
        "2026-08-20T00:00:00Z",
    )
}

fn approved_proposal_from(
    provider: &OpnsenseFirewallProvider,
    proposal: QuarantineProposal,
) -> QuarantineProposal {
    // AUD-025: approval is an immutable receipt binding the exact
    // action - never a bare state mutation. The journey must go
    // through the real approve() binding.
    let approved = proposal.approve(
        nexus_domain::ApprovalId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105").unwrap(),
        nexus_domain::PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        nexus_domain::ApprovalClass::Human,
        "2026-08-20T00:00:00Z",
    );
    // AUD-029: automated containment ALWAYS notifies the owner.
    provider
        .notify_owner(&approved, "person-owner-1", "push")
        .unwrap()
}

// ---------------------------------------------------------------------------
// LF-009: the full sentinel-quarantine journey
// ---------------------------------------------------------------------------

#[test]
fn ep031_m5_lf009_sentinel_quarantine() {
    // ---- 1. Controlled current network condition (fixtures) ----
    // Zeek: an unknown device (192.168.40.77) port-scans the managed
    // host 192.168.40.1.
    let zeek_lines = vec![
        r#"{"_path":"notice","ts":1720000000.0,"uid":"C1","id.orig_h":"192.168.40.77","id.orig_p":40000,"id.resp_h":"192.168.40.1","id.resp_p":22,"proto":"tcp","note":"Scan::Portscan","msg":"port scan","src":"192.168.40.77","dst":"192.168.40.1","p":22,"n":1,"actions":[],"suppress_for":0,"dropped":false}"#
            .to_string(),
    ];
    let (zeek_port, zeek_handle) = spawn_zeek_fixture(zeek_lines);

    // Suricata (Enhanced profile): the same unknown device
    // (192.168.40.77) triggers a documented ET SCAN alert on the
    // scanned host. AUD-030: the advertised Suricata profile must
    // operate, not merely exist as vocabulary.
    let suricata_lines = vec![
        format!(
            r#"{{"timestamp":"2026-08-20T00:00:01.000000+0000","flow_id":9101,"event_type":"alert","src_ip":"{SCANNER}","src_port":40000,"dest_ip":"{TARGET}","dest_port":22,"proto":"TCP","alert":{{"action":"allowed","gid":1,"signature_id":2018358,"rev":10,"signature":"ET SCAN Potential SSH Scan","category":"Attempted Information Leak","severity":2}}}}"#
        )
        .to_string(),
    ];
    let (suricata_port, suricata_handle) = spawn_suricata_fixture(suricata_lines);

    // CrowdSec: a ban decision exists for the scanner.
    let (cs_port, cs_handle) = spawn_crowdsec_fixture();

    // Wazuh: the target host reports a high-level (12) alert.
    let (wz_port, wz_handle) = spawn_wazuh_fixture();

    // osquery: the production collector endpoint (self-hosted server);
    // the test plays the enrolled node over a REAL socket.
    let osq_ep = HttpOsqueryEndpoint::new(
        CANARY_OSQ_SECRET.to_string(),
        vec![DistributedQuery {
            id: "listening_ports".to_string(),
            query: "SELECT address, port, protocol, pid FROM listening_ports;".to_string(),
        }],
    );
    let osq_port = osq_ep.serve().expect("osquery serve");
    {
        // Node side (real socket): enroll, read the issued query, and
        // report an observed wildcard listener on the target host.
        let mut stream = TcpStream::connect(("127.0.0.1", osq_port)).expect("connect osquery");
        let body =
            format!(r#"{{"enroll_secret":"{CANARY_OSQ_SECRET}","host_identifier":"srv-lab-1"}}"#);
        let req = format!(
            "POST /enroll HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).expect("enroll write");
        let mut resp = String::new();
        stream.read_to_string(&mut resp).expect("enroll read");
        let node_key = serde_json::from_str::<serde_json::Value>(
            &resp[resp.find("\r\n\r\n").map(|i| i + 4).unwrap_or(resp.len())..],
        )
        .expect("enroll json")
        .get("node_key")
        .and_then(|v| v.as_str())
        .expect("node_key")
        .to_string();
        assert!(!node_key.is_empty());

        let mut stream = TcpStream::connect(("127.0.0.1", osq_port)).expect("connect osquery 2");
        let body = format!(r#"{{"node_key":"{node_key}"}}"#);
        let req = format!(
            "POST /distributed_read HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).expect("read write");
        let mut resp = String::new();
        stream.read_to_string(&mut resp).expect("read read");
        let v: serde_json::Value = serde_json::from_str(
            &resp[resp.find("\r\n\r\n").map(|i| i + 4).unwrap_or(resp.len())..],
        )
        .expect("read json");
        assert!(v["queries"]["listening_ports"]
            .as_str()
            .unwrap_or("")
            .contains("listening_ports"));

        let mut stream = TcpStream::connect(("127.0.0.1", osq_port)).expect("connect osquery 3");
        let body = format!(
            r#"{{"node_key":"{node_key}","queries":{{"listening_ports":[{{"address":"0.0.0.0","port":"8443","protocol":"tcp","pid":"42"}}]}},"statuses":{{"listening_ports":0}}}}"#
        );
        let req = format!(
            "POST /distributed_write HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).expect("write write");
        let mut resp = String::new();
        stream.read_to_string(&mut resp).expect("write read");
        let v: serde_json::Value = serde_json::from_str(
            &resp[resp.find("\r\n\r\n").map(|i| i + 4).unwrap_or(resp.len())..],
        )
        .expect("write json");
        assert_eq!(v["node_invalid"], serde_json::Value::Bool(false));
    }

    // OPNsense: no existing quarantine rule for the unknown scanner.
    let (opn_port, opn_rules, opn_handle) = spawn_opnsense_fixture(Vec::new());

    // ---- 2. Production connectors (real transports to fixtures) ----
    let zeek = ZeekNetworkDetectionProvider::new(JsonLinesZeekTransport::new(
        TcpStream::connect(("127.0.0.1", zeek_port)).expect("zeek connect"),
    ));
    let suricata = SuricataNetworkDetectionProvider::new(JsonLinesSuricataTransport::new(
        TcpStream::connect(("127.0.0.1", suricata_port)).expect("suricata connect"),
    ));
    let crowdsec = CrowdSecThreatIntelProvider::new(HttpCrowdSecTransport::new(
        format!("http://127.0.0.1:{cs_port}"),
        CANARY_CS_ID,
        CANARY_CS_PASS,
        Duration::from_secs(5),
    ));
    let wazuh = WazuhEndpointTelemetryProvider::new(HttpWazuhTransport::new(
        format!("http://127.0.0.1:{wz_port}"),
        CANARY_WZ_USER,
        CANARY_WZ_PASS,
        Duration::from_secs(5),
    ));
    let osquery = OsqueryEndpointTelemetryProvider::new(osq_ep.clone());
    let opnsense = Arc::new(OpnsenseFirewallProvider::new(
        Box::new(HttpOpnsenseTransport::new(
            format!("http://127.0.0.1:{opn_port}"),
            CANARY_OPN_KEY,
            CANARY_OPN_SECRET,
            Duration::from_secs(2),
        )),
        tenant(),
        CANARY_OPN_KEY,
        CANARY_OPN_SECRET,
    ));

    // ---- 3. OBSERVED: raw sensor events over real transports ----
    let zeek_caps = zeek.capabilities();
    assert!(zeek_caps.contains(SentinelCapabilityKind::ReadFindings));
    let zeek_events = zeek.read_events(&tenant()).expect("zeek events");
    assert!(
        zeek_events
            .iter()
            .any(|e| e.correlation.as_deref() == Some(&format!("src={SCANNER}"))),
        "zeek observed scan from the unknown device"
    );

    let suricata_caps = suricata.capabilities();
    assert!(suricata_caps.contains(SentinelCapabilityKind::ReadFindings));
    let suricata_events = suricata.read_events(&tenant()).expect("suricata events");
    assert_eq!(suricata_events.len(), 1, "one observed ET SCAN alert");
    assert!(
        suricata_events[0].evidence_ref.contains("ET SCAN"),
        "suricata observed scan evidence"
    );
    assert_eq!(
        suricata_events[0].correlation.as_deref(),
        Some(&format!("src={SCANNER}") as &str)
    );

    let cs_event = crowdsec
        .lookup_reputation(&tenant(), SCANNER)
        .expect("crowdsec lookup")
        .expect("crowdsec ban decision observed");
    assert!(cs_event.evidence_ref.contains(SCANNER));

    let wz_events = wazuh.read_telemetry(&tenant()).expect("wazuh telemetry");
    assert_eq!(wz_events.len(), 1);
    assert!(wz_events[0].severity == nexus_sentinel::FindingSeverity::High);

    let osq_events = osquery
        .read_telemetry(&tenant())
        .expect("osquery telemetry");
    assert_eq!(osq_events.len(), 1);
    assert!(osq_events[0].evidence_ref.contains("8443"));

    let opn_caps = opnsense.capabilities();
    assert!(opn_caps.contains(SentinelCapabilityKind::ReadFirewallTelemetry));
    assert!(opn_caps.contains(SentinelCapabilityKind::Containment));

    // ---- 4. DERIVED: normalized facts with provenance ----
    let normalized_facts = serde_json::json!([
        {
            "source": "zeek",
            "provider": "ZeekNetworkDetectionProvider",
            "profile": "ZEEK",
            "resource": "C1",
            "observed": true,
            "fact": format!("Scan::Portscan from {SCANNER} to {TARGET}:22 (src={SCANNER})"),
            "event_id": zeek_events[0].event_id.as_str()
        },
        {
            "source": "suricata",
            "provider": "SuricataNetworkDetectionProvider",
            "profile": "SURICATA",
            "resource": "9101",
            "observed": true,
            "fact": format!("ET SCAN Potential SSH Scan from {SCANNER} to {TARGET}:22 (src={SCANNER})"),
            "event_id": suricata_events[0].event_id.as_str()
        },
        {
            "source": "crowdsec",
            "provider": "CrowdSecThreatIntelProvider",
            "profile": "CROWDSEC",
            "resource": SCANNER,
            "observed": true,
            "fact": format!("ban decision crowdsecurity/port-scan for {SCANNER}"),
            "event_id": cs_event.event_id.as_str()
        },
        {
            "source": "wazuh",
            "provider": "WazuhEndpointTelemetryProvider",
            "profile": "WAZUH",
            "resource": "srv-lab-1",
            "observed": true,
            "fact": format!("rule level 12 alert Host brute force attack on {TARGET}"),
            "event_id": wz_events[0].event_id.as_str()
        },
        {
            "source": "osquery",
            "provider": "OsqueryEndpointTelemetryProvider",
            "profile": "OSQUERY",
            "resource": "srv-lab-1",
            "observed": true,
            "fact": "wildcard listening socket 0.0.0.0:8443 observed via listening_ports",
            "event_id": osq_events[0].event_id.as_str()
        }
    ]);

    // ---- 5. CORRELATED: incident over compatible observed facts ----
    // All observed events join the incident window; the correlation
    // KEY is the shared observed source indicator (192.168.40.77)
    // corroborated by the network plane (Zeek) and the reputation
    // plane (CrowdSec). Confidence derives from independent planes on
    // the SAME indicator, never from raw sensor count.
    let triage = SentinelTriageService;
    let mut all_events = zeek_events.clone();
    all_events.extend(suricata_events.clone());
    all_events.push(cs_event.clone());
    all_events.extend(wz_events.clone());
    all_events.extend(osq_events.clone());
    let incident = triage
        .triage_events(
            &tenant(),
            IncidentCorrelationId::new("corr-lf009").unwrap(),
            &all_events,
        )
        .expect("triage correlates");
    assert_eq!(incident.confidence, CorrelationConfidence::High);
    assert!(incident.summary.contains(SCANNER));
    assert_eq!(
        incident.event_ids.len(),
        all_events.len(),
        "dedup, no flood"
    );

    // ---- 6. TRIAGED: bounded priority case ----
    let case = triage
        .prioritize(
            &tenant(),
            TriageCaseId::new("case-lf009").unwrap(),
            &incident,
        )
        .expect("prioritize");
    assert_eq!(
        case.priority,
        nexus_sentinel_advanced::TriagePriority::Critical
    );

    // ---- 7. INVESTIGATED: evidence preserved ----
    let investigator = SentinelInvestigationService;
    let investigation = investigator
        .investigate(&tenant(), &incident)
        .expect("investigate");
    assert!(investigation.evidence_refs.len() >= 4);

    // ---- 8. RECOMMENDED: quarantine proposal (DATA, zero mutation) ----
    // The provider proposal is DATA - creating it mutates nothing. It
    // carries the provider-specific reversibility proof (AUD-031):
    // only a proposal the provider certifies as reversible can
    // preauthorize auto-execution.
    let device = unknown_device();
    let proposed = opnsense
        .propose_containment(&tenant(), None, &device, Some(SCANNER))
        .expect("propose containment");
    assert_eq!(proposed.state, QuarantineState::Proposed);
    assert!(proposed.preauthorized && proposed.reversible);
    assert_eq!(proposed.approval_class, ApprovalClass::Human);
    let reversibility_proof = format!(
        "opnsense:proposal:{}:reversible",
        proposed.proposal_id.as_str()
    );

    let planner = SentinelResponsePlanner;
    let plan = planner
        .plan_response(
            &tenant(),
            ResponsePlanId::new("plan-lf009").unwrap(),
            &incident,
            ResponseKind::Quarantine,
            ApprovalClass::Human,
            Some(&reversibility_proof),
        )
        .expect("quarantine plan")
        // AUD-033: the plan BINDS the exact proposal it will verify.
        // Verification of this plan can never read back a different
        // proposal's evidence.
        .with_quarantine(proposed.proposal_id.as_str());
    assert!(
        plan.preauthorized,
        "high-confidence bounded containment with provider reversibility proof may be preauthorized"
    );
    assert_eq!(plan.kind, ResponseKind::Quarantine);
    assert_eq!(
        plan.reversibility_proof.as_deref(),
        Some(reversibility_proof.as_str()),
        "preauthorization binds the provider reversibility proof"
    );

    // AUD-031: a bounded plan WITHOUT the provider reversibility
    // proof is NOT preauthorized - it may execute under human
    // approval but never auto-execute.
    let no_proof_plan = planner
        .plan_response(
            &tenant(),
            ResponsePlanId::new("plan-no-proof").unwrap(),
            &incident,
            ResponseKind::Quarantine,
            ApprovalClass::Human,
            None,
        )
        .expect("bounded plan without proof is still a plan");
    assert!(
        !no_proof_plan.preauthorized,
        "bounded plan without provider reversibility proof must fail closed"
    );

    // DESTRUCTIVE NEVER PREAUTHORIZED: no threat score may mint
    // authorization. Planning a wipe under Policy fails closed; even
    // under Human it is never preauthorized.
    let denied = planner.plan_response(
        &tenant(),
        ResponsePlanId::new("plan-destructive-denied").unwrap(),
        &incident,
        ResponseKind::Wipe,
        ApprovalClass::Policy,
        None,
    );
    assert!(
        denied.is_err(),
        "destructive response without human procedure must fail closed"
    );
    let human_wipe = planner
        .plan_response(
            &tenant(),
            ResponsePlanId::new("plan-destructive-human").unwrap(),
            &incident,
            ResponseKind::Wipe,
            ApprovalClass::Human,
            None,
        )
        .expect("human wipe plan allowed");
    assert!(
        !human_wipe.preauthorized,
        "destructive response is never preauthorized"
    );

    // ---- 9. AUTHORIZED: policy permits (approved + reversible) ----
    let approved = approved_proposal_from(&opnsense, proposed);

    // ---- 10. EXECUTED: real provider mutation (addRule + apply) ----
    let applied = opnsense
        .apply_containment(&approved)
        .expect("apply containment");
    assert_eq!(applied.state, QuarantineState::Applied);
    assert!(
        applied.rule_ref.is_some(),
        "provider acceptance rule_ref required"
    );

    // ---- 11. VERIFIED: independent exact-target readback ----
    let verifier = SentinelVerificationService::new();
    verifier.bind_firewall(opnsense.clone());
    verifier.register_applied(applied.clone());
    let verification = verifier
        .verify_response(
            &tenant(),
            VerificationRecordId::new("verify-lf009").unwrap(),
            &plan,
        )
        .expect("verify response");
    assert_eq!(
        verification.state,
        nexus_sentinel_advanced::VerificationState::Verified,
        "independent exact-target readback must verify"
    );
    {
        let st = opn_rules.lock().unwrap();
        assert!(
            st.iter().any(|r| r.description
                == format!("nexus-quarantine-{}", applied.proposal_id.as_str())
                && r.enabled
                && r.action == "block"),
            "created rule must exist enabled block in provider state"
        );
    }

    // AUD-033 hostile live-fire: a plan bound to a DIFFERENT proposal
    // is refused BEFORE any firewall readback - even against the real
    // engine, proposal B's evidence can never verify plan A.
    let other_plan = planner
        .plan_response(
            &tenant(),
            ResponsePlanId::new("plan-other").unwrap(),
            &incident,
            ResponseKind::Quarantine,
            ApprovalClass::Human,
            Some(&reversibility_proof),
        )
        .expect("other quarantine plan")
        .with_quarantine("proposal-other");
    let denied = verifier.verify_response(
        &tenant(),
        VerificationRecordId::new("verify-other").unwrap(),
        &other_plan,
    );
    assert!(
        denied.is_err(),
        "cross-proposal verification must fail closed"
    );

    // ---- 12. REVOKED: reversible rollback (toggleRule 0 + apply) ----
    let revoked = opnsense
        .revoke_containment(&applied)
        .expect("revoke containment");
    assert_eq!(revoked.state, QuarantineState::Revoked);
    let after = opnsense
        .verify_containment(&revoked)
        .expect("verify after revoke");
    assert!(!after.verified, "revoked rule must no longer verify");

    // ---- 13. Redaction canary: zero leakage ----
    let audit = opnsense.audit();
    let audit_text = serde_json::to_string(&audit).unwrap_or_default();
    for canary in [
        CANARY_OPN_KEY,
        CANARY_OPN_SECRET,
        CANARY_CS_ID,
        CANARY_CS_PASS,
        CANARY_WZ_USER,
        CANARY_WZ_PASS,
        CANARY_OSQ_SECRET,
    ] {
        assert!(
            !audit_text.contains(canary),
            "canary {canary} leaked into audit"
        );
    }
    for ring in [wazuh.audit_entries()] {
        let text = serde_json::to_string(&ring).unwrap_or_default();
        for canary in [
            CANARY_CS_ID,
            CANARY_CS_PASS,
            CANARY_WZ_USER,
            CANARY_WZ_PASS,
            CANARY_OSQ_SECRET,
        ] {
            assert!(
                !text.contains(canary),
                "canary {canary} leaked into connector audit"
            );
        }
    }
    {
        let ring = osquery.audit_entries();
        let text = serde_json::to_string(&ring).unwrap_or_default();
        for canary in [
            CANARY_CS_ID,
            CANARY_CS_PASS,
            CANARY_WZ_USER,
            CANARY_WZ_PASS,
            CANARY_OSQ_SECRET,
        ] {
            assert!(
                !text.contains(canary),
                "canary {canary} leaked into connector audit"
            );
        }
    }

    // ---- 14. Current-run machine-readable evidence ----
    let evidence = serde_json::json!({
        "run_id": run_id(),
        "node": "EP-031",
        "milestone": "M5",
        "proof": "LF-009",
        "surface": "documented Zeek JSON Streaming Logs + documented Suricata EVE JSON + documented CrowdSec LAPI + documented Wazuh server API + documented osquery TLS remote API + documented OPNsense firewall automation API",
        "transport": "JsonLinesZeekTransport over REAL socket + JsonLinesSuricataTransport over REAL socket + HttpCrowdSecTransport + HttpWazuhTransport + HttpOsqueryEndpoint (REAL std::net sockets) + HttpOpnsenseTransport",
        "sensors": {
            "zeek": { "events": zeek_events.len(), "source": SCANNER, "kind": "ScanDetected" },
            "suricata": { "events": suricata_events.len(), "source": SCANNER, "kind": "ScanDetected", "signature": "ET SCAN Potential SSH Scan" },
            "crowdsec": { "event": true, "indicator": SCANNER, "action": "ban" },
            "wazuh": { "alerts": wz_events.len(), "rule_level": 12, "severity": "HIGH" },
            "osquery": { "events": osq_events.len(), "wildcard_listener": "0.0.0.0:8443" }
        },
        "normalized_facts": normalized_facts,
        "incident": {
            "correlation_id": incident.correlation_id.as_str(),
            "severity": incident.severity.as_str(),
            "confidence": incident.confidence.as_str(),
            "summary": incident.summary,
            "event_count": incident.event_ids.len()
        },
        "triage": { "priority": case.priority.as_str() },
        "investigation": { "evidence_refs": investigation.evidence_refs.len() },
        "response": {
            "kind": plan.kind.as_str(),
            "preauthorized": plan.preauthorized,
            "state": plan.state.as_str(),
            "destructive_denied": true,
            "destructive_never_preauthorized": true
        },
        "execution": { "state": applied.state.as_str(), "rule_ref": applied.rule_ref },
        "verification": {
            "state": verification.state.as_str(),
            // AUD-033: the verified record binds the plan to the EXACT
            // applied proposal; a different proposal's evidence can
            // never satisfy this plan.
            "plan_binds_proposal": matches!(
                plan.quarantine_proposal_ref.as_deref(),
                Some(r) if r == applied.proposal_id.as_str()
            )
        },
        "rollback": { "state": revoked.state.as_str(), "verify_after_revoke": false },
        "correlation_rule": "incident confidence derives from independent observation planes corroborating the SAME observed source indicator; raw sensor count never inflates confidence",
        "redaction": "ZERO_LEAKAGE",
        "certification": {
            "advanced_contract": "INTERNAL_CERTIFIED",
            "zeek_connector": "TRANSPORT_CERTIFIED over real sockets vs controlled fixtures",
            "suricata_connector": "TRANSPORT_CERTIFIED over real sockets vs controlled fixtures (AUD-030)",
            "crowdsec_connector": "TRANSPORT_CERTIFIED over real sockets vs controlled fixtures",
            "wazuh_connector": "TRANSPORT_CERTIFIED over real sockets vs controlled fixtures",
            "osquery_connector": "TRANSPORT_CERTIFIED over real sockets vs controlled fixture node",
            "opnsense_connector": "TRANSPORT_CERTIFIED over real sockets vs controlled fixtures",
            "real_sensors": "NOT_ASSERTED",
            "real_firewall_appliance": "NOT_ASSERTED"
        }
    });
    write_evidence(evidence);

    // ---- 15. Zero-orphan cleanup: fixture threads bounded and done ----
    let _ = zeek_handle.join();
    let _ = suricata_handle.join();
    let _ = cs_handle.join();
    let _ = wz_handle.join();
    let _ = opn_handle.join();
}
