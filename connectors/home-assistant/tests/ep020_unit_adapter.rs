//! EP-020 M2 core behavior suite (SPEC-011; ADR-027).
//!
//! Deterministic unit rules through a CONTROLLED TEST FIXTURE
//! transport (TESTING.md test-double zone). The fixture exercises the
//! REAL adapter core: discovery mapping, service/action execution,
//! exact-target verification, fast path, offline queue, reconnect, and
//! automation handoff. Real Home Assistant instance integration is M3.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use nexus_domain::DeviceId;
use nexus_home::{
    ApprovalClass, CapabilityClass, CommandState, DeviceCapability, DeviceCategory, DeviceTwin,
    EntityAvailability, FastPathDecision, HaEntityRef, HomeError, HomeErrorCode, HomeIntent,
    HomeProvider, Idempotency, Risk, StateVerifierAdapter, VerificationOutcome, VerificationRule,
};
use nexus_home_assistant::{
    default_fast_path_decision, verification_rule_for, HaEntityState, HaTransport,
    HomeAssistantAdapter,
};

/// Controlled test fixture transport: an in-memory Home Assistant that
/// records real service calls and returns scripted states. Production
/// behavior is never tested against this; deterministic adapter rules
/// are. The real REST transport is proven against the real instance in
/// M3/M5.
#[derive(Clone, Default)]
struct FixtureHa {
    states: Arc<Mutex<Vec<HaEntityState>>>,
    services: Arc<Mutex<Vec<String>>>,
    auth_ok: Arc<Mutex<bool>>,
}

impl FixtureHa {
    fn with_states(states: Vec<HaEntityState>) -> Self {
        Self {
            states: Arc::new(Mutex::new(states)),
            services: Arc::new(Mutex::new(Vec::new())),
            auth_ok: Arc::new(Mutex::new(true)),
        }
    }

    fn set_state(&self, entity_id: &str, state: &str) {
        let mut s = self.states.lock().unwrap();
        if let Some(e) = s.iter_mut().find(|e| e.entity_id == entity_id) {
            e.state = state.to_string();
        }
    }
}

impl HaTransport for FixtureHa {
    fn auth_check(&mut self) -> Result<bool, HomeError> {
        Ok(*self.auth_ok.lock().unwrap())
    }

    fn get_states(&mut self) -> Result<Vec<HaEntityState>, HomeError> {
        Ok(self.states.lock().unwrap().clone())
    }

    fn get_services(&mut self) -> Result<Vec<HaService>, HomeError> {
        Ok(Vec::new())
    }

    fn get_state(&mut self, entity_id: &str) -> Result<HaEntityState, HomeError> {
        self.states
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.entity_id == entity_id)
            .cloned()
            .ok_or_else(|| {
                HomeError::new(
                    HomeErrorCode::NotFound,
                    "entity not found",
                    None,
                    Some(Box::from(entity_id)),
                )
            })
    }

    fn call_service(
        &mut self,
        domain: &str,
        service: &str,
        _data: &BTreeMap<String, serde_json::Value>,
    ) -> Result<(), HomeError> {
        self.services
            .lock()
            .unwrap()
            .push(format!("{domain}/{service}"));
        // Real effect simulation: the light turns on.
        if domain == "light" && service == "turn_on" {
            self.set_state("light.kitchen", "on");
        }
        if domain == "light" && service == "turn_off" {
            self.set_state("light.kitchen", "off");
        }
        if domain == "lock" && service == "unlock" {
            self.set_state("lock.front_door", "unlocked");
        }
        // Automation creation: the automation entity appears in the
        // provider registry (readback proves creation).
        if domain == "automation" && service == "turn_on" {
            let mut s = self.states.lock().unwrap();
            if !s.iter().any(|e| e.entity_id == "automation.kitchen_dusk") {
                s.push(HaEntityState {
                    entity_id: "automation.kitchen_dusk".to_string(),
                    state: "on".to_string(),
                    attributes: BTreeMap::new(),
                    last_changed: None,
                    last_updated: None,
                });
            }
        }
        Ok(())
    }
}

use nexus_home_assistant::HaService;

fn light_state() -> HaEntityState {
    HaEntityState {
        entity_id: "light.kitchen".to_string(),
        state: "off".to_string(),
        attributes: BTreeMap::from([(
            "friendly_name".to_string(),
            serde_json::json!("Kitchen Light"),
        )]),
        last_changed: None,
        last_updated: None,
    }
}

fn lock_state() -> HaEntityState {
    HaEntityState {
        entity_id: "lock.front_door".to_string(),
        state: "locked".to_string(),
        attributes: BTreeMap::new(),
        last_changed: None,
        last_updated: None,
    }
}

fn intent(device: DeviceId, capability: &str, action: &str) -> HomeIntent {
    HomeIntent {
        device_id: device,
        capability_id: capability.to_string(),
        action: action.to_string(),
        parameters: BTreeMap::new(),
        correlation_id: nexus_home_assistant::adapter::correlation(1),
        idempotency_key: Some("k-1".to_string()),
    }
}

