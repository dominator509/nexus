//! EP-035 M2 SetupWizard state-machine tests (SPEC-004/016).

use nexus_domain::CorrelationId;
use nexus_setup::{
    RemoteVerification, SetupErrorCode, SetupWizardState, WizardState, WizardStep, WizardStepStatus,
};

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("00000000-0000-7000-8000-00000000000{n}")).unwrap()
}

#[test]
fn ep035_unit_wizard_begins_not_started_with_all_steps_pending() {
    let wizard = SetupWizardState::not_started(correlation(1), 1000);
    assert_eq!(wizard.state, WizardState::NotStarted);
    assert_eq!(wizard.steps.len(), 8);
    for record in &wizard.steps {
        assert_eq!(record.status, WizardStepStatus::Pending);
    }
    wizard.validate().unwrap();
}

#[test]
fn ep035_unit_wizard_accepts_canonical_start() {
    let started = SetupWizardState::not_started(correlation(1), 1000)
        .advance(WizardState::InProgress, 1001)
        .unwrap();
    assert_eq!(started.state, WizardState::InProgress);
}

#[test]
fn ep035_unit_wizard_rejects_not_started_to_completed_leap() {
    let wizard = SetupWizardState::not_started(correlation(1), 1000);
    let err = wizard.advance(WizardState::Completed, 1001).unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Policy);
}

#[test]
fn ep035_unit_wizard_rejects_failed_to_completed_without_recovery() {
    let wizard = SetupWizardState::not_started(correlation(1), 1000)
        .advance(WizardState::InProgress, 1001)
        .unwrap()
        .advance(WizardState::Failed, 1002)
        .unwrap();
    assert!(wizard.advance(WizardState::Completed, 1003).is_err());
}

#[test]
fn ep035_unit_wizard_cannot_complete_with_unverified_steps() {
    let wizard = SetupWizardState::not_started(correlation(1), 1000)
        .advance(WizardState::InProgress, 1001)
        .unwrap();
    let err = wizard.advance(WizardState::Completed, 1002).unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Policy);
}

#[test]
fn ep035_unit_wizard_complete_local_is_never_verified() {
    let wizard = SetupWizardState::not_started(correlation(1), 1000)
        .advance(WizardState::InProgress, 1001)
        .unwrap()
        .advance_step(
            WizardStep::DeploymentChoice,
            WizardStepStatus::InProgress,
            1002,
            None,
        )
        .unwrap()
        .advance_step(
            WizardStep::DeploymentChoice,
            WizardStepStatus::CompleteLocal,
            1003,
            None,
        )
        .unwrap();
    let record = wizard.step_record(WizardStep::DeploymentChoice).unwrap();
    assert_eq!(record.status, WizardStepStatus::CompleteLocal);
    assert!(record.verification.is_none());
    // COMPLETE_LOCAL -> VERIFIED without a record is rejected.
    let err = wizard
        .advance_step(
            WizardStep::DeploymentChoice,
            WizardStepStatus::Verified,
            1004,
            None,
        )
        .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Verification);
}

#[test]
fn ep035_unit_wizard_verified_requires_remote_verification_record() {
    let wizard = SetupWizardState::not_started(correlation(1), 1000)
        .advance(WizardState::InProgress, 1001)
        .unwrap()
        .advance_step(
            WizardStep::DeploymentChoice,
            WizardStepStatus::InProgress,
            1002,
            None,
        )
        .unwrap()
        .advance_step(
            WizardStep::DeploymentChoice,
            WizardStepStatus::CompleteLocal,
            1003,
            None,
        )
        .unwrap()
        .verify_step(
            WizardStep::DeploymentChoice,
            RemoteVerification {
                verified_at_unix_s: 1005,
                verifier: "setup-probe".to_string(),
            },
        )
        .unwrap();
    let record = wizard.step_record(WizardStep::DeploymentChoice).unwrap();
    assert_eq!(record.status, WizardStepStatus::Verified);
    assert_eq!(
        record.verification.as_ref().unwrap().verifier,
        "setup-probe"
    );
}

#[test]
fn ep035_unit_wizard_rejects_verification_record_on_non_verified_step() {
    let wizard = SetupWizardState::not_started(correlation(1), 1000)
        .advance(WizardState::InProgress, 1001)
        .unwrap()
        .advance_step(
            WizardStep::DeploymentChoice,
            WizardStepStatus::InProgress,
            1002,
            None,
        )
        .unwrap();
    let err = wizard
        .advance_step(
            WizardStep::DeploymentChoice,
            WizardStepStatus::InProgress,
            1003,
            Some(RemoteVerification {
                verified_at_unix_s: 1003,
                verifier: "probe".to_string(),
            }),
        )
        .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);
}

#[test]
fn ep035_unit_wizard_completes_only_when_every_step_verified() {
    let mut wizard = SetupWizardState::not_started(correlation(1), 1000)
        .advance(WizardState::InProgress, 1001)
        .unwrap();
    for step in [
        WizardStep::DeploymentChoice,
        WizardStep::HardwareProfile,
        WizardStep::OwnerBootstrap,
        WizardStep::RecoveryMaterial,
        WizardStep::EdgeEnrollment,
        WizardStep::Discovery,
        WizardStep::IntegrationReview,
        WizardStep::PlanReview,
    ] {
        wizard = wizard
            .advance_step(step, WizardStepStatus::InProgress, 1002, None)
            .unwrap()
            .advance_step(step, WizardStepStatus::CompleteLocal, 1003, None)
            .unwrap()
            .verify_step(
                step,
                RemoteVerification {
                    verified_at_unix_s: 1004,
                    verifier: "probe".to_string(),
                },
            )
            .unwrap();
    }
    let completed = wizard.advance(WizardState::Completed, 1005).unwrap();
    assert_eq!(completed.state, WizardState::Completed);
}

#[test]
fn ep035_unit_wizard_round_trips_serialization() {
    let wizard = SetupWizardState::not_started(correlation(1), 1000)
        .advance(WizardState::InProgress, 1001)
        .unwrap()
        .advance_step(
            WizardStep::DeploymentChoice,
            WizardStepStatus::InProgress,
            1002,
            None,
        )
        .unwrap();
    let wire = serde_json::to_string(&wizard).unwrap();
    let parsed: SetupWizardState = serde_json::from_str(&wire).unwrap();
    assert_eq!(parsed.state, WizardState::InProgress);
    assert_eq!(
        parsed
            .step_record(WizardStep::DeploymentChoice)
            .unwrap()
            .status,
        WizardStepStatus::InProgress
    );
}

#[test]
fn ep035_unit_wizard_rejects_unknown_wire_field() {
    let wizard = SetupWizardState::not_started(correlation(1), 1000);
    let mut value = serde_json::to_value(&wizard).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("forged".to_string(), serde_json::json!(true));
    let err = serde_json::from_value::<SetupWizardState>(value).unwrap_err();
    assert!(err.to_string().contains("forged"));
}
