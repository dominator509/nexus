//! EP-020 Home Assistant provider contracts (SPEC-011; ADR-027).
//!
//! Provider-neutral home plane: canonical device twins, intents, the
//! deterministic local fast path, exact-target state verification, and
//! automation handoff. Home Assistant is the primary home control
//! provider; the concrete adapter lives behind the `HomeProvider` /
//! `HomeAssistantProvider` ports (connector boundary in M2).
//!
//! Permanent invariants:
//!
//! - COMMAND ACCEPTED != DEVICE CHANGED != DEVICE VERIFIED.
//! - Physical device commands never use `POST /api/states/<entity_id>`;
//!   they use the real HA service/action mechanism.
//! - Unrelated state changes never satisfy verification.
//! - Unknown/unavailable remains unknown.
//! - The model may propose device/action/parameters; it can never call
//!   Home Assistant directly outside the Action Gateway.

#![forbid(unsafe_code)]

pub mod contract;
pub mod error;
pub mod mapping;
pub mod vocabulary;

pub use contract::{
    AreaId, AutomationCondition, AutomationHandle, AutomationSpec, AutomationStatus,
    AutomationTrigger, CommandReceipt, DeviceCapability, DeviceTwin, HaDeviceRef, HaEntityRef,
    HomeAssistantProvider, HomeIntent, HomeProvider, StateObservation, StateVerifier,
    StateVerifierAdapter, VerificationRule,
};
pub use error::{HomeError, HomeErrorCode};
pub use mapping::{canonical_action, category_from_provider_domain, is_strong_provider_identity};
pub use vocabulary::{
    CommandState, DeviceCategory, EntityAvailability, FastPathDecision, ProviderConnectionState,
    VerificationOutcome,
};

// Re-export canonical ids and vocabulary from nexus-domain so callers
// have a single import surface and locked names are never redefined.
pub use nexus_domain::{
    ApprovalClass, CapabilityClass, CorrelationId, DeviceId, Idempotency, PersonId, Risk, TenantId,
};
