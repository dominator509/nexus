//! EP-035 M2 RecoveryFlow no-blind-replay tests (SPEC-004/016).

use nexus_domain::{CorrelationId, PersonId, TenantId};
use nexus_setup::{
    decide_recovery, RecoveryDecision, RecoveryEvidence, RecoveryFailureClass, RecoveryKit,
    RecoveryKitId, RecoveryMaterialKind, RecoveryMutationState, RecoveryOutcome, SetupErrorCode,
};

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("00000000-0000-7000-8000-00000000000{n}")).unwrap()
}

fn evidence(
    failure_class: RecoveryFailureClass,
    mutation_known: bool,
    mutation_occurred: Option<bool>,
    mutation_state: Option<RecoveryMutationState>,
) -> RecoveryEvidence {
    RecoveryEvidence {
        failure_class,
        mutation_known,
        mutation_occurred,
        mutation_state,
        correlation: None,
    }
}

#[test]
fn ep035_unit_recovery_ambiguous_forces_reconcile_never_blind_retry() {
    let decision = decide_recovery(&evidence(
        RecoveryFailureClass::Ambiguous,
        false,
        None,
        None,
    ));
    assert_eq!(decision.outcome, RecoveryOutcome::Reconcile);
    assert!(!decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_ambiguous_retry_safe_only_after_reconcile() {
    // AUD-045: safe retry requires BOTH reconciliation AND an explicit
    // negative mutation observation.
    let decision = decide_recovery(&evidence(
        RecoveryFailureClass::Ambiguous,
        true,
        Some(false),
        Some(RecoveryMutationState::Reconciled),
    ));
    assert_eq!(decision.outcome, RecoveryOutcome::Retryable);
    assert!(decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_ambiguous_reconciled_with_mutation_occurred_not_retry_safe() {
    // AUD-045 hostile: AMBIGUOUS + RECONCILED where the mutation DID
    // occur must NOT be retried (duplicate consequential effect).
    let decision = decide_recovery(&evidence(
        RecoveryFailureClass::Ambiguous,
        true,
        Some(true),
        Some(RecoveryMutationState::Reconciled),
    ));
    assert_eq!(decision.outcome, RecoveryOutcome::Reconcile);
    assert!(!decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_ambiguous_reconciled_without_observation_not_retry_safe() {
    // AUD-045 hostile: AMBIGUOUS + RECONCILED with NO explicit negative
    // mutation observation must not be retry-safe.
    let decision = decide_recovery(&evidence(
        RecoveryFailureClass::Ambiguous,
        false,
        None,
        Some(RecoveryMutationState::Reconciled),
    ));
    assert_eq!(decision.outcome, RecoveryOutcome::Reconcile);
    assert!(!decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_timeout_with_unknown_mutation_never_blind_retried() {
    let decision = decide_recovery(&evidence(RecoveryFailureClass::Timeout, false, None, None));
    assert_eq!(decision.outcome, RecoveryOutcome::Reconcile);
    assert!(!decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_timeout_with_known_no_mutation_is_safe_retry() {
    let decision = decide_recovery(&evidence(
        RecoveryFailureClass::Timeout,
        true,
        Some(false),
        None,
    ));
    assert_eq!(decision.outcome, RecoveryOutcome::Retryable);
    assert!(decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_validation_is_non_retryable() {
    let decision = decide_recovery(&evidence(
        RecoveryFailureClass::Validation,
        true,
        None,
        None,
    ));
    assert_eq!(decision.outcome, RecoveryOutcome::NonRetryable);
    assert!(!decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_authorization_requires_reauthentication() {
    let decision = decide_recovery(&evidence(
        RecoveryFailureClass::Authorization,
        true,
        None,
        None,
    ));
    assert_eq!(decision.outcome, RecoveryOutcome::Reauthenticate);
    assert!(!decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_conflict_resumes_checkpoint() {
    let decision = decide_recovery(&evidence(RecoveryFailureClass::Conflict, true, None, None));
    assert_eq!(decision.outcome, RecoveryOutcome::ResumeCheckpoint);
    assert!(!decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_internal_requires_manual_intervention() {
    let decision = decide_recovery(&evidence(RecoveryFailureClass::Internal, true, None, None));
    assert_eq!(decision.outcome, RecoveryOutcome::ManualIntervention);
    assert!(!decision.retry_safe);
}

#[test]
fn ep035_unit_recovery_kit_binds_canonical_schema() {
    let kit = RecoveryKit::new(
        RecoveryKitId::new("kit-1").unwrap(),
        PersonId::new("00000000-0000-7000-8000-000000000011").unwrap(),
        TenantId::new("00000000-0000-7000-8000-000000000012").unwrap(),
        RecoveryMaterialKind::RecoveryCodes,
        1000,
        2000,
        correlation(1),
    )
    .unwrap();
    assert_eq!(kit.material_kind, RecoveryMaterialKind::RecoveryCodes);
    assert!(kit.is_expired(3000));
    assert!(!kit.is_expired(1500));
    let wire = serde_json::to_value(&kit).unwrap();
    assert_eq!(wire["material_kind"], "RECOVERY_CODES");
}

#[test]
fn ep035_unit_recovery_kit_rejects_invalid_window_and_unknown() {
    let err = RecoveryKit::new(
        RecoveryKitId::new("kit-1").unwrap(),
        PersonId::new("00000000-0000-7000-8000-000000000011").unwrap(),
        TenantId::new("00000000-0000-7000-8000-000000000012").unwrap(),
        RecoveryMaterialKind::DeviceBackup,
        2000,
        1000,
        correlation(1),
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);

    let kit = RecoveryKit::new(
        RecoveryKitId::new("kit-1").unwrap(),
        PersonId::new("00000000-0000-7000-8000-000000000011").unwrap(),
        TenantId::new("00000000-0000-7000-8000-000000000012").unwrap(),
        RecoveryMaterialKind::RecoveryCodes,
        1000,
        2000,
        correlation(1),
    )
    .unwrap();
    let mut value = serde_json::to_value(&kit).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("forged".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<RecoveryKit>(value).is_err());
}

#[test]
fn ep035_unit_recovery_decision_serializes_structurally() {
    let decision = decide_recovery(&evidence(
        RecoveryFailureClass::Ambiguous,
        false,
        None,
        None,
    ));
    let wire = serde_json::to_value(&decision).unwrap();
    assert_eq!(wire["outcome"], "RECONCILE");
    assert_eq!(wire["retry_safe"], false);
    let parsed: RecoveryDecision = serde_json::from_value(wire).unwrap();
    assert_eq!(parsed.outcome, RecoveryOutcome::Reconcile);
}
