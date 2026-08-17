//! EP-024 M3 appliance integration suite: the REAL nexus-appliances
//! adapter against the REAL pinned Home Assistant container through
//! the EP-020-certified provider boundary (composition, not
//! duplication).
//!
//! Env:
//!   NEXUS_HA_BASE       e.g. http://127.0.0.1:8124 (REQUIRED)
//!   NEXUS_HA_TOKEN      fresh OAuth token minted by the fixture
//!                       bootstrap (REQUIRED; never persisted)
//!   NEXUS_HA_CONTAINER  container name (default nexus-ep024-ha)
//!
//! Two classes:
//!   - `ep024_integration_appliances_probe_*`: read-only proofs that
//!     are safe to run concurrently (provider reachable + real auth,
//!     bad credential fails closed, discovery, stable identity,
//!     unknown entity NotFound).
//!   - `ep024_integration_appliances_journey_live`: ONE sequential
//!     journey owning all stateful phases (switch command + exact
//!     readback, runtime canary fan mode + exact readback, capability
//!     mapping from real features, unsupported capability denied
//!     before provider mutation, wrong-target never verifies, HA
//!     restart preserves mapping/functionality, provider offline maps
//!     honestly, zero secret leakage).
//!
//! All tests are LIVE-STACK (`#[ignore]` convention): the workspace
//! battery stays green without the container; the M3 gate
//! (scripts/ep024-m3-tests.sh) runs them with `--ignored` against the
//! real container, so the proofs remain mandatory.

use std::env;
use std::process::Command;
use std::time::{Duration, Instant};

use nexus_appliances::{
    appliance_device_id, ApplianceAdapter, ApplianceCommand, ApplianceErrorCode, ApplianceSelector,
    ApplianceTransport, HaApplianceTransport,
};
use nexus_devices::ApplianceProvider;

const SWITCH: &str = "input_boolean.nexus_app_switch";
const FAN: &str = "fan.nexus_app_fan";

fn base_url() -> String {
    env::var("NEXUS_HA_BASE")
        .unwrap_or_else(|_| panic!("NEXUS_HA_BASE required (fixture bootstrap sets it)"))
}

fn token() -> String {
    env::var("NEXUS_HA_TOKEN")
        .unwrap_or_else(|_| panic!("NEXUS_HA_TOKEN required (fixture bootstrap sets it)"))
}

fn container_name() -> String {
    env::var("NEXUS_HA_CONTAINER").unwrap_or_else(|_| "nexus-ep024-ha".to_string())
}

fn transport() -> HaApplianceTransport {
    HaApplianceTransport::new(base_url(), token())
}

