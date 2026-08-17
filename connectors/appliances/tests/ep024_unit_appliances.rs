//! EP-024 M3 appliance unit suite: deterministic adapter rules against
//! a controlled fixture transport (TESTING.md test-double zone).
//!
//! The REAL provider integration suite is ep024_integration_appliances.rs
//! (live Home Assistant container); this suite proves the adapter's
//! deterministic invariants: stable identity, capability mapping from
//! real features, availability truth table, capability-gated dispatch
//! (no provider mutation for unsupported commands), SUBMITTED-never-
//! VERIFIED, exact-target verification, and robot-authority isolation.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use nexus_appliances::{
    appliance_device_id, stable_appliance_id, ApplianceAdapter, ApplianceCommand,
    ApplianceCommandState, ApplianceEntity, ApplianceError, ApplianceErrorCode, ApplianceSelector,
    ApplianceTransport,
};
use nexus_devices::mapper::DeviceCapabilityMapper;
use nexus_devices::verifier::VerificationOutcome;
use nexus_devices::vocabulary::{ApplianceCapability, ApplianceDeviceId, DeviceAvailability};
use nexus_devices::ApplianceProvider;

const SWITCH: &str = "input_boolean.nexus_app_switch";
const FAN: &str = "fan.nexus_app_fan";

fn entity(entity_id: &str, state: &str) -> ApplianceEntity {
    let domain = entity_id.split('.').next().unwrap_or_default().to_string();
    ApplianceEntity {
        entity_id: entity_id.to_string(),
        domain,
        state: state.to_string(),
        attributes: BTreeMap::new(),
    }
}

/// Controlled fixture transport: an in-memory entity registry whose
/// state changes only when `invoke` is called (test-double zone). The
/// test keeps an Arc clone to inspect invocation counts and state, so
/// it can prove unsupported commands NEVER reached the provider.
#[derive(Clone, Default)]
struct FixtureTransport {
    entities: Arc<Mutex<Vec<ApplianceEntity>>>,
    invocations: Arc<Mutex<Vec<String>>>,
    fail_next_read: Arc<Mutex<Option<ApplianceErrorCode>>>,
}

impl FixtureTransport {
    fn new(entities: Vec<ApplianceEntity>) -> Self {
        Self {
            entities: Arc::new(Mutex::new(entities)),
            invocations: Arc::new(Mutex::new(Vec::new())),
            fail_next_read: Arc::new(Mutex::new(None)),
        }
    }

    fn invocation_count(&self) -> usize {
        self.invocations.lock().expect("invocations lock").len()
    }

    fn fail_reads_with(&self, code: ApplianceErrorCode) {
        *self.fail_next_read.lock().expect("fail lock") = Some(code);
    }

    fn state_of(&self, entity_id: &str) -> String {
        self.entities
            .lock()
            .expect("entities lock")
            .iter()
            .find(|e| e.entity_id == entity_id)
            .map(|e| e.state.clone())
            .unwrap_or_default()
    }
}

impl ApplianceTransport for FixtureTransport {
    fn list_appliances(&self) -> Result<Vec<ApplianceEntity>, ApplianceError> {
        if let Some(code) = self.fail_next_read.lock().expect("fail lock").take() {
            return Err(ApplianceError::new(
                code,
                "fixture read failure",
                None,
                None,
            ));
        }
        Ok(self.entities.lock().expect("entities lock").clone())
    }

    fn read_appliance(&self, entity_id: &str) -> Result<ApplianceEntity, ApplianceError> {
        if let Some(code) = self.fail_next_read.lock().expect("fail lock").take() {
            return Err(ApplianceError::new(
                code,
                "fixture read failure",
                None,
                None,
            ));
        }
        self.entities
            .lock()
            .expect("entities lock")
            .iter()
            .find(|e| e.entity_id == entity_id)
            .cloned()
            .ok_or_else(|| ApplianceError::not_found(format!("fixture entity {entity_id} absent")))
    }

