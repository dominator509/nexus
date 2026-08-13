//! Nexus authentication domain layer (EP-007).
//!
//! Owns the provider-neutral authentication model: OIDC client and token
//! validation, passkey enrollment, device enrollment, auth sessions,
//! step-up challenges, and recovery kits (SPEC-005). This crate may
//! import `nexus-domain` (typed IDs and canonical vocabulary) and
//! `nexus-identity` (identity records it composes) plus serde only. No
//! infrastructure, database, network, or vendor crate may be imported
//! here (SPEC-001 acceptance: "infrastructure crates cannot be imported
//! by the domain crate"); the dependency-direction tests enforce this
//! boundary.
//!
//! INV-003 + SPEC-005 behavior 4: presence evidence and identity
//! confidence are evidence, never cryptographic authentication. Only
//! `StepUpChallenge` satisfaction (cryptographic) authorizes R3/R4.

#![forbid(unsafe_code)]

pub mod device;
pub mod oidc;
pub mod passkey;
pub mod recovery;
pub mod session;
pub mod step_up;
pub mod vocabulary;

pub use device::{DeviceEnrollment, DeviceEnrollmentError, VerificationEvidence};
pub use oidc::{
    GrantFlow, OidcClient, OidcError, ServiceIdentity, TokenClaims, TokenValidationOutcome,
    ValidatedToken,
};
pub use passkey::{
    PasskeyAssertion, PasskeyChallenge, PasskeyError, RegisteredCredential, RegistrationResponse,
};
pub use recovery::{RecoveryError, RecoveryKit, RecoveryKitState};
pub use session::{AuthSession, SessionAuditRecord, SessionServiceError};
pub use step_up::{StepUpChallenge, StepUpError, StepUpResponse};
pub use vocabulary::{
    AuthVocabularyError, AuthenticationStrength, DeviceEnrollmentState, PasskeyState,
    RecoveryMaterialKind, StepUpState, TokenClass,
};
