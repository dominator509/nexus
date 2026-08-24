//! nexus-chaos: EP-040 M5 real bounded chaos live-fire (SPEC-008; M5 fence
//! tests/chaos/).
//!
//! M5 composes the whole EP-040 ladder into a final live-fire:
//! M1 contract (ChaosScenario, FailureInjectionKind, BlastRadius,
//! TestingErrorCode) -> M2 execution/evidence (FileEvidenceStore,
//! GateResult) -> M3 real provider transport (PostgresTransport) -> M4
//! security/hardware models (RuntimeToken, SecurityEvidenceStore,
//! HardwareCertifier) -> M5 real chaos injection with recovery
//! assertions and current-run evidence.
//!
//! Permanent invariants proven here (never weakened to close the node):
//! - CHAOS INJECTED != SYSTEM HARDENED
//! - NO FAILURE OBSERVED != RESILIENCE PROVEN
//! - RECOVERY ATTEMPTED != RECOVERED
//! - CLEANUP ATTEMPTED != RESOURCE CLEAN
//! - CHAOS INJECTION SUCCEEDED != RESILIENCE CERTIFIED
//! - FAILURE CLASSIFICATION must be typed, never collapsed into a
//!   generic shell exit 1.

pub mod engine;
pub mod evidence;
pub mod failure;
pub mod injection;
pub mod pressure;
pub mod scenario;

pub use engine::{ChaosEngine, ScenarioOutcome};
pub use evidence::{ChaosEvidenceStore, ChaosScenarioEvidence};
pub use failure::ChaosFailureClass;
pub use injection::{
    corrupt_evidence_bytes, revoke_runtime_credential, silent_peer_accept, terminate_and_recover,
    unavailable_port_probe,
};
pub use pressure::{probe_disk_pressure, PressureProbe};
pub use scenario::{chaos_scenarios, register_chaos_scenarios, ChaosScenarioId};
