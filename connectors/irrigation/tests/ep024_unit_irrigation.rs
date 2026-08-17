//! EP-024 M4 irrigation unit suite: deterministic adapter rules
//! against a controlled fixture transport (TESTING.md test-double
//! zone).
//!
//! The REAL provider failure suite is ep024_failure_irrigation.rs
//! (live Home Assistant container). This suite proves the adapter's
//! deterministic invariants: stable zone identity, capability mapping
//! from real features, availability truth table, capability-gated
//! dispatch (no provider mutation for unsupported commands),
//! SUBMITTED-never-VERIFIED, exact-target verification, in-flight
//! idempotency (real concurrent duplicate -> Conflict), bounded
//! recovery, and observability (counters, redacted audit, correlation).

use std::collections::BTreeMap;
use std::sync::{Arc, Barrier, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use nexus_devices::mapper::DeviceCapabilityMapper;
use nexus_devices::verifier::VerificationOutcome;
use nexus_devices::vocabulary::{DeviceAvailability, IrrigationCapability, IrrigationZoneId};
use nexus_devices::IrrigationProvider;
use nexus_irrigation::{
    irrigation_zone_id, stable_zone_id, zone_state_value, IrrigationAdapter, IrrigationCommand,
    IrrigationCommandState, IrrigationError, IrrigationErrorCode, IrrigationTransport,
    IrrigationZone, IrrigationZoneSelector,
};

const ZONE_A: &str = "input_boolean.nexus_zone_a";
const ZONE_B: &str = "input_boolean.nexus_zone_b";

fn zone(entity_id: &str, state: &str) -> IrrigationZone {
    let domain = entity_id.split('.').next().unwrap_or_default().to_string();
    IrrigationZone {
        entity_id: entity_id.to_string(),
        domain,
        state: state.to_string(),
        attributes: BTreeMap::new(),
    }
}

/// Gate state shared with the provider invoke: `entered` is set the
/// instant invoke begins (bounded synchronization for the concurrency
/// proof); `blocked` parks invoke on demand.
#[derive(Default)]
struct GateState {
    blocked: bool,
    entered: bool,
}

/// Controlled fixture transport: an in-memory zone registry whose state
/// changes only when `invoke` is called. The test keeps an Arc clone to
/// inspect invocations and state, and to prove unsupported commands
/// NEVER reached the provider. A Condvar gate blocks invoke on demand
/// (real concurrency proof for the in-flight idempotency guard).
#[derive(Clone, Default)]
struct FixtureTransport {
    zones: Arc<Mutex<Vec<IrrigationZone>>>,
    invocations: Arc<Mutex<Vec<String>>>,
    fail_next_read: Arc<Mutex<Option<IrrigationErrorCode>>>,
    gate: Arc<(Mutex<GateState>, Condvar)>,
}

impl FixtureTransport {
    fn new(zones: Vec<IrrigationZone>) -> Self {
        Self {
            zones: Arc::new(Mutex::new(zones)),
            invocations: Arc::new(Mutex::new(Vec::new())),
            fail_next_read: Arc::new(Mutex::new(None)),
            gate: Arc::new((Mutex::new(GateState::default()), Condvar::new())),
        }
    }

    fn invocation_count(&self) -> usize {
        self.invocations.lock().expect("invocations lock").len()
    }

    fn state_of(&self, entity_id: &str) -> String {
        self.zones
            .lock()
            .expect("zones lock")
            .iter()
            .find(|z| z.entity_id == entity_id)
            .map(|z| z.state.clone())
            .unwrap_or_default()
    }

    fn fail_reads_with(&self, code: IrrigationErrorCode) {
        *self.fail_next_read.lock().expect("fail lock") = Some(code);
    }

    fn set_block_invoke(&self, blocked: bool) {
        let (lock, cvar) = &*self.gate;
        let mut gate = lock.lock().expect("gate lock");
        gate.blocked = blocked;
        if !blocked {
            cvar.notify_all();
        }
    }

    /// Bounded wait until the provider invoke has ENTERED the gate.
    /// Because the in-flight entry is registered before invoke is
    /// reached, success guarantees the first command is in-flight.
    /// A timeout produces a test failure, never a hanging test.
    fn wait_entered(&self, timeout: Duration) -> Result<(), String> {
        let (lock, cvar) = &*self.gate;
        let mut gate = lock.lock().expect("gate lock");
        while !gate.entered {
            let (g, wait) = cvar.wait_timeout(gate, timeout).expect("gate wait");
            gate = g;
            if wait.timed_out() {
                return Err("provider invoke never entered the gate within the timeout".to_string());
            }
        }
        Ok(())
    }
}

impl IrrigationTransport for FixtureTransport {
    fn list_zones(&self) -> Result<Vec<IrrigationZone>, IrrigationError> {
        if let Some(code) = self.fail_next_read.lock().expect("fail lock").take() {
            return Err(IrrigationError::new(
                code,
                "fixture read failure",
                None,
                None,
            ));
        }
        Ok(self.zones.lock().expect("zones lock").clone())
    }

    fn read_zone(&self, entity_id: &str) -> Result<IrrigationZone, IrrigationError> {
        if let Some(code) = self.fail_next_read.lock().expect("fail lock").take() {
            return Err(IrrigationError::new(
                code,
                "fixture read failure",
                None,
                None,
            ));
        }
        self.zones
            .lock()
            .expect("zones lock")
            .iter()
            .find(|z| z.entity_id == entity_id)
            .cloned()
            .ok_or_else(|| IrrigationError::not_found(format!("fixture zone {entity_id} absent")))
    }

    fn invoke(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), IrrigationError> {
        {
            let (lock, cvar) = &*self.gate;
            let mut gate = lock.lock().expect("gate lock");
            gate.entered = true;
            cvar.notify_all();
            while gate.blocked {
                gate = cvar.wait(gate).expect("gate wait");
            }
        }
        let mut zones = self.zones.lock().expect("zones lock");
        let index = zones
            .iter()
            .position(|z| z.entity_id == entity_id)
            .ok_or_else(|| {
                IrrigationError::not_found(format!("fixture zone {entity_id} absent"))
            })?;
        self.invocations
            .lock()
            .expect("invocations lock")
            .push(format!("{domain}.{service}:{entity_id}:{data:?}"));
        match service {
            "turn_on" => {
                zones[index].state = "on".to_string();
            }
            "turn_off" => {
                zones[index].state = "off".to_string();
            }
            _ => {
                return Err(IrrigationError::new(
                    IrrigationErrorCode::External,
                    format!("fixture unknown service {domain}.{service}"),
                    None,
                    None,
                ));
            }
        }
        Ok(())
    }
}

fn zone_a_id() -> IrrigationZoneId {
    irrigation_zone_id(ZONE_A).expect("zone A canonical id")
}

fn zone_b_id() -> IrrigationZoneId {
    irrigation_zone_id(ZONE_B).expect("zone B canonical id")
}

fn adapter_with(transport: FixtureTransport) -> IrrigationAdapter<FixtureTransport> {
    IrrigationAdapter::new(
        transport,
        IrrigationZoneSelector::entities([ZONE_A.to_string(), ZONE_B.to_string()]),
    )
}

#[test]
fn ep024_unit_irrigation_stable_identity_order_free() {
    let transport = FixtureTransport::new(vec![zone(ZONE_B, "off"), zone(ZONE_A, "off")]);
    let adapter = adapter_with(transport);
    let first = adapter.list_zones().expect("discovery");
    assert_eq!(first.len(), 2);
    assert!(first.contains(&zone_a_id()));
    assert!(first.contains(&zone_b_id()));

    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off"), zone(ZONE_B, "off")]);
    let adapter = adapter_with(transport);
    let second = adapter.list_zones().expect("discovery");
    assert_eq!(second, first);
    assert_eq!(stable_zone_id(ZONE_A), zone_a_id().as_str());
}

#[test]
fn ep024_unit_irrigation_capabilities_from_real_features() {
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off")]);
    let adapter = adapter_with(transport);
    let caps = adapter.capabilities(&zone_a_id()).expect("caps");
    assert!(caps.contains(&IrrigationCapability::ZoneControl));
    // No schedule/moisture surface on the fixture -> never advertised.
    assert!(!caps.contains(&IrrigationCapability::ScheduleControl));
    assert!(!caps.contains(&IrrigationCapability::MoistureReadback));
}

#[test]
fn ep024_unit_irrigation_zone_command_submitted_never_verified() {
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off")]);
    let adapter = adapter_with(transport);
    let caps = adapter.capabilities(&zone_a_id()).expect("caps");

    let receipt = adapter
        .execute(&zone_a_id(), IrrigationCommand::ZoneOn, &caps)
        .expect("zone on");
    assert_eq!(receipt.state, IrrigationCommandState::Submitted);
    assert_ne!(receipt.state, IrrigationCommandState::Verified);
    assert_eq!(receipt.zone, zone_a_id().as_str());

    let outcome = adapter
        .verify(&zone_a_id(), IrrigationCommand::ZoneOn, "ON")
        .expect("verify");
    assert_eq!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep024_unit_irrigation_exact_target_wrong_zone_never_verifies() {
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off"), zone(ZONE_B, "off")]);
    let adapter = adapter_with(transport);
    let caps = adapter.capabilities(&zone_a_id()).expect("caps");

    adapter
        .execute(&zone_a_id(), IrrigationCommand::ZoneOn, &caps)
        .expect("A on");
    let err = adapter
        .verify(&zone_b_id(), IrrigationCommand::ZoneOn, "ON")
        .expect_err("wrong zone must fail");
    assert_eq!(err.code, IrrigationErrorCode::Verification);

    // Verifier-level invariant: an observation recorded from zone B
    // (even with the desired state) can never verify zone A.
    use nexus_devices::verifier::{DeviceCommandVerifier, DeviceStateObservation};
    let verifier = DeviceCommandVerifier;
    let observation = DeviceStateObservation {
        device: zone_b_id().as_str().to_string(),
        state: Some("ON".to_string()),
    };
    let outcome = verifier.verify(zone_a_id().as_str(), "ON", &observation);
    assert_eq!(outcome, VerificationOutcome::UnrelatedChange);
}

#[test]
fn ep024_unit_irrigation_unsupported_command_fails_closed_before_provider() {
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off")]);
    let adapter = adapter_with(transport.clone());
    let caps = adapter.capabilities(&zone_a_id()).expect("caps");

    let err = adapter
        .execute(&zone_a_id(), IrrigationCommand::SetSchedule, &caps)
        .expect_err("zone must not accept SET_SCHEDULE");
    assert_eq!(err.code, IrrigationErrorCode::Policy);
    assert_eq!(transport.invocation_count(), 0);
    assert_eq!(transport.state_of(ZONE_A), "off");

    // The denied command is observable: Policy counter + redacted audit.
    let counters = adapter.counters();
    assert_eq!(counters.get("SET_SCHEDULE:POLICY"), Some(&1));
    let audit = adapter.audit();
    assert!(audit.iter().any(|e| e.outcome == "POLICY"));
    assert!(audit
        .iter()
        .all(|e| e.correlation.starts_with("irrigation-")));
}

#[test]
fn ep024_unit_irrigation_duplicate_inflight_command_conflicts() {
    // Real concurrency: a blocking transport keeps the first command
    // in-flight; the duplicate must be refused with Conflict before it
    // reaches the provider (idempotency, M4 directive 1).
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off")]);
    transport.set_block_invoke(true);
    let adapter = Arc::new(adapter_with(transport.clone()));
    let caps = adapter.capabilities(&zone_a_id()).expect("caps");

    let barrier = Arc::new(Barrier::new(2));
    let barrier2 = Arc::clone(&barrier);
    let adapter2 = Arc::clone(&adapter);
    let caps2 = caps.clone();
    let zone_a = zone_a_id();

    let handle = thread::spawn(move || {
        barrier2.wait();
        adapter2
            .execute(&zone_a, IrrigationCommand::ZoneOn, &caps2)
            .expect("first command must be in-flight")
    });

    barrier.wait();
    // Deterministic, bounded: wait until the first command has ENTERED
    // the provider transport. Because the in-flight entry is registered
    // before invoke is reached, this guarantees the first command is
    // in-flight before the duplicate is submitted (no timing race).
    transport
        .wait_entered(Duration::from_secs(10))
        .expect("first command must enter the provider transport");
    let err = adapter
        .execute(&zone_a_id(), IrrigationCommand::ZoneOn, &caps)
        .expect_err("duplicate must conflict");
    assert_eq!(err.code, IrrigationErrorCode::Conflict);
    // The duplicate never reached the provider.
    assert_eq!(transport.invocation_count(), 0);

    // Release the first command and let it complete.
    transport.set_block_invoke(false);
    let receipt = handle.join().expect("thread");
    assert_eq!(receipt.state, IrrigationCommandState::Submitted);
    assert_eq!(transport.invocation_count(), 1);
    assert_eq!(transport.state_of(ZONE_A), "on");
}

#[test]
fn ep024_unit_irrigation_availability_truth_table() {
    // Present + usable -> AVAILABLE.
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off")]);
    let adapter = adapter_with(transport);
    assert_eq!(
        adapter.availability(&zone_a_id()).expect("avail"),
        DeviceAvailability::Available
    );

    // Provider-unavailable -> UNAVAILABLE (never OFF).
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "unavailable")]);
    let adapter = adapter_with(transport);
    assert_eq!(
        adapter.availability(&zone_a_id()).expect("avail"),
        DeviceAvailability::Unavailable
    );

    // Unknown -> AVAILABLE (present + usable; never claimed OFF).
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "unknown")]);
    let adapter = adapter_with(transport);
    assert_eq!(
        adapter.availability(&zone_a_id()).expect("avail"),
        DeviceAvailability::Available
    );

    // Unknown entity -> NotFound.
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off")]);
    let adapter = adapter_with(transport);
    let ghost = irrigation_zone_id("input_boolean.nexus_ghost").expect("ghost id");
    let err = adapter.availability(&ghost).expect_err("unknown");
    assert_eq!(err.code, IrrigationErrorCode::NotFound);

    // Provider offline -> UNAVAILABLE.
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off")]);
    let adapter = adapter_with(transport.clone());
    transport.fail_reads_with(IrrigationErrorCode::Unavailable);
    assert_eq!(
        adapter.availability(&zone_a_id()).expect("avail"),
        DeviceAvailability::Unavailable
    );
}

