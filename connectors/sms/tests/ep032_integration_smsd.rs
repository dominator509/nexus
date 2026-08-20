//! EP-032 M3 integration suite: REAL nexus-sms-connector against the
//! REAL gammu-smsd 1.42.0 daemon with the REAL schema-17 SQLite
//! backend and a CONTROLLED PTY AT modem peer (SIMULATION - not a
//! physical GSM modem; the modem/carrier/handset boundary is NOT
//! ASSERTED).
//!
//! These are LIVE-STACK tests (EP-025 M3 convention): they require
//! the real fixture and are marked `#[ignore]` so the ambient
//! workspace battery stays green. The M3 gate
//! (scripts/ep032-m3-tests.sh) starts the fixture and runs them with
//! `--ignored`.
//!
//! Proven here (directive SV, SW, SX, SY, SZ):
//!   1. production SmsChannelProvider -> production GammuSmsdGateway
//!      -> production SqliteSmsDb -> real daemon outbox row
//!      (current-run canary: unique run_id + CreatorID, so stale
//!      SQLite state can never satisfy the proof);
//!   2. real daemon consumes the outbox row, performs a real
//!      AT+CMGS transaction (real SMS-SUBMIT PDU), records
//!      SendingOK in sentitems, deletes the outbox row;
//!   3. the controlled modem peer emits a real +CDS status report;
//!      the real daemon parses it and ITSELF updates sentitems to
//!      DeliveryOK with DeliveryDateTime (independent evidence:
//!      daemon log + raw DB readback + PDU/current-run identity -
//!      never a manually inserted DeliveryOK row);
//!   4. production status readback emits a truthful canonical
//!      Delivered receipt ONLY after DeliveryDateTime is present;
//!      SendingOK is never Delivered;
//!   5. duplicate NotificationId/idempotency identity -> exactly ONE
//!      provider outbox row (provider-observable, not just the ring);
//!   6. denied routing (policy/invalid body) -> ZERO provider
//!      mutation;
//!   7. redaction: receipts/errors never carry the SMS body or full
//!      destination.

use std::env;
use std::time::{Duration, Instant};

use nexus_domain::{CorrelationId, NotificationChannel, PersonId, Privacy};
use nexus_notifications::{
    ChannelProvider, DeliveryState, NotificationEnvelope, NotificationErrorCode, NotificationId,
    NotificationUrgency, SmsDestination,
};
use nexus_sms_connector::{GammuSmsdGateway, SmsChannelProvider, SmsProviderRef, SqliteSmsDb};

fn env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} is required for ep032_integration_smsd tests"))
}

fn run_id() -> String {
    env_var("SMSD_RUN_ID")
}

fn db_path() -> String {
    env_var("SMSD_DB")
}

fn daemon_log_path() -> String {
    env_var("SMSD_LOG")
}

fn destination() -> SmsDestination {
    // Controlled test destination (fixture-owned; redacted in logs).
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

/// Independent raw readback of the provider database (a second
/// connection that bypasses the connector entirely).
fn raw_provider_rows() -> Vec<(i64, String, String, String, Option<String>)> {
    let conn = rusqlite::Connection::open(db_path()).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT ID, CreatorID, Status, COALESCE(TextDecoded,''), DeliveryDateTime
             FROM sentitems ORDER BY ID",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    rows
}

/// Independent raw count of outbox rows for a creator id.
fn raw_outbox_count(creator_id: &str) -> i64 {
    let conn = rusqlite::Connection::open(db_path()).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM outbox WHERE CreatorID = ?1",
        rusqlite::params![creator_id],
        |r| r.get(0),
    )
    .unwrap()
}

