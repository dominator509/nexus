//! Nexus pure domain layer (EP-002).
//!
//! Owns the canonical typed identifiers and vocabulary tables from SPEC-001,
//! SPEC-003, and SPEC-022. This crate is intentionally dependency-light:
//! only serde for serialization. No infrastructure, database, network, or
//! vendor crate may be imported here (SPEC-001 acceptance: "infrastructure
//! crates cannot be imported by the domain crate"). `cargo tree` and the
//! dependency-direction tests enforce this boundary.

#![forbid(unsafe_code)]

pub mod id;
pub mod vocabulary;

pub use id::{
    ArtifactId, BusinessId, CapabilityId, CorrelationId, DeviceId, EventId, HouseholdId, IdError,
    NexusId, ObjectiveId, PersonId, SkillId, TaskId, TenantId,
};
pub use vocabulary::{
    ApprovalClass, Availability, CapabilityClass, Idempotency, Locality, MemoryType,
    NotificationChannel, PrincipalType, Privacy, Reversal, Risk, Route, Tier,
};
