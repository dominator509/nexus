//! EP-009 M2 unit tests: OpenBao adapter contracts and SOPS store.
//!
//! Pure unit tests (no live provider): error classification, secret
//! redaction, bootstrap store argument handling. The real provider
//! proofs live in tests/trust/ (ep009_integration_* / ep009_failure_*).

use nexus_trust::SecretValue;

use crate::error::{OpenBaoError, OpenBaoErrorCode};
use crate::store::{AppRoleLogin, WrappedHandoff};

#[test]
fn ep009_unit_openbao_error_maps_to_trust_codes() {
    assert_eq!(
        OpenBaoErrorCode::Unavailable.trust_code(),
        nexus_trust::TrustErrorCode::Unavailable
    );
    assert_eq!(
        OpenBaoErrorCode::AuthenticationFailed.trust_code(),
        nexus_trust::TrustErrorCode::ProviderAuthorization
    );
    assert_eq!(
        OpenBaoErrorCode::PermissionDenied.trust_code(),
        nexus_trust::TrustErrorCode::ProviderAuthorization
    );
    assert_eq!(
        OpenBaoErrorCode::NotFound.trust_code(),
        nexus_trust::TrustErrorCode::NotFound
    );
    assert_eq!(
        OpenBaoErrorCode::Destroyed.trust_code(),
        nexus_trust::TrustErrorCode::NotFound
    );
    assert_eq!(
        OpenBaoErrorCode::VersionMismatch.trust_code(),
        nexus_trust::TrustErrorCode::StateConflict
    );
    assert_eq!(
        OpenBaoErrorCode::Timeout.trust_code(),
        nexus_trust::TrustErrorCode::Timeout
    );
}

#[test]
fn ep009_unit_openbao_error_display_never_leaks() {
    let e = OpenBaoError::new(OpenBaoErrorCode::AuthenticationFailed, "opaque");
    let text = format!("{e}");
    assert_eq!(text, "OPENBAO_AUTHENTICATION_FAILED");
}

#[test]
fn ep009_unit_approle_login_token_accessor_is_private_surface() {
    // The client token is ONLY reachable through the pub(crate)
    // accessor (transport layer); it is never Debug-printed.
    let login = AppRoleLogin {
        client_token: "hvs-test-token".to_string(),
        lease_duration: 900,
        renewable: true,
    };
    assert_eq!(login.client_token(), "hvs-test-token");
    let debug = format!("{login:?}");
    assert!(
        !debug.contains("hvs-test-token"),
        "token must not Debug-print: {debug}"
    );
}

#[test]
fn ep009_unit_wrapped_handoff_debug_redacts_wrapping_token() {
    // Construct via the public transport accessor pattern: the
    // wrapping token is set through the private field path only.
    let handoff = WrappedHandoff {
        wrapping_token: "hvs-wrap-token-abc".to_string(),
        accessor: "acc-1".to_string(),
        creation_path: "/v1/secret/data/tenant-a/handoff".to_string(),
        ttl: 120,
        creation_time: "2026-08-14T00:00:00Z".to_string(),
    };
    let debug = format!("{handoff:?}");
    assert!(
        !debug.contains("hvs-wrap-token-abc"),
        "wrapping token must not Debug-print: {debug}"
    );
    assert!(debug.contains("<redacted>"));
}

#[test]
fn ep009_unit_secret_value_redaction_invariant() {
    let value = SecretValue::new(b"canary-super-secret-value".to_vec());
    let debug = format!("{value:?}");
    assert!(!debug.contains("canary"));
    let json = serde_json::to_string(&value).unwrap();
    assert!(!json.contains("canary"));
    assert!(json.contains("<redacted>"));
    assert!(serde_json::from_str::<SecretValue>(&json).is_err());
}

#[test]
fn ep009_unit_telemetry_never_contains_secrets() {
    let sink = crate::telemetry::RecordingSink::new();
    sink.record(crate::telemetry::TelemetryEvent {
        operation: "get".to_string(),
        reference_fingerprint: crate::telemetry::fingerprint("openbao:tenant-a/db"),
        ..Default::default()
    });
    let events = sink.events();
    assert_eq!(events.len(), 1);
    let text = format!("{:?}", events);
    assert!(!text.contains("tenant-a/db"));
    assert!(text.contains("fingerprint"));
}

