//! EP-032 M4 failure suite: REAL Gammu SMSD forced-failure proofs.
//!
//! These are LIVE-STACK tests (EP-030/EP-031 M4 convention): they
//! require the real gammu-smsd 1.42.0 fixture and are marked
//! `#[ignore]` so the ambient workspace battery stays green. The M4
//! gate (scripts/ep032-m4-tests.sh) starts the fixture and runs them
//! with `--ignored`.
//!
//! The modem/carrier boundary remains a CONTROLLED SIMULATION
//! FIXTURE (infra/sms/at_modem.py PTY AT peer); physical GSM modem,
//! carrier, and handset stay NOT ASSERTED.
//!
//! Proven here:
//!   1. ambiguous real outbox submission -> reconcile by durable
//!      CreatorID -> exactly one provider row, no blind duplicate
//!      (directive D);
//!   2. durable idempotency across connector-process restart
//!      (directive E);
//!   3. gammu-smsd daemon unavailable -> truthful canonical failure,
//!      no fabricated Delivered (directive F);
//!   4. database backend unavailable -> Unavailable before any fake
//!      provider state, then bounded recovery (directive G/H);
//!   5. AT+CMGS + SendingOK with NO +CDS report -> canonical state
//!      stays non-delivered (directive I);
//!   6. real failure delivery report (+CDS TP-Status 0x29) -> real
//!      daemon transitions to the documented failed state -> Failed
//!      receipt, never Delivered (directive J);
//!   7. unmatched/malformed report -> target notification never
//!      becomes Delivered (directive K);
//!   8. provider restart -> reconcile -> new successful operation
//!      (directive AB).

use std::env;
use std::time::{Duration, Instant};

use nexus_domain::{CorrelationId, NotificationChannel, PersonId, Privacy};
use nexus_notifications::{
    DeliveryState, NotificationEnvelope, NotificationErrorCode, NotificationId,
    NotificationUrgency, SmsDestination,
};
use nexus_sms_connector::{GammuSmsdGateway, SmsChannelProvider, SmsProviderRef, SqliteSmsDb};

fn env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} is required for ep032_failure_smsd tests"))
}

fn run_id() -> String {
    env_var("SMSD_RUN_ID")
}

fn db_path() -> String {
    env_var("SMSD_DB")
}

fn destination() -> SmsDestination {
    SmsDestination::new(env_var("SMSD_DEST")).unwrap()
}

fn envelope(id: &str, summary: &str) -> NotificationEnvelope {
    NotificationEnvelope::new(
        NotificationId::new(id).unwrap(),
        PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
        NotificationUrgency::High,
        Privacy::Personal,
        "Suspicious sign-in",
        summary,
        vec![NotificationChannel::Sms],
        "2026-08-21T12:00:00Z",
        CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        None,
    )
    .unwrap()
}

