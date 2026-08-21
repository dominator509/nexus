//! EP-035 Nexus Setup Wizard and Onboarding behavior (SPEC-004 /
//! SPEC-016).
//!
//! Provider-neutral setup behavior: SetupWizard, DeploymentChoice,
//! HardwareProfiler, OwnerBootstrap, EdgeEnrollment, DiscoveryWizard,
//! IntegrationCard, and RecoveryFlow. State truthfulness is structural
//! - SELECTED != PROVISIONED, CONFIGURED != HEALTHY, DISCOVERED !=
//!   TRUSTED, COMPLETE_LOCAL != VERIFIED, OWNER_DETAILS !=
//!   OWNER_AUTHORIZED, AMBIGUOUS_MUTATION != SAFE_TO_RETRY - and invalid
//!   states fail closed.
//!
//! Dependency direction: this crate depends only on nexus-domain and
//! serde/serde_json. Provider implementations never appear here.

#![forbid(unsafe_code)]

pub mod error;
pub mod model;
pub mod port;
pub mod vocabulary;
pub mod wizard;

pub use error::{SetupError, SetupErrorCode, SetupResult};
pub use model::{
    decide_recovery, is_valid_integration_transition, resolve_first_owner, CredentialId,
    CredentialKind, DeploymentIntentRecord, DeploymentProfile, DeploymentVerification,
    DeploymentVerificationEvidence, DiscoveryObservation, DiscoveryReport, EdgeEnrollmentRequest,
    EnrollmentCredential, EnrollmentId, FirstOwnerDecision, FirstOwnerRecord,
    HardwareCapabilityDeclaration, HardwareFact, HardwareProfile, HardwareValue, IntegrationCard,
    IntegrationId, IntegrationSelection, ObservationId, OwnerBootstrapRequest, ProfileId,
    RecoveryDecision, RecoveryEvidence, RecoveryKit, RecoveryKitId, RedactedEnrollmentCredential,
};
pub use nexus_domain::{CorrelationId, PersonId, TenantId};
pub use port::{
    DeploymentChoicePort, DiscoveryWizardPort, EdgeEnrollmentPort, HardwareProfilerPort,
    IntegrationCardPort, OwnerBootstrapPort, RecoveryFlowPort, SetupWizardPort,
};
pub use vocabulary::{
    contains_hostile_authority_token, CapabilityCertificationState, DeploymentMode,
    DeploymentVerificationState, DiscoveryKind, EnrollmentCredentialState, EnrollmentTrustState,
    HardwareProvenance, IntegrationStatus, OwnerBootstrapState, RecoveryFailureClass,
    RecoveryMaterialKind, RecoveryMutationState, RecoveryOutcome, ReleaseChannel, WizardState,
    WizardStep, WizardStepStatus, HOSTILE_AUTHORITY_TOKENS,
};
pub use wizard::{
    is_valid_step_transition, is_valid_wizard_transition, RemoteVerification, SetupWizardState,
    WizardStepRecord, ALL_WIZARD_STEPS,
};