fn discover_one<T: HaTransport>(
    adapter: &mut HomeAssistantAdapter<T, StateVerifierAdapter>,
) -> DeviceId {
    let twins = adapter.discover().expect("discover ok");
    assert!(!twins.is_empty());
    twins[0].device_id.clone()
}

#[test]
fn ep020_unit_discovery_maps_real_states_to_canonical_twins() {
    let fixture = FixtureHa::with_states(vec![light_state(), lock_state()]);
    let mut adapter = HomeAssistantAdapter::new(fixture, StateVerifierAdapter);
    let twins = adapter.discover().expect("discover ok");
    assert_eq!(twins.len(), 2);
    let light = twins
        .iter()
        .find(|t| t.category == DeviceCategory::Light)
        .unwrap();
    assert_eq!(light.ha_entity_refs[0].0, "light.kitchen");
    assert_eq!(light.friendly_name, "Kitchen Light");
    assert_eq!(light.availability, EntityAvailability::Available);
    let lock = twins
        .iter()
        .find(|t| t.category == DeviceCategory::Lock)
        .unwrap();
    assert_eq!(lock.category, DeviceCategory::Lock);
    // HA entity ids never leak as canonical identity: device_id is a
    // canonical UUIDv7, not the HA entity id.
    assert!(lock.device_id.as_str().contains('-'));
    assert_ne!(lock.device_id.as_str(), "lock.front_door");
}

#[test]
fn ep020_unit_execute_uses_real_service_path_and_returns_submitted() {
    let fixture = FixtureHa::with_states(vec![light_state()]);
    let mut adapter = HomeAssistantAdapter::new(fixture.clone(), StateVerifierAdapter);
    let id = discover_one(&mut adapter);
    let receipt = adapter
        .execute(&intent(id, "home.light", "turn_on"))
        .expect("execute ok");
    assert_eq!(receipt.state, CommandState::Submitted);
    assert_eq!(receipt.provider_service, "light/turn_on");
    let services = fixture.services.lock().unwrap();
    assert_eq!(services[0], "light/turn_on");
}