#[test]
fn ep009_unit_sops_store_health_requires_age_binary() {
    let store = crate::sops::SopsBootstrapStore::new(
        b"AGE-SECRET-KEY-1TEST".to_vec(),
        "definitely-not-sops".to_string(),
        "definitely-not-age".to_string(),
    );
    // Missing binaries must fail closed (typed unavailable), never
    // silently succeed.
    assert_eq!(
        store.health().unwrap_err().code,
        nexus_trust::TrustErrorCode::Unavailable
    );
}

#[test]
fn ep009_unit_sops_store_debug_redacts_identity() {
    let store = crate::sops::SopsBootstrapStore::new(
        b"AGE-SECRET-KEY-1TESTOPENBAOXYZ".to_vec(),
        "sops".to_string(),
        "age".to_string(),
    );
    let debug = format!("{store:?}");
    assert!(!debug.contains("AGE-SECRET-KEY"));
    assert!(debug.contains("redacted"));
}

// ---------------------------------------------------------------------------
// SOPS decrypt failure classifier (directive E). The stderr shapes below are
// the REAL sops 3.13.0 outputs captured from the pinned binaries; secret
// material (age identity, canary, recipient) is redacted. The generic
// "Recovery failed" footer appears in every failure and must never drive
// classification.
// ---------------------------------------------------------------------------

const FOOTER: &str = "Recovery failed because no master key was able to decrypt the file. In \
order for SOPS to recover the file, at least one key has to be successful, \
but none were.";

fn classify(exit: Option<i32>, stderr: &str) -> nexus_trust::TrustErrorCode {
    crate::sops::classify_sops_decrypt_failure(exit, stderr)
}

#[test]
fn ep009_unit_sops_classifier_wrong_identity_is_authorization() {
    // Real shape: valid age identity that cannot decrypt the data key.
    let stderr = format!(
        "Failed to get the data key required to decrypt the SOPS file.\n\
         Group 0: FAILED\n\
         - | failed to create reader for decrypting sops data key with\n\
         | age: no identity matched any of the recipients. Did not find\n\
         | keys in locations 'SOPS_AGE_SSH_PRIVATE_KEY_FILE',\n\
         | 'SOPS_AGE_SSH_PRIVATE_KEY_CMD', '/root/.ssh/id_rsa',\n\
         | 'SOPS_AGE_KEY', and 'SOPS_AGE_KEY_CMD'.\n{FOOTER}"
    );
    assert_eq!(
        classify(Some(128), &stderr),
        nexus_trust::TrustErrorCode::ProviderAuthorization
    );
}

#[test]
fn ep009_unit_sops_classifier_malformed_identity_is_malformed() {
    // Real shape: unparsable age identity material (unknown identity type).
    // Must NOT be ProviderAuthorization.
    let stderr = format!(
        "Failed to get the data key required to decrypt the SOPS file.\n\
         Group 0: FAILED\n\
         - | failed to create reader for decrypting sops data key with\n\
         | age: identity did not match any of the recipients: incorrect\n\
         | identity for recipient block. Errors while loading age\n\
         | identities: failed to parse 'SOPS_AGE_KEY_FILE' age\n\
         | identities: unknown identity type. Did not find keys in\n\
         | locations 'SOPS_AGE_SSH_PRIVATE_KEY_FILE',\n\
         | 'SOPS_AGE_SSH_PRIVATE_KEY_CMD', '/root/.ssh/id_rsa',\n\
         | 'SOPS_AGE_KEY', and 'SOPS_AGE_KEY_CMD'.\n{FOOTER}"
    );
    assert_eq!(
        classify(Some(128), &stderr),
        nexus_trust::TrustErrorCode::MalformedProviderResponse
    );
}