#[test]
fn ep024_unit_irrigation_bounded_recovery_releases_stuck_entries() {
    // After a failed command, the in-flight entry must be released so a
    // retry after provider recovery is possible (M4 directive 3).
    let transport = FixtureTransport::new(vec![zone(ZONE_A, "off")]);
    let adapter = adapter_with(transport.clone());
    let caps = adapter.capabilities(&zone_a_id()).expect("caps");

    // Provider goes offline mid-command.
    transport.fail_reads_with(IrrigationErrorCode::Unavailable);
    let err = adapter
        .execute(&zone_a_id(), IrrigationCommand::ZoneOn, &caps)
        .expect_err("provider offline");
    assert_eq!(err.code, IrrigationErrorCode::Unavailable);

    // The failed command released its in-flight entry: a retry is not
    // a false Conflict (the provider is still offline).
    transport.fail_reads_with(IrrigationErrorCode::Unavailable);
    let err = adapter
        .execute(&zone_a_id(), IrrigationCommand::ZoneOn, &caps)
        .expect_err("still offline");
    assert_eq!(err.code, IrrigationErrorCode::Unavailable);
    assert_ne!(err.code, IrrigationErrorCode::Conflict);

    // Bounded recovery: no stuck entries.
    assert_eq!(adapter.recover(), 0);

    // The observability ring recorded the failure with correlation.
    let counters = adapter.counters();
    assert_eq!(counters.get("ZONE_ON:UNAVAILABLE"), Some(&2));
}

