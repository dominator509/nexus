//! EP-025 M4 live failure suite: REAL forced-failure proofs against
//! the REAL pinned Asterisk 22.10.1 container with REAL controlled
//! SIP fixtures (baresip + reject_endpoint.py), routed through the
//! REAL nexus-telephony Stasis app and REAL ARI mixing bridges.
//!
//! These are LIVE-STACK tests (Ep-023/Ep-024/Ep-025 convention):
//! they require the real fixture and are marked `#[ignore]` so the
//! ambient workspace battery stays green. The M4 gate
//! (scripts/ep025-m4-tests.sh) starts the fixture (baresip a/b/c/d,
//! reject_endpoint.py responders for r/s/t, ARI observer) and runs
//! them with `--ignored --test-threads=1`.
//!
//! Proven here (directive M4):
//!   - typed BUSY: real SIP 486 -> real ChannelDestroyed cause 17
//!     -> Nexus Busy (event stream is the terminal authority);
//!   - typed REJECTED: real SIP 603 -> cause 21 -> Nexus Rejected;
//!   - typed NO_ANSWER: bounded ARI originate timeout -> Asterisk
//!     destroys the ringing channel (real provider lifecycle) -> cause
//!     18/19/102 -> Nexus NoAnswer;
//!   - wrong ARI credential fails closed (no fabricated availability);
//!   - Asterisk unavailable -> honest failure, no fake CallSession;
//!   - one-way media: a real peer that answers but sends NO RTP ->
//!     production never claims two-way audio verified;
//!   - mid-call media loss: a real peer whose RTP source goes silent
//!     while the call stays signaling-active -> production still does
//!     not claim verified media;
//!   - restart during an active call: call lost is observed honestly,
//!     ARI reconnects, endpoints re-register, a new real call with
//!     two-way media succeeds;
//!   - ambiguous originate: a lost control response must NOT trigger a
//!     blind second originate; reconcile_originate finds the real
//!     channel and Asterisk holds exactly one call;
//!   - non-Stasis DTMF: a channel outside Stasis -> real HTTP 409 ->
//!     canonical Conflict, never success;
//!   - event-stream disconnect: consumer marks itself disconnected,
//!     terminal classification returns Verification (UNKNOWN) instead
//!     of fabricating BUSY/REJECTED, reconnect resumes, and the gap
//!     never resurrects a call (exact-target store).

use std::env;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use nexus_asterisk::adapter::AsteriskAdapter;
use nexus_asterisk::events::{run_event_consumer, EventStore};
use nexus_asterisk::transport::{AriChannel, AriTransport, ChannelSelector, RestAriTransport};
use nexus_telephony::error::{CallError, CallErrorCode};
use nexus_telephony::provider::TelephonyProvider;
use nexus_telephony::vocabulary::{
    CallCapability, CallPolicy, CallState, DisclosurePolicy, MediaState, SipEndpointId,
};

const STASIS_APP: &str = "nexus-telephony";
const CONTAINER: &str = "nexus-ep025-ast";
const WS_HOST: &str = "127.0.0.1";
const WS_PORT: u16 = 8088;

fn env_var(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("{name} is required for ep025_failure_live tests"))
}

fn ari_env() -> (String, String, String) {
    (
        env_var("NEXUS_ARI_URL"),
        env_var("NEXUS_ARI_USER"),
        env_var("NEXUS_ARI_PASSWORD"),
    )
}

fn policy() -> CallPolicy {
    CallPolicy {
        allowed_capabilities: vec![
            CallCapability::Dial,
            CallCapability::Answer,
            CallCapability::Hangup,
            CallCapability::Dtmf,
            CallCapability::Status,
        ],
        max_duration_seconds: 120,
        cost_cap: 1.0,
        disclosure: DisclosurePolicy::new(false, true, "US", 0).expect("disclosure policy"),
    }
}

fn transport() -> RestAriTransport {
    let (base, user, pass) = ari_env();
    RestAriTransport::new(base, user, pass, Duration::from_secs(10)).expect("real ARI transport")
}

fn adapter() -> AsteriskAdapter {
    AsteriskAdapter::new(Box::new(transport()), policy())
}

fn endpoint(name: &str) -> SipEndpointId {
    SipEndpointId::new(name).expect("canonical endpoint id")
}

