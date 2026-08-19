//! EP-027 M4 LIVE forced-failure tests: production nexus-hylafax
//! against the real HylaFAX controlled fixture, exercising REAL
//! failure mechanisms (terminating the server process, policy denial,
//! credential rejection) with fail-closed proofs, bounded recovery,
//! redaction canaries, and zero-orphan teardown.
//!
//! These tests are gated by HYLAFAX_LIVE=1 and MUST run inside the
//! fixture network namespace (HYLAFAX_HOST=172.17.0.3), sequentially
//! (--test-threads=1): the hfaxd-down test mutates shared fixture
//! state (kills and restarts the real hfaxd process).
//!
//! Controlled fixture credential: known TEST-ONLY password (see the
//! M3 live test header). Never committed in evidence.

use std::io::Read;
use std::net::SocketAddr;
use std::process::Stdio;

use nexus_fax::{
    submit_governed, FaxDirection, FaxDocument, FaxDocumentId, FaxErrorCode, FaxJob, FaxJobId,
    FaxNumber, FaxProviderKind, FaxScanStatus, FaxSendRequest, FaxState, FaxStatus,
};
use nexus_hylafax::transport::HylaFaxTcpTransport;
use nexus_hylafax::{build_hylafax_provider, HylaFaxTransport};

fn live() -> bool {
    std::env::var("HYLAFAX_LIVE")
        .map(|v| v == "1")
        .unwrap_or(false)
}

fn host() -> String {
    std::env::var("HYLAFAX_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn port() -> u16 {
    std::env::var("HYLAFAX_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4559)
}

fn username() -> String {
    std::env::var("HYLAFAX_USER").unwrap_or_else(|_| "nexustest".to_string())
}

fn password() -> String {
    std::env::var("HYLAFAX_PASS").unwrap_or_else(|_| "nexustest-pw".to_string())
}

fn make_document() -> (String, String) {
    use sha2::{Digest, Sha256};
    let body = "%!PS-Adobe-3.0\n%%BoundingBox: 24 36 577 777\n%%Pages: 1\n%%EndComments\n0 0 moveto\n(EP-027 M4 failure fixture document) show\nshowpage\n%%EOF\n";
    let path = "/tmp/ep027-m4-fail-doc.ps";
    std::fs::write(path, body).expect("write doc");
    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    let digest = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    (path.to_string(), digest)
}

fn make_job(id: &str, key: &str, doc_path: &str, digest: &str, approval_class: u8) -> FaxJob {
    let from_num: String = format!("+1555{}", "0100");
    let to_num: String = format!("+1555{}", "0200");
    FaxJob {
        id: FaxJobId::new(id).expect("job id"),
        direction: FaxDirection::Outbound,
        from: FaxNumber::new(from_num).expect("from"),
        to: FaxNumber::new(to_num).expect("to"),
        document: FaxDocument {
            id: FaxDocumentId::new(format!("doc-{id}")).expect("doc id"),
            filename: "ep027-m4-fail-doc.ps".into(),
            content_type: "application/postscript".into(),
            size_bytes: 140,
            pages: 1,
            sha256: digest.to_string(),
            storage_ref: doc_path.to_string(),
            scan_status: FaxScanStatus::Clean,
        },
        carrier: FaxProviderKind::HylaFax,
        status: FaxStatus {
            state: FaxState::Queued,
            carrier: FaxProviderKind::HylaFax,
            attempts: 0,
            max_attempts: 3,
            pages: 1,
            carrier_job_id: None,
            detail: "queued".into(),
        },
        idempotency_key: key.to_string(),
        approval_class,
        correlation: None,
    }
}

fn sendq_job_count() -> usize {
    let dir = std::path::Path::new("/var/spool/hylafax/sendq");
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with('q'))
                .count()
        })
        .unwrap_or(0)
}

/// Wait (bounded) until TCP connect to host:port FAILS.
fn wait_port_closed(secs: u64) -> bool {
    let addr: SocketAddr = format!("{}:{}", host(), port())
        .parse()
        .expect("socket addr");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while std::time::Instant::now() < deadline {
        let ok = std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500));
        if ok.is_err() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    false
}