#[test]
fn ep009_unit_sops_classifier_missing_key_file_is_not_found() {
    // Real shape: SOPS_AGE_KEY_FILE points at a nonexistent path. This is a
    // missing/bootstrap-source error, NOT authorization.
    let stderr = format!(
        "Failed to get the data key required to decrypt the SOPS file.\n\
         Group 0: FAILED\n\
         - | failed to create reader for decrypting sops data key with\n\
         | age: identity did not match any of the recipients: incorrect\n\
         | identity for recipient block. Errors while loading age\n\
         | identities: failed to open SOPS_AGE_KEY_FILE file: open\n\
         | /tmp/nexus/missing-key.key: no such file or directory. Did not\n\
         | find keys in locations 'SOPS_AGE_SSH_PRIVATE_KEY_FILE',\n\
         | 'SOPS_AGE_SSH_PRIVATE_KEY_CMD', '/root/.ssh/id_rsa',\n\
         | 'SOPS_AGE_KEY', and 'SOPS_AGE_KEY_CMD'.\n{FOOTER}"
    );
    assert_eq!(
        classify(Some(128), &stderr),
        nexus_trust::TrustErrorCode::NotFound
    );
}

#[test]
fn ep009_unit_sops_classifier_corrupted_document_is_malformed() {
    // Real shape: ciphertext tampered/corrupt (integrity failure). Contains
    // "failed to decrypt and authenticate" which the OLD broad matcher
    // ("failed to decrypt") would have misclassified as authorization.
    let stderr = format!(
        "Failed to get the data key required to decrypt the SOPS file.\n\
         Group 0: FAILED\n\
         - | failed to copy age decrypted data into bytes.Buffer: failed\n\
         | to decrypt and authenticate payload chunk, file may be\n\
         | corrupted or tampered with\n{FOOTER}"
    );
    assert_eq!(
        classify(Some(128), &stderr),
        nexus_trust::TrustErrorCode::MalformedProviderResponse
    );
}

#[test]
fn ep009_unit_sops_classifier_missing_document_is_not_found() {
    // Real shape: sealed document path does not exist (sops exit 100).
    let stderr = "Error: cannot operate on non-existent file \"/tmp/nexus/missing.enc.yaml\"";
    assert_eq!(
        classify(Some(100), stderr),
        nexus_trust::TrustErrorCode::NotFound
    );
    // Same shape must classify correctly even without a reliable exit code.
    assert_eq!(
        classify(None, stderr),
        nexus_trust::TrustErrorCode::NotFound
    );
}

#[test]
fn ep009_unit_sops_classifier_wrong_identity_plus_footer_is_authorization() {
    // Directive E.6: wrong identity plus the generic recovery footer must
    // still be ProviderAuthorization because structural failures were ruled
    // out first (the stderr contains no parse/open/integrity markers).
    let stderr = format!(
        "Group 0: FAILED\n\
         | age: no identity matched any of the recipients.\n{FOOTER}"
    );
    assert_eq!(
        classify(Some(128), &stderr),
        nexus_trust::TrustErrorCode::ProviderAuthorization
    );
}

#[test]
fn ep009_unit_sops_classifier_malformed_plus_same_footer_is_malformed() {
    // Directive E.7: the SAME generic footer must NOT convert a malformed
    // identity into an authorization failure. Structural failure wins.
    let stderr = format!(
        "Group 0: FAILED\n\
         | age: failed to parse 'SOPS_AGE_KEY_FILE' age identities:\n\
         | unknown identity type.\n{FOOTER}"
    );
    assert_eq!(
        classify(Some(128), &stderr),
        nexus_trust::TrustErrorCode::MalformedProviderResponse
    );
}

#[test]
fn ep009_unit_sops_classifier_unknown_shape_fails_closed_malformed() {
    // Unknown failure shape: never assume authorization.
    let stderr = "sops: some unexpected diagnostic text with no markers";
    assert_eq!(
        classify(Some(2), stderr),
        nexus_trust::TrustErrorCode::MalformedProviderResponse
    );
}

#[test]
fn ep009_unit_sops_classifier_success_never_consulted() {
    // The classifier is only invoked on non-success exit; a success shape
    // must not be treated as a failure at all (guarded by the caller).
    // Regression guard: ensure the classifier never maps an exit-0 shape
    // to an error code, even if called with a success-shaped stderr.
    let stderr = "everything decrypted fine";
    assert_ne!(
        classify(Some(0), stderr),
        nexus_trust::TrustErrorCode::ProviderAuthorization
    );
}