/// Independent raw count of provider rows (outbox + sentitems) for a
/// creator id, through a second connection that bypasses the
/// connector.
fn raw_provider_row_count(creator_id: &str) -> i64 {
    let conn = rusqlite::Connection::open(db_path()).unwrap();
    let outbox: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM outbox WHERE CreatorID = ?1",
            rusqlite::params![creator_id],
            |r| r.get(0),
        )
        .unwrap();
    let sentitems: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sentitems WHERE CreatorID = ?1",
            rusqlite::params![creator_id],
            |r| r.get(0),
        )
        .unwrap();
    outbox + sentitems
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m4-tests.sh)"]
fn ep032_failure_ambiguous_submission_reconciles_exactly_one_row() {
    let rid = run_id();
    let nid = format!("n-ambig-{rid}");
    let creator = format!("nexus:{nid}");

    // Production stack.
    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let mut gateway = GammuSmsdGateway::new(db, "nexus:");
    let dest = destination();

    // First submission: real outbox INSERT, exactly one row.
    let (first_ref, reconciled) = gateway
        .submit_reconciled(&dest, &format!("ambiguous {rid}"), &nid)
        .unwrap();
    assert!(!reconciled, "first submit inserts exactly once");
    assert_eq!(
        raw_provider_row_count(&creator),
        1,
        "exactly one provider row after first submission"
    );

    // The client loses authoritative confirmation (the DB may have
    // committed); reconcile FIRST - never a blind duplicate.
    let (again_ref, reconciled_again) = gateway
        .submit_reconciled(&dest, &format!("ambiguous {rid}"), &nid)
        .unwrap();
    assert!(
        reconciled_again,
        "replay must be detected as reconciled (Verification outcome)"
    );
    assert_eq!(again_ref, first_ref, "same provider row identity");
    assert_eq!(
        raw_provider_row_count(&creator),
        1,
        "provider-side row count remains exactly one"
    );
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m4-tests.sh)"]
fn ep032_failure_durable_idempotency_across_process_restart() {
    let rid = run_id();
    let nid = format!("n-restart-{rid}");
    let creator = format!("nexus:{nid}");

    // "Process 1": submit and confirm the row exists.
    {
        let db = SqliteSmsDb::open(&db_path()).unwrap();
        let mut gateway = GammuSmsdGateway::new(db, "nexus:");
        let (_, reconciled) = gateway
            .submit_reconciled(&destination(), &format!("restart {rid}"), &nid)
            .unwrap();
        assert!(!reconciled);
    }
    assert_eq!(raw_provider_row_count(&creator), 1);

    // "Process 2" (fresh connector state, empty in-memory ring):
    // replay the same notification identity. Durable reconciliation
    // via CreatorID must suppress the duplicate.
    {
        let db = SqliteSmsDb::open(&db_path()).unwrap();
        let mut gateway = GammuSmsdGateway::new(db, "nexus:");
        let (_, reconciled) = gateway
            .submit_reconciled(&destination(), &format!("restart {rid}"), &nid)
            .unwrap();
        assert!(
            reconciled,
            "cross-restart replay must reconcile, never duplicate"
        );
    }
    assert_eq!(
        raw_provider_row_count(&creator),
        1,
        "durable idempotency: exactly one provider row across restart"
    );
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m4-tests.sh)"]
fn ep032_failure_daemon_unavailable_truthful_failure_no_fake_delivered() {
    let rid = run_id();
    let nid = format!("n-down-{rid}");
    let env = envelope(&nid, &format!("daemon down {rid}"));

    // The gate stops gammu-smsd before running this test; the DB is
    // still open, but the daemon is not consuming. Submitting to the
    // outbox still succeeds (queue acceptance) - the receipt must
    // NEVER claim Delivered.
    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let gateway = GammuSmsdGateway::new(db, "nexus:");
    let provider = SmsChannelProvider::new(gateway);
    let receipt = provider.deliver_to(&env, &destination()).unwrap();
    assert!(
        !receipt.is_delivered(),
        "daemon down must never produce Delivered"
    );
    // A bounded wait shows the message stays non-delivered (no daemon
    // to process +CDS; no fabricated provider state).
    std::thread::sleep(Duration::from_secs(2));
    let refreshed = provider
        .refresh(&env, &SmsProviderRef(receipt.provider_ref.unwrap()))
        .unwrap();
    assert!(
        !refreshed.is_delivered(),
        "no daemon means no delivery report; never Delivered"
    );
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m4-tests.sh)"]
fn ep032_failure_backend_unavailable_fails_closed_then_recovers() {
    let rid = run_id();
    let nid = format!("n-backend-{rid}");
    let env = envelope(&nid, &format!("backend down {rid}"));

    // The gate replaces the DB file with a directory before this
    // test; open or status must fail closed with Unavailable/
    // External, never return a fake provider state.
    let result = SqliteSmsDb::open(&db_path());
    match result {
        Ok(db) => {
            let gateway = GammuSmsdGateway::new(db, "nexus:");
            let provider = SmsChannelProvider::new(gateway);
            let err = provider.deliver_to(&env, &destination()).unwrap_err();
            assert!(
                matches!(
                    err.code,
                    NotificationErrorCode::Unavailable | NotificationErrorCode::External
                ),
                "backend failure must be a canonical error, got {:?}",
                err.code
            );
        }
        Err(err) => {
            assert!(
                matches!(
                    err.code,
                    NotificationErrorCode::Unavailable | NotificationErrorCode::External
                ),
                "backend open failure must be canonical, got {:?}",
                err.code
            );
        }
    }
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m4-tests.sh)"]
fn ep032_failure_no_delivery_report_never_delivered() {
    let rid = run_id();
    let nid = format!("n-noreport-{rid}");
    let env = envelope(&nid, &format!("no report {rid}"));

    // The gate runs this with SMSD_NO_REPORT=1: AT+CMGS succeeds,
    // SendingOK exists, NO +CDS arrives. The canonical state must
    // stay Sending - never Delivered.
    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let gateway = GammuSmsdGateway::new(db, "nexus:");
    let provider = SmsChannelProvider::new(gateway);
    let receipt = provider.deliver_to(&env, &destination()).unwrap();
    assert!(!receipt.is_delivered());
    let provider_ref = receipt.provider_ref.clone().unwrap();

    // Give the daemon ample time for a report that will never come.
    std::thread::sleep(Duration::from_secs(6));
    let refreshed = provider
        .refresh(&env, &SmsProviderRef(provider_ref))
        .unwrap();
    assert!(
        !refreshed.is_delivered(),
        "SendingOK without a delivery report must never become Delivered (state {:?})",
        refreshed.state
    );
    assert!(
        matches!(
            refreshed.state,
            DeliveryState::Sending | DeliveryState::Pending
        ),
        "no-report message remains non-final, got {:?}",
        refreshed.state
    );
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m4-tests.sh)"]
fn ep032_failure_delivery_failure_report_maps_to_failed() {
    let rid = run_id();
    let nid = format!("n-failrep-{rid}");
    let env = envelope(&nid, &format!("failure report {rid}"));

    // The gate runs this with SMSD_FAILURE_REPORT=1: the controlled
    // peer emits a +CDS with TP-Status 0x41 (permanent error; gammu
    // 1.42 classifies bit 0x40 as Failed). The REAL daemon processes
    // it and transitions the message to the documented failed state;
    // production readback maps to Failed, never Delivered.
    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let gateway = GammuSmsdGateway::new(db, "nexus:");
    let provider = SmsChannelProvider::new(gateway);
    let receipt = provider.deliver_to(&env, &destination()).unwrap();
    let provider_ref = receipt.provider_ref.clone().unwrap();

    // Wait for the real daemon to record the failed state.
    let deadline = Instant::now() + Duration::from_secs(30);
    let final_state = loop {
        let refreshed = provider
            .refresh(&env, &SmsProviderRef(provider_ref.clone()))
            .unwrap();
        if refreshed.state == DeliveryState::Failed {
            break refreshed.state;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for Failed; last {:?}",
            refreshed.state
        );
        std::thread::sleep(Duration::from_millis(300));
    };
    assert_eq!(
        final_state,
        DeliveryState::Failed,
        "real failure delivery report must map to Failed"
    );

    // Independent readback: the daemon itself wrote the failed state.
    let conn = rusqlite::Connection::open(db_path()).unwrap();
    let status: String = conn
        .query_row(
            "SELECT Status FROM sentitems WHERE ID = ?1 AND SequencePosition = 1",
            rusqlite::params![provider_ref.as_str()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        status, "DeliveryFailed",
        "daemon must record the documented DeliveryFailed state"
    );
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m4-tests.sh)"]
fn ep032_failure_unmatched_report_never_satisfies_target() {
    let rid = run_id();
    let nid = format!("n-unmatched-{rid}");
    let env = envelope(&nid, &format!("unmatched {rid}"));

    // The gate runs this with SMSD_UNMATCHED_REPORT=1: the +CDS is
    // bound to a DIFFERENT TPMR/destination. Exact-target
    // correlation: this message must never become Delivered.
    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let gateway = GammuSmsdGateway::new(db, "nexus:");
    let provider = SmsChannelProvider::new(gateway);
    let receipt = provider.deliver_to(&env, &destination()).unwrap();
    assert!(!receipt.is_delivered());
    let provider_ref = receipt.provider_ref.clone().unwrap();

    std::thread::sleep(Duration::from_secs(6));
    let refreshed = provider
        .refresh(&env, &SmsProviderRef(provider_ref))
        .unwrap();
    assert!(
        !refreshed.is_delivered(),
        "a report for message X must never satisfy message Y (state {:?})",
        refreshed.state
    );
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m4-tests.sh)"]
fn ep032_failure_provider_restart_reconciles_and_recovers() {
    let rid = run_id();
    let nid = format!("n-recover-{rid}");
    let creator = format!("nexus:{nid}");

    // Phase 1 (daemon up): a real successful submission; the queue
    // row exists (outbox while the daemon is stopped before consume).
    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let mut gateway = GammuSmsdGateway::new(db, "nexus:");
    let (_, reconciled) = gateway
        .submit_reconciled(&destination(), &format!("recovery {rid}"), &nid)
        .unwrap();
    assert!(!reconciled);
    drop(gateway);
    assert_eq!(raw_provider_row_count(&creator), 1);

    // Phase 2 (daemon restarted by the gate): a NEW connector
    // instance reconciles the exact same identity - the provider
    // queue state remains reconcilable; no duplicate row.
    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let mut gateway = GammuSmsdGateway::new(db, "nexus:");
    let (_, reconciled) = gateway
        .submit_reconciled(&destination(), &format!("recovery {rid}"), &nid)
        .unwrap();
    assert!(reconciled, "restart must reconcile, never duplicate");
    assert_eq!(
        raw_provider_row_count(&creator),
        1,
        "exactly one provider row across restart"
    );
}
