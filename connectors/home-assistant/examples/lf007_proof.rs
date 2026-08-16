//! LF-007 conditional-home-workflow (EP-020 M5; SPEC-011; ADR-027).
//!
//! Real conditional automations in the REAL Home Assistant instance,
//! provisioned through the PRODUCTION create path (directive B/C):
//!   canonical Nexus AutomationSpec
//!   -> AutomationHandoffAdapter::create()
//!   -> POST /api/config/automation/config/<id> (the real supported HA
//!      provisioning API: validates, writes automations.yaml
//!      atomically, fires automation.reload with CONF_ID)
//!   -> the runnable automation entity appears through provider readback
//!      and is enabled BEFORE create() returns a handle.
//!
//! No YAML is pre-written for these automations (the fixture starts
//! with an EMPTY automations.yaml; only the fixture entities exist at
//! boot). The two conditional automations are created at runtime:
//!   - `automation.nexus_lf007_cond_true` : trigger switch1 -> "on",
//!     condition switch1=="on" (TRUE)  -> action turns switch2 ON
//!     (conditional EXECUTION);
//!   - `automation.nexus_lf007_cond_false`: trigger switch1 -> "off",
//!     condition switch1=="on" (FALSE) -> action (switch2 turn_off) is
//!     CANCELLED and switch2 stays on.
//!
//! Persistence is proven by readback after a real container restart
//! (the config API wrote automations.yaml durably; HA reloads it).
//!
//! Temporal boundary (honest): real Temporal workflow persistence and
//! determinism machinery is proven by the EP-019 workflow suite; wiring
//! a Temporal-hosted conditional home workflow is owned by the
//! Temporal-owning/deployment nodes and is NOT claimed here.
//!
//! Phases (env PHASE=create|persist|exec): the Python driver restarts
//! the container between create and persist. Evidence JSON per phase is
//! printed to stdout.

use std::collections::BTreeMap;
use std::env;

use nexus_domain::DeviceId;
use nexus_home::{
    AutomationCondition, AutomationHandle, AutomationSpec, AutomationTrigger, HaEntityRef,
    HomeIntent,
};
use nexus_home_assistant::{AutomationHandoffAdapter, HaTransport, RestTransport};

const SWITCH1: &str = "input_boolean.nexus_test_switch";
const SWITCH2: &str = "input_boolean.nexus_test_switch_2";
const AUTOMATION_TRUE: &str = "automation.nexus_lf007_cond_true";
const AUTOMATION_FALSE: &str = "automation.nexus_lf007_cond_false";

fn correlation(n: u8) -> nexus_domain::CorrelationId {
    nexus_domain::CorrelationId::new(format!("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7{n:03}"))
        .expect("valid UUIDv7")
}

fn intent(switch2_on: bool, n: u8) -> HomeIntent {
    let mut parameters = BTreeMap::new();
    parameters.insert("entity_id".to_string(), serde_json::json!(SWITCH2));
    HomeIntent {
        device_id: DeviceId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6501").expect("valid UUIDv7"),
        capability_id: "home.switch".to_string(),
        action: if switch2_on { "turn_on" } else { "turn_off" }.to_string(),
        parameters,
        correlation_id: correlation(n),
        idempotency_key: None,
    }
}

fn spec_cond_true() -> AutomationSpec {
    AutomationSpec {
        name: "Nexus LF007 Cond True".to_string(),
        trigger: "switch1 on".to_string(),
        action: intent(true, 11),
        enabled: true,
        provider_trigger: Some(AutomationTrigger {
            entity: HaEntityRef::new(SWITCH1).expect("valid ref"),
            to_state: "on".to_string(),
        }),
        provider_condition: Some(AutomationCondition {
            entity: HaEntityRef::new(SWITCH1).expect("valid ref"),
            state: "on".to_string(),
        }),
    }
}

fn spec_cond_false() -> AutomationSpec {
    AutomationSpec {
        name: "Nexus LF007 Cond False".to_string(),
        trigger: "switch1 off".to_string(),
        action: intent(false, 12),
        enabled: true,
        provider_trigger: Some(AutomationTrigger {
            entity: HaEntityRef::new(SWITCH1).expect("valid ref"),
            to_state: "off".to_string(),
        }),
        provider_condition: Some(AutomationCondition {
            entity: HaEntityRef::new(SWITCH1).expect("valid ref"),
            state: "on".to_string(),
        }),
    }
}

fn main() {
    let base = env::var("NEXUS_HA_BASE").unwrap_or_else(|_| "http://127.0.0.1:8123".to_string());
    let token = env::var("NEXUS_HA_TOKEN").expect("NEXUS_HA_TOKEN required");
    let phase = env::var("PHASE").unwrap_or_else(|_| "create".to_string());
    let result = match phase.as_str() {
        "create" => create(&base, &token),
        "persist" => persist(&base, &token),
        "exec" => exec(&base, &token),
        other => Err(format!("unknown phase {other}").into()),
    };
    match result {
        Ok(evidence) => println!("{evidence}"),
        Err(e) => {
            eprintln!("LF-007 {phase} FAIL: {e}");
            std::process::exit(1);
        }
    }
}

