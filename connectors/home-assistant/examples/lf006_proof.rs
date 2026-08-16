//! LF-006 deterministic-home-control (EP-020 M5; SPEC-011; ADR-027).
//!
//! Real proof against the REAL Home Assistant container: the production
//! `nexus-home-assistant` adapter drives a live instance end to end.
//!   - real authentication (RestTransport auth_check);
//!   - real discovery -> canonical DeviceTwin for the CONTROLLED
//!     TEST_FIXTURE entity `light.nexus_test_light`;
//!   - deterministic fast path decision EXECUTE_LOCALLY - the proof
//!     process never constructs a model provider, so "no model call
//!     occurred" is structural: the command path is deterministic
//!     (SPEC-011 fast path) and there is no model surface in process;
//!   - real service/action execution (`light.turn_on` through the REST
//!     transport) -> CommandReceipt SUBMITTED at most
//!     (COMMAND ACCEPTED != DEVICE VERIFIED);
//!   - exact-target verification via fresh readback -> VERIFIED;
//!   - an audit event exists: the driver observes the real HA
//!     `state_changed` event for the exact target on the WebSocket.
//!
//! Evidence JSON is printed to stdout; the Python driver
//! (tests/home/test_ep020_livefire.py) asserts every field.

use std::collections::BTreeMap;
use std::env;

use nexus_home::{
    CommandState, DeviceCategory, EntityAvailability, FastPathDecision, HaEntityRef, HomeIntent,
    HomeProvider, StateVerifierAdapter, VerificationOutcome,
};
use nexus_home_assistant::{
    default_fast_path_decision, HaTransport, HomeAssistantAdapter, RestTransport,
};

const LIGHT_ENTITY: &str = "light.nexus_test_light";

fn correlation(n: u8) -> nexus_domain::CorrelationId {
    nexus_domain::CorrelationId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7{n:03}"))
        .expect("valid UUIDv7")
}

fn main() {
    let base = env::var("NEXUS_HA_BASE").unwrap_or_else(|_| "http://127.0.0.1:8123".to_string());
    let token = env::var("NEXUS_HA_TOKEN").expect("NEXUS_HA_TOKEN required");
    let result = run(&base, &token);
    match result {
        Ok(evidence) => println!("{}", evidence),
        Err(e) => {
            eprintln!("LF-006 FAIL: {e}");
            std::process::exit(1);
        }
    }
}

fn run(base: &str, token: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 1. REAL authentication.
    let mut transport = RestTransport::new(base, token);
    let auth_ok = transport
        .auth_check()
        .map_err(|e| format!("auth_check failed: {e}"))?;
    assert!(auth_ok, "authentication rejected");

    let mut adapter = HomeAssistantAdapter::new(transport, StateVerifierAdapter);

    // 2. REAL discovery -> canonical twin.
    let twins = adapter
        .discover()
        .map_err(|e| format!("discover failed: {e}"))?;
    let light = twins
        .iter()
        .find(|t| t.ha_entity_refs.iter().any(|r| r.0 == LIGHT_ENTITY))
        .ok_or("fixture light not discovered")?;
    assert_eq!(light.category, DeviceCategory::Light);
    assert_eq!(light.availability, EntityAvailability::Available);
    assert!(light.is_available());
    let device_id = light.device_id.clone();

    // 3. Canonical intent (low-risk light command).
    let mut parameters = BTreeMap::new();
    parameters.insert("entity_id".to_string(), serde_json::json!(LIGHT_ENTITY));
    let intent = HomeIntent {
        device_id: device_id.clone(),
        capability_id: "home.light".to_string(),
        action: "turn_on".to_string(),
        parameters,
        correlation_id: correlation(1),
        idempotency_key: Some("lf006-turn-on-1".to_string()),
    };

    // 4. Deterministic fast path: EXECUTE_LOCALLY, no model call. The
    // proof process never constructs a model provider; the decision is
    // deterministic from policy + twin registry alone.
    let decision = default_fast_path_decision(&intent, light);
    assert_eq!(decision, FastPathDecision::ExecuteLocally);

    // 5. REAL service/action execution -> SUBMITTED at most.
    let receipt = adapter
        .execute(&intent)
        .map_err(|e| format!("execute failed: {e}"))?;
    assert_eq!(receipt.state, CommandState::Submitted);
    assert_eq!(receipt.target_ha_entity, HaEntityRef::new(LIGHT_ENTITY)?);
    assert_eq!(receipt.provider_service, "light/turn_on");
    assert_ne!(receipt.state, CommandState::Verified);

    // 6. Exact-target verification via fresh readback -> VERIFIED.
    // The template light re-renders asynchronously after the service
    // call is accepted; verification uses a bounded window (same as the
    // M3 integration suite), never a fabricated single-shot pass.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut outcome = adapter
        .verify(&receipt)
        .map_err(|e| format!("verify failed: {e}"))?;
    while outcome != VerificationOutcome::Verified && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(500));
        outcome = adapter
            .verify(&receipt)
            .map_err(|e| format!("verify failed: {e}"))?;
    }
    assert_eq!(outcome, VerificationOutcome::Verified);

    // Cleanup: turn the fixture light back off through the real path.
    let mut off_params = BTreeMap::new();
    off_params.insert("entity_id".to_string(), serde_json::json!(LIGHT_ENTITY));
    let off_intent = HomeIntent {
        device_id,
        capability_id: "home.light".to_string(),
        action: "turn_off".to_string(),
        parameters: off_params,
        correlation_id: correlation(2),
        idempotency_key: Some("lf006-turn-off-1".to_string()),
    };
    let _ = adapter.execute(&off_intent);

    // Evidence JSON (driver asserts each field).
    Ok(format!(
        "{{\"proof\":\"LF-006\",\"auth\":true,\"discovered\":true,\"entity\":\"{}\",\"category\":\"{}\",\"fast_path\":\"{}\",\"no_model_call\":true,\"receipt_state\":\"{}\",\"verification\":\"{}\",\"target_entity\":\"{}\",\"provider_service\":\"{}\"}}",
        LIGHT_ENTITY,
        DeviceCategory::Light.as_str(),
        decision.as_str(),
        receipt.state.as_str(),
        outcome.as_str(),
        receipt.target_ha_entity.0,
        receipt.provider_service,
    ))
}
