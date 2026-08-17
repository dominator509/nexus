//! EP-024 M5 vacuum live-fire + forced-failure suite: the REAL
//! nexus-vacuum adapter against the REAL pinned Home Assistant
//! container through the EP-020-certified provider boundary
//! (composition, not duplication).
//!
//! Env:
//!   NEXUS_HA_BASE       e.g. http://127.0.0.1:8126 (REQUIRED)
//!   NEXUS_HA_TOKEN      fresh OAuth token minted by the fixture
//!                       bootstrap (REQUIRED; never persisted)
//!   NEXUS_HA_CONTAINER  container name (default nexus-ep024-vac)
//!
//! Two classes:
//!   - `ep024_failure_vacuum_probe_*`: read-only proofs safe to run
//!     concurrently (provider reachable + real auth, bad credential
//!     fails closed, silent HTTP peer -> TIMEOUT, refused endpoint ->
//!     UNAVAILABLE distinct, malformed provider response fails closed,
//!     unknown vacuum NotFound, capability mapping from the REAL
//!     provider feature bits, unsupported command (incl. MapReadback
//!     without a map surface) denied before provider mutation,
//!     observability redaction).
//!   - `ep024_failure_vacuum_journey_live`: ONE sequential journey
//!     owning all stateful phases (StartClean -> CLEANING, Pause ->
//!     PAUSED, ReturnHome -> RETURNING -> DOCKED through the real
//!     auto-dock automation, Dock -> same provider action, wrong-target
//!     never verifies, retry-after-completion is not a Conflict,
//!     correlation continuity, provider restart -> stable identity,
//!     provider offline -> UNAVAILABLE, bounded recovery, provider
//!     restored -> fresh readback only, zero secret leakage).
//!
//! All tests are LIVE-STACK (`#[ignore]` convention): the workspace
//! battery stays green without the container; the M5 gate
//! (scripts/ep024-m5-tests.sh) runs them with `--ignored` against the
//! real container, so the proofs remain mandatory.
//!
//! Classification:
//!   - nexus-vacuum adapter: REAL_PRODUCTION_IMPLEMENTATION
//!   - Home Assistant provider dependency: PROVIDER_CERTIFIED (EP-020)
//!   - template vacuum fixtures: CONTROLLED_TEST_FIXTURE
//!   - physical robot vacuum / SLAM map: NOT ASSERTED / DEFERRED
//!   - map provider path: NOT CERTIFIED (no real map surface on the
//!     controlled fixture; never fabricated)

use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use nexus_devices::verifier::{DeviceCommandVerifier, DeviceStateObservation, VerificationOutcome};
use nexus_devices::vocabulary::{DeviceAvailability, VacuumCapability};
use nexus_devices::{DevicesErrorCode, VacuumProvider};
use nexus_vacuum::{
    vacuum_device_id, vacuum_state_value, HaVacuumTransport, VacuumActivityState, VacuumAdapter,
    VacuumCommand, VacuumCommandState, VacuumDeviceSelector, VacuumErrorCode, VacuumTransport,
};

const VACUUM_A: &str = "vacuum.nexus_vacuum_a";
const VACUUM_B: &str = "vacuum.nexus_vacuum_b";

fn base_url() -> String {
    env::var("NEXUS_HA_BASE")
        .unwrap_or_else(|_| panic!("NEXUS_HA_BASE required (fixture bootstrap sets it)"))
}

fn token() -> String {
    env::var("NEXUS_HA_TOKEN")
        .unwrap_or_else(|_| panic!("NEXUS_HA_TOKEN required (fixture bootstrap sets it)"))
}

fn container_name() -> String {
    env::var("NEXUS_HA_CONTAINER").unwrap_or_else(|_| "nexus-ep024-vac".to_string())
}

fn transport() -> HaVacuumTransport {
    HaVacuumTransport::new(base_url(), token())
}

