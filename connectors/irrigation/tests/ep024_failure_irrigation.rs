//! EP-024 M4 irrigation forced-failure suite: the REAL nexus-irrigation
//! adapter against the REAL pinned Home Assistant container through the
//! EP-020-certified provider boundary (composition, not duplication).
//!
//! Env:
//!   NEXUS_HA_BASE       e.g. http://127.0.0.1:8125 (REQUIRED)
//!   NEXUS_HA_TOKEN      fresh OAuth token minted by the fixture
//!                       bootstrap (REQUIRED; never persisted)
//!   NEXUS_HA_CONTAINER  container name (default nexus-ep024-irr)
//!
//! Two classes:
//!   - `ep024_failure_probe_*`: read-only proofs safe to run
//!     concurrently (provider reachable + real auth, bad credential
//!     fails closed, silent HTTP peer -> TIMEOUT, refused endpoint ->
//!     UNAVAILABLE distinct from TIMEOUT, malformed provider response
//!     fails closed, unknown zone NotFound, unknown provider state
//!     never safe/closed, observability redaction, capability-gated
//!     denial before provider mutation).
//!   - `ep024_failure_journey_live`: ONE sequential journey owning all
//!     stateful phases (exact-target zone commands + fresh readback,
//!     wrong-target never verifies, retry after completion is not a
//!     Conflict, correlation continuity, provider offline -> UNAVAILABLE,
//!     bounded recovery, provider restored -> fresh readback only,
//!     zero secret leakage).
//!
//! All tests are LIVE-STACK (`#[ignore]` convention): the workspace
//! battery stays green without the container; the M4 gate
//! (scripts/ep024-m4-tests.sh) runs them with `--ignored` against the
//! real container, so the proofs remain mandatory.
//!
//! Classification:
//!   - nexus-irrigation adapter: REAL_PRODUCTION_IMPLEMENTATION
//!   - Home Assistant provider dependency: PROVIDER_CERTIFIED (EP-020)
//!   - zone fixtures (input_boolean / template sensor):
//!     CONTROLLED_TEST_FIXTURE
//!   - physical irrigation hardware / water flow: NOT ASSERTED / DEFERRED

use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use nexus_devices::verifier::{DeviceCommandVerifier, DeviceStateObservation, VerificationOutcome};
use nexus_devices::vocabulary::{DeviceAvailability, IrrigationCapability};
use nexus_devices::{DevicesErrorCode, IrrigationProvider};
use nexus_irrigation::{
    irrigation_zone_id, zone_state_value, HaIrrigationTransport, IrrigationAdapter,
    IrrigationCommand, IrrigationCommandState, IrrigationErrorCode, IrrigationTransport,
    IrrigationZoneSelector,
};

const ZONE_A: &str = "input_boolean.nexus_zone_a";
const ZONE_B: &str = "input_boolean.nexus_zone_b";
const ZONE_UNKNOWN: &str = "sensor.nexus_zone_unknown";

fn base_url() -> String {
    env::var("NEXUS_HA_BASE")
        .unwrap_or_else(|_| panic!("NEXUS_HA_BASE required (fixture bootstrap sets it)"))
}

fn token() -> String {
    env::var("NEXUS_HA_TOKEN")
        .unwrap_or_else(|_| panic!("NEXUS_HA_TOKEN required (fixture bootstrap sets it)"))
}

fn container_name() -> String {
    env::var("NEXUS_HA_CONTAINER").unwrap_or_else(|_| "nexus-ep024-irr".to_string())
}

fn transport() -> HaIrrigationTransport {
    HaIrrigationTransport::new(base_url(), token())
}

fn adapter() -> IrrigationAdapter<HaIrrigationTransport> {
    IrrigationAdapter::new(
        transport(),
        IrrigationZoneSelector::entities([ZONE_A.to_string(), ZONE_B.to_string()]),
    )
    .with_observability_secrets(vec![token()])
}

