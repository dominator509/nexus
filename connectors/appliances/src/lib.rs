//! EP-024 appliance connector (SPEC-011; M3).
//!
//! Provider-neutral appliance adapter core behind the nexus-devices
//! `ApplianceProvider` port, composed through the EP-020-certified
//! Home Assistant provider boundary (`nexus-home-assistant` owns HA
//! authentication, the REST surface, and transport semantics; this
//! crate owns appliance semantics only).
//!
//! Permanent invariants:
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.
//! - Unknown targets are NotFound, never Verified and never benign.
//! - Unsupported capabilities fail closed (Policy) before any provider
//!   service call.
//! - Provider-unavailable entities map to UNAVAILABLE, never OFF.
//! - Stable appliance identity derives from the provider entity id,
//!   never enumeration order.
//! - Device capability discovered != principal authorized (EP-008 owns
//!   authorization).
//! - Robot authority is never widened by this or any other device
//!   class.
//!
//! Classification:
//! - EP-024 appliance adapter: REAL_PRODUCTION_IMPLEMENTATION
//! - Home Assistant provider dependency: PROVIDER_CERTIFIED via EP-020
//!   + M3 composition proof (this crate reuses the certified transport)
//! - controlled switch/fan fixture: CONTROLLED_TEST_FIXTURE
//! - physical appliance hardware: NOT ASSERTED / DEFERRED

#![forbid(unsafe_code)]

pub mod adapter;
pub mod error;
pub mod mapping;
pub mod transport;

pub use adapter::{appliance_device_id, read_state, ApplianceAdapter};
pub use error::{ApplianceError, ApplianceErrorCode};
pub use mapping::{capabilities_for, stable_appliance_id, ApplianceSelector};
pub use transport::{
    ApplianceCommand, ApplianceCommandReceipt, ApplianceCommandState, ApplianceEntity,
    ApplianceState, ApplianceTransport, HaApplianceTransport,
};