    fn invoke(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), ApplianceError> {
        let mut entities = self.entities.lock().expect("entities lock");
        let index = entities
            .iter()
            .position(|e| e.entity_id == entity_id)
            .ok_or_else(|| {
                ApplianceError::not_found(format!("fixture entity {entity_id} absent"))
            })?;
        self.invocations
            .lock()
            .expect("invocations lock")
            .push(format!("{domain}.{service}:{entity_id}:{data:?}"));
        // Real mutation semantics: turn_on -> state on; turn_off ->
        // state off; set_percentage -> attributes.percentage.
        match service {
            "turn_on" => {
                entities[index].state = "on".to_string();
            }
            "turn_off" => {
                entities[index].state = "off".to_string();
            }
            "set_percentage" => {
                entities[index].attributes.insert(
                    "percentage".to_string(),
                    data.get("percentage").cloned().unwrap_or(Value::from(0)),
                );
            }
            _ => {
                return Err(ApplianceError::new(
                    ApplianceErrorCode::External,
                    format!("fixture unknown service {domain}.{service}"),
                    None,
                    None,
                ));
            }
        }
        Ok(())
    }
}

fn switch_device() -> ApplianceDeviceId {
    appliance_device_id(SWITCH).expect("switch canonical id")
}

fn fan_device() -> ApplianceDeviceId {
    appliance_device_id(FAN).expect("fan canonical id")
}

fn adapter_with(transport: FixtureTransport) -> ApplianceAdapter<FixtureTransport> {
    ApplianceAdapter::new(
        transport,
        ApplianceSelector::entities([SWITCH.to_string(), FAN.to_string()]),
    )
}

#[test]
fn ep024_unit_appliance_discovery_stable_identity_order_free() {
    let transport = FixtureTransport::new(vec![entity(FAN, "off"), entity(SWITCH, "off")]);
    let adapter = adapter_with(transport);

    let first = adapter.list_devices().expect("discovery");
    assert_eq!(first.len(), 2);
    assert!(first.contains(&fan_device()));
    assert!(first.contains(&switch_device()));

    // Ordering changes must not change canonical identity.
    let transport = FixtureTransport::new(vec![entity(SWITCH, "off"), entity(FAN, "off")]);
    let adapter = adapter_with(transport);
    let second = adapter.list_devices().expect("discovery");
    assert_eq!(second.len(), 2);
    assert!(second.contains(&fan_device()));
    assert!(second.contains(&switch_device()));

    // Enumeration index is never identity: the canonical id derives
    // deterministically from the provider entity id.
    assert_eq!(stable_appliance_id(FAN), fan_device().as_str());
}

#[test]
fn ep024_unit_appliance_capabilities_from_real_features() {
    let transport = FixtureTransport::new(vec![entity(SWITCH, "off"), entity(FAN, "on")]);
    let adapter = adapter_with(transport);

    let switch_caps = adapter.capabilities(&switch_device()).expect("switch caps");
    assert!(switch_caps.contains(&ApplianceCapability::PowerControl));
    assert!(!switch_caps.contains(&ApplianceCapability::ModeControl));
    assert!(switch_caps.contains(&ApplianceCapability::StatusReadback));

    // The real template fan exposes a percentage attribute when on;
    // capability mapping derives ModeControl from that real feature.
    let mut fan = entity(FAN, "on");
    fan.attributes
        .insert("percentage".to_string(), Value::from(50));
    let transport = FixtureTransport::new(vec![fan]);
    let adapter = adapter_with(transport);
    let fan_caps = adapter.capabilities(&fan_device()).expect("fan caps");
    assert!(fan_caps.contains(&ApplianceCapability::PowerControl));
    assert!(fan_caps.contains(&ApplianceCapability::ModeControl));
    assert!(fan_caps.contains(&ApplianceCapability::StatusReadback));
}