fn adapter() -> VacuumAdapter<HaVacuumTransport> {
    VacuumAdapter::new(
        transport(),
        VacuumDeviceSelector::entities([VACUUM_A.to_string(), VACUUM_B.to_string()]),
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

fn wait_vacuums(adapter: &VacuumAdapter<HaVacuumTransport>, timeout: u64) {
    let device_a = vacuum_device_id(VACUUM_A).expect("vacuum A id");
    let device_b = vacuum_device_id(VACUUM_B).expect("vacuum B id");
    let deadline = Instant::now() + Duration::from_secs(timeout);
    while Instant::now() < deadline {
        if let Ok(devices) = adapter.list_devices() {
            if devices.contains(&device_a) && devices.contains(&device_b) {
                return;
            }
        }
        thread::sleep(Duration::from_secs(2));
    }
    panic!("fixture vacuums did not become active after restart");
}

fn vacuum_state(t: &HaVacuumTransport, entity_id: &str) -> String {
    t.read_vacuum(entity_id).expect("real read").state
}

/// Bounded wait for a vacuum to reach an expected provider state.
fn wait_state(
    t: &HaVacuumTransport,
    entity_id: &str,
    expected: &str,
    timeout: u64,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(timeout);
    while Instant::now() < deadline {
        if let Ok(device) = t.read_vacuum(entity_id) {
            if device.state == expected {
                return Ok(());
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
    Err(format!("vacuum {entity_id} never reached {expected:?}"))
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
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_provider_reachable_and_authenticated() {
    let t = transport();
    assert!(
        t.auth_check().expect("real auth check"),
        "auth_check must succeed with the freshly minted EP-020-certified token"
    );
    let devices = t.list_vacuums().expect("real /api/states");
    assert!(!devices.is_empty(), "real provider returned zero entities");
    let adapter = adapter();
    let discovered = adapter.list_devices().expect("real discovery");
    let device_a = vacuum_device_id(VACUUM_A).expect("vacuum A id");
    let device_b = vacuum_device_id(VACUUM_B).expect("vacuum B id");
    assert!(
        discovered.contains(&device_a),
        "vacuum A must be discovered"
    );
    assert!(
        discovered.contains(&device_b),
        "vacuum B must be discovered"
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_bad_credential_real_401() {
    // Real bad credential: a bogus token must NOT authenticate. The
    // EP-020 boundary reports the real HTTP 401 as External (its
    // documented contract); the authorization gate is auth_check,
    // which must return Ok(false) for the bogus credential.
    let t = HaVacuumTransport::new(base_url(), "ep024-bogus-token");
    assert!(
        !t.auth_check().expect("auth check with bad credential"),
        "bogus token must fail auth_check"
    );
    let err = t.list_vacuums().expect_err("must fail");
    assert_ne!(err.code, VacuumErrorCode::NotFound, "never benign");
    assert_eq!(err.code, VacuumErrorCode::External);
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_silent_peer_times_out() {
    // Accepted silent HTTP peer: the transport's bounded request
    // timeout must fire -> TIMEOUT, never an infinite hang and never
    // a fabricated result.
    let silent = silent_peer();
    let t = HaVacuumTransport::new(silent, "ep024-token");
    let started = Instant::now();
    let err = t.list_vacuums().expect_err("silent peer must time out");
    assert_eq!(
        err.code,
        VacuumErrorCode::Timeout,
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
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_refused_endpoint_unavailable_not_timeout() {
    // Closed/refused endpoint: immediate connection refusal ->
    // UNAVAILABLE, DISTINCT from TIMEOUT.
    let port = refused_port();
    let t = HaVacuumTransport::new(format!("http://127.0.0.1:{port}"), "ep024-token");
    let err = t.list_vacuums().expect_err("refused endpoint must fail");
    assert_eq!(
        err.code,
        VacuumErrorCode::Unavailable,
        "refused endpoint must map to UNAVAILABLE, got {:?}",
        err.code
    );
    assert_ne!(
        err.code,
        VacuumErrorCode::Timeout,
        "refused endpoint must be distinct from TIMEOUT"
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_malformed_provider_response_fails_closed() {
    // Malformed provider response: the adapter must fail closed (an
    // error), never fabricate vacuums or states from garbage.
    let malformed = malformed_json_peer();
    let t = HaVacuumTransport::new(malformed, "ep024-token");
    let err = t.list_vacuums().expect_err("malformed response must fail");
    assert_ne!(err.code, VacuumErrorCode::NotFound, "never benign");
    assert_ne!(
        err.code,
        VacuumErrorCode::Unavailable,
        "not a connection failure"
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_unknown_vacuum_not_found() {
    // Unknown vacuum -> NotFound at the adapter boundary, never
    // Verified and never a benign vacuum state.
    let adapter = adapter();
    let ghost = vacuum_device_id("vacuum.nexus_ghost").expect("ghost id");
    let err = adapter
        .capabilities(&ghost)
        .expect_err("unknown vacuum must fail");
    assert_eq!(err.code, DevicesErrorCode::NotFound);

    // Registry-level unknown (configured target absent from the real
    // provider registry) -> NotFound from the transport.
    let t = transport();
    let err = t
        .read_vacuum("vacuum.nexus_ghost")
        .expect_err("absent registry entity must fail");
    assert_eq!(err.code, VacuumErrorCode::NotFound);
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_capabilities_from_real_features() {
    // Capability discovery from the REAL provider feature bits: the
    // fixture vacuum advertises start/pause/return_to_base, so the
    // adapter must map StartClean/Pause/ReturnHome/Dock - and NEVER
    // MapReadback (no map surface). The observed supported_features
    // value is recorded for evidence.
    let t = transport();
    let device = t.read_vacuum(VACUUM_A).expect("read vacuum A");
    let observed = device.supported_features().unwrap_or(0);
    eprintln!("observed vacuum A supported_features={observed}");
    assert_ne!(observed, 0, "the real provider must publish feature bits");

    let adapter = adapter();
    let device_a = vacuum_device_id(VACUUM_A).expect("vacuum A id");
    let caps = adapter.capabilities(&device_a).expect("A caps");
    assert!(
        caps.contains(&VacuumCapability::StartClean),
        "start is configured on the fixture -> StartClean must be advertised: {caps:?}"
    );
    assert!(
        caps.contains(&VacuumCapability::Pause),
        "pause is configured on the fixture -> Pause must be advertised: {caps:?}"
    );
    assert!(
        caps.contains(&VacuumCapability::ReturnHome),
        "return_to_base is configured -> ReturnHome must be advertised: {caps:?}"
    );
    assert!(
        caps.contains(&VacuumCapability::Dock),
        "return_to_base is configured -> Dock must be advertised (same provider action): {caps:?}"
    );
    assert!(
        !caps.contains(&VacuumCapability::MapReadback),
        "no map surface -> MapReadback must NEVER be advertised: {caps:?}"
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_unsupported_command_denied_before_provider() {
    // MapReadback without provider map support -> Policy (fail
    // closed), never success, never a provider mutation.
    let adapter = adapter();
    let device_a = vacuum_device_id(VACUUM_A).expect("vacuum A id");
    let caps = adapter.capabilities(&device_a).expect("A caps");
    let t = transport();
    let before = vacuum_state(&t, VACUUM_A);
    let err = adapter
        .map_readback(&device_a, &caps)
        .expect_err("no map surface");
    assert_eq!(err.code, VacuumErrorCode::Policy);
    assert_eq!(
        vacuum_state(&t, VACUUM_A),
        before,
        "denied command must not mutate the provider"
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_malformed_command_parameters_rejected() {
    // Malformed command parameters -> canonical validation failure
    // BEFORE any provider call (the vocabulary is the parameter
    // surface for this contract).
    let err = VacuumCommand::parse("MOP_THE_FLOOR").expect_err("must reject");
    assert_eq!(err.code, VacuumErrorCode::Vocabulary);
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_probe_observability_redaction_canary_absent() {
    // The adapter's audit ring is configured with the REAL provider
    // token as the redaction secret. After real failures, no stored
    // audit detail or counter may contain the credential, and every
    // entry carries a canonical vacuum correlation id.
    let adapter = adapter();
    let device_a = vacuum_device_id(VACUUM_A).expect("vacuum A id");
    let caps = adapter.capabilities(&device_a).expect("A caps");

    // Real failures: unknown vacuum NotFound + capability denial.
    let ghost = vacuum_device_id("vacuum.nexus_ghost").expect("ghost id");
    let _ = adapter.capabilities(&ghost);
    let _ = adapter.map_readback(&device_a, &caps);

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
            entry.correlation.starts_with("vacuum-"),
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
#[ignore = "requires live HA container; run via scripts/ep024-m5-tests.sh"]
fn ep024_failure_vacuum_journey_live() {
    let adapter = adapter();
    let device_a = vacuum_device_id(VACUUM_A).expect("vacuum A id");
    let device_b = vacuum_device_id(VACUUM_B).expect("vacuum B id");
    let caps_a = adapter.capabilities(&device_a).expect("A caps");
    let caps_b = adapter.capabilities(&device_b).expect("B caps");

    // --- baseline -----------------------------------------------------------
    let t = transport();
    assert_eq!(vacuum_state(&t, VACUUM_A), "docked", "A baseline docked");
    assert_eq!(vacuum_state(&t, VACUUM_B), "docked", "B baseline docked");

    // --- F: real StartClean -> CLEANING -------------------------------------
    let receipt = adapter
        .execute(&device_a, VacuumCommand::StartClean, &caps_a)
        .expect("A start clean");
    assert_eq!(
        receipt.state,
        VacuumCommandState::Submitted,
        "receipt is SUBMITTED at most, never VERIFIED"
    );
    wait_state(&t, VACUUM_A, "cleaning", 30).expect("A reaches cleaning");
    assert_eq!(
        vacuum_state(&t, VACUUM_A),
        "cleaning",
        "independent fresh readback must observe A cleaning"
    );
    adapter
        .verify(&device_a, VacuumCommand::StartClean, "CLEANING")
        .expect("A verified cleaning");
    assert_eq!(
        vacuum_state(&t, VACUUM_B),
        "docked",
        "commanding A must never touch B"
    );

    // --- G: real Pause -> PAUSED ---------------------------------------------
    let receipt = adapter
        .execute(&device_a, VacuumCommand::Pause, &caps_a)
        .expect("A pause");
    assert_eq!(receipt.state, VacuumCommandState::Submitted);
    wait_state(&t, VACUUM_A, "paused", 30).expect("A reaches paused");
    adapter
        .verify(&device_a, VacuumCommand::Pause, "PAUSED")
        .expect("A verified paused");

    // Resume so ReturnHome starts from an active state.
    adapter
        .execute(&device_a, VacuumCommand::StartClean, &caps_a)
        .expect("A resume");
    wait_state(&t, VACUUM_A, "cleaning", 30).expect("A cleaning again");

    // --- H: real ReturnHome -> RETURNING (distinct) -> DOCKED -----------------
    let receipt = adapter
        .execute(&device_a, VacuumCommand::ReturnHome, &caps_a)
        .expect("A return home");
    assert_eq!(receipt.state, VacuumCommandState::Submitted);
    wait_state(&t, VACUUM_A, "returning", 30).expect("A reaches RETURNING");
    assert_eq!(
        vacuum_state(&t, VACUUM_A),
        "returning",
        "RETURNING must be observed distinct from DOCKED"
    );
    adapter
        .verify(&device_a, VacuumCommand::ReturnHome, "RETURNING")
        .expect("A verified RETURNING");
    // The real auto-dock automation performs the DOCKED transition.
    wait_state(&t, VACUUM_A, "docked", 60).expect("A docks after returning");
    adapter
        .verify(&device_a, VacuumCommand::ReturnHome, "DOCKED")
        .expect("A verified DOCKED after the real transition");

    // --- I: Dock maps to the SAME provider action ----------------------------
    adapter
        .execute(&device_b, VacuumCommand::StartClean, &caps_b)
        .expect("B start clean");
    wait_state(&t, VACUUM_B, "cleaning", 30).expect("B cleaning");
    let receipt = adapter
        .execute(&device_b, VacuumCommand::Dock, &caps_b)
        .expect("B dock");
    assert_eq!(receipt.state, VacuumCommandState::Submitted);
    wait_state(&t, VACUUM_B, "returning", 30).expect("B RETURNING via Dock");
    wait_state(&t, VACUUM_B, "docked", 60).expect("B docks via Dock");
    adapter
        .verify(&device_b, VacuumCommand::Dock, "DOCKED")
        .expect("B verified DOCKED via Dock");

    // --- J: wrong-target state transition never verifies ---------------------
    adapter
        .execute(&device_b, VacuumCommand::StartClean, &caps_b)
        .expect("B clean");
    wait_state(&t, VACUUM_B, "cleaning", 30).expect("B cleaning");
    let err = adapter
        .verify(&device_a, VacuumCommand::StartClean, "CLEANING")
        .expect_err("B's transition must not satisfy A's plan");
    assert_eq!(err.code, VacuumErrorCode::Verification);
    // Verifier-level invariant.
    let verifier = DeviceCommandVerifier;
    let observation = DeviceStateObservation {
        device: device_b.as_str().to_string(),
        state: Some("CLEANING".to_string()),
    };
    assert_eq!(
        verifier.verify(device_a.as_str(), "CLEANING", &observation),
        VerificationOutcome::UnrelatedChange
    );

    // --- N: retry after completion is NOT a Conflict --------------------------
    let receipt = adapter
        .execute(&device_b, VacuumCommand::StartClean, &caps_b)
        .expect("re-issuing a completed command must not conflict");
    assert_eq!(
        receipt.state,
        VacuumCommandState::Submitted,
        "a completed command released its in-flight entry"
    );

    // --- correlation continuity on failure (L) ------------------------------
    let err = adapter
        .verify(&device_a, VacuumCommand::StartClean, "CLEANING")
        .expect_err("A is docked; CLEANING verification fails");
    assert_eq!(err.code, VacuumErrorCode::Verification);
    let correlation = err.correlation.expect("error must carry correlation");
    assert!(
        correlation.starts_with("vacuum-"),
        "canonical correlation: {correlation}"
    );

    // --- U: restart -> stable canonical identity -----------------------------
    // Leave B docked before restart for a deterministic baseline.
    adapter
        .execute(&device_b, VacuumCommand::ReturnHome, &caps_b)
        .expect("B return home");
    wait_state(&t, VACUUM_B, "docked", 60).expect("B docked before restart");
    docker(&["restart", &container_name()]).expect("docker restart");
    wait_http_up(300).expect("HA ready after restart");
    wait_vacuums(&adapter, 180);
    let devices_after = adapter.list_devices().expect("rediscovery");
    assert!(
        devices_after.contains(&device_a) && devices_after.contains(&device_b),
        "canonical identity must be unchanged after restart"
    );
    let caps_a_after = adapter
        .capabilities(&device_a)
        .expect("A caps after restart");
    adapter
        .execute(&device_a, VacuumCommand::StartClean, &caps_a_after)
        .expect("A start clean after restart");
    wait_state(&t, VACUUM_A, "cleaning", 60).expect("A cleaning after restart");
    adapter
        .verify(&device_a, VacuumCommand::StartClean, "CLEANING")
        .expect("A verified cleaning after restart");
    adapter
        .execute(&device_a, VacuumCommand::ReturnHome, &caps_a_after)
        .expect("A return home after restart");
    wait_state(&t, VACUUM_A, "docked", 60).expect("A docked after restart");

    // --- P: provider offline -> UNAVAILABLE, never benign --------------------
    docker(&["stop", &container_name()]).expect("docker stop");
    thread::sleep(Duration::from_secs(3));
    let err = adapter
        .execute(&device_a, VacuumCommand::StartClean, &caps_a_after)
        .expect_err("provider offline");
    assert_eq!(err.code, VacuumErrorCode::Unavailable);
    let avail = adapter.availability(&device_a).expect("availability");
    assert_eq!(
        avail,
        DeviceAvailability::Unavailable,
        "provider offline must map to UNAVAILABLE, never DOCKED/SAFE"
    );

    // --- bounded recovery: no stuck entries, no fabricated state -------------
    let cleared = adapter.recover();
    assert_eq!(cleared, 0, "failed commands must already release entries");

    // --- provider restored -> fresh readback only (N) ------------------------
    docker(&["start", &container_name()]).expect("docker start");
    wait_http_up(300).expect("HA ready after offline phase");
    wait_vacuums(&adapter, 180);
    let avail = adapter
        .availability(&device_a)
        .expect("availability after restore");
    assert_eq!(
        avail,
        DeviceAvailability::Available,
        "availability returns only when the provider reports it"
    );
    adapter
        .execute(&device_a, VacuumCommand::StartClean, &caps_a_after)
        .expect("A start clean after restore");
    wait_state(&t, VACUUM_A, "cleaning", 60).expect("A cleaning after restore");
    adapter
        .verify(&device_a, VacuumCommand::StartClean, "CLEANING")
        .expect("A verified cleaning after restore");
    adapter
        .execute(&device_a, VacuumCommand::ReturnHome, &caps_a_after)
        .expect("A return home after restore");
    wait_state(&t, VACUUM_A, "docked", 60).expect("A docked after restore");

    // --- zero secret leakage across the whole journey ------------------------
    let token = token();
    for entry in adapter.audit() {
        assert!(
            !entry.detail.contains(&token),
            "journey audit leaked the provider token: {}",
            entry.detail
        );
    }

    // Canonical state mapping sanity: A is docked, never claimed
    // otherwise by the production mapper.
    let device = t.read_vacuum(VACUUM_A).expect("A read");
    assert_eq!(
        vacuum_state_value(&device),
        Some(VacuumActivityState::Docked),
        "canonical state mapping from the real provider state"
    );
}
