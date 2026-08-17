//! EP-024 irrigation connector (SPEC-011; M4).
//!
//! Provider-neutral irrigation adapter core behind the nexus-devices
//! `IrrigationProvider` port, composed through the EP-020-certified
//! Home Assistant provider boundary (`nexus-home-assistant` owns HA
//! authentication, the REST surface, and transport semantics; this
//! crate owns irrigation semantics only).
//!
//! M4 owns forced failures, abuse cases, and observability:
//!   - unavailable dependency (real container stop);
//!   - duplicate request (in-flight idempotency Conflict);
//!   - denied permission (capability gate Policy before provider
//!     mutation);
//!   - malformed input (real provider rejection);
//!   - partial side effect (one zone failing never mutates another);
//!   - bounded recovery (in-flight entries released on failure);
//!   - bounded redacted audit ring + counters + correlation.
//!
//! Permanent invariants (mirroring M2/M3):
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.
//! - Unknown zones are NotFound, never Verified and never benign.
//! - Unsupported capabilities fail closed (Policy) before any provider
//!   service call.
//! - Provider-unavailable zones map to UNAVAILABLE, never OFF.
//! - Stable zone identity derives from the provider entity id, never
//!   enumeration order.
//! - Device capability discovered != principal authorized (EP-008 owns
//!   authorization).
//!
//! Classification:
//! - EP-024 irrigation adapter: REAL_PRODUCTION_IMPLEMENTATION
//! - Home Assistant provider dependency: PROVIDER_CERTIFIED via EP-020
//!   + M4 composition proof (this crate reuses the certified transport)
//! - controlled zone fixture: CONTROLLED_TEST_FIXTURE
//! - physical irrigation hardware: NOT ASSERTED / DEFERRED

#![forbid(unsafe_code)]

pub mod adapter;
pub mod error;
pub mod observability;
pub mod transport;

pub use adapter::{
    capabilities_for, has_zone_control, irrigation_zone_id, stable_zone_id, zone_state_value,
    IrrigationAdapter, IrrigationZoneSelector,
};
pub use error::{IrrigationError, IrrigationErrorCode};
pub use observability::{IrrigationAuditEntry, IrrigationObservability};
pub use transport::{
    HaIrrigationTransport, IrrigationCommand, IrrigationCommandReceipt, IrrigationCommandState,
    IrrigationTransport, IrrigationZone, IrrigationZoneState,
};