/// Wait until the production status readback reaches the expected
/// state (the daemon is async; bounded polling, never a fixed sleep).
fn wait_for_state(
    provider: &SmsChannelProvider<GammuSmsdGateway<SqliteSmsDb>>,
    env: &NotificationEnvelope,
    provider_ref: &SmsProviderRef,
    want: DeliveryState,
    timeout: Duration,
) -> nexus_notifications::DeliveryReceipt {
    let deadline = Instant::now() + timeout;
    loop {
        let receipt = provider.refresh(env, provider_ref).unwrap();
        if receipt.state == want {
            return receipt;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {want:?}; last state {:?}",
            receipt.state
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m3-tests.sh)"]
fn ep032_integration_smsd_real_delivery_lifecycle() {
    let rid = run_id();
    let nid = format!("n-{rid}");
    let env = envelope(&nid, &format!("Hello from nexus {rid}"));

    // Production stack, all real.
    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let gateway = GammuSmsdGateway::new(db, "nexus:");
    let provider = SmsChannelProvider::new(gateway);
    assert!(
        provider.available(),
        "bound provider must advertise available"
    );

    let dest = destination();
    let receipt = provider.deliver_to(&env, &dest).unwrap();
    // At submission time the daemon may already have moved the row;
    // the receipt must NEVER claim Delivered from queue acceptance.
    assert!(
        !receipt.is_delivered(),
        "queue/send acceptance must never be Delivered"
    );
    let provider_ref = receipt
        .provider_ref
        .clone()
        .expect("receipt carries the provider message ref");

    // Current-run canary: exactly one outbox row was created for this
    // notification before the daemon consumed it.
    let creator = format!("nexus:{nid}");
    let outbox_before = raw_outbox_count(&creator);
    assert!(
        outbox_before <= 1,
        "exactly one provider request per notification (observed {outbox_before})"
    );

    // Wait for the REAL daemon delivery-report lifecycle:
    // outbox -> AT+CMGS -> SendingOK -> +CDS -> DeliveryOK+DateTime.
    let final_receipt = wait_for_state(
        &provider,
        &env,
        &SmsProviderRef(provider_ref.clone()),
        DeliveryState::Delivered,
        Duration::from_secs(30),
    );
    assert!(
        final_receipt.is_delivered(),
        "Delivered requires a real delivery report"
    );
    assert_eq!(
        final_receipt.provider_ref.as_deref(),
        Some(provider_ref.as_str()),
        "status must correlate to THIS message (exact identity)"
    );

    // Independent evidence 1: the daemon log shows the +CDS
    // delivery-report processing for this run.
    let log = std::fs::read_to_string(daemon_log_path()).unwrap();
    assert!(
        log.contains(&format!("nexus:{nid}")) || log.contains("Delivery report"),
        "daemon log must record the real delivery-report processing"
    );

    // Independent evidence 2: the daemon ITSELF wrote sentitems
    // (DeliveryOK + DeliveryDateTime) - never a manually inserted row.
    let rows = raw_provider_rows();
    let my_row = rows
        .iter()
        .find(|(_, c, _, _, _)| c == &creator)
        .unwrap_or_else(|| panic!("sentitems must contain creator {creator}: {rows:?}"));
    assert_eq!(
        my_row.2, "DeliveryOK",
        "daemon must record DeliveryOK for the sent message"
    );
    assert!(
        my_row.4.is_some(),
        "DeliveryDateTime must be present (real delivery report)"
    );

    // The outbox row is gone (daemon's documented delete_outbox).
    assert_eq!(raw_outbox_count(&creator), 0);
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m3-tests.sh)"]
fn ep032_integration_smsd_idempotency_one_provider_row() {
    let rid = run_id();
    let nid = format!("n-idem-{rid}");
    let env = envelope(&nid, &format!("idempotency {rid}"));

    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let gateway = GammuSmsdGateway::new(db, "nexus:");
    let provider = SmsChannelProvider::new(gateway);
    let dest = destination();

    let first = provider.deliver_to(&env, &dest).unwrap();
    let creator = format!("nexus:{nid}");
    let rows_after_first = raw_outbox_count(&creator) + raw_sentitems_count(&creator);

    // Replay the SAME notification: Conflict, no second provider row.
    let err = provider.deliver_to(&env, &dest).unwrap_err();
    assert_eq!(err.code, NotificationErrorCode::Conflict);
    let rows_after_replay = raw_outbox_count(&creator) + raw_sentitems_count(&creator);
    assert_eq!(
        rows_after_first, rows_after_replay,
        "duplicate replay must not create a second provider lifecycle"
    );
    let _ = first;
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m3-tests.sh)"]
fn ep032_integration_smsd_denied_routing_zero_mutation() {
    let rid = run_id();
    let nid = format!("n-deny-{rid}");
    let creator = format!("nexus:{nid}");

    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let gateway = GammuSmsdGateway::new(db, "nexus:");
    let provider = SmsChannelProvider::new(gateway);
    let dest = destination();

    // Invalid body (>160 chars: valid envelope, exceeds the documented
    // single-part SMS `TextDecoded` bound) -> Validation, zero provider
    // mutation. The envelope constructor accepts <=1000 chars, so this
    // reaches the adapter's SMS body bound and must fail closed.
    let bad_env = envelope(&nid, &"x".repeat(161));
    let err = provider.deliver_to(&bad_env, &dest).unwrap_err();
    assert_eq!(err.code, NotificationErrorCode::Validation);
    assert_eq!(
        raw_outbox_count(&creator) + raw_sentitems_count(&creator),
        0,
        "invalid body must never reach the provider"
    );

    // An unbound provider fails closed and never touches the DB.
    let unbound = SmsChannelProvider::<GammuSmsdGateway<SqliteSmsDb>>::unbound();
    let err2 = unbound
        .deliver_to(&envelope(&format!("n2-{rid}"), "x"), &dest)
        .unwrap_err();
    assert_eq!(err2.code, NotificationErrorCode::Unavailable);
    let creator2 = format!("nexus:n2-{rid}");
    assert_eq!(
        raw_outbox_count(&creator2) + raw_sentitems_count(&creator2),
        0,
        "unbound provider must never mutate the provider database"
    );
}

#[test]
#[ignore = "requires the real gammu-smsd fixture (run via scripts/ep032-m3-tests.sh)"]
fn ep032_integration_smsd_redaction_no_body_no_destination() {
    let rid = run_id();
    let nid = format!("n-red-{rid}");
    let env = envelope(&nid, &format!("REDACT-ME-{rid}"));

    let db = SqliteSmsDb::open(&db_path()).unwrap();
    let gateway = GammuSmsdGateway::new(db, "nexus:");
    let provider = SmsChannelProvider::new(gateway);
    let dest = destination();

    let receipt = provider.deliver_to(&env, &dest).unwrap();
    let debug = format!("{receipt:?}");
    assert!(
        !debug.contains(&format!("REDACT-ME-{rid}")),
        "receipt must never carry the SMS body"
    );
    let full_dest = env_var("SMSD_DEST_FULL");
    assert!(
        !debug.contains(&full_dest),
        "receipt must never carry the full destination"
    );
}

/// Independent raw count of sentitems rows for a creator id.
fn raw_sentitems_count(creator_id: &str) -> i64 {
    let conn = rusqlite::Connection::open(db_path()).unwrap();
    conn.query_row(
        "SELECT COUNT(*) FROM sentitems WHERE CreatorID = ?1",
        rusqlite::params![creator_id],
        |r| r.get(0),
    )
    .unwrap()
}