fn docker(args: &[&str]) -> Result<String, String> {
    let out = Command::new("/usr/bin/docker")
        .args(args)
        .output()
        .map_err(|e| format!("docker {args:?} failed: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "docker {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn wait_http_up(timeout: u64) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();
    let deadline = Instant::now() + Duration::from_secs(timeout);
    loop {
        if Instant::now() > deadline {
            return Err("HA did not become ready".to_string());
        }
        // BARE probe (no token): 401 (auth required) or 200 means the
        // server is up. Never poll with the token before auth has
        // fully loaded (http.ban).
        let status = client
            .get(format!("{}/api/", base_url()))
            .send()
            .map(|r| r.status().as_u16())
            .unwrap_or(0);
        if status == 200 || status == 401 {
            return Ok(());
        }
        thread::sleep(Duration::from_secs(2));
    }
}

fn wait_zones(adapter: &IrrigationAdapter<HaIrrigationTransport>, timeout: u64) {
    let zone_a = irrigation_zone_id(ZONE_A).expect("zone A id");
    let zone_b = irrigation_zone_id(ZONE_B).expect("zone B id");
    let deadline = Instant::now() + Duration::from_secs(timeout);
    while Instant::now() < deadline {
        if let Ok(zones) = adapter.list_zones() {
            if zones.contains(&zone_a) && zones.contains(&zone_b) {
                return;
            }
        }
        thread::sleep(Duration::from_secs(2));
    }
    panic!("fixture zones did not become active after restart");
}

fn zone_state(t: &HaIrrigationTransport, entity_id: &str) -> String {
    t.read_zone(entity_id).expect("real read").state
}

/// Bind a local TCP listener that ACCEPTS connections and never
/// responds (a silent HTTP peer). Returns the base URL.
fn silent_peer() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind silent peer");
    let port = listener.local_addr().expect("addr").port();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            // Accept and hold the connection without writing.
            let mut stream = stream;
            let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
            let mut buf = [0u8; 4096];
            loop {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        }
    });
    format!("http://127.0.0.1:{port}")
}

/// Serve ONE malformed JSON response (200 with garbage body), then
/// keep accepting/holding (fail closed path never retries).
fn malformed_json_peer() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind malformed peer");
    let port = listener.local_addr().expect("addr").port();
    thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let body = b"{{{{ not json";
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(body);
            let _ = stream.flush();
            // Hold the connection so the client can read the complete
            // malformed body (an immediate drop can surface as a
            // truncated connection error instead of a parse failure).
            thread::sleep(Duration::from_millis(500));
        }
    });
    format!("http://127.0.0.1:{port}")
}

/// A port with no listener (connection refused).
fn refused_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind probe");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}

