//! EP-024 M5 vacuum unit suite: deterministic adapter rules
//! against a controlled fixture transport (TESTING.md test-double
//! zone).
//!
//! The REAL provider failure suite is ep024_failure_vacuum.rs (live
//! Home Assistant container). This suite proves the adapter's
//! deterministic invariants: stable vacuum identity, capability
//! mapping from real feature bits, availability truth table,
//! capability-gated dispatch (no provider mutation for unsupported
//! commands), SUBMITTED-never-VERIFIED, exact-target verification,
//! in-flight idempotency (real concurrent duplicate -> Conflict),
//! bounded recovery, and observability (counters, redacted audit,
//! correlation).

use std::collections::BTreeMap;
use std::sync::{Arc, Barrier, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::Value;

use nexus_devices::mapper::DeviceCapabilityMapper;
use nexus_devices::verifier::VerificationOutcome;
use nexus_devices::vocabulary::{DeviceAvailability, VacuumCapability, VacuumDeviceId};
use nexus_devices::VacuumProvider;
use nexus_vacuum::{
    capabilities_for, stable_vacuum_id, vacuum_device_id, vacuum_state_value, VacuumActivityState,
    VacuumAdapter, VacuumCommand, VacuumCommandState, VacuumDevice, VacuumDeviceSelector,
    VacuumError, VacuumErrorCode, VacuumTransport,
};

const VACUUM_A: &str = "vacuum.nexus_vacuum_a";
const VACUUM_B: &str = "vacuum.nexus_vacuum_b";

/// Real HA feature bits (observed pinned build: template vacuum
/// publishes START=4096, PAUSE=4, RETURN_HOME=16; the live probe
/// verifies the observed value).
const HA_FEATURE_START: u64 = 4096;
const HA_FEATURE_PAUSE: u64 = 4;
const HA_FEATURE_RETURN_HOME: u64 = 16;

fn vacuum(entity_id: &str, state: &str, features: Option<u64>) -> VacuumDevice {
    let domain = entity_id.split('.').next().unwrap_or_default().to_string();
    let mut attributes = BTreeMap::new();
    if let Some(f) = features {
        attributes.insert("supported_features".to_string(), Value::from(f));
    }
    VacuumDevice {
        entity_id: entity_id.to_string(),
        domain,
        state: state.to_string(),
        attributes,
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

/// Controlled fixture transport: an in-memory vacuum registry whose
/// state changes only when `invoke` is called. The test keeps an Arc
/// clone to inspect invocations and state, and to prove unsupported
/// commands NEVER reached the provider. A Condvar gate blocks invoke
/// on demand (real concurrency proof for the in-flight idempotency
/// guard).
#[derive(Clone, Default)]
struct FixtureTransport {
    devices: Arc<Mutex<Vec<VacuumDevice>>>,
    invocations: Arc<Mutex<Vec<String>>>,
    fail_next_read: Arc<Mutex<Option<VacuumErrorCode>>>,
    gate: Arc<(Mutex<GateState>, Condvar)>,
}

impl FixtureTransport {
    fn new(devices: Vec<VacuumDevice>) -> Self {
        Self {
            devices: Arc::new(Mutex::new(devices)),
            invocations: Arc::new(Mutex::new(Vec::new())),
            fail_next_read: Arc::new(Mutex::new(None)),
            gate: Arc::new((Mutex::new(GateState::default()), Condvar::new())),
        }
    }

    fn invocation_count(&self) -> usize {
        self.invocations.lock().expect("invocations lock").len()
    }

    fn state_of(&self, entity_id: &str) -> String {
        self.devices
            .lock()
            .expect("devices lock")
            .iter()
            .find(|d| d.entity_id == entity_id)
            .map(|d| d.state.clone())
            .unwrap_or_default()
    }

    fn fail_reads_with(&self, code: VacuumErrorCode) {
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

impl VacuumTransport for FixtureTransport {
    fn list_vacuums(&self) -> Result<Vec<VacuumDevice>, VacuumError> {
        if let Some(code) = self.fail_next_read.lock().expect("fail lock").take() {
            return Err(VacuumError::new(code, "fixture read failure", None, None));
        }
        Ok(self.devices.lock().expect("devices lock").clone())
    }

    fn read_vacuum(&self, entity_id: &str) -> Result<VacuumDevice, VacuumError> {
        if let Some(code) = self.fail_next_read.lock().expect("fail lock").take() {
            return Err(VacuumError::new(code, "fixture read failure", None, None));
        }
        self.devices
            .lock()
            .expect("devices lock")
            .iter()
            .find(|d| d.entity_id == entity_id)
            .cloned()
            .ok_or_else(|| VacuumError::not_found(format!("fixture vacuum {entity_id} absent")))
    }

    fn invoke(
        &self,
        domain: &str,
        service: &str,
        entity_id: &str,
        data: &BTreeMap<String, Value>,
    ) -> Result<(), VacuumError> {
        {
            let (lock, cvar) = &*self.gate;
            let mut gate = lock.lock().expect("gate lock");
            gate.entered = true;
            cvar.notify_all();
            while gate.blocked {
                gate = cvar.wait(gate).expect("gate wait");
            }
        }
        let mut devices = self.devices.lock().expect("devices lock");
        let index = devices
            .iter()
            .position(|d| d.entity_id == entity_id)
            .ok_or_else(|| VacuumError::not_found(format!("fixture vacuum {entity_id} absent")))?;
        self.invocations
            .lock()
            .expect("invocations lock")
            .push(format!("{domain}.{service}:{entity_id}:{data:?}"));
        match service {
            "start" => {
                devices[index].state = "cleaning".to_string();
            }
            "pause" => {
                devices[index].state = "paused".to_string();
            }
            "return_to_base" => {
                devices[index].state = "returning".to_string();
            }
            _ => {
                return Err(VacuumError::new(
                    VacuumErrorCode::External,
                    format!("fixture unknown service {domain}.{service}"),
                    None,
                    None,
                ));
            }
        }
        Ok(())
    }
}

fn vacuum_a_id() -> VacuumDeviceId {
    vacuum_device_id(VACUUM_A).expect("vacuum A canonical id")
}

fn vacuum_b_id() -> VacuumDeviceId {
    vacuum_device_id(VACUUM_B).expect("vacuum B canonical id")
}

fn adapter_with(transport: FixtureTransport) -> VacuumAdapter<FixtureTransport> {
    VacuumAdapter::new(
        transport,
        VacuumDeviceSelector::entities([VACUUM_A.to_string(), VACUUM_B.to_string()]),
    )
}

#[test]
fn ep024_unit_vacuum_stable_identity_order_free() {
    let transport = FixtureTransport::new(vec![
        vacuum(VACUUM_B, "docked", Some(HA_FEATURE_START)),
        vacuum(VACUUM_A, "docked", Some(HA_FEATURE_START)),
    ]);
    let adapter = adapter_with(transport);
    let first = adapter.list_devices().expect("discovery");
    assert_eq!(first.len(), 2);
    assert!(first.contains(&vacuum_a_id()));
    assert!(first.contains(&vacuum_b_id()));

    let transport = FixtureTransport::new(vec![
        vacuum(VACUUM_A, "docked", Some(HA_FEATURE_START)),
        vacuum(VACUUM_B, "docked", Some(HA_FEATURE_START)),
    ]);
    let adapter = adapter_with(transport);
    let second = adapter.list_devices().expect("discovery");
    assert_eq!(second, first);
    assert_eq!(stable_vacuum_id(VACUUM_A), vacuum_a_id().as_str());
}

#[test]
fn ep024_unit_vacuum_capabilities_from_real_features() {
    // A vacuum exposing START + PAUSE + RETURN_HOME advertises exactly
    // those capabilities (Dock and ReturnHome share the return-home
    // feature) and NEVER MapReadback without a real map surface.
    let mapper = DeviceCapabilityMapper;
    let device = vacuum(
        VACUUM_A,
        "docked",
        Some(HA_FEATURE_START | HA_FEATURE_PAUSE | HA_FEATURE_RETURN_HOME),
    );
    let caps = capabilities_for(&device, &mapper).expect("caps");
    assert!(caps.contains(&VacuumCapability::StartClean));
    assert!(caps.contains(&VacuumCapability::Pause));
    assert!(caps.contains(&VacuumCapability::ReturnHome));
    assert!(caps.contains(&VacuumCapability::Dock));
    assert!(!caps.contains(&VacuumCapability::MapReadback));

    // A vacuum with only START never advertises PAUSE/RETURN_HOME.
    let device = vacuum(VACUUM_A, "docked", Some(HA_FEATURE_START));
    let caps = capabilities_for(&device, &mapper).expect("caps");
    assert!(caps.contains(&VacuumCapability::StartClean));
    assert!(!caps.contains(&VacuumCapability::Pause));
    assert!(!caps.contains(&VacuumCapability::ReturnHome));
    assert!(!caps.contains(&VacuumCapability::Dock));
}

#[test]
fn ep024_unit_vacuum_command_submitted_never_verified() {
    let transport = FixtureTransport::new(vec![vacuum(VACUUM_A, "docked", Some(HA_FEATURE_START))]);
    let adapter = adapter_with(transport);
    let caps = adapter.capabilities(&vacuum_a_id()).expect("caps");

    let receipt = adapter
        .execute(&vacuum_a_id(), VacuumCommand::StartClean, &caps)
        .expect("start clean");
    assert_eq!(receipt.state, VacuumCommandState::Submitted);
    assert_ne!(receipt.state, VacuumCommandState::Verified);
    assert_eq!(receipt.device, vacuum_a_id().as_str());

    let outcome = adapter
        .verify(&vacuum_a_id(), VacuumCommand::StartClean, "CLEANING")
        .expect("verify");
    assert_eq!(outcome, VerificationOutcome::Verified);
}

#[test]
fn ep024_unit_vacuum_exact_target_wrong_vacuum_never_verifies() {
    let transport = FixtureTransport::new(vec![
        vacuum(VACUUM_A, "docked", Some(HA_FEATURE_START)),
        vacuum(VACUUM_B, "docked", Some(HA_FEATURE_START)),
    ]);
    let adapter = adapter_with(transport);
    let caps = adapter.capabilities(&vacuum_a_id()).expect("caps");

    adapter
        .execute(&vacuum_a_id(), VacuumCommand::StartClean, &caps)
        .expect("A clean");
    let err = adapter
        .verify(&vacuum_b_id(), VacuumCommand::StartClean, "CLEANING")
        .expect_err("wrong vacuum must fail");
    assert_eq!(err.code, VacuumErrorCode::Verification);

    // Verifier-level invariant: an observation recorded from vacuum B
    // (even with the desired state) can never verify vacuum A.
    use nexus_devices::verifier::{DeviceCommandVerifier, DeviceStateObservation};
    let verifier = DeviceCommandVerifier;
    let observation = DeviceStateObservation {
        device: vacuum_b_id().as_str().to_string(),
        state: Some("CLEANING".to_string()),
    };
    let outcome = verifier.verify(vacuum_a_id().as_str(), "CLEANING", &observation);
    assert_eq!(outcome, VerificationOutcome::UnrelatedChange);
}

#[test]
fn ep024_unit_vacuum_unsupported_command_fails_closed_before_provider() {
    let transport = FixtureTransport::new(vec![vacuum(VACUUM_A, "docked", Some(HA_FEATURE_START))]);
    let adapter = adapter_with(transport.clone());
    let caps = adapter.capabilities(&vacuum_a_id()).expect("caps");

    // PAUSE is not advertised (fixture has only START): Policy before
    // any provider mutation.
    let err = adapter
        .execute(&vacuum_a_id(), VacuumCommand::Pause, &caps)
        .expect_err("vacuum must not accept PAUSE");
    assert_eq!(err.code, VacuumErrorCode::Policy);
    assert_eq!(transport.invocation_count(), 0);
    assert_eq!(transport.state_of(VACUUM_A), "docked");

    // MAP_READBACK without a map surface: Policy (fail closed), never
    // success, never a provider action.
    let err = adapter
        .map_readback(&vacuum_a_id(), &caps)
        .expect_err("no map surface");
    assert_eq!(err.code, VacuumErrorCode::Policy);

    // The denied commands are observable: Policy counters + redacted
    // audit with canonical correlation.
    let counters = adapter.counters();
    assert_eq!(counters.get("PAUSE:POLICY"), Some(&1));
    assert_eq!(counters.get("MAP_READBACK:POLICY"), Some(&1));
    let audit = adapter.audit();
    assert!(audit.iter().any(|e| e.outcome == "POLICY"));
    assert!(audit.iter().all(|e| e.correlation.starts_with("vacuum-")));
}

#[test]
fn ep024_unit_vacuum_duplicate_inflight_command_conflicts() {
    // Real concurrency: a blocking transport keeps the first command
    // in-flight; the duplicate must be refused with Conflict before it
    // reaches the provider (idempotency, no double physical action).
    let transport = FixtureTransport::new(vec![vacuum(VACUUM_A, "docked", Some(HA_FEATURE_START))]);
    transport.set_block_invoke(true);
    let adapter = Arc::new(adapter_with(transport.clone()));
    let caps = adapter.capabilities(&vacuum_a_id()).expect("caps");

    let barrier = Arc::new(Barrier::new(2));
    let barrier2 = Arc::clone(&barrier);
    let adapter2 = Arc::clone(&adapter);
    let caps2 = caps.clone();
    let vacuum_a = vacuum_a_id();

    let handle = thread::spawn(move || {
        barrier2.wait();
        adapter2
            .execute(&vacuum_a, VacuumCommand::StartClean, &caps2)
            .expect("first command must be in-flight")
    });

    barrier.wait();
    // Deterministic, bounded: wait until the first command has ENTERED
    // the provider transport. Because the in-flight entry is
    // registered before invoke is reached, this guarantees the first
    // command is in-flight before the duplicate is submitted (no
    // timing race).
    transport
        .wait_entered(Duration::from_secs(10))
        .expect("first command must enter the provider transport");
    let err = adapter
        .execute(&vacuum_a_id(), VacuumCommand::StartClean, &caps)
        .expect_err("duplicate must conflict");
    assert_eq!(err.code, VacuumErrorCode::Conflict);
    // The duplicate never reached the provider.
    assert_eq!(transport.invocation_count(), 0);

    // Release the first command and let it complete.
    transport.set_block_invoke(false);
    let receipt = handle.join().expect("thread");
    assert_eq!(receipt.state, VacuumCommandState::Submitted);
    assert_eq!(transport.invocation_count(), 1);
    assert_eq!(transport.state_of(VACUUM_A), "cleaning");
}

#[test]
fn ep024_unit_vacuum_availability_truth_table() {
    // Known + reachable -> AVAILABLE.
    let transport = FixtureTransport::new(vec![vacuum(VACUUM_A, "docked", Some(HA_FEATURE_START))]);
    let adapter = adapter_with(transport);
    assert_eq!(
        adapter.availability(&vacuum_a_id()).expect("avail"),
        DeviceAvailability::Available
    );

    // Provider-unavailable -> UNAVAILABLE (never DOCKED).
    let transport = FixtureTransport::new(vec![vacuum(VACUUM_A, "unavailable", None)]);
    let adapter = adapter_with(transport);
    assert_eq!(
        adapter.availability(&vacuum_a_id()).expect("avail"),
        DeviceAvailability::Unavailable
    );

    // Unknown -> AVAILABLE (present + usable; never claimed safe).
    let transport = FixtureTransport::new(vec![vacuum(VACUUM_A, "unknown", None)]);
    let adapter = adapter_with(transport);
    assert_eq!(
        adapter.availability(&vacuum_a_id()).expect("avail"),
        DeviceAvailability::Available
    );

    // Unknown entity -> NotFound.
    let transport = FixtureTransport::new(vec![vacuum(VACUUM_A, "docked", None)]);
    let adapter = adapter_with(transport);
    let ghost = vacuum_device_id("vacuum.nexus_ghost").expect("ghost id");
    let err = adapter.availability(&ghost).expect_err("unknown");
    assert_eq!(err.code, VacuumErrorCode::NotFound);

    // Provider offline -> UNAVAILABLE.
    let transport = FixtureTransport::new(vec![vacuum(VACUUM_A, "docked", None)]);
    let adapter = adapter_with(transport.clone());
    transport.fail_reads_with(VacuumErrorCode::Unavailable);
    assert_eq!(
        adapter.availability(&vacuum_a_id()).expect("avail"),
        DeviceAvailability::Unavailable
    );
}

#[test]
fn ep024_unit_vacuum_bounded_recovery_releases_stuck_entries() {
    // After a failed command, the in-flight entry must be released so
    // a retry after provider recovery is possible.
    let transport = FixtureTransport::new(vec![vacuum(VACUUM_A, "docked", Some(HA_FEATURE_START))]);
    let adapter = adapter_with(transport.clone());
    let caps = adapter.capabilities(&vacuum_a_id()).expect("caps");

    // Provider goes offline mid-command.
    transport.fail_reads_with(VacuumErrorCode::Unavailable);
    let err = adapter
        .execute(&vacuum_a_id(), VacuumCommand::StartClean, &caps)
        .expect_err("provider offline");
    assert_eq!(err.code, VacuumErrorCode::Unavailable);

    // The failed command released its in-flight entry: a retry is not
    // a false Conflict (the provider is still offline).
    transport.fail_reads_with(VacuumErrorCode::Unavailable);
    let err = adapter
        .execute(&vacuum_a_id(), VacuumCommand::StartClean, &caps)
        .expect_err("still offline");
    assert_eq!(err.code, VacuumErrorCode::Unavailable);
    assert_ne!(err.code, VacuumErrorCode::Conflict);

    // Bounded recovery: no stuck entries.
    assert_eq!(adapter.recover(), 0);

    // The observability ring recorded the failure with correlation.
    let counters = adapter.counters();
    assert_eq!(counters.get("START_CLEAN:UNAVAILABLE"), Some(&2));
}

#[test]
fn ep024_unit_vacuum_state_value_unavailable_never_safe() {
    let mut d = vacuum(VACUUM_A, "unavailable", None);
    assert_eq!(vacuum_state_value(&d), None);
    d.state = "unknown".to_string();
    assert_eq!(vacuum_state_value(&d), None);
    d.state = "docked".to_string();
    assert_eq!(vacuum_state_value(&d), Some(VacuumActivityState::Docked));
    d.state = "cleaning".to_string();
    assert_eq!(vacuum_state_value(&d), Some(VacuumActivityState::Cleaning));
    d.state = "paused".to_string();
    assert_eq!(vacuum_state_value(&d), Some(VacuumActivityState::Paused));
    d.state = "returning".to_string();
    assert_eq!(vacuum_state_value(&d), Some(VacuumActivityState::Returning));
    // Unknown provider string -> None, never safe.
    d.state = "some-custom-state".to_string();
    assert_eq!(vacuum_state_value(&d), None);
}

#[test]
fn ep024_unit_vacuum_mapper_uses_ep010_taxonomy() {
    let mapper = DeviceCapabilityMapper;
    let clean = mapper.map("vacuum.clean").expect("clean");
    assert_eq!(clean.class.as_str(), "COMMAND");
    assert_eq!(clean.risk.as_str(), "R1");
    assert_eq!(clean.approval.as_str(), "NONE");
    assert_eq!(clean.idempotency.as_str(), "REQUIRED");
    let map = mapper.map("vacuum.map").expect("map");
    assert_eq!(map.class.as_str(), "QUERY");
    assert_eq!(map.risk.as_str(), "R0");
    let unknown = mapper.map("vacuum.teleport").expect_err("reject");
    assert_eq!(unknown.code, nexus_devices::DevicesErrorCode::Vocabulary);
}
