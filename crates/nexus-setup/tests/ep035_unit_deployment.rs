//! EP-035 M2 DeploymentChoice intent-only tests (SPEC-016).

use nexus_domain::CorrelationId;
use nexus_setup::{
    DeploymentIntentRecord, DeploymentMode, DeploymentProfile, DeploymentVerificationEvidence,
    DeploymentVerificationState, ProfileId, ReleaseChannel, SetupErrorCode,
};

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("00000000-0000-7000-8000-00000000000{n}")).unwrap()
}

fn profile(mode: DeploymentMode) -> DeploymentProfile {
    DeploymentProfile::new(
        ProfileId::new("profile-local").unwrap(),
        mode,
        ReleaseChannel::Stable,
        vec!["core".to_string(), "edge".to_string()],
        vec![serde_json::json!({"id": "home"})],
        serde_json::json!({"enabled": true}),
        serde_json::json!({"enabled": false}),
    )
    .unwrap()
}

#[test]
fn ep035_unit_deployment_selection_is_unverified_always() {
    let record =
        DeploymentIntentRecord::select(profile(DeploymentMode::FullyLocal), correlation(1), 1000);
    assert_eq!(
        record.verification.state,
        DeploymentVerificationState::Unverified
    );
    assert!(record.verification.evidence.is_none());
}

#[test]
fn ep035_unit_deployment_verified_requires_evidence() {
    let record =
        DeploymentIntentRecord::select(profile(DeploymentMode::FullyLocal), correlation(1), 1000);
    let err = record
        .clone()
        .set_verification(DeploymentVerificationState::Verified, None)
        .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Verification);
    let verified = record
        .set_verification(
            DeploymentVerificationState::Verified,
            Some(DeploymentVerificationEvidence {
                verified_at_unix_s: 1001,
                evidence_id: "ev-1".to_string(),
                verifier: "probe".to_string(),
            }),
        )
        .unwrap();
    assert_eq!(
        verified.verification.state,
        DeploymentVerificationState::Verified
    );
    assert_eq!(
        verified.verification.evidence.as_ref().unwrap().evidence_id,
        "ev-1"
    );
}

#[test]
fn ep035_unit_deployment_intent_never_claims_runtime_or_health() {
    let record =
        DeploymentIntentRecord::select(profile(DeploymentMode::Hybrid), correlation(1), 1000);
    let wire = serde_json::to_value(&record).unwrap();
    let obj = wire.as_object().unwrap();
    assert_eq!(obj["verification"]["state"], "UNVERIFIED");
    assert!(!obj.contains_key("healthy"));
    assert!(!obj.contains_key("running"));
    assert!(!obj.contains_key("reachable"));
}

#[test]
fn ep035_unit_deployment_verifying_and_failed_are_explicit() {
    let record =
        DeploymentIntentRecord::select(profile(DeploymentMode::FullyLocal), correlation(1), 1000);
    let verifying = record
        .clone()
        .set_verification(DeploymentVerificationState::Verifying, None)
        .unwrap();
    assert_eq!(
        verifying.verification.state,
        DeploymentVerificationState::Verifying
    );
    let failed = record
        .set_verification(DeploymentVerificationState::Failed, None)
        .unwrap();
    assert_eq!(
        failed.verification.state,
        DeploymentVerificationState::Failed
    );
}

#[test]
fn ep035_unit_deployment_evidence_only_valid_for_verified() {
    let record =
        DeploymentIntentRecord::select(profile(DeploymentMode::FullyLocal), correlation(1), 1000);
    let err = record
        .set_verification(
            DeploymentVerificationState::Verifying,
            Some(DeploymentVerificationEvidence {
                verified_at_unix_s: 1001,
                evidence_id: "ev-1".to_string(),
                verifier: "probe".to_string(),
            }),
        )
        .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);
}

#[test]
fn ep035_unit_deployment_round_trips_serialization() {
    let record =
        DeploymentIntentRecord::select(profile(DeploymentMode::Managed), correlation(1), 1000);
    let wire = serde_json::to_string(&record).unwrap();
    let parsed: DeploymentIntentRecord = serde_json::from_str(&wire).unwrap();
    assert_eq!(parsed.profile.mode, DeploymentMode::Managed);
    assert_eq!(
        parsed.verification.state,
        DeploymentVerificationState::Unverified
    );
}

#[test]
fn ep035_unit_deployment_rejects_unknown_wire_fields() {
    let record =
        DeploymentIntentRecord::select(profile(DeploymentMode::FullyLocal), correlation(1), 1000);
    let mut value = serde_json::to_value(&record).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("forged".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<DeploymentIntentRecord>(value).is_err());
}

#[test]
fn ep035_unit_deployment_rejects_bad_profile_shape() {
    let err = DeploymentProfile::new(
        ProfileId::new("p").unwrap(),
        DeploymentMode::Byoc,
        ReleaseChannel::Beta,
        vec![],
        vec![],
        serde_json::json!([1, 2]),
        serde_json::json!({}),
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);
}
