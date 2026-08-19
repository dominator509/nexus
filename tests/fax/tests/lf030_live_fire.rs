//! LF-030 fax lifecycle live-fire (EP-027 M5).
//!
//! Runs the REAL outbound fax lifecycle through the production
//! nexus-hylafax connector (FaxProvider port) against the real pinned
//! HylaFAX fixture (hfaxd + faxq, minichip/hylafax@sha256:00decb6c...,
//! 6.0.6-8.1), then writes current-run machine-readable evidence under
//! `.agent/state/evidence/LF-030-ep027-m5.json` embedding
//! `EP027_M5_RUN_ID` (stale evidence never satisfies the gate).
//!
//! Lifecycle proved: governed submit -> provider-assigned carrier job
//! id -> exact-target LIST readback (SUBMITTED ceiling, never
//! DELIVERED) -> independent spool oracle (sendq job file + docq
//! artifact byte-exact digest) -> idempotent replay dedup -> real 530
//! wrong-password failure path. Certification boundaries are recorded
//! honestly: physical modem / PSTN / remote receiver / DELIVERED are
//! NOT ASSERTED.
//!
//! Gated by HYLAFAX_LIVE=1; must run inside the fixture network
//! namespace (EPRT data listener reachability).

use std::path::PathBuf;

use nexus_fax::{
    submit_governed, FaxDirection, FaxDocument, FaxDocumentId, FaxErrorCode, FaxJob, FaxJobId,
    FaxNumber, FaxProvider, FaxProviderKind, FaxScanStatus, FaxSendRequest, FaxState, FaxStatus,
};
use nexus_hylafax::build_hylafax_provider;

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

fn run_id() -> String {
    std::env::var("EP027_M5_RUN_ID").unwrap_or_else(|_| "local-run".to_string())
}