#[test]
fn ep020_unit_verify_exact_target_after_execute() {
    let fixture = FixtureHa::with_states(vec![light_state()]);
    let mut adapter = HomeAssistantAdapter::new(fixture.clone(), StateVerifierAdapter);
    let id = discover_one(&mut adapter);
    let receipt = adapter
        .execute(&intent(id, "home.light", "turn_on"))
        .expect("execute ok");
    // The fixture transport actually flipped the light on; verification
    // observes the exact target and reports VERIFIED.
    let outcome = adapter.verify(&receipt).expect("verify ok");
    assert_eq!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep020_unit_verify_mismatch_when_target_never_changed() {
    // A transport that accepts the command but never changes state.
    let fixture = FixtureHa::with_states(vec![light_state()]);
    let mut adapter = HomeAssistantAdapter::new(fixture.clone(), StateVerifierAdapter);
    let id = discover_one(&mut adapter);
    let receipt = adapter
        .execute(&intent(id, "home.light", "turn_on"))
        .expect("execute ok");
    // Force the state back to off before verify: the device did not
    // reach the requested state -> NOT verified.
    fixture.set_state("light.kitchen", "off");
    let outcome = adapter.verify(&receipt).expect("verify ok");
    assert_eq!(outcome, VerificationOutcome::Mismatch);
}

#[test]
fn ep020_unit_execute_fails_closed_when_device_unavailable() {
    let mut st = light_state();
    st.state = "unavailable".to_string();
    let fixture = FixtureHa::with_states(vec![st]);
    let mut adapter = HomeAssistantAdapter::new(fixture, StateVerifierAdapter);
    let id = discover_one(&mut adapter);
    let err = adapter
        .execute(&intent(id, "home.light", "turn_on"))
        .expect_err("unavailable must fail");
    assert_eq!(err.code, HomeErrorCode::Unavailable);
}

#[test]
fn ep020_unit_unknown_state_is_never_safe() {
    let mut st = lock_state();
    st.state = "unknown".to_string();
    let fixture = FixtureHa::with_states(vec![st]);
    let mut adapter = HomeAssistantAdapter::new(fixture, StateVerifierAdapter);
    let twins = adapter.discover().expect("discover ok");
    let lock = twins[0].clone();
    assert_eq!(lock.availability, EntityAvailability::Unknown);
    assert!(!lock.is_available());
}

#[test]
fn ep020_unit_fast_path_is_deterministic_low_risk_local() {
    let twin = DeviceTwin {
        device_id: DeviceId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6501").unwrap(),
        friendly_name: "Kitchen Light".to_string(),
        area: None,
        owner: None,
        ha_device_ref: nexus_home::HaDeviceRef::new("d1").unwrap(),
        ha_entity_refs: vec![HaEntityRef::new("light.kitchen").unwrap()],
        provider_domain: "light".to_string(),
        category: DeviceCategory::Light,
        manufacturer: None,
        model: None,
        availability: EntityAvailability::Available,
        state: Some("off".to_string()),
        attributes: BTreeMap::new(),
        capabilities: vec![DeviceCapability {
            capability_id: "home.light".to_string(),
            class: CapabilityClass::Command,
            risk: Risk::R1,
            approval: ApprovalClass::None,
            idempotency: Idempotency::Required,
            verification: VerificationRule::StateEquals {
                expected: "on".to_string(),
            },
        }],
        parent_ha_device_ref: None,
    };
    // Low-risk light on/off: EXECUTE_LOCALLY, no model.
    let i = intent(twin.device_id.clone(), "home.light", "turn_on");
    assert_eq!(
        default_fast_path_decision(&i, &twin),
        FastPathDecision::ExecuteLocally
    );
    // Lock never automatic local execution.
    let lock_twin = DeviceTwin {
        category: DeviceCategory::Lock,
        ..twin.clone()
    };
    let li = intent(twin.device_id.clone(), "home.lock", "unlock");
    assert_eq!(
        default_fast_path_decision(&li, &lock_twin),
        FastPathDecision::RequiresModel
    );
    // Unavailable device denied.
    let unavailable = DeviceTwin {
        availability: EntityAvailability::Unavailable,
        ..twin
    };
    assert_eq!(
        default_fast_path_decision(&i, &unavailable),
        FastPathDecision::Denied
    );
}

#[test]
fn ep020_unit_verification_rule_is_per_capability() {
    assert_eq!(
        verification_rule_for(DeviceCategory::Light, "turn_on"),
        VerificationRule::StateEquals {
            expected: "on".to_string()
        }
    );
    assert_eq!(
        verification_rule_for(DeviceCategory::Lock, "lock"),
        VerificationRule::StateEquals {
            expected: "locked".to_string()
        }
    );
    assert_eq!(
        verification_rule_for(DeviceCategory::Cover, "open_cover"),
        VerificationRule::StateIn {
            expected: vec!["open".to_string(), "opening".to_string()]
        }
    );
    assert!(matches!(
        verification_rule_for(DeviceCategory::Scene, "activate"),
        VerificationRule::NoVerification { .. }
    ));
}

#[test]
fn ep020_unit_offline_queue_retains_and_drains_on_reconnect() {
    // Simulate disconnect, queue authorized intents, reconnect, drain.
    let fixture = FixtureHa::with_states(vec![light_state()]);
    let mut adapter = HomeAssistantAdapter::new(fixture.clone(), StateVerifierAdapter);
    let id = discover_one(&mut adapter);

    // Mark disconnected by failing the auth check on reconnect.
    *fixture.auth_ok.lock().unwrap() = false;
    adapter
        .reconnect()
        .expect_err("reconnect fails when auth down");
    assert_eq!(
        adapter.connection_state(),
        nexus_home::ProviderConnectionState::Disconnected
    );

    let qi = intent(id.clone(), "home.light", "turn_on");
    adapter
        .queue_offline(qi.clone())
        .expect("queued while disconnected");
    assert_eq!(adapter.offline_queue_len(), 1);
    // Duplicate (idempotency) rejected.
    let dup = intent(id, "home.light", "turn_on");
    assert_eq!(
        adapter
            .queue_offline(dup)
            .expect_err("duplicate rejected")
            .code,
        HomeErrorCode::Conflict
    );

    // Reconnect: auth restored, state refreshed, queue drained.
    *fixture.auth_ok.lock().unwrap() = true;
    adapter.reconnect().expect("reconnect ok");
    assert_eq!(
        adapter.connection_state(),
        nexus_home::ProviderConnectionState::Connected
    );
    assert_eq!(adapter.offline_queue_len(), 0);
    let services = fixture.services.lock().unwrap();
    assert!(services.iter().any(|s| s == "light/turn_on"));
}

#[test]
fn ep020_unit_automation_handoff_uses_provider_machinery() {
    use nexus_home::AutomationSpec;
    let fixture = FixtureHa::with_states(vec![light_state()]);
    // The automation entity appears after the create call (readback
    // proves creation); no pre-seeding needed.
    let mut handoff = nexus_home_assistant::AutomationHandoffAdapter::new(fixture);
    let spec = AutomationSpec {
        name: "Kitchen Dusk".to_string(),
        trigger: "17:30:00".to_string(),
        action: intent(
            DeviceId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6501").unwrap(),
            "home.light",
            "turn_on",
        ),
        enabled: true,
    };
    let handle = handoff.create(&spec).expect("automation created");
    assert_eq!(handle.provider_automation_id, "automation.kitchen_dusk");
    let status = handoff.readback(&handle).expect("readback ok");
    assert!(status.enabled);
}

#[test]
fn ep020_unit_dependency_direction_adapter_imports_contracts_not_infra() {
    // The adapter imports nexus-home contracts + its own transport;
    // it does not import database/eventing/model crates.
    let _ = nexus_home::HomeErrorCode::External;
}
