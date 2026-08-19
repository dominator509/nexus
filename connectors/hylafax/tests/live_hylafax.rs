//! EP-027 M3 LIVE integration tests: production nexus-hylafax against
//! the real HylaFAX controlled fixture (hfaxd + faxq in the
//! nexus-hylafax-fixture container, image minichip/hylafax:latest at
//! sha256:00decb6c89fb4337534e9b4e82ff279cb53a492124bd083015cf82c354111613,
//! HylaFAX 6.0.6-8.1).
//!
//! These tests are gated by HYLAFAX_LIVE=1 so the default test run
//! stays hermetic; the M3 gate script exports the variable after
//! proving the fixture is up (hfaxd reachable, faxq live).
//!
//! The tests MUST run inside the fixture network namespace
//! (HYLAFAX_HOST=172.17.0.3): the EPRT data listener must be reachable
//! by hfaxd, and the wildcard hosts.hfaxd entry is what exercises the
//! real password path (331 -> PASS -> 230/530). The localhost entry
//! auto-authenticates (230 without password), so the wrong-password
//! case uses the container's eth0 address to force the real 530.
//!
//! Controlled fixture credential: the `nexustest` user has a known
//! test-only password (set via hosts.hfaxd crypt hash). It is a
//! fixture secret, never committed and never present in evidence.