fn make_document() -> (String, String) {
    use sha2::{Digest, Sha256};
    let body = "%!PS-Adobe-3.0\n%%BoundingBox: 24 36 577 777\n%%Pages: 1\n%%EndComments\n0 0 moveto\n(LF-030 live-fire fixture document) show\nshowpage\n%%EOF\n";
    let path = "/tmp/lf030-live-doc.ps";
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
            filename: "lf030-live-doc.ps".into(),
            content_type: "application/postscript".into(),
            size_bytes: 150,
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

#[test]
fn ep027_m5_lf030_lifecycle() {
    if !live() {
        eprintln!("skipping LF-030 live-fire test (HYLAFAX_LIVE != 1)");
        return;
    }
    let (doc_path, digest) = make_document();
    let provider = build_hylafax_provider(host(), port(), username(), password(), 1);

    // 1. Governed submit -> provider-assigned carrier job id.
    let job = make_job("job-lf030", "key-lf030", &doc_path, &digest, 2);
    let req = FaxSendRequest {
        job: job.id.clone(),
        idempotency_key: "key-lf030".into(),
        approval_class: 2,
    };
    let carrier = submit_governed(&provider, &job, &req, 1).expect("governed submit");
    eprintln!("EP-027 M5 LF-030: carrier job id = {carrier}");

    // 2. Exact-target provider readback (provider-assigned id).
    let status = provider
        .status(&FaxJobId::new(carrier.as_str()).expect("carrier id"))
        .expect("provider readback");
    assert_eq!(
        status.state,
        FaxState::Submitted,
        "queued job maps to SUBMITTED, never DELIVERED"
    );
    assert_eq!(
        status.carrier_job_id.as_ref().map(|c| c.as_str()),
        Some(carrier.as_str())
    );

    // 3. Independent spool oracle: sendq job file + byte-exact docq
    //    artifact for the exact carrier job.
    let sendq_path = format!("/var/spool/hylafax/sendq/q{carrier}");
    assert!(
        std::path::Path::new(&sendq_path).exists(),
        "sendq job file missing for carrier {carrier}"
    );
    let mut matched = false;
    let docq_dir = std::path::Path::new("/var/spool/hylafax/docq");
    for entry in std::fs::read_dir(docq_dir).expect("read docq dir") {
        let entry = entry.expect("docq entry");
        let name = entry.file_name().to_string_lossy().to_string();
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
            assert_eq!(stored, digest, "docq artifact digest mismatch");
            matched = true;
        }
    }
    assert!(matched, "docq artifact for carrier {carrier} not found");

    // 4. Idempotent replay dedup: same key -> same carrier job id,
    //    zero second provider mutation.
    let before_replay = sendq_job_count();
    let req2 = FaxSendRequest {
        job: job.id.clone(),
        idempotency_key: "key-lf030".into(),
        approval_class: 2,
    };
    let carrier2 = submit_governed(&provider, &job, &req2, 1).expect("replay");
    assert_eq!(carrier.as_str(), carrier2.as_str(), "replay must dedup");
    assert_eq!(
        before_replay,
        sendq_job_count(),
        "replay must not create a second job"
    );

    // 5. Real 530 wrong-password failure path (zero mutation).
    let bad = build_hylafax_provider(host(), port(), username(), "wrong-password-canary-abc", 1);
    let before_fail = sendq_job_count();
    let err = submit_governed(&bad, &job, &req, 1).expect_err("wrong password must fail");
    assert_eq!(err.code, FaxErrorCode::Authorization);
    assert_eq!(before_fail, sendq_job_count(), "530 must not mutate spool");

    // 6. Machine-readable current-run evidence (redacted; stale
    //    evidence never satisfies the gate: run_id must match).
    let evidence = serde_json::json!({
        "proof": "LF-030",
        "node": "EP-027",
        "milestone": "M5",
        "run_id": run_id(),
        "provider": {
            "kind": "HYLA_FAX",
            "runtime_version": "3:6.0.6-8.1~ubuntu0.18.04.1",
            "image_digest": "sha256:00decb6c89fb4337534e9b4e82ff279cb53a492124bd083015cf82c354111613",
            "hfaxd_port": port(),
            "faxq_running": true,
            "container": "CONTROLLED_TEST_FIXTURE"
        },
        "lifecycle": {
            "submitted_carrier_job_id": carrier.as_str(),
            "readback_state": "SUBMITTED",
            "readback_exact_target": true,
            "spool_sendq_file": format!("/var/spool/hylafax/sendq/q{carrier}"),
            "document_digest": digest,
            "document_stored_byte_exact": true,
            "replay_dedup_same_id": true,
            "replay_zero_second_mutation": true,
            "wrong_password_530": {
                "canonical_code": "AUTHORIZATION",
                "zero_mutation": true
            }
        },
        "certification": {
            "nexus_hylafax": "IMPLEMENTED",
            "hfaxd_control_protocol": "PROTOCOL_CERTIFIED",
            "active_eprt_data_channel": "PROTOCOL_CERTIFIED",
            "mode_z_stot_upload": "PROTOCOL_CERTIFIED",
            "hylafax_6_0_6_8_1_fixture": "PROVIDER_CERTIFIED",
            "faxq_job_acceptance": "PROVIDER_CERTIFIED",
            "exact_provider_query_readback": "PROVIDER_CERTIFIED",
            "document_transfer_integrity": "CERTIFIED_FOR_TESTED_PATH",
            "container": "CONTROLLED_TEST_FIXTURE",
            "physical_modem": "NOT_ASSERTED",
            "pstn": "NOT_ASSERTED",
            "remote_fax_receiver": "NOT_ASSERTED",
            "delivered": "NOT_ASSERTED"
        }
    });
    // Anchor to the workspace root (CARGO_MANIFEST_DIR parent), never to
    // the test binary's cwd: cargo runs tests with cwd = package root, so a
    // relative path would land outside the canonical evidence location
    // (.agent/state/evidence/ at the workspace root).
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().expect("workspace root");
    let evidence_path = workspace_root.join(".agent/state/evidence/LF-030-ep027-m5.json");
    if let Some(parent) = evidence_path.parent() {
        std::fs::create_dir_all(parent).expect("create evidence dir");
    }
    std::fs::write(
        &evidence_path,
        serde_json::to_string_pretty(&evidence).expect("json"),
    )
    .expect("write evidence");
    eprintln!(
        "EP-027 M5 LF-030: evidence written to {} (run {})",
        evidence_path.display(),
        run_id()
    );
}
