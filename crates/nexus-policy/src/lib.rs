//! Nexus authorization policy domain (EP-008).
//!
//! Owns the provider-neutral authorization model: relationship
//! authorization (OpenFGA tuple shape), contextual policy evaluation
//! (OPA decision shape), risk classification, short-lived capability
//! grants, approval assertions, the deterministic action gateway
//! contract, action receipts, and verification plans (SPEC-005,
//! SPEC-006). This crate may import `nexus-domain` (typed IDs and
//! canonical vocabulary), `nexus-identity` (principals and devices),
//! and `nexus-auth` (authentication strength) plus serde only. No
//! infrastructure, database, network, or vendor crate may be imported
//! here (SPEC-001 acceptance: "infrastructure crates cannot be imported
//! by the domain crate"); the dependency-direction tests enforce this
//! boundary.
//!
//! INV-003 + SPEC-005 behavior 4: R3 and R4 actions require a
//! cryptographic step-up or explicit preauthorization; R4 never accepts
//! model approval. Presence evidence and identity confidence are
//! evidence, never authorization on their own.

#![forbid(unsafe_code)]

pub mod approval;
pub mod capability;
pub mod error;
pub mod gateway;
pub mod policy;
pub mod receipt;
pub mod relationship;
pub mod risk;
pub mod verification;
pub mod vocabulary;

pub use approval::{ApprovalAssertion, ApprovalAssertionError, ApprovalDecision};
pub use capability::{CapabilityGrant, CapabilityGrantError, GrantState};
pub use error::{PolicyError, PolicyErrorCode};
pub use gateway::{ActionDecision, ActionGateway, ActionRequest, ActionRequestError, DenialReason};
pub use policy::{ContextPolicyEngine, PolicyDecision, PolicyInput};
pub use receipt::{ActionReceipt, ReceiptError, ReceiptState};
pub use relationship::{
    RelationshipAuthorizer, RelationshipDecision, RelationshipError, RelationshipTuple,
};
pub use risk::{RiskAssessmentInput, RiskClass, RiskClassifier, RiskClassifierError};
pub use verification::{ExpectedState, VerificationPlan, VerificationResult, Verifier};
pub use vocabulary::{ActionLifecycleState, PolicyVocabularyError};

#[cfg(test)]
mod lib_tests;