fn state_of(
    transport: &mut RestTransport,
    entity: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let state = transport
        .get_state(entity)
        .map_err(|e| format!("get_state {entity}: {e}"))?;
    Ok(state.state)
}

fn set_switch(
    transport: &mut RestTransport,
    entity: &str,
    on: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut data = BTreeMap::new();
    data.insert("entity_id".to_string(), serde_json::json!(entity));
    transport
        .call_service(
            "input_boolean",
            if on { "turn_on" } else { "turn_off" },
            &data,
        )
        .map_err(|e| format!("set_switch {entity} failed: {e}").into())
}

fn wait_state(
    transport: &mut RestTransport,
    entity: &str,
    expected: &str,
    timeout: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout);
    loop {
        if state_of(transport, entity)? == expected {
            return Ok(true);
        }
        if std::time::Instant::now() >= deadline {
            return Ok(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn create(base: &str, token: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Production create path: canonical specs -> adapter create() ->
    // real HA config API -> readback-before-success.
    let mut transport = RestTransport::new(base, token);
    assert!(transport.auth_check()?, "auth failed");
    let mut handoff = AutomationHandoffAdapter::new(transport);

    let h_true = handoff
        .create(&spec_cond_true())
        .map_err(|e| format!("create cond-true failed: {e}"))?;
    assert_eq!(h_true.provider_automation_id, AUTOMATION_TRUE);
    let h_false = handoff
        .create(&spec_cond_false())
        .map_err(|e| format!("create cond-false failed: {e}"))?;
    assert_eq!(h_false.provider_automation_id, AUTOMATION_FALSE);

    // Readback: both automations exist and are enabled.
    let s_true = handoff
        .readback(&h_true)
        .map_err(|e| format!("readback cond-true failed: {e}"))?;
    let s_false = handoff
        .readback(&h_false)
        .map_err(|e| format!("readback cond-false failed: {e}"))?;
    assert!(s_true.enabled, "cond-true automation not enabled");
    assert!(s_false.enabled, "cond-false automation not enabled");

    Ok("{\"proof\":\"LF-007\",\"phase\":\"create\",\"created\":true,\"automation_true\":\"on\",\"automation_false\":\"on\",\"readback_enabled\":true}"
        .to_string())
}

fn persist(base: &str, token: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Runs AFTER the driver restarted the container: the automations
    // created through the config API must have been reloaded durably
    // (automations.yaml persisted on the mounted config dir) and the
    // production adapter must still read/identify them.
    let mut transport = RestTransport::new(base, token);
    assert!(transport.auth_check()?, "auth failed after restart");
    let mut handoff = AutomationHandoffAdapter::new(transport);

    let h_true = AutomationHandle {
        provider_automation_id: AUTOMATION_TRUE.to_string(),
        name: "Nexus LF007 Cond True".to_string(),
    };
    let h_false = AutomationHandle {
        provider_automation_id: AUTOMATION_FALSE.to_string(),
        name: "Nexus LF007 Cond False".to_string(),
    };
    let s_true = handoff
        .readback(&h_true)
        .map_err(|e| format!("readback cond-true after restart failed: {e}"))?;
    let s_false = handoff
        .readback(&h_false)
        .map_err(|e| format!("readback cond-false after restart failed: {e}"))?;
    assert!(s_true.enabled, "cond-true automation lost after restart");
    assert!(s_false.enabled, "cond-false automation lost after restart");

    Ok("{\"proof\":\"LF-007\",\"phase\":\"persist\",\"persisted\":true,\"automation_true\":\"on\",\"automation_false\":\"on\",\"readback_enabled\":true}"
        .to_string())
}

fn exec(base: &str, token: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut transport = RestTransport::new(base, token);
    assert!(transport.auth_check()?, "auth failed");

    // Baseline: switch1 off, switch2 off.
    set_switch(&mut transport, SWITCH1, false)?;
    set_switch(&mut transport, SWITCH2, false)?;
    assert!(
        wait_state(&mut transport, SWITCH2, "off", 10)?,
        "switch2 baseline"
    );

    // Conditional EXECUTION: switch1 on -> condition true -> switch2 on.
    set_switch(&mut transport, SWITCH1, true)?;
    let executed = wait_state(&mut transport, SWITCH2, "on", 15)?;
    assert!(executed, "conditional action did not execute");

    // Conditional CANCELLATION: switch1 off -> cond-false trigger fires
    // but its condition (switch1 is on) is FALSE -> switch2 must stay on.
    set_switch(&mut transport, SWITCH1, false)?;
    std::thread::sleep(std::time::Duration::from_secs(3));
    let s2 = state_of(&mut transport, SWITCH2)?;
    assert_eq!(s2, "on", "cancelled automation still ran its action");

    // Cleanup.
    set_switch(&mut transport, SWITCH2, false)?;

    Ok(format!(
        "{{\"proof\":\"LF-007\",\"phase\":\"exec\",\"conditional_execution\":true,\"conditional_cancellation\":true,\"switch2_after_cancel\":\"{s2}\"}}"
    ))
}