/// Wait (bounded) until hfaxd answers with a 220 greeting.
fn wait_greeting(secs: u64) -> bool {
    let addr: SocketAddr = format!("{}:{}", host(), port())
        .parse()
        .expect("socket addr");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    while std::time::Instant::now() < deadline {
        if let Ok(mut s) =
            std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(500))
        {
            let mut buf = [0u8; 128];
            s.set_read_timeout(Some(std::time::Duration::from_millis(500)))
                .ok();
            if let Ok(n) = s.read(&mut buf) {
                if n > 0 && String::from_utf8_lossy(&buf[..n]).contains("220") {
                    return true;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    false
}

#[test]
fn ep027_failure_hfaxd_down_truthful_unavailable() {
    if !live() {
        eprintln!("skipping live hylafax failure test (HYLAFAX_LIVE != 1)");
        return;
    }
    // REAL failure mechanism: terminate the real hfaxd process.
    let _ = std::process::Command::new("pkill")
        .args(["-x", "hfaxd"])
        .status();
    assert!(
        wait_port_closed(10),
        "hfaxd port did not close after termination"
    );

    // The transport must fail closed with Unavailable - never
    // fabricate a session.
    let transport = HylaFaxTcpTransport::new(host(), port(), username(), password());
    let err = transport
        .connect_authenticate(&host(), port(), &username(), &password())
        .expect_err("hfaxd down must fail closed");
    assert_eq!(
        err.code,
        FaxErrorCode::Unavailable,
        "down provider must be Unavailable, got {}",
        err.code.as_str()
    );
    eprintln!("EP-027 M4 live: hfaxd down -> Unavailable: {}", err.message);

    // Bounded recovery: restart the real hfaxd and prove a fresh
    // session authenticates (the fixture is left RUNNING).
    let child = std::process::Command::new("/usr/sbin/hfaxd")
        .args(["-i", &port().to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn hfaxd");
    drop(child);
    assert!(wait_greeting(30), "hfaxd did not recover after restart");
    let recovered = HylaFaxTcpTransport::new(host(), port(), username(), password());
    recovered
        .connect_authenticate(&host(), port(), &username(), &password())
        .expect("recovered session authenticates");
    let _ = recovered.quit();
    eprintln!("EP-027 M4 live: hfaxd recovered and re-authenticated");
}

#[test]
fn ep027_failure_policy_denied_zero_mutation() {
    if !live() {
        eprintln!("skipping live hylafax failure test (HYLAFAX_LIVE != 1)");
        return;
    }
    let before = sendq_job_count();
    let provider = build_hylafax_provider(host(), port(), username(), password(), 1);
    let (doc_path, digest) = make_document();
    let job = make_job("job-fail-policy", "key-fail-policy", &doc_path, &digest, 0);
    let req = FaxSendRequest {
        job: job.id.clone(),
        idempotency_key: "key-fail-policy".into(),
        approval_class: 0,
    };
    let err = submit_governed(&provider, &job, &req, 1).expect_err("policy must deny");
    assert_eq!(err.code, FaxErrorCode::Policy);
    let after = sendq_job_count();
    assert_eq!(
        before, after,
        "denied send must create zero provider jobs (before {before}, after {after})"
    );
    eprintln!("EP-027 M4 live: policy denial zero provider mutation ({before} -> {after})");
}

#[test]
fn ep027_failure_redaction_canaries() {
    if !live() {
        eprintln!("skipping live hylafax failure test (HYLAFAX_LIVE != 1)");
        return;
    }
    // Trigger a REAL failure path (wrong password -> 530) and scan the
    // entire audit ring for credential leakage. The wrong password is
    // a unique canary string so any leak is unambiguous.
    let canary = "wrong-password-canary-xyz";
    let bad = build_hylafax_provider(host(), port(), username(), canary, 1);
    let (doc_path, digest) = make_document();
    let job = make_job("job-fail-redact", "key-fail-redact", &doc_path, &digest, 2);
    let req = FaxSendRequest {
        job: job.id.clone(),
        idempotency_key: "key-fail-redact".into(),
        approval_class: 2,
    };
    let err = submit_governed(&bad, &job, &req, 1).expect_err("wrong password must fail");
    assert_eq!(err.code, FaxErrorCode::Authorization);

    let entries = bad.audit();
    assert!(!entries.is_empty(), "audit ring must record the failure");
    for entry in &entries {
        assert!(
            !entry.detail.contains(canary),
            "wrong-password canary leaked into audit detail: {}",
            entry.detail
        );
        assert!(
            !entry.detail.contains(&password()),
            "fixture password leaked into audit detail: {}",
            entry.detail
        );
        for (k, v) in &entry.fields {
            assert!(
                !v.contains(canary) && !v.contains(&password()),
                "credential leaked into audit field {k}: {v}"
            );
        }
    }
    eprintln!(
        "EP-027 M4 live: redaction canaries clean across {} audit entries",
        entries.len()
    );
}
