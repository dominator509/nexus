//! EP-035 provider-neutral Setup Wizard and Onboarding ports.
//!
//! M2 declares the eight ports; infrastructure adapters implement them
//! in later milestones (M3 real dependencies, M4 forced failures, M5
//! live-fire). Domain rules stay pure and I/O moves behind these ports.

use nexus_domain::{CorrelationId, PersonId};

use crate::error::SetupResult;
use crate::model::{
    DeploymentIntentRecord, DeploymentProfile, DiscoveryReport, EnrollmentCredential,
    FirstOwnerDecision, FirstOwnerRecord, HardwareProfile, IntegrationCard, IntegrationSelection,
    OwnerBootstrapRequest, RecoveryDecision, RecoveryEvidence, RecoveryKit,
};
use crate::vocabulary::{
    DeploymentVerificationState, IntegrationStatus, WizardStep, WizardStepStatus,
};
use crate::wizard::{RemoteVerification, SetupWizardState};

/// SetupWizard port: models state, not visual progress.
pub trait SetupWizardPort {
    fn begin(&self, correlation: CorrelationId) -> SetupResult<SetupWizardState>;
    fn advance(
        &self,
        state: &SetupWizardState,
        to_state: crate::vocabulary::WizardState,
    ) -> SetupResult<SetupWizardState>;
    fn advance_step(
        &self,
        state: &SetupWizardState,
        step: WizardStep,
        to_status: WizardStepStatus,
    ) -> SetupResult<SetupWizardState>;
    fn verify_remote(
        &self,
        state: &SetupWizardState,
        step: WizardStep,
        verification: RemoteVerification,
    ) -> SetupResult<SetupWizardState>;
}

/// DeploymentChoice port: selection is intent only.
pub trait DeploymentChoicePort {
    fn select(
        &self,
        profile: DeploymentProfile,
        correlation: CorrelationId,
        selected_at_unix_s: u64,
    ) -> SetupResult<DeploymentIntentRecord>;
    fn verify(
        &self,
        record: &DeploymentIntentRecord,
        state: DeploymentVerificationState,
        evidence: Option<crate::model::DeploymentVerificationEvidence>,
    ) -> SetupResult<DeploymentIntentRecord>;
}

/// HardwareProfiler port: facts carry provenance; declarations never
/// certify without measured evidence.
pub trait HardwareProfilerPort {
    fn profile(&self, correlation: CorrelationId) -> SetupResult<HardwareProfile>;
}

/// OwnerBootstrap port: security-critical first-owner ladder with
/// deterministic idempotent/conflict semantics.
pub trait OwnerBootstrapPort {
    fn initialize(
        &self,
        request: &OwnerBootstrapRequest,
        known: Option<&FirstOwnerRecord>,
        principal_id: PersonId,
    ) -> SetupResult<FirstOwnerDecision>;
}

/// EdgeEnrollment port: trust layers; BootstrapToken secrets never
/// leak through any surface.
pub trait EdgeEnrollmentPort {
    fn request_enrollment(&self, request: crate::model::EdgeEnrollmentRequest) -> SetupResult<()>;
    fn issue_credential(
        &self,
        credential: EnrollmentCredential,
    ) -> SetupResult<EnrollmentCredential>;
}

/// DiscoveryWizard port: observations are data, never authority.
pub trait DiscoveryWizardPort {
    fn observe(&self, correlation: CorrelationId) -> SetupResult<DiscoveryReport>;
    fn select(&self, selection: IntegrationSelection) -> SetupResult<IntegrationSelection>;
}

/// IntegrationCard port: truthful configuration-versus-health status.
pub trait IntegrationCardPort {
    fn create(&self, card: IntegrationCard) -> SetupResult<IntegrationCard>;
    fn transition(
        &self,
        card: &IntegrationCard,
        to_status: IntegrationStatus,
        at_unix_s: u64,
    ) -> SetupResult<IntegrationCard>;
}

/// RecoveryFlow port: no blind replay after ambiguous mutation.
pub trait RecoveryFlowPort {
    fn decide(&self, evidence: &RecoveryEvidence) -> SetupResult<RecoveryDecision>;
    fn issue_recovery_kit(&self, kit: RecoveryKit) -> SetupResult<RecoveryKit>;
}