fn adapter() -> ApplianceAdapter<HaApplianceTransport> {
    ApplianceAdapter::new(
        transport(),
        ApplianceSelector::entities([SWITCH.to_string(), FAN.to_string()]),
    )
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
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn entity_state(transport: &HaApplianceTransport, entity_id: &str) -> String {
    transport
        .read_appliance(entity_id)
        .expect("real read")
        .state
}

/// Bounded wait for the fixture entities to be ACTIVE after a
/// container restart (HA serves /api/ before integrations finish
/// loading; entity readiness is the real signal).
fn wait_entities(adapter: &ApplianceAdapter<HaApplianceTransport>, timeout: u64) {
    let switch_id = appliance_device_id(SWITCH).expect("switch id");
    let fan_id = appliance_device_id(FAN).expect("fan id");
    let deadline = Instant::now() + Duration::from_secs(timeout);
    while Instant::now() < deadline {
        if let Ok(devices) = adapter.list_devices() {
            if devices.contains(&switch_id) && devices.contains(&fan_id) {
                return;
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    panic!("fixture entities did not become active after restart");
}

// ---------------------------------------------------------------------------
// Read-only probe tests (safe to run concurrently)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live HA container (NEXUS_HA_BASE/NEXUS_HA_TOKEN); run via scripts/ep024-m3-tests.sh"]
fn ep024_integration_appliances_probe_provider_reachable_and_authenticated() {
    // 1. real HA provider reachable; 2. real authentication boundary.
    let t = transport();
    assert!(
        t.auth_check().expect("real auth check"),
        "auth_check must succeed with the freshly minted EP-020-certified token"
    );
    let all = t.list_appliances().expect("real /api/states");
    assert!(!all.is_empty(), "real provider returned zero entities");
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m3-tests.sh"]
fn ep024_integration_appliances_probe_bad_credential_fails_closed() {
    // Real bad credential: a bogus token must NOT authenticate. The
    // transport composes through the EP-020 boundary, so this proves
    // the provider boundary rejects invalid credentials.
    let t = HaApplianceTransport::new(base_url(), "ep024-bogus-token");
    assert!(
        !t.auth_check().expect("auth check with bad credential"),
        "bogus token must fail auth_check"
    );
    let err = t.list_appliances().expect_err("must fail");
    assert_ne!(err.code, ApplianceErrorCode::NotFound);
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m3-tests.sh"]
fn ep024_integration_appliances_probe_discovers_fixture_appliances() {
    let adapter = adapter();
    let devices = adapter.list_devices().expect("real discovery");
    let switch = appliance_device_id(SWITCH).expect("switch id");
    let fan = appliance_device_id(FAN).expect("fan id");
    assert!(
        devices.contains(&switch),
        "switch fixture must be discovered: {devices:?}"
    );
    assert!(
        devices.contains(&fan),
        "fan fixture must be discovered: {devices:?}"
    );
    // Internal HA entities are never appliances (explicit allowlist).
    for device in &devices {
        assert!(
            device.as_str() != "sun.sun" && !device.as_str().contains("nexus_app_fan_speed"),
            "non-appliance entity leaked into discovery"
        );
    }
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m3-tests.sh"]
fn ep024_integration_appliances_probe_stable_identity_across_discovery() {
    let adapter = adapter();
    let first = adapter.list_devices().expect("discovery 1");
    let second = adapter.list_devices().expect("discovery 2");
    assert_eq!(first, second, "repeated discovery must be identical");
    // Identity is derived deterministically from the provider entity
    // id (never enumeration index / display name).
    assert_eq!(
        appliance_device_id(SWITCH).expect("switch id").as_str(),
        first
            .iter()
            .find(|d| *d == &appliance_device_id(SWITCH).expect("switch id"))
            .expect("switch present")
            .as_str()
    );
}

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m3-tests.sh"]
fn ep024_integration_appliances_probe_unknown_entity_not_found() {
    // Unknown appliance -> NotFound at the adapter boundary, never a
    // benign device state.
    let adapter = adapter();
    let ghost = appliance_device_id("switch.nexus_ghost").expect("ghost id");
    let err = adapter
        .capabilities(&ghost)
        .expect_err("unknown entity must fail");
    assert_eq!(err.code, nexus_devices::DevicesErrorCode::NotFound);
}

// ---------------------------------------------------------------------------
// Sequential live journey (stateful; the ONLY test that mutates the
// container's fixture state)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires live HA container; run via scripts/ep024-m3-tests.sh"]
fn ep024_integration_appliances_journey_live() {
    let adapter = adapter();
    let switch_id = appliance_device_id(SWITCH).expect("switch id");
    let fan_id = appliance_device_id(FAN).expect("fan id");

    // --- baseline: switch off -------------------------------------------------
    let t = transport();
    assert_eq!(
        entity_state(&t, SWITCH),
        "off",
        "switch baseline must be off"
    );
    let switch_caps = adapter.capabilities(&switch_id).expect("switch caps");
    assert!(switch_caps.contains(&nexus_devices::vocabulary::ApplianceCapability::PowerControl));
    assert!(
        !switch_caps.contains(&nexus_devices::vocabulary::ApplianceCapability::ModeControl),
        "a switch fixture must never advertise fan-speed control"
    );

    // --- 6. switch command + exact readback -----------------------------------
    // off baseline -> turn_on -> independently observe on -> turn_off ->
    // independently observe off. SUBMITTED != VERIFIED.
    let receipt = adapter
        .execute(&switch_id, ApplianceCommand::PowerOn, &switch_caps)
        .expect("switch power on");
    assert_eq!(
        receipt.state,
        nexus_appliances::ApplianceCommandState::Submitted,
        "receipt is SUBMITTED at most, never VERIFIED"
    );
    assert_eq!(
        entity_state(&t, SWITCH),
        "on",
        "independent readback must observe on"
    );
    adapter
        .verify(&switch_id, ApplianceCommand::PowerOn, "ON")
        .expect("switch verified on");
    adapter
        .execute(&switch_id, ApplianceCommand::PowerOff, &switch_caps)
        .expect("switch power off");
    assert_eq!(
        entity_state(&t, SWITCH),
        "off",
        "independent readback must observe off"
    );
    adapter
        .verify(&switch_id, ApplianceCommand::PowerOff, "OFF")
        .expect("switch verified off");

    // --- 7. runtime canary fan mode + exact readback ---------------------------
    let fan_caps = adapter.capabilities(&fan_id).expect("fan caps");
    assert!(fan_caps.contains(&nexus_devices::vocabulary::ApplianceCapability::ModeControl));
    let canary: u8 = 1
        + (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos() as u8)
            % 100;
    let canary = canary.max(1);
    let canary_str = canary.to_string();
    let receipt = adapter
        .execute_mode(&fan_id, &canary_str, &fan_caps)
        .expect("fan set mode");
    assert_eq!(
        receipt.state,
        nexus_appliances::ApplianceCommandState::Submitted
    );
    adapter
        .verify(&fan_id, ApplianceCommand::SetMode, &canary_str)
        .expect("fan mode read back exactly");
    let fan_entity = t.read_appliance(FAN).expect("fan read");
    // The real provider reports percentage as a JSON number (37.0);
    // assert the exact numeric value (integral-float normalization is
    // proven at the adapter/verify layer above).
    assert_eq!(
        fan_entity
            .attributes
            .get("percentage")
            .and_then(serde_json::Value::as_f64),
        Some(f64::from(canary)),
        "the exact runtime-generated percentage must be observed"
    );
    // Fan power surface still works.
    adapter
        .execute(&fan_id, ApplianceCommand::PowerOff, &fan_caps)
        .expect("fan off");

    // --- 5. unsupported capability denied BEFORE provider mutation -------------
    let switch_state_before = entity_state(&t, SWITCH);
    let err = adapter
        .execute(&switch_id, ApplianceCommand::SetMode, &switch_caps)
        .expect_err("switch must not accept SET_MODE");
    assert_eq!(err.code, ApplianceErrorCode::Policy);
    assert_eq!(
        entity_state(&t, SWITCH),
        switch_state_before,
        "denied command must not mutate the provider"
    );

    // --- 8. wrong-target state can never verify --------------------------------
    // Command appliance A (switch) ON; change appliance B (fan) ON
    // independently; B's change cannot satisfy A's VerificationPlan.
    adapter
        .execute(&switch_id, ApplianceCommand::PowerOn, &switch_caps)
        .expect("A on");
    adapter
        .verify(&switch_id, ApplianceCommand::PowerOn, "ON")
        .expect("A verified on");
    let fan_caps = adapter.capabilities(&fan_id).expect("fan caps");
    adapter
        .execute(&fan_id, ApplianceCommand::PowerOn, &fan_caps)
        .expect("B on");
    adapter
        .verify(&fan_id, ApplianceCommand::PowerOn, "ON")
        .expect("B verified on");
    let err = adapter
        .verify(&switch_id, ApplianceCommand::PowerOff, "OFF")
        .expect_err("B's change must not satisfy A's plan");
    assert_eq!(err.code, ApplianceErrorCode::Verification);

    // --- 10. provider restart preserves mapping/functionality ------------------
    docker(&["restart", &container_name()]).expect("docker restart");
    wait_http_up(300).expect("HA ready after restart");
    wait_entities(&adapter, 180);
    // Entity state is NOT claimed persistent (fixture semantics: the
    // template fan resets); identity and function must be.
    let devices_after = adapter.list_devices().expect("rediscovery");
    assert!(
        devices_after.contains(&switch_id) && devices_after.contains(&fan_id),
        "canonical identity must be unchanged after restart"
    );
    let switch_caps_after = adapter.capabilities(&switch_id).expect("switch caps after");
    adapter
        .execute(&switch_id, ApplianceCommand::PowerOn, &switch_caps_after)
        .expect("switch power on after restart");
    assert_eq!(
        entity_state(&t, SWITCH),
        "on",
        "command/readback must work after restart"
    );
    adapter
        .execute(&switch_id, ApplianceCommand::PowerOff, &switch_caps_after)
        .expect("switch power off after restart");

    // --- 11. provider offline maps honestly ------------------------------------
    docker(&["stop", &container_name()]).expect("docker stop");
    // Give the transport a moment to observe the closed connection.
    std::thread::sleep(Duration::from_secs(3));
    let avail = adapter.availability(&switch_id).expect("availability");
    assert_eq!(
        avail,
        nexus_devices::vocabulary::DeviceAvailability::Unavailable,
        "provider offline must map to UNAVAILABLE, never a benign device state"
    );
    docker(&["start", &container_name()]).expect("docker start");
    wait_http_up(300).expect("HA ready after offline phase");

    // --- 12. zero secret leakage ------------------------------------------------
    // An error path must never leak the provider credential.
    let ghost = appliance_device_id("switch.nexus_ghost").expect("ghost id");
    let err = adapter.capabilities(&ghost).expect_err("ghost must fail");
    let token = token();
    assert!(
        !err.message.contains(&token),
        "error path leaked the provider token: {err}"
    );
    assert!(
        !format!("{err:?}").contains(&token),
        "debug output leaked token"
    );
}