fn session_id(s: &str) -> nexus_telephony::CallSessionId {
    nexus_telephony::CallSessionId::new(s).expect("session id")
}

fn wait_state(a: &AsteriskAdapter, session: &str, expected: CallState, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    let id = session_id(session);
    while Instant::now() < deadline {
        if let Ok(state) = a.session_state(&id) {
            if state == expected {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

fn wait_media(a: &AsteriskAdapter, session: &str, expected: MediaState, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    let id = session_id(session);
    while Instant::now() < deadline {
        if let Ok(media) = a.media_state(&id) {
            if media == expected {
                return true;
            }
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

fn wait_gone(a: &AsteriskAdapter, session: &str, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    let id = session_id(session);
    while Instant::now() < deadline {
        match a.session_state(&id) {
            Err(e) if e.code == CallErrorCode::NotFound => return true,
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    false
}

/// Spawn the production WS event consumer; return (store, stop).
fn start_consumer() -> (Arc<Mutex<EventStore>>, Arc<AtomicBool>) {
    let store = Arc::new(Mutex::new(EventStore::new()));
    let stop = connect_consumer(store.clone());
    // Wait for the real WS subscription to be live (observability
    // flag; no sleeps-and-assume).
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if store.lock().unwrap().connected {
            return (store, stop);
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("EP-025 M4: event consumer never connected to real Asterisk WS");
}

/// Spawn a consumer on an EXISTING store (reconnect case) and return
/// its stop flag.
fn connect_consumer(store: Arc<Mutex<EventStore>>) -> Arc<AtomicBool> {
    let (_, user, pass) = ari_env();
    let stop = Arc::new(AtomicBool::new(false));
    let s = store.clone();
    let sp = stop.clone();
    let u = user.clone();
    let p = pass.clone();
    std::thread::spawn(move || {
        run_event_consumer(WS_HOST, WS_PORT, &u, &p, STASIS_APP, s, sp);
    });
    stop
}

fn wait_connected(store: &Arc<Mutex<EventStore>>, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if store.lock().unwrap().connected {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

fn wait_disconnected(store: &Arc<Mutex<EventStore>>, secs: u64) -> bool {
    let deadline = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < deadline {
        if !store.lock().unwrap().connected {
            return true;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    false
}

/// Spawn a controlled baresip endpoint (fixture). The mid-call
/// provider restart kills the running baresip processes (M3
/// observation), so the restart test re-spawns them and waits for
/// real per-AOR registration before placing the new call.
fn spawn_baresip(name: &str) -> std::process::Child {
    let dir = env_var(&format!("NEXUS_BARESIP_{}_DIR", name.to_uppercase()));
    let audio = env_var(&format!("NEXUS_EP025_AUDIO_{}_DIR", name.to_uppercase()));
    Command::new("baresip")
        .args(["-f", &dir, "-s", "-v", "-p", &audio])
        .current_dir("/usr/lib/baresip/modules")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn baresip endpoint")
}

/// Read the store's recorded cause for a session (session id == real
/// channel id).
fn store_cause(store: &Arc<Mutex<EventStore>>, session: &str) -> Option<(u32, String)> {
    store.lock().unwrap().causes.get(session).cloned()
}

/// Number of current contacts Asterisk itself shows for an AOR
/// (per-AOR readiness invariant, M3; via the real CLI surface).
fn aor_contacts(aor: &str) -> usize {
    let out = Command::new("/usr/bin/docker")
        .args([
            "exec",
            CONTAINER,
            "/usr/sbin/asterisk",
            "-rx",
            &format!("pjsip show aor {aor}"),
        ])
        .output()
        .expect("docker exec pjsip show aor");
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .filter(|l| l.trim_start().starts_with(&format!("Contact:  {aor}/")))
        .count()
}

/// Clear the baresip capture dirs of any dump-*.wav (decoded or raw)
/// so a subsequent proof is bound to the CURRENT call only. Used by
/// the restart proof (section E): after the provider restart and
/// re-spawn, the pre-restart call's captures are stale and must not
/// satisfy the post-restart media guard.
fn clear_audio_dumps() {
    for name in ["A", "B"] {
        let dir = env_var(&format!("NEXUS_EP025_AUDIO_{name}_DIR"));
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                    if fname.starts_with("dump-") && fname.ends_with(".wav") {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
    }
}

/// Place a two-leg real call (both legs into the real Stasis app),
/// answer both, add both to a real mixing bridge, and wait for the
/// authoritative bridge membership (media_state == TransportActive).
fn place_bridged_call(
    a: &AsteriskAdapter,
    first: &str,
    second: &str,
    token: &str,
) -> (
    nexus_telephony::CallSession,
    nexus_telephony::CallSession,
    String,
) {
    let s1 = a
        .originate_stasis(&endpoint(first), STASIS_APP, token, Some(token))
        .expect("originate first leg");
    assert!(
        wait_state(a, s1.id.as_str(), CallState::Answered, 25),
        "first leg {first} did not reach Answered"
    );
    let s2 = a
        .originate_stasis(
            &endpoint(second),
            STASIS_APP,
            token,
            Some(&format!("{token}-2")),
        )
        .expect("originate second leg");
    assert!(
        wait_state(a, s2.id.as_str(), CallState::Answered, 25),
        "second leg {second} did not reach Answered"
    );
    let bridge = a
        .create_mixing_bridge("ep025-m4-bridge")
        .expect("create bridge");
    a.add_to_bridge(&s1.id, &bridge).expect("add first leg");
    a.add_to_bridge(&s2.id, &bridge).expect("add second leg");
    assert!(
        wait_media(a, s1.id.as_str(), MediaState::TransportActive, 15),
        "first leg never TransportActive (bridge membership)"
    );
    assert!(
        wait_media(a, s2.id.as_str(), MediaState::TransportActive, 15),
        "second leg never TransportActive (bridge membership)"
    );
    (s1, s2, bridge)
}

/// Tear down both legs of a bridged call and delete the bridge
/// (zero-orphan discipline; a leaked leg keeps the endpoint busy and
/// a leaked empty bridge fails the gate's orphan audit).
fn teardown_bridged_call(
    a: &AsteriskAdapter,
    s1: &nexus_telephony::CallSession,
    s2: &nexus_telephony::CallSession,
    bridge: &str,
) {
    let _ = a.hangup(&s1.id);
    let _ = a.hangup(&s2.id);
    let _ = a.delete_bridge(bridge);
}

// ---------------------------------------------------------------------
// 1. Typed REJECTED: real SIP 603 -> ChannelDestroyed cause 21
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_rejected_603_typed_rejected() {
    let (store, stop) = start_consumer();
    let a = adapter().with_event_store(store.clone());
    let token = format!("ep025-reject-{}", std::process::id());
    let session = a
        .originate_stasis(&endpoint("endpoint-r"), STASIS_APP, &token, Some(&token))
        .expect("originate to endpoint-r (603 responder)");
    let state = a
        .wait_terminal(&session.id, Duration::from_secs(30))
        .expect("terminal classification");
    assert_eq!(state, CallState::Rejected, "603 must map to Rejected");
    let cause = store_cause(&store, session.id.as_str());
    assert!(
        matches!(cause, Some((21, _))),
        "real ChannelDestroyed cause 21 expected, got {cause:?}"
    );
    // Cleanup: no channels left behind.
    let _ = a.hangup(&session.id);
    stop.store(true, Ordering::Relaxed);
}

// ---------------------------------------------------------------------
// 2. Typed BUSY: real SIP 486 -> ChannelDestroyed cause 17
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_busy_486_typed_busy() {
    let (store, stop) = start_consumer();
    let a = adapter().with_event_store(store.clone());
    let token = format!("ep025-busy-{}", std::process::id());
    let session = a
        .originate_stasis(&endpoint("endpoint-r"), STASIS_APP, &token, Some(&token))
        .expect("originate to endpoint-r (486 responder)");
    let state = a
        .wait_terminal(&session.id, Duration::from_secs(30))
        .expect("terminal classification");
    assert_eq!(state, CallState::Busy, "486 must map to Busy");
    let cause = store_cause(&store, session.id.as_str());
    assert!(
        matches!(cause, Some((17, _))),
        "real ChannelDestroyed cause 17 expected, got {cause:?}"
    );
    let _ = a.hangup(&session.id);
    stop.store(true, Ordering::Relaxed);
}

// ---------------------------------------------------------------------
// 3. Typed NO_ANSWER: bounded provider originate timeout -> cause
//    18/19/102 -> NoAnswer (NOT a local sleep)
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_no_answer_bounded_provider_timeout() {
    let (store, stop) = start_consumer();
    let a = adapter().with_event_store(store.clone());
    let token = format!("ep025-noanswer-{}", std::process::id());
    // Bounded originate: Asterisk itself destroys the ringing channel
    // when the ARI timeout expires (real provider lifecycle). The
    // manual-answer endpoint never answers.
    let session = a
        .originate_stasis_bounded(&endpoint("endpoint-c"), STASIS_APP, &token, Some(&token), 8)
        .expect("bounded originate to endpoint-c (manual answer)");
    let state = a
        .wait_terminal(&session.id, Duration::from_secs(30))
        .expect("terminal classification");
    assert_eq!(
        state,
        CallState::NoAnswer,
        "ringing timeout must map to NoAnswer"
    );
    let cause = store_cause(&store, session.id.as_str());
    assert!(
        matches!(cause, Some((18 | 19 | 102, _))),
        "provider-destroyed ringing channel cause 18/19/102 expected, got {cause:?}"
    );
    let _ = a.hangup(&session.id);
    stop.store(true, Ordering::Relaxed);
}

// ---------------------------------------------------------------------
// 4. Wrong ARI credential fails closed (no fabricated availability)
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_wrong_ari_credential_fails_closed() {
    let (base, user, _pass) = ari_env();
    let bad = RestAriTransport::new(
        base,
        user,
        "definitely-wrong-password",
        Duration::from_secs(10),
    )
    .expect("transport with wrong password");
    let a = AsteriskAdapter::new(Box::new(bad), policy());
    match a.provider_available() {
        Ok(true) => panic!("provider must NOT report available with wrong ARI credential"),
        Ok(false) => panic!("provider_available returned false without error"),
        Err(e) => {
            assert_eq!(
                e.code,
                CallErrorCode::Authorization,
                "wrong ARI credential must map to Authorization, got {:?}",
                e.code
            );
        }
    }
}

// ---------------------------------------------------------------------
// 5. Asterisk unavailable -> honest failure, no fake CallSession
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_asterisk_unavailable_honest() {
    let (store, stop) = start_consumer();
    let a = adapter().with_event_store(store.clone());
    // Stop the real container (gate restarts it afterwards).
    let st = Command::new("/usr/bin/docker")
        .args(["stop", CONTAINER])
        .output()
        .expect("docker stop");
    assert!(st.status.success(), "docker stop failed");

    match a.provider_available() {
        Ok(true) => panic!("provider must not report available while Asterisk is stopped"),
        Ok(false) => {}
        Err(e) => {
            assert!(
                matches!(
                    e.code,
                    CallErrorCode::Unavailable | CallErrorCode::Timeout | CallErrorCode::External
                ),
                "stopped provider must fail honestly, got {:?}",
                e.code
            );
        }
    }
    // Originate must fail honestly: no fake CallSession, no fabricated
    // terminal state.
    if let Ok(s) = a.originate_stasis(
        &endpoint("endpoint-a"),
        STASIS_APP,
        "ep025-down",
        Some("ep025-down"),
    ) {
        panic!(
            "originate succeeded while provider is down: session {}",
            s.id
        );
    }
    // Restart and wait for real health again.
    let st = Command::new("/usr/bin/docker")
        .args(["start", CONTAINER])
        .output()
        .expect("docker start");
    assert!(st.status.success(), "docker start failed");
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut healthy = false;
    while Instant::now() < deadline {
        if a.provider_available().unwrap_or(false) {
            healthy = true;
            break;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    assert!(healthy, "Asterisk did not become healthy again within 120s");
    // Consumer must reconnect to the real WS after the restart.
    assert!(
        wait_connected(&store, 60),
        "event consumer did not reconnect after restart"
    );
    // Full fixture re-registration before the suite continues: every
    // controlled endpoint must be back (per-AOR, M3 invariant).
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut all_reg = false;
    while Instant::now() < deadline {
        let counts = [
            aor_contacts("endpoint-a"),
            aor_contacts("endpoint-b"),
            aor_contacts("endpoint-c"),
            aor_contacts("endpoint-d"),
            aor_contacts("endpoint-r"),
            aor_contacts("endpoint-s"),
            aor_contacts("endpoint-t"),
        ];
        if counts.iter().all(|&n| n == 1) {
            all_reg = true;
            break;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    assert!(
        all_reg,
        "controlled endpoints did not fully re-register after restart"
    );
    stop.store(true, Ordering::Relaxed);
}

// ---------------------------------------------------------------------
// 6. One-way media: silent peer (answers, sends NO RTP) -> production
//    never claims two-way audio verified
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_one_way_media_not_verified() {
    let (store, stop) = start_consumer();
    let a = adapter().with_event_store(store.clone());
    let token = format!("ep025-oneway-{}", std::process::id());
    // A = the DEDICATED controlled sender responder (endpoint-u, real
    // PCMU RTP source, wire identity src port 12120), B = the silent
    // peer (endpoint-s, a=recvonly, sends NO RTP). A baresip A-side
    // is intentionally NOT used here: the prior DTMF/dialplan bridge
    // poisons baresip's RTP sequence state (documented M3 SSRC/jbuf
    // defect), which would starve the wire proof. The responder pair
    // makes the directionality deterministic: bytes flow A -> B
    // (captured at 12060), nothing flows B -> A (src 12060 stays
    // zero). endpoint-u is distinct from endpoint-t (mid-call-loss
    // proof) so no cross-test RTP can satisfy the wrong guard.
    let (s_a, s_s, bridge) = place_bridged_call(&a, "endpoint-u", "endpoint-s", &token);
    // Hold the bridged call so the sender's real RTP window
    // accumulates on the wire capture (>=50 forwarded packets to the
    // silent peer is the gate's locked minimum; 2s at 20ms cadence
    // yields ~100).
    std::thread::sleep(Duration::from_secs(2));
    // Signaling is bridged (media path established in the bridge)...
    assert_eq!(
        a.media_state(&s_s.id).expect("media state"),
        MediaState::TransportActive,
        "silent peer leg must still be TransportActive (bridged)"
    );
    // ...but production must NOT report verified two-way audio.
    assert_ne!(
        a.media_state(&s_s.id).expect("media state"),
        MediaState::TwoWayAudioVerified,
        "one-way media must never be reported as two-way verified"
    );
    teardown_bridged_call(&a, &s_a, &s_s, &bridge);
    stop.store(true, Ordering::Relaxed);
}

// ---------------------------------------------------------------------
// 7. Mid-call media loss: sender peer goes silent while the call stays
//    signaling-active -> production does not claim verified media
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_mid_call_media_loss_not_verified() {
    let (store, stop) = start_consumer();
    let a = adapter().with_event_store(store.clone());
    let token = format!("ep025-midloss-{}", std::process::id());
    // endpoint-t sends real PCMU silence RTP for 8s then goes silent
    // (gate captures: RTP from 172.17.0.1:12070 stops after the
    // window while the call remains Up).
    let (s_a, s_t, bridge) = place_bridged_call(&a, "endpoint-a", "endpoint-t", &token);
    assert_eq!(
        a.media_state(&s_t.id).expect("media state"),
        MediaState::TransportActive,
        "sender peer leg must start bridged"
    );
    // Wait past the sender window (8s) + margin.
    std::thread::sleep(Duration::from_secs(12));
    // Call remains signaling-active (bridged)...
    assert_eq!(
        a.session_state(&s_t.id).expect("session state"),
        CallState::Bridged,
        "call must remain signaling-active after media source goes silent"
    );
    // ...but production still does NOT claim two-way audio verified.
    assert_ne!(
        a.media_state(&s_t.id).expect("media state"),
        MediaState::TwoWayAudioVerified,
        "media-degraded call must never be reported as two-way verified"
    );
    teardown_bridged_call(&a, &s_a, &s_t, &bridge);
    stop.store(true, Ordering::Relaxed);
}

// ---------------------------------------------------------------------
// 8. Restart during an active call: call loss observed honestly, ARI
//    reconnect, re-registration, new real call with two-way media
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_restart_during_active_call() {
    let (store, stop) = start_consumer();
    let a = adapter().with_event_store(store.clone());
    let token = format!("ep025-restart-{}", std::process::id());
    // Establish a real active call a<->b (both auto-answer with real
    // canary audio).
    let (s1, s2, _bridge1) = place_bridged_call(&a, "endpoint-a", "endpoint-b", &token);
    assert_eq!(
        a.media_state(&s1.id).expect("media"),
        MediaState::TransportActive
    );

    // Restart the REAL Asterisk container mid-call.
    let st = Command::new("/usr/bin/docker")
        .args(["restart", CONTAINER])
        .output()
        .expect("docker restart");
    assert!(st.status.success(), "docker restart failed");

    // Observe the call honestly: the channels die with the provider.
    // Do NOT synthesize continuity.
    assert!(
        wait_gone(&a, s1.id.as_str(), 60) || !a.session_state(&s1.id).is_ok(),
        "call leg 1 must not survive a provider restart (or must be gone)"
    );

    // ARI reconnect: consumer marks itself connected again.
    assert!(
        wait_connected(&store, 120),
        "event consumer did not reconnect after restart"
    );

    // The controlled baresip endpoints die on a mid-call provider
    // restart (M3 observation): re-spawn them and wait for real
    // per-AOR registration before the new call. Keep the Child
    // handles and reap them at test end (no zombie processes).
    let mut ba = spawn_baresip("a");
    let mut bb = spawn_baresip("b");

    // Endpoints re-register: per-AOR usable contacts return.
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut reg = false;
    while Instant::now() < deadline {
        if aor_contacts("endpoint-a") == 1 && aor_contacts("endpoint-b") == 1 {
            reg = true;
            break;
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    assert!(reg, "endpoints did not re-register after restart");

    // Bind the post-restart media proof to the CURRENT restart call
    // only: clear any stale pre-restart captures so the gate's dec
    // evidence cannot be satisfied by old media artifacts (section E).
    clear_audio_dumps();

    // New real call succeeds and reaches bridged two-way media again.
    let (n1, n2, bridge2) =
        place_bridged_call(&a, "endpoint-a", "endpoint-b", &format!("{token}-new"));
    assert_eq!(
        a.media_state(&n1.id).expect("media"),
        MediaState::TransportActive,
        "post-restart call must reach bridged media"
    );
    assert_eq!(
        a.media_state(&n2.id).expect("media"),
        MediaState::TransportActive,
        "post-restart call must reach bridged media (leg 2)"
    );
    // Hold the post-restart call so the canary audio accumulates into
    // the (already-cleared) capture dirs: the gate's restart media
    // guard requires nonzero decoded WAV evidence (>10000 bytes) from
    // the CURRENT restart call only. 6s at 8 kHz/16-bit yields ~96 KB
    // dec per side.
    std::thread::sleep(Duration::from_secs(6));
    let _ = a.hangup(&s1.id);
    let _ = a.hangup(&s2.id);
    teardown_bridged_call(&a, &n1, &n2, &bridge2);
    stop.store(true, Ordering::Relaxed);
    // Reap the re-spawned baresip fixtures (no zombie processes).
    let _ = ba.kill();
    let _ = bb.kill();
    let _ = ba.wait();
    let _ = bb.wait();
}

// ---------------------------------------------------------------------
// 9. Ambiguous originate: lost control response -> reconcile against
//    real state, NO blind second originate
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_ambiguous_originate_no_blind_retry() {
    // Wrap the REAL transport: the first originate IS forwarded to
    // real Asterisk (the call really rings) but the response is lost.
    let wrapper = DropFirstOriginate::new(transport());
    let a = AsteriskAdapter::new(Box::new(wrapper), policy());
    // The reconcile token must be a real caller NUMBER: Asterisk
    // parses bare digit strings as the caller number (an
    // alphanumeric string becomes the caller NAME with an empty
    // number, which reconcile cannot match).
    let token = std::process::id().to_string();

    let result = a.originate_stasis(&endpoint("endpoint-d"), STASIS_APP, &token, Some(&token));
    match result {
        Ok(s) => panic!(
            "originate must report the lost response as failure, got session {}",
            s.id
        ),
        Err(e) => {
            assert_eq!(
                e.code,
                CallErrorCode::Unavailable,
                "lost control response must surface as Unavailable, got {:?}",
                e.code
            );
        }
    }

    // Reconcile against REAL Asterisk channel state: the call is real
    // (the INVITE went out) but its control result was ambiguous.
    let session = a
        .reconcile_originate(&token, Duration::from_secs(15))
        .expect("reconcile must find the real channel");

    // Asterisk must hold EXACTLY ONE channel for this logical call:
    // no blind retry created a duplicate.
    let channels = a.transport().list_channels().expect("list channels");
    let matches = channels
        .iter()
        .filter(|c| {
            c.caller
                .as_ref()
                .map(|x| x.number == token)
                .unwrap_or(false)
        })
        .count();
    assert_eq!(
        matches, 1,
        "ambiguous originate must NOT create a duplicate call (found {matches})"
    );

    let _ = a.hangup(&session.id);
}

// ---------------------------------------------------------------------
// 10. Non-Stasis DTMF: channel outside Stasis -> real HTTP 409 ->
//     canonical Conflict, never success
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_non_stasis_dtmf_409() {
    let a = adapter();
    let token = format!("ep025-dtmf409-{}", std::process::id());
    // Plain originate into the dialplan (extension 100 -> Dial to
    // endpoint-a; baresip-a auto-answers). The channel is handled by
    // the Dial application, NOT by the Stasis app.
    let session = a
        .originate(&endpoint("endpoint-b"), "internal", "100", Some(&token))
        .expect("dialplan originate");
    // The dialplan Dial() bridges the legs in a REAL basic bridge, so
    // the production state is Bridged (Up + real bridge membership),
    // not Answered. Either is the signaling-active precondition.
    let reached = {
        let deadline = Instant::now() + Duration::from_secs(25);
        let mut ok = false;
        while Instant::now() < deadline {
            match a.session_state(&session.id) {
                Ok(CallState::Answered | CallState::Bridged) => {
                    ok = true;
                    break;
                }
                _ => std::thread::sleep(Duration::from_millis(300)),
            }
        }
        ok
    };
    assert!(reached, "dialplan channel did not reach Answered/Bridged");
    let err = a
        .send_dtmf(&session.id, "5")
        .expect_err("send_dtmf must fail");
    assert_eq!(
        err.code,
        CallErrorCode::Conflict,
        "non-Stasis DTMF must map 409 -> Conflict, got {:?}",
        err.code
    );
    let _ = a.hangup(&session.id);
}

// ---------------------------------------------------------------------
// 11. Event-stream disconnect: no fabricated terminal state, no
//     resurrection, reconnect resumes
// ---------------------------------------------------------------------
#[test]
#[ignore]
fn ep025_live_event_stream_disconnect_no_fabrication() {
    let (store, stop) = start_consumer();
    let a = adapter().with_event_store(store.clone());

    // Simulate a real WS disconnect: stop the consumer thread and WAIT
    // for the store to observe the stream is down (bounded; the read
    // times out within 30s when no events flow). No call is active
    // during the wait, so nothing races the disconnect signal.
    stop.store(true, Ordering::Relaxed);
    assert!(
        wait_disconnected(&store, 60),
        "consumer must mark the event stream disconnected"
    );

    // A call terminates during the gap: the channel disappears with
    // NO typed cause recorded (the stream is down). Nexus must NOT
    // fabricate BUSY/REJECTED/NO_ANSWER from the missing channel.
    let token1 = format!("ep025-gap-{}", std::process::id());
    let s1 = a
        .originate_stasis(&endpoint("endpoint-r"), STASIS_APP, &token1, Some(&token1))
        .expect("originate during stream gap");
    let err = a
        .wait_terminal(&s1.id, Duration::from_secs(25))
        .expect_err("terminal classification without the stream must be honest");
    assert_eq!(
        err.code,
        CallErrorCode::Verification,
        "lost terminal cause must surface as Verification/UNKNOWN, got {:?}",
        err.code
    );

    // Reconnect: a NEW consumer on the SAME store resumes the real
    // subscription.
    let stop2 = connect_consumer(store.clone());
    assert!(wait_connected(&store, 20), "consumer did not reconnect");

    // A new call terminates with the stream live: typed Rejected.
    let token2 = format!("ep025-after-{}", std::process::id());
    let s2 = a
        .originate_stasis(&endpoint("endpoint-r"), STASIS_APP, &token2, Some(&token2))
        .expect("originate after reconnect");
    let state = a
        .wait_terminal(&s2.id, Duration::from_secs(30))
        .expect("terminal classification after reconnect");
    assert_eq!(
        state,
        CallState::Rejected,
        "reconnected stream must classify 603"
    );

    // Exact-target: the gap session was NEVER resurrected by the
    // second session's event; its cause stays absent in the store.
    assert!(
        store_cause(&store, s1.id.as_str()).is_none(),
        "gap session must not be retroactively terminated by another call's event"
    );
    assert!(
        matches!(store_cause(&store, s2.id.as_str()), Some((21, _))),
        "reconnected session must carry its own typed cause"
    );
    let _ = a.hangup(&s2.id);
    stop2.store(true, Ordering::Relaxed);
}

/// Delegating transport that loses the FIRST originate-with-app
/// control response while still placing the real call (ambiguous
/// originate fixture). Every other call is forwarded unchanged.
struct DropFirstOriginate {
    inner: RestAriTransport,
    dropped: AtomicU32,
}

impl DropFirstOriginate {
    fn new(inner: RestAriTransport) -> Self {
        Self {
            inner,
            dropped: AtomicU32::new(0),
        }
    }
}

impl AriTransport for DropFirstOriginate {
    fn health(&self) -> Result<(), CallError> {
        self.inner.health()
    }
    fn list_channels(&self) -> Result<Vec<AriChannel>, CallError> {
        self.inner.list_channels()
    }
    fn channel_state(&self, channel: &ChannelSelector) -> Result<AriChannel, CallError> {
        self.inner.channel_state(channel)
    }
    fn originate(
        &self,
        endpoint: &SipEndpointId,
        context: &str,
        extension: &str,
        caller_id: Option<&str>,
    ) -> Result<AriChannel, CallError> {
        self.inner
            .originate(endpoint, context, extension, caller_id)
    }
    fn originate_with_app(
        &self,
        endpoint: &SipEndpointId,
        app: &str,
        app_args: &str,
        caller_id: Option<&str>,
    ) -> Result<AriChannel, CallError> {
        let result = self
            .inner
            .originate_with_app(endpoint, app, app_args, caller_id);
        if result.is_ok() && self.dropped.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(CallError::unavailable(
                "originate control response lost (fixture)",
            ));
        }
        result
    }
    fn originate_with_app_bounded(
        &self,
        endpoint: &SipEndpointId,
        app: &str,
        app_args: &str,
        caller_id: Option<&str>,
        timeout_secs: u64,
    ) -> Result<AriChannel, CallError> {
        self.inner
            .originate_with_app_bounded(endpoint, app, app_args, caller_id, timeout_secs)
    }
    fn create_bridge(
        &self,
        bridge_type: &str,
        name: &str,
    ) -> Result<nexus_asterisk::transport::AriBridge, CallError> {
        self.inner.create_bridge(bridge_type, name)
    }
    fn get_bridge(
        &self,
        bridge_id: &str,
    ) -> Result<nexus_asterisk::transport::AriBridge, CallError> {
        self.inner.get_bridge(bridge_id)
    }
    fn delete_bridge(&self, bridge_id: &str) -> Result<(), CallError> {
        self.inner.delete_bridge(bridge_id)
    }
    fn add_channel_to_bridge(
        &self,
        bridge_id: &str,
        channel: &ChannelSelector,
    ) -> Result<(), CallError> {
        self.inner.add_channel_to_bridge(bridge_id, channel)
    }
    fn endpoint_state(
        &self,
        resource: &str,
    ) -> Result<nexus_asterisk::transport::AriEndpoint, CallError> {
        self.inner.endpoint_state(resource)
    }
    fn list_bridges(&self) -> Result<Vec<nexus_asterisk::transport::AriBridge>, CallError> {
        self.inner.list_bridges()
    }
    fn answer(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        self.inner.answer(channel)
    }
    fn hangup(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        self.inner.hangup(channel)
    }
    fn bridge(&self, channel: &ChannelSelector, bridge_id: &str) -> Result<(), CallError> {
        self.inner.bridge(channel, bridge_id)
    }
    fn r#continue(
        &self,
        channel: &ChannelSelector,
        context: &str,
        extension: &str,
    ) -> Result<(), CallError> {
        self.inner.r#continue(channel, context, extension)
    }
    fn send_dtmf(&self, channel: &ChannelSelector, digits: &str) -> Result<(), CallError> {
        self.inner.send_dtmf(channel, digits)
    }
    fn start_moh(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        self.inner.start_moh(channel)
    }
    fn stop_moh(&self, channel: &ChannelSelector) -> Result<(), CallError> {
        self.inner.stop_moh(channel)
    }
    fn redirect(
        &self,
        channel: &ChannelSelector,
        context: &str,
        extension: &str,
    ) -> Result<(), CallError> {
        self.inner.redirect(channel, context, extension)
    }
}