#[test]
fn ep024_unit_appliance_switch_command_submitted_never_verified() {
    let transport = FixtureTransport::new(vec![entity(SWITCH, "off")]);
    let adapter = adapter_with(transport);
    let caps = adapter.capabilities(&switch_device()).expect("caps");

    let receipt = adapter
        .execute(&switch_device(), ApplianceCommand::PowerOn, &caps)
        .expect("power on");
    assert_eq!(receipt.state, ApplianceCommandState::Submitted);
    assert_ne!(receipt.state, ApplianceCommandState::Verified);
    assert_eq!(receipt.device, switch_device().as_str());

    // Exact-target readback verifies the real mutation.
    let outcome = adapter
        .verify(&switch_device(), ApplianceCommand::PowerOn, "ON")
        .expect("verify");
    assert_eq!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep024_unit_appliance_exact_target_wrong_device_never_verifies() {
    use nexus_devices::verifier::{DeviceCommandVerifier, DeviceStateObservation};

    let transport = FixtureTransport::new(vec![entity(SWITCH, "off"), entity(FAN, "off")]);
    let adapter = adapter_with(transport);
    let caps = adapter.capabilities(&switch_device()).expect("caps");

    // Command appliance A (switch) ON...
    adapter
        .execute(&switch_device(), ApplianceCommand::PowerOn, &caps)
        .expect("power on");
    adapter
        .verify(&switch_device(), ApplianceCommand::PowerOn, "ON")
        .expect("A verified");

    // ...then change appliance B (fan) independently ON.
    let fan_caps = adapter.capabilities(&fan_device()).expect("fan caps");
    adapter
        .execute(&fan_device(), ApplianceCommand::PowerOn, &fan_caps)
        .expect("fan on");

    // B's change cannot satisfy A's VerificationPlan: A is still ON,
    // so verifying A as OFF is MISMATCH (the fan's ON never helps).
    let err = adapter
        .verify(&switch_device(), ApplianceCommand::PowerOff, "OFF")
        .expect_err("B's change must not satisfy A's plan");
    assert_eq!(err.code, ApplianceErrorCode::Verification);

    // Verifier-level invariant: an observation recorded from device B
    // (even with the desired state) can never verify target A.
    let verifier = DeviceCommandVerifier;
    let observation = DeviceStateObservation {
        device: fan_device().as_str().to_string(),
        state: Some("ON".to_string()),
    };
    let outcome = verifier.verify(switch_device().as_str(), "ON", &observation);
    assert_eq!(outcome, VerificationOutcome::UnrelatedChange);
}

#[test]
fn ep024_unit_appliance_unsupported_command_fails_closed_before_provider() {
    let transport = FixtureTransport::new(vec![entity(SWITCH, "off")]);
    let adapter = adapter_with(transport.clone());
    let caps = adapter.capabilities(&switch_device()).expect("caps");
    // The switch fixture has no mode surface: SET_MODE must be refused
    // with Policy BEFORE any provider service call.
    let err = adapter
        .execute(&switch_device(), ApplianceCommand::SetMode, &caps)
        .expect_err("switch must not accept fan-speed commands");
    assert_eq!(err.code, ApplianceErrorCode::Policy);

    // No provider mutation happened, and no invocation reached the
    // transport.
    assert_eq!(transport.invocation_count(), 0);
    assert_eq!(transport.state_of(SWITCH), "off");
}

#[test]
fn ep024_unit_appliance_availability_truth_table() {
    // Present + usable -> AVAILABLE.
    let transport = FixtureTransport::new(vec![entity(SWITCH, "off")]);
    let adapter = adapter_with(transport.clone());
    assert_eq!(
        adapter.availability(&switch_device()).expect("avail"),
        DeviceAvailability::Available
    );

    // Provider-unavailable entity -> UNAVAILABLE (never OFF).
    let transport = FixtureTransport::new(vec![entity(SWITCH, "unavailable")]);
    let adapter = adapter_with(transport);
    assert_eq!(
        adapter.availability(&switch_device()).expect("avail"),
        DeviceAvailability::Unavailable
    );

    // "unknown" state (observed: template entities before first
    // actuation) is present + usable -> AVAILABLE; its state is never
    // claimed as OFF but the device is not down.
    let transport = FixtureTransport::new(vec![entity(SWITCH, "unknown")]);
    let adapter = adapter_with(transport);
    assert_eq!(
        adapter.availability(&switch_device()).expect("avail"),
        DeviceAvailability::Available
    );

    // Unknown entity -> NotFound, never a benign state.
    let transport = FixtureTransport::new(vec![entity(SWITCH, "off")]);
    let adapter = adapter_with(transport);
    let unknown = appliance_device_id("fan.nexus_ghost").expect("id");
    let err = adapter.availability(&unknown).expect_err("unknown");
    assert_eq!(err.code, ApplianceErrorCode::NotFound);

    // Provider offline -> UNAVAILABLE.
    let transport = FixtureTransport::new(vec![entity(SWITCH, "off")]);
    let adapter = adapter_with(transport.clone());
    transport.fail_reads_with(ApplianceErrorCode::Unavailable);
    assert_eq!(
        adapter.availability(&switch_device()).expect("avail"),
        DeviceAvailability::Unavailable
    );
}

#[test]
fn ep024_unit_appliance_mapper_uses_ep010_taxonomy() {
    let mapper = DeviceCapabilityMapper;
    let power = mapper.map("appliance.power").expect("power");
    assert_eq!(power.class.as_str(), "COMMAND");
    assert_eq!(power.risk.as_str(), "R1");
    assert_eq!(power.approval.as_str(), "NONE");
    assert_eq!(power.idempotency.as_str(), "REQUIRED");

    let mode = mapper.map("appliance.mode").expect("mode");
    assert_eq!(mode.class.as_str(), "COMMAND");
    assert_eq!(mode.risk.as_str(), "R1");

    let status = mapper.map("appliance.status").expect("status");
    assert_eq!(status.class.as_str(), "QUERY");
    assert_eq!(status.risk.as_str(), "R0");

    let unknown = mapper.map("appliance.self_destruct").expect_err("reject");
    assert_eq!(unknown.code, nexus_devices::DevicesErrorCode::Vocabulary);
}

#[test]
fn ep024_unit_appliance_discovery_never_manufactures_robot_authority() {
    // The appliance adapter discovers only the configured appliance
    // entities; the robot id space is never reachable from this
    // connector. This regression proves the M1/M2 invariant that other
    // device classes cannot widen robotics authority.
    let transport = FixtureTransport::new(vec![
        entity(SWITCH, "off"),
        entity(FAN, "off"),
        entity("robot.nexus_cleaner", "on"),
    ]);
    let adapter = adapter_with(transport);
    let devices = adapter.list_devices().expect("discovery");
    // The robot entity is NOT selected (explicit allowlist) and the
    // adapter never fabricates robot devices.
    assert_eq!(devices.len(), 2);
    assert!(devices.contains(&switch_device()));
    assert!(devices.contains(&fan_device()));
    for device in &devices {
        assert!(
            !device.as_str().contains("robot"),
            "robot id space must not appear in appliance discovery"
        );
    }
}

#[test]
fn ep024_unit_appliance_unknown_target_read_fails_closed() {
    let transport = FixtureTransport::new(vec![entity(SWITCH, "off")]);
    let adapter = adapter_with(transport);
    let ghost = appliance_device_id("fan.nexus_ghost").expect("id");
    let err = adapter
        .execute(
            &ghost,
            ApplianceCommand::PowerOn,
            &[ApplianceCapability::PowerControl],
        )
        .expect_err("unknown target");
    assert_eq!(err.code, ApplianceErrorCode::NotFound);
}