use nexus_fax::{
    submit_governed, FaxDirection, FaxDocument, FaxDocumentId, FaxJob, FaxJobId, FaxNumber,
    FaxProvider, FaxProviderKind, FaxScanStatus, FaxSendRequest, FaxState, FaxStatus,
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

/// Create a controlled runtime document (PostScript) with a known
/// digest; returns (path, sha256).
fn make_document() -> (String, String) {
    use sha2::{Digest, Sha256};
    let body = "%!PS-Adobe-3.0\n%%BoundingBox: 24 36 577 777\n%%Pages: 1\n%%EndComments\n0 0 moveto\n(EP-027 M3 live fixture document) show\nshowpage\n%%EOF\n";
    let path = "/tmp/ep027-m3-live-doc.ps";
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

fn make_job(id: &str, key: &str, doc_path: &str, digest: &str) -> FaxJob {
    let from_num: String = format!("+1555{}", "0100");
    let to_num: String = format!("+1555{}", "0200");
    FaxJob {
        id: FaxJobId::new(id).expect("job id"),
        direction: FaxDirection::Outbound,
        from: FaxNumber::new(from_num).expect("from"),
        to: FaxNumber::new(to_num).expect("to"),
        document: FaxDocument {
            id: FaxDocumentId::new(format!("doc-{id}")).expect("doc id"),
            filename: "ep027-m3-live-doc.ps".into(),
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
        approval_class: 2,
        correlation: None,
    }
}

/// Independent provider/spool verification (controlled fixture only):
/// the exact carrier job file exists in sendq and the stored document
/// in docq matches the uploaded digest byte-for-byte.
fn assert_spool_oracle(carrier: &str, digest: &str) {
    let sendq_path = format!("/var/spool/hylafax/sendq/q{carrier}");
    assert!(
        std::path::Path::new(&sendq_path).exists(),
        "sendq job file {sendq_path} must exist for carrier {carrier}"
    );
    let docq_dir = std::path::Path::new("/var/spool/hylafax/docq");
    let mut matched = false;
    for entry in std::fs::read_dir(docq_dir).expect("read docq dir") {
        let entry = entry.expect("docq entry");
        let name = entry.file_name().to_string_lossy().to_string();
        // docq/doc<docid>.ps.<jobid>
        if name.ends_with(&format!(".ps.{carrier}")) {
            let bytes = std::fs::read(entry.path()).expect("read docq artifact");
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let stored = hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            assert_eq!(stored, digest, "docq artifact {name} digest mismatch");
            matched = true;
        }
    }
    assert!(
        matched,
        "docq artifact for carrier {carrier} not found (.ps.{carrier})"
    );
}

#[test]
fn ep027_live_hylafax_full_submission_round_trip() {
    if !live() {
        eprintln!("skipping live hylafax test (HYLAFAX_LIVE != 1)");
        return;
    }
    let (doc_path, digest) = make_document();
    let provider = build_hylafax_provider(host(), port(), username(), password(), 1);
    let job = make_job("job-live-1", "key-live-1", &doc_path, &digest);
    let req = FaxSendRequest {
        job: job.id.clone(),
        idempotency_key: "key-live-1".into(),
        approval_class: 2,
    };
    let carrier = submit_governed(&provider, &job, &req, 1).expect("governed submit");
    assert!(
        !carrier.as_str().is_empty(),
        "provider job id must not be empty"
    );
    eprintln!("EP-027 M3 live: carrier job id = {carrier}");

    // Exact-target provider readback: query by the provider-assigned
    // carrier job id (never a same-destination/owner heuristic).
    let status = provider
        .status(&FaxJobId::new(carrier.as_str()).expect("carrier id"))
        .expect("provider readback");
    assert_eq!(
        status.state,
        FaxState::Submitted,
        "queued job maps to SUBMITTED, never DELIVERED"
    );
    assert_eq!(status.carrier, FaxProviderKind::HylaFax);
    assert_eq!(
        status.carrier_job_id.as_ref().map(|c| c.as_str()),
        Some(carrier.as_str()),
        "readback must bind to the exact provider job"
    );

    // Independent spool oracle: sendq job file + byte-exact docq
    // artifact for the exact carrier job.
    assert_spool_oracle(carrier.as_str(), &digest);

    // Replay with the same idempotency key must NOT create a second
    // hfaxd job: same provider job id returned, zero second mutation.
    let req2 = FaxSendRequest {
        job: job.id.clone(),
        idempotency_key: "key-live-1".into(),
        approval_class: 2,
    };
    let carrier2 = submit_governed(&provider, &job, &req2, 1).expect("replay");
    assert_eq!(
        carrier.as_str(),
        carrier2.as_str(),
        "replay must deduplicate to the same provider job id"
    );
    eprintln!("EP-027 M3 live: replay deduplicated to {carrier2}");
}

#[test]
fn ep027_live_hylafax_wrong_password_fails_closed() {
    if !live() {
        eprintln!("skipping live hylafax test (HYLAFAX_LIVE != 1)");
        return;
    }
    let provider = build_hylafax_provider(host(), port(), username(), "definitely-wrong", 1);
    let (doc_path, digest) = make_document();
    let job = make_job("job-live-2", "key-live-2", &doc_path, &digest);
    let req = FaxSendRequest {
        job: job.id.clone(),
        idempotency_key: "key-live-2".into(),
        approval_class: 2,
    };
    let err =
        submit_governed(&provider, &job, &req, 1).expect_err("wrong password must fail closed");
    assert_eq!(
        err.code,
        nexus_fax::FaxErrorCode::Authorization,
        "real hfaxd 530 must map to the canonical Authorization error"
    );
    eprintln!("EP-027 M3 live: wrong password rejected: {}", err.message);
}

#[test]
fn ep027_live_hylafax_scheduler_nak_not_submitted() {
    if !live() {
        eprintln!("skipping live hylafax test (HYLAFAX_LIVE != 1)");
        return;
    }
    // Regression for the observed scheduler NAK: a job with incomplete
    // required configuration (no document attached; observed 460
    // "scheduler NAK'd request") must be rejected by the real
    // scheduler and NEVER claimed SUBMITTED. Uses the raw transport
    // directly; the governed adapter never submits intentionally
    // broken jobs during normal execution.
    let transport = HylaFaxTcpTransport::new(host(), port(), username(), password());
    transport
        .connect_authenticate(&host(), port(), &username(), &password())
        .expect("auth");
    transport.prepare_transfer().expect("prepare transfer");
    let job_id = transport.create_job().expect("jnew");
    for (k, v) in [
        ("FROMUSER", username()),
        ("DIALSTRING", "15551234567".to_string()),
        ("LASTTIME", "000259".to_string()),
        ("MAXDIALS", "12".to_string()),
        ("MAXTRIES", "3".to_string()),
    ] {
        transport.set_job_parameter(k, &v).expect("jparm");
    }
    let err = transport
        .submit_job()
        .expect_err("scheduler must NAK an incomplete job");
    // The transport maps the server NAK to Unavailable; the message
    // carries the exact server code (460 observed; 504 for other
    // missing requirements). Either way, no SUBMITTED is claimed.
    assert!(
        err.message.contains("460") || err.message.contains("504"),
        "expected scheduler NAK code in message, got: {}",
        err.message
    );
    eprintln!(
        "EP-027 M3 live: scheduler NAK for job {job_id}: {}",
        err.message
    );
    let _ = transport.quit();
}
