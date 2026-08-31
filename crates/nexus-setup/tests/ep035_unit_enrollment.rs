//! EP-035 M2 EdgeEnrollment trust-layer and secret-redaction tests.

use nexus_domain::CorrelationId;
use nexus_setup::{
    CredentialId, EdgeEnrollmentRequest, EnrollmentCredential, EnrollmentCredentialState,
    SetupErrorCode,
};

fn correlation(n: u8) -> CorrelationId {
    CorrelationId::new(format!("00000000-0000-7000-8000-00000000000{n}")).unwrap()
}

fn credential_id(n: u8) -> CredentialId {
    CredentialId::new(format!("cred-{n}")).unwrap()
}

const SECRET_CANARY: &str = "bootstrap-secret-canary-9f2c";

fn credential(overrides_state: Option<EnrollmentCredentialState>) -> EnrollmentCredential {
    EnrollmentCredential::new(
        credential_id(1),
        1000,
        2000,
        overrides_state.unwrap_or(EnrollmentCredentialState::Issued),
        "nonce-canary",
        SECRET_CANARY,
    )
    .unwrap()
}

#[test]
fn ep035_unit_enrollment_secret_never_appears_in_any_surface() {
    let cred = credential(None);
    let debug = format!("{cred:?}");
    let display = format!("{cred}");
    let json = serde_json::to_string(&cred).unwrap();
    let redacted_json = serde_json::to_string(&cred.redacted()).unwrap();
    assert!(!debug.contains(SECRET_CANARY));
    assert!(!debug.contains("nonce-canary"));
    assert!(!display.contains(SECRET_CANARY));
    assert!(!json.contains(SECRET_CANARY));
    assert!(!json.contains("nonce-canary"));
    assert!(!redacted_json.contains(SECRET_CANARY));
    assert!(!redacted_json.contains("nonce-canary"));
}

#[test]
fn ep035_unit_enrollment_issued_credential_is_usable_in_window() {
    assert!(credential(None).is_usable(1500));
}

#[test]
fn ep035_unit_enrollment_expired_credential_is_never_usable() {
    let expired = EnrollmentCredential::new(
        credential_id(1),
        1000,
        1499,
        EnrollmentCredentialState::Issued,
        "n",
        "s",
    )
    .unwrap();
    assert!(!expired.is_usable(1500));
}

#[test]
fn ep035_unit_enrollment_used_revoked_expired_never_valid_again() {
    assert!(!credential(Some(EnrollmentCredentialState::Used)).is_usable(1500));
    assert!(!credential(Some(EnrollmentCredentialState::Revoked)).is_usable(1500));
    assert!(!credential(Some(EnrollmentCredentialState::Expired)).is_usable(1500));
}

#[test]
fn ep035_unit_enrollment_credential_parse_rejects_invalid_window() {
    let err = EnrollmentCredential::new(
        credential_id(1),
        2000,
        1000,
        EnrollmentCredentialState::Issued,
        "n",
        "s",
    )
    .unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Validation);
}

#[test]
fn ep035_unit_enrollment_credential_rejects_unknown_wire_fields() {
    let cred = credential(None);
    let mut value = serde_json::to_value(&cred).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("forged".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<EnrollmentCredential>(value).is_err());
}

#[test]
fn ep035_unit_enrollment_request_is_typed_and_deny_unknown() {
    let request = EdgeEnrollmentRequest::new(
        "living-room-edge",
        "https://edge.local",
        credential_id(1),
        correlation(2),
    )
    .unwrap();
    assert_eq!(request.device_label, "living-room-edge");
    let mut value = serde_json::to_value(&request).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("forged".to_string(), serde_json::json!(true));
    assert!(serde_json::from_value::<EdgeEnrollmentRequest>(value).is_err());
}

// ---------------------------------------------------------------------------
// AUD-043 regressions: one-time enrollment consumption bound to secret
// possession in the SAME atomic transition.
// ---------------------------------------------------------------------------

#[test]
fn ep035_unit_enrollment_claim_requires_secret() {
    let cred = credential(None);
    // Correct secret -> atomic claim succeeds and marks USED.
    let claimed = cred.claim(SECRET_CANARY, 1500).unwrap();
    assert_eq!(claimed.state, EnrollmentCredentialState::Used);
    // The original is unchanged (pure transition).
    assert_eq!(cred.state, EnrollmentCredentialState::Issued);
    // A used credential cannot be claimed again even with the secret.
    let err = claimed.claim(SECRET_CANARY, 1500).unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Conflict);
}

#[test]
fn ep035_unit_enrollment_claim_wrong_secret_fails_closed() {
    let cred = credential(None);
    // Wrong secret: the claim MUST fail even though the credential ID
    // is valid (a caller knowing only the ID cannot consume it).
    let err = cred.claim("wrong-secret", 1500).unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Verification);
    // The credential is untouched: still ISSUED and usable.
    assert_eq!(cred.state, EnrollmentCredentialState::Issued);
    assert!(cred.is_usable(1500));
}

#[test]
fn ep035_unit_enrollment_claim_empty_secret_fails_closed() {
    let cred = credential(None);
    let err = cred.claim("", 1500).unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Verification);
    assert_eq!(cred.state, EnrollmentCredentialState::Issued);
}

#[test]
fn ep035_unit_enrollment_claim_expired_credential_fails_closed() {
    let cred = EnrollmentCredential::new(
        credential_id(1),
        1000,
        1499,
        EnrollmentCredentialState::Issued,
        "n",
        SECRET_CANARY,
    )
    .unwrap();
    // Expired: even the correct secret cannot claim it.
    let err = cred.claim(SECRET_CANARY, 1500).unwrap_err();
    assert_eq!(err.code, SetupErrorCode::Conflict);
}