#[test]
fn ep024_unit_irrigation_zone_state_value_unavailable_never_off() {
    let mut z = zone(ZONE_A, "unavailable");
    assert_eq!(zone_state_value(&z), None);
    z.state = "unknown".to_string();
    assert_eq!(zone_state_value(&z), None);
    z.state = "off".to_string();
    assert_eq!(zone_state_value(&z), Some("OFF".to_string()));
    z.state = "on".to_string();
    assert_eq!(zone_state_value(&z), Some("ON".to_string()));
}

#[test]
fn ep024_unit_irrigation_mapper_uses_ep010_taxonomy() {
    let mapper = DeviceCapabilityMapper;
    let zone = mapper.map("irrigation.zone").expect("zone");
    assert_eq!(zone.class.as_str(), "COMMAND");
    assert_eq!(zone.risk.as_str(), "R1");
    assert_eq!(zone.approval.as_str(), "NONE");
    assert_eq!(zone.idempotency.as_str(), "REQUIRED");
    let schedule = mapper.map("irrigation.schedule").expect("schedule");
    assert_eq!(schedule.class.as_str(), "COMMAND");
    let moisture = mapper.map("irrigation.moisture").expect("moisture");
    assert_eq!(moisture.class.as_str(), "QUERY");
    assert_eq!(moisture.risk.as_str(), "R0");
    let unknown = mapper.map("irrigation.flood").expect_err("reject");
    assert_eq!(unknown.code, nexus_devices::DevicesErrorCode::Vocabulary);
}