// ---------------------------------------------------------------------------
// Read-only probe tests (safe to run concurrently)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_provider_reachable_and_authenticated() {
    let t = transport();
    assert!(
        t.auth_check().expect("real auth check"),
        "auth_check must succeed with the freshly minted EP-020-certified token"
    );
    let zones = t.list_zones().expect("real /api/states");
    assert!(!zones.is_empty(), "real provider returned zero entities");
    let adapter = adapter();
    let discovered = adapter.list_zones().expect("real discovery");
    let zone_a = irrigation_zone_id(ZONE_A).expect("zone A id");
    let zone_b = irrigation_zone_id(ZONE_B).expect("zone B id");
    assert!(discovered.contains(&zone_a), "zone A must be discovered");
    assert!(discovered.contains(&zone_b), "zone B must be discovered");
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_bad_credential_real_401() {
    // Real bad credential: a bogus token must NOT authenticate. The
    // EP-020 boundary reports the real HTTP 401 as External (its
    // documented contract); the authorization gate is auth_check,
    // which must return Ok(false) for the bogus credential.
    let t = HaIrrigationTransport::new(base_url(), "ep024-bogus-token");
    assert!(
        !t.auth_check().expect("auth check with bad credential"),
        "bogus token must fail auth_check"
    );
    let err = t.list_zones().expect_err("must fail");
    assert_ne!(err.code, IrrigationErrorCode::NotFound, "never benign");
    // The failure is honest: the transport surfaces the provider's
    // classification (External for HTTP 401), never a fabricated zone
    // or state.
    assert_eq!(err.code, IrrigationErrorCode::External);
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_silent_peer_times_out() {
    // Accepted silent HTTP peer: the transport's bounded request
    // timeout must fire -> TIMEOUT, never an infinite hang and never
    // a fabricated result. The bounded timeout is part of the
    // irrigation transport composition (consequential domain).
    let silent = silent_peer();
    let t = HaIrrigationTransport::new(silent, "ep024-token");
    let started = Instant::now();
    let err = t.list_zones().expect_err("silent peer must time out");
    assert_eq!(
        err.code,
        IrrigationErrorCode::Timeout,
        "silent peer must map to TIMEOUT, got {:?}",
        err.code
    );
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(30),
        "silent peer must not hang; took {elapsed:?}"
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_refused_endpoint_unavailable_not_timeout() {
    // Closed/refused endpoint: immediate connection refusal ->
    // UNAVAILABLE, DISTINCT from TIMEOUT.
    let port = refused_port();
    let t = HaIrrigationTransport::new(format!("http://127.0.0.1:{port}"), "ep024-token");
    let err = t.list_zones().expect_err("refused endpoint must fail");
    assert_eq!(
        err.code,
        IrrigationErrorCode::Unavailable,
        "refused endpoint must map to UNAVAILABLE, got {:?}",
        err.code
    );
    assert_ne!(
        err.code,
        IrrigationErrorCode::Timeout,
        "refused endpoint must be distinct from TIMEOUT"
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_malformed_provider_response_fails_closed() {
    // Malformed provider response: the adapter must fail closed (an
    // error), never fabricate zones or states from garbage.
    let malformed = malformed_json_peer();
    let t = HaIrrigationTransport::new(malformed, "ep024-token");
    let err = t.list_zones().expect_err("malformed response must fail");
    assert_ne!(err.code, IrrigationErrorCode::NotFound, "never benign");
    assert_ne!(
        err.code,
        IrrigationErrorCode::Unavailable,
        "not a connection failure"
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_unknown_zone_not_found() {
    // Unknown irrigation zone -> NotFound at the adapter boundary,
    // never Verified and never a benign zone state.
    let adapter = adapter();
    let ghost = irrigation_zone_id("input_boolean.nexus_ghost").expect("ghost id");
    let err = adapter
        .capabilities(&ghost)
        .expect_err("unknown zone must fail");
    assert_eq!(err.code, DevicesErrorCode::NotFound);

    // Registry-level unknown (configured target absent from the real
    // provider registry) -> NotFound from the transport.
    let t = transport();
    let err = t
        .read_zone("input_boolean.nexus_ghost")
        .expect_err("absent registry entity must fail");
    assert_eq!(err.code, IrrigationErrorCode::NotFound);
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_unknown_state_never_safe_closed() {
    // The real template sensor reports state "unknown" (its template
    // references a non-existent entity; a template SENSOR renders the
    // literal string, unlike a template binary_sensor which the pinned
    // build normalizes to off). An unknown provider state must never
    // be mapped to a safe/closed state (OFF) and must never satisfy
    // verification.
    let t = transport();
    let zone = t.read_zone(ZONE_UNKNOWN).expect("read unknown fixture");
    assert_eq!(
        zone.state, "unknown",
        "fixture must report the real unknown state"
    );
    assert_eq!(zone_state_value(&zone), None, "unknown is never ON/OFF");

    let adapter = IrrigationAdapter::new(
        transport(),
        IrrigationZoneSelector::entities([ZONE_UNKNOWN.to_string()]),
    )
    .with_observability_secrets(vec![token()]);
    let unknown_id = irrigation_zone_id(ZONE_UNKNOWN).expect("unknown id");
    let avail = adapter.availability(&unknown_id).expect("availability");
    assert_eq!(
        avail,
        DeviceAvailability::Available,
        "present + usable (unknown) is AVAILABLE, never claimed OFF"
    );
    let caps = adapter.capabilities(&unknown_id).expect("caps");
    assert!(
        !caps.contains(&IrrigationCapability::ZoneControl),
        "a binary_sensor fixture has no controllable zone surface"
    );
    let err = adapter
        .verify(&unknown_id, IrrigationCommand::ZoneOff, "OFF")
        .expect_err("unknown state must never verify OFF");
    assert_eq!(err.code, IrrigationErrorCode::Verification);
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_malformed_command_parameters_rejected() {
    // Malformed command parameters -> canonical validation failure
    // BEFORE any provider call (the vocabulary is the parameter
    // surface for this binary-zone contract).
    let err = IrrigationCommand::parse("FLOOD").expect_err("must reject");
    assert_eq!(err.code, IrrigationErrorCode::Vocabulary);
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_unsupported_command_denied_before_provider() {
    let adapter = adapter();
    let zone_a = irrigation_zone_id(ZONE_A).expect("zone A id");
    let caps = adapter.capabilities(&zone_a).expect("caps");
    assert!(
        !caps.contains(&IrrigationCapability::ScheduleControl),
        "fixture zones must never advertise schedule control"
    );
    let t = transport();
    let before = zone_state(&t, ZONE_A);
    let err = adapter
        .execute(&zone_a, IrrigationCommand::SetSchedule, &caps)
        .expect_err("SET_SCHEDULE must be denied");
    assert_eq!(err.code, IrrigationErrorCode::Policy);
    assert_eq!(
        zone_state(&t, ZONE_A),
        before,
        "denied command must not mutate the provider"
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_probe_observability_redaction_canary_absent() {
    // The adapter's audit ring is configured with the REAL provider
    // token as the redaction secret. After real failures, no stored
    // audit detail or counter may contain the credential, and every
    // entry carries a canonical irrigation correlation id.
    let adapter = adapter();
    let zone_a = irrigation_zone_id(ZONE_A).expect("zone A id");
    let caps = adapter.capabilities(&zone_a).expect("caps");

    // Real failures: unknown zone NotFound + capability denial Policy.
    let ghost = irrigation_zone_id("input_boolean.nexus_ghost").expect("ghost id");
    let _ = adapter.capabilities(&ghost);
    let _ = adapter.execute(&zone_a, IrrigationCommand::SetSchedule, &caps);

    let token = token();
    let audit = adapter.audit();
    assert!(!audit.is_empty(), "failures must be audited");
    for entry in &audit {
        assert!(
            !entry.detail.contains(&token),
            "audit leaked the provider token: {}",
            entry.detail
        );
        assert!(
            entry.correlation.starts_with("irrigation-"),
            "correlation id must be canonical: {}",
            entry.correlation
        );
    }
    for (key, _) in adapter.counters() {
        assert!(!key.contains(&token), "counter key leaked the token");
    }
}

// ---------------------------------------------------------------------------
// Sequential live journey (stateful; the ONLY test that mutates the
// container's fixture state)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m4-tests.sh"]
fn ep024_failure_journey_live() {
    let adapter = adapter();
    let zone_a = irrigation_zone_id(ZONE_A).expect("zone A id");
    let zone_b = irrigation_zone_id(ZONE_B).expect("zone B id");
    let caps_a = adapter.capabilities(&zone_a).expect("A caps");
    let caps_b = adapter.capabilities(&zone_b).expect("B caps");

    // --- baseline -----------------------------------------------------------
    let t = transport();
    assert_eq!(zone_state(&t, ZONE_A), "off", "A baseline off");
    assert_eq!(zone_state(&t, ZONE_B), "off", "B baseline off");

    // --- exact-target zone command + fresh readback (E/F) --------------------
    let receipt = adapter
        .execute(&zone_a, IrrigationCommand::ZoneOn, &caps_a)
        .expect("A on");
    assert_eq!(
        receipt.state,
        IrrigationCommandState::Submitted,
        "receipt is SUBMITTED at most, never VERIFIED"
    );
    assert_eq!(
        zone_state(&t, ZONE_A),
        "on",
        "independent fresh readback must observe A on"
    );
    adapter
        .verify(&zone_a, IrrigationCommand::ZoneOn, "ON")
        .expect("A verified on");
    assert_eq!(
        zone_state(&t, ZONE_B),
        "off",
        "commanding A must never touch B"
    );

    // --- zone B independent change never satisfies A's plan -----------------
    adapter
        .execute(&zone_b, IrrigationCommand::ZoneOn, &caps_b)
        .expect("B on");
    adapter
        .verify(&zone_b, IrrigationCommand::ZoneOn, "ON")
        .expect("B verified on");
    let err = adapter
        .verify(&zone_a, IrrigationCommand::ZoneOff, "OFF")
        .expect_err("B's change must not satisfy A's plan");
    assert_eq!(err.code, IrrigationErrorCode::Verification);
    // Verifier-level invariant: an observation recorded from zone B
    // (even with the desired state) can never verify zone A.
    let verifier = DeviceCommandVerifier;
    let observation = DeviceStateObservation {
        device: zone_b.as_str().to_string(),
        state: Some("ON".to_string()),
    };
    assert_eq!(
        verifier.verify(zone_a.as_str(), "ON", &observation),
        VerificationOutcome::UnrelatedChange
    );

    // --- retry after completion is NOT a Conflict (H.8/I) --------------------
    let receipt = adapter
        .execute(&zone_a, IrrigationCommand::ZoneOn, &caps_a)
        .expect("re-issuing a completed command must not conflict");
    assert_eq!(
        receipt.state,
        IrrigationCommandState::Submitted,
        "a completed command released its in-flight entry"
    );

    // --- correlation continuity on failure (L) ------------------------------
    let err = adapter
        .verify(&zone_a, IrrigationCommand::ZoneOff, "OFF")
        .expect_err("A is on; OFF verification fails");
    assert_eq!(err.code, IrrigationErrorCode::Verification);
    let correlation = err.correlation.expect("error must carry correlation");
    assert!(
        correlation.starts_with("irrigation-"),
        "canonical correlation: {correlation}"
    );

    // --- provider unavailable -> UNAVAILABLE, never benign (H.1) -------------
    docker(&["stop", &container_name()]).expect("docker stop");
    thread::sleep(Duration::from_secs(3));
    let err = adapter
        .execute(&zone_a, IrrigationCommand::ZoneOn, &caps_a)
        .expect_err("provider offline");
    assert_eq!(err.code, IrrigationErrorCode::Unavailable);
    let avail = adapter.availability(&zone_a).expect("availability");
    assert_eq!(
        avail,
        DeviceAvailability::Unavailable,
        "provider offline must map to UNAVAILABLE, never OFF"
    );

    // --- bounded recovery: no stuck entries, no fabricated state (H.13) ------
    // The failed command released its in-flight entry; recover() has
    // nothing to clear and never fabricates recovery success.
    let cleared = adapter.recover();
    assert_eq!(cleared, 0, "failed commands must already release entries");

    // --- provider restored -> fresh readback only (N) ------------------------
    docker(&["start", &container_name()]).expect("docker start");
    wait_http_up(300).expect("HA ready after offline phase");
    wait_zones(&adapter, 180);
    // Available ONLY because the provider actually reports it (fresh
    // readback; no stale cache can produce recovery success).
    let avail = adapter
        .availability(&zone_a)
        .expect("availability after restore");
    assert_eq!(
        avail,
        DeviceAvailability::Available,
        "availability returns only when the provider reports it"
    );
    adapter
        .execute(&zone_a, IrrigationCommand::ZoneOff, &caps_a)
        .expect("A off after restore");
    assert_eq!(
        zone_state(&t, ZONE_A),
        "off",
        "command/readback works after provider restore"
    );
    adapter
        .verify(&zone_a, IrrigationCommand::ZoneOff, "OFF")
        .expect("A verified off after restore");

    // --- zero secret leakage across the whole journey ------------------------
    let token = token();
    for entry in adapter.audit() {
        assert!(
            !entry.detail.contains(&token),
            "journey audit leaked the provider token: {}",
            entry.detail
        );
    }
}
