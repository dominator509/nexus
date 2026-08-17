//! EP-024 provider-neutral device contracts (SPEC-011 behaviors 5-7).
//!
//! Nexus devices are the provider-neutral plane for media, appliances,
//! irrigation, vacuums, and future robots. Home Assistant is the
//! preferred provider for commodity devices; direct providers exist
//! only for capability or reliability gaps. Commands are target-scoped
//! and verified. Future robots receive no broader authority than
//! declared capabilities.
//!
//! Permanent invariants (SPEC-011):
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.
//! - A provider advertises only capabilities proven through supported
//!   or observed authenticated paths; unbound providers fail closed and
//!   never fabricate devices, states, or events (Reality rule).
//! - Verification binds to the exact target device and the requested
//!   action's expected result; an unrelated change never satisfies
//!   verification.
//! - Robot capabilities declare physical workspace, speed, force,
//!   safety interlocks, emergency stop, human presence, and approval
//!   class before activation (behavior 6).
//! - Offline edge operation permits only cached low-risk capabilities
//!   and queues canonical synchronization (behavior 7).
//! - Capability mapping is deterministic: provider domain names are
//!   normalized at the infrastructure boundary and never become domain
//!   contracts.

#![forbid(unsafe_code)]

pub mod error;
pub mod mapper;
pub mod provider;
pub mod robot;
pub mod verifier;
pub mod vocabulary;

pub use error::{DevicesError, DevicesErrorCode};
pub use mapper::DeviceCapabilityMapper;
pub use provider::{
    ApplianceProvider, IrrigationProvider, MediaProvider, RobotProvider, VacuumProvider,
};
pub use robot::RobotSafetyDeclaration;
pub use verifier::{DeviceCommandVerifier, DeviceStateObservation, VerificationOutcome};
pub use vocabulary::{
    ApplianceCapability, ApplianceDeviceId, DeviceAvailability, DeviceClass, IrrigationCapability,
    IrrigationZoneId, MediaCapability, MediaDeviceId, RobotCapability, RobotId, VacuumCapability,
    VacuumDeviceId,
};

// Re-export canonical ids and vocabulary from nexus-domain so callers
// have a single import surface and locked names are never redefined.
pub use nexus_domain::{
    ApprovalClass, CapabilityClass, CorrelationId, DeviceId, Idempotency, PersonId, Risk, TenantId,
};
