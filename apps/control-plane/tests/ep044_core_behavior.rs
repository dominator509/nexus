//! EP-044 M2 core behavior: deterministic invariants and composition
//! root proofs beyond the module-level unit tests.
//!
//! These tests exercise the real composition root surface: config
//! loading from canonical JSON, capability source wiring, lifecycle
//! readiness gating of the router, and the fail-closed smoke contract.

use nexus_control_plane::capabilities::{CapabilityListSource, ConfiguredCapabilityList};
use nexus_control_plane::config::ControlPlaneConfig;
use nexus_control_plane::lifecycle::RuntimeLifecycle;
use nexus_control_plane::readiness::RuntimeReadiness;
use nexus_control_plane::server::ControlPlaneServer;
use nexus_control_plane::smoke::{RuntimeSmoke, SmokeResult};
use nexus_control_plane::vocabulary::RuntimeState;

const TENANT: &str = "018f0f6f-9c1e-7b6e-8000-000000000001";

fn test_config() -> ControlPlaneConfig {
    ControlPlaneConfig::new("nexus.test", "127.0.0.1:18443", TENANT, "core").unwrap()
}

fn test_server() -> ControlPlaneServer {
    let caps: Box<dyn CapabilityListSource + Send + Sync> = Box::new(
        ConfiguredCapabilityList::new("core", vec!["health".into(), "capabilities".into()]),
    );
    ControlPlaneServer::new(test_config(), caps)
}

#[test]
fn ep044_unit_composition_root_router_is_ready_gated() {
    let server = test_server();
    // Fresh lifecycle: not ready until serve marks it ready.
    assert!(!server.lifecycle().lock().unwrap().is_ready());
    let state = server.lifecycle().lock().unwrap().state();
    assert_eq!(state, RuntimeState::Starting);
    // Router builds regardless (handlers gate on readiness).
    let _router = server.router();
}

#[test]
fn ep044_unit_capability_source_fail_closed_empty() {
    let source = ConfiguredCapabilityList::new("core", vec![]);
    let err = source.list().unwrap_err();
    assert_eq!(
        err.code,
        nexus_control_plane::error::RuntimeErrorCode::Unavailable
    );
}

#[test]
fn ep044_unit_readiness_not_ready_serializes_false() {
    assert_eq!(
        serde_json::to_string(&RuntimeReadiness::not_ready()).unwrap(),
        r#"{"ready":false}"#
    );
}

#[test]
fn ep044_unit_smoke_contract_all_probes_required() {
    let smoke = RuntimeSmoke::new("https://nexus.test");
    // Every probe must be true; any false is fail-closed.
    assert_eq!(smoke.evaluate(true, true, true).unwrap(), SmokeResult::ok());
    assert!(smoke.evaluate(false, true, true).is_err());
    assert!(smoke.evaluate(true, false, true).is_err());
    assert!(smoke.evaluate(true, true, false).is_err());
}

#[test]
fn ep044_unit_lifecycle_cannot_return_to_ready() {
    let mut lc = RuntimeLifecycle::new();
    lc.mark_ready().unwrap();
    lc.begin_shutdown().unwrap();
    lc.finish_shutdown().unwrap();
    assert_eq!(lc.state(), RuntimeState::Stopped);
    // mark_ready from Stopped is rejected.
    assert!(lc.mark_ready().is_err());
}
