//! Nexus identity domain layer (EP-003).
//!
//! Owns the provider-neutral identity model: principals, people,
//! households, businesses, devices, sessions, presence evidence, identity
//! confidence, interaction context, and privacy context (SPEC-001,
//! SPEC-005). This crate may import `nexus-domain` (typed IDs and canonical
//! vocabulary) and serde only. No infrastructure, database, network, or
//! vendor crate may be imported here (SPEC-001 acceptance: "infrastructure
//! crates cannot be imported by the domain crate"); the dependency-direction
//! tests enforce this boundary.
//!
//! INV-003: presence evidence and identity confidence are evidence, never
//! cryptographic authentication. No type in this crate can authorize an
//! R3/R4 action on its own.

#![forbid(unsafe_code)]

pub mod business;
pub mod device;
pub mod household;
pub mod interaction;
pub mod person;
pub mod presence;
pub mod principal;
pub mod privacy;
pub mod session;

pub use business::{BusinessBinding, BusinessRole};
pub use device::{DeviceIdentity, DeviceKind, TrustLevel};
pub use household::Household;
pub use interaction::InteractionContext;
pub use person::{LifecycleState, PersonProfile};
pub use presence::{ConfidenceLevel, EvidenceKind, IdentityConfidence, PresenceEvidence};
pub use principal::Principal;
pub use privacy::PrivacyContext;
pub use session::{Session, SessionState};
