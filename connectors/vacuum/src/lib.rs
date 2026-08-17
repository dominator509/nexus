//! EP-024 M5 vacuum connector (SPEC-011; M5).
//!
//! Provider-neutral vacuum adapter core behind the nexus-devices
//! `VacuumProvider` port, composed through the EP-020-certified Home
//! Assistant provider boundary (`nexus-home-assistant` owns HA
//! authentication, the REST surface, and transport semantics; this
//! crate owns vacuum semantics only).
//!
//! M5 owns the REAL vacuum provider live-fire and node closure:
//!   - real capability discovery from provider feature bits (never
//!     assumed);
//!   - real StartClean -> CLEANING, Pause -> PAUSED, ReturnHome ->
//!     RETURNING -> DOCKED transitions through real provider actions
//!     and exact-target readback;
//!   - Dock/ReturnHome distinct Nexus capabilities mapping to the SAME
//!     provider action (vacuum.return_to_base) - explicit mapping;
//!   - MapReadback REAL data only (never fabricated); safe metadata
//!     only (digest/dimensions/reference), never raw household imagery;
//!   - no blind retry of ambiguous physical commands (UNKNOWN OUTCOME
//!     -> VERIFY FIRST);
//!   - bounded redacted audit ring + counters + correlation;
//!   - bounded recovery + ops diagnostic.
//!
//! Permanent invariants (mirroring M2/M3/M4):
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.
//! - Unknown vacuums are NotFound, never Verified and never benign.
//! - Unsupported capabilities fail closed (Policy) before any provider
//!   service call.
//! - Provider-unavailable/unknown states are never mapped to a safe
//!   state (DOCKED/IDLE/SAFE/COMPLETED).
//! - Stable vacuum identity derives from the provider entity id, never
//!   enumeration order.
//! - Device capability discovered != principal authorized (EP-008 owns
//!   authorization; a valid HA credential is infrastructure access
//!   only and never grants cleaning/map/robot authority).
//! - RobotProvider authority is never widened by vacuum support.
//!
//! Classification:
//! - EP-024 vacuum adapter: REAL_PRODUCTION_IMPLEMENTATION
//! - Home Assistant provider dependency: PROVIDER_CERTIFIED via EP-020
//!   + M5 composition proof (this crate reuses the certified transport)
//! - controlled vacuum fixture: CONTROLLED_TEST_FIXTURE
//! - physical robot vacuum / SLAM map: NOT ASSERTED / DEFERRED
//! - map provider path: NOT CERTIFIED (no real map surface on the
//!   controlled fixture; never fabricated)

#![forbid(unsafe_code)]

pub mod adapter;
pub mod error;
pub mod observability;
pub mod transport;

pub use adapter::{
    capabilities_for, has_real_map_surface, stable_vacuum_id, vacuum_device_id, vacuum_state_value,
    VacuumAdapter, VacuumDeviceSelector, VacuumMapMetadata,
};
pub use error::{VacuumError, VacuumErrorCode};
pub use observability::{VacuumAuditEntry, VacuumObservability};
pub use transport::{
    HaVacuumTransport, VacuumActivityState, VacuumCommand, VacuumCommandReceipt,
    VacuumCommandState, VacuumDevice, VacuumTransport,
};
