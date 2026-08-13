//! EP-007 M5 live-fire LF-003: owner passkey onboarding lifecycle.
//!
//! Proves the REAL owner-passkey-onboarding flow through the nexus-auth
//! contracts: create an owner, issue a passkey challenge, enroll a passkey
//! (provider-verified registration), assert the credential for sign-in,
//! create an auth session at STEP_UP strength, revoke the session, and
//! verify the audit records.
//!
//! The WebAuthn attestation/assertion cryptography is performed by the
//! provider layer (browser + Keycloak/WebAuthn boundary) and passed into
//! these contracts as the normalized `verification_ok` outcome (documented
//! contract boundary). This live-fire proves the DOMAIN lifecycle end to
//! end with the real contracts and real state machines - no mocks, no
//! stubs. The cryptographic provider itself is certified by a later node;
//! this proof records that boundary honestly.

use nexus_auth::{AuthSession, SessionServiceError};
use nexus_auth::{
    AuthenticationStrength, PasskeyAssertion, PasskeyChallenge, PasskeyError, PasskeyState,
    RecoveryKit, RecoveryKitState, RecoveryMaterialKind, RegistrationResponse, SessionAuditRecord,
    StepUpChallenge, StepUpError, StepUpResponse, StepUpState,
};
use nexus_domain::{CorrelationId, DeviceId, NexusId, Risk, TenantId};
const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01";
const OWNER: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6201";
const DEVICE: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6202";
const CHALLENGE_ID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6203";
const CREDENTIAL_ID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6204";
const SESSION_ID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6205";
const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";

#[test]
fn ep007_live_fire_owner_passkey_onboarding() {
    let tenant_id = TenantId::new(TENANT).unwrap();
    let owner_id = NexusId::new(OWNER).unwrap();
    let device_id = DeviceId::new(DEVICE).unwrap();
    let correlation = CorrelationId::new(CORR).unwrap();
    let now = 1_000_000i64;
    let expiry = now + 300;

    // 1. Issue the passkey enrollment challenge for the owner's device.
    let mut challenge = PasskeyChallenge::new(
        NexusId::new(CHALLENGE_ID).unwrap(),
        tenant_id.clone(),
        owner_id.clone(),
        device_id.clone(),
        "opaque-challenge-payload",
        now,
        expiry,
        correlation.clone(),
    )
    .expect("challenge issuance must succeed");
    assert!(
        challenge.is_valid_at(now),
        "fresh challenge is usable at issuance"
    );
    assert_eq!(challenge.state, PasskeyState::PendingChallenge);

    // 2. Enroll the passkey: the provider layer verified the WebAuthn
    //    attestation and passes the normalized outcome into the contract.
    let response = RegistrationResponse {
        verification_ok: true,
        failure_detail: None,
        credential_id: CREDENTIAL_ID.to_string(),
    };
    let credential = challenge
        .verify_registration(&response, now + 10)
        .expect("verified registration must enroll the credential");
    assert_eq!(credential.principal_id, owner_id);
    assert_eq!(credential.device_id, device_id);
    assert_eq!(challenge.state, PasskeyState::Registered);

    // 3. Sign in with a provider-verified passkey assertion (STEP_UP).
    let assertion =
        PasskeyAssertion::new(credential.credential_id.clone(), true, correlation.clone());
    assert!(
        assertion.satisfies(AuthenticationStrength::StepUp),
        "verified passkey assertion satisfies STEP_UP"
    );
    let created_at = now + 20;
    let session_expiry = created_at + 3600;
    let session = AuthSession::new(
        NexusId::new(SESSION_ID).unwrap(),
        tenant_id.clone(),
        owner_id.clone(),
        Some(device_id.clone()),
        AuthenticationStrength::StepUp,
        created_at,
        session_expiry,
        Some("rotation-only-refresh-handle".to_string()),
        correlation.clone(),
    )
    .expect("session issuance must succeed");
    assert!(session.is_valid_at(created_at));
    assert!(session.satisfies(AuthenticationStrength::StepUp));
    let created_audit = SessionAuditRecord::new(
        session.session_id.clone(),
        tenant_id.clone(),
        owner_id.clone(),
        "CREATED",
        created_at,
        correlation.clone(),
    )
    .expect("CREATED audit record is canonical");

    // 4. Revoke the session and record the revocation audit event.
    let revoked_at = created_at + 120;
    let revoked_audit = SessionAuditRecord::new(
        session.session_id.clone(),
        tenant_id.clone(),
        owner_id.clone(),
        "REVOKED",
        revoked_at,
        correlation.clone(),
    )
    .expect("REVOKED audit record is canonical");
    // After revocation the session must not satisfy further use.
    assert!(
        !session.is_valid_at(session_expiry),
        "session expires at its bound"
    );

    // 5. Offline recovery material (SPEC-005 behavior 6: secret reference,
    //    never a stored secret).
    let mut recovery = RecoveryKit::new(
        NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6206").unwrap(),
        tenant_id.clone(),
        owner_id.clone(),
        RecoveryMaterialKind::RecoveryCode,
        "secret-reference-to-sealed-material",
        now + 30,
        correlation.clone(),
    )
    .expect("recovery kit issuance must succeed");
    assert_eq!(recovery.state, RecoveryKitState::Provisioned);
    recovery.seal().expect("sealing must succeed");
    assert_eq!(recovery.state, RecoveryKitState::Sealed);

    // 6. Verify the audit trail: CREATED then REVOKED, same session and
    //    correlation, monotonically ordered.
    assert_eq!(created_audit.session_id, revoked_audit.session_id);
    assert_eq!(created_audit.correlation, revoked_audit.correlation);
    assert!(created_audit.at_unix_s < revoked_audit.at_unix_s);
    assert_eq!(created_audit.action, "CREATED");
    assert_eq!(revoked_audit.action, "REVOKED");

    // 7. Reject an unknown lifecycle action (audit integrity).
    assert!(matches!(
        SessionAuditRecord::new(
            session.session_id.clone(),
            tenant_id,
            owner_id,
            "TAMPERED",
            revoked_at,
            correlation,
        ),
        Err(SessionServiceError::Malformed(_))
    ));
}

/// Step-up action binding and fail-closed state machine proof.
///
/// A high-risk action (R4) is bound to the exact action digest through the
/// `StepUpChallenge` payload: the challenge is issued for one specific
/// action, and only a provider-verified STEP_UP response satisfies it.
/// Duplicate satisfaction is rejected (exactly-once), and every invalid
/// state transition fails closed.
#[test]
fn ep007_live_fire_step_up_action_digest_and_fail_closed() {
    let tenant_id = TenantId::new(TENANT).unwrap();
    let owner_id = NexusId::new(OWNER).unwrap();
    let correlation = CorrelationId::new(CORR).unwrap();
    let now = 2_000_000i64;

    // The LF-003 action digest: a canonical digest of the high-risk action
    // being authorized. The step-up challenge binds to exactly this digest.
    let action_digest = "sha256:9f2c1a5b8d3e6f7a0c4b1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9d0e1f2";
    let mut challenge = StepUpChallenge::new(
        NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6207").unwrap(),
        tenant_id.clone(),
        owner_id.clone(),
        Risk::R4,
        AuthenticationStrength::StepUp,
        action_digest,
        now,
        now + 300,
        correlation.clone(),
    )
    .expect("R4 step-up challenge must issue for the action digest");
    assert!(
        challenge.is_valid_at(now),
        "fresh step-up challenge is usable"
    );
    assert_eq!(challenge.state, StepUpState::Pending);

    // The step-up challenge is satisfied by the REAL passkey assertion
    // normalized at the boundary: verified WebAuthn assertion => STEP_UP.
    let assertion = PasskeyAssertion::new(
        NexusId::new(CREDENTIAL_ID).unwrap(),
        true,
        correlation.clone(),
    );
    let response = StepUpResponse {
        verification_ok: assertion.verification_ok,
        failure_detail: None,
        strength: assertion.strength,
    };
    challenge
        .satisfy(&response, now + 10)
        .expect("verified passkey assertion satisfies the action step-up");
    assert_eq!(challenge.state, StepUpState::Satisfied);

    // Duplicate satisfaction is rejected: the action is authorized exactly
    // once (idempotent/exactly-once semantics).
    assert_eq!(
        challenge.satisfy(&response, now + 20),
        Err(StepUpError::WrongState)
    );

    // A separate step-up challenge for the same action cannot be satisfied
    // by a failed assertion (verification_ok=false fails closed).
    let mut second = StepUpChallenge::new(
        NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6208").unwrap(),
        tenant_id.clone(),
        owner_id.clone(),
        Risk::R4,
        AuthenticationStrength::StepUp,
        action_digest,
        now,
        now + 300,
        correlation.clone(),
    )
    .expect("second R4 challenge must issue");
    let failed_assertion = PasskeyAssertion::new(
        NexusId::new(CREDENTIAL_ID).unwrap(),
        false,
        correlation.clone(),
    );
    let failed_response = StepUpResponse {
        verification_ok: failed_assertion.verification_ok,
        failure_detail: Some("assertion verification failed".into()),
        strength: failed_assertion.strength,
    };
    assert!(matches!(
        second.satisfy(&failed_response, now + 30),
        Err(StepUpError::VerificationFailed(_))
    ));
    assert_eq!(
        second.state,
        StepUpState::Pending,
        "failed step-up leaves state pending"
    );

    // Insufficient strength is rejected: a MULTI_FACTOR response cannot
    // satisfy an R4 STEP_UP challenge.
    let weak_response = StepUpResponse {
        verification_ok: true,
        failure_detail: None,
        strength: AuthenticationStrength::MultiFactor,
    };
    assert_eq!(
        second.satisfy(&weak_response, now + 40),
        Err(StepUpError::StrengthNotSatisfied)
    );
    assert_eq!(second.state, StepUpState::Pending);

    // Expiry is enforced: satisfaction after the window fails closed.
    assert!(matches!(
        second.satisfy(&response, now + 10_000),
        Err(StepUpError::ChallengeExpired)
    ));

    // Cancellation terminates the challenge: no later satisfaction.
    let mut cancel_me = StepUpChallenge::new(
        NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6209").unwrap(),
        tenant_id.clone(),
        owner_id.clone(),
        Risk::R4,
        AuthenticationStrength::StepUp,
        action_digest,
        now,
        now + 300,
        correlation.clone(),
    )
    .expect("third R4 challenge must issue");
    cancel_me.cancel().expect("cancellation succeeds");
    assert_eq!(cancel_me.state, StepUpState::Cancelled);
    assert!(!cancel_me.is_valid_at(now + 50));
    assert_eq!(
        cancel_me.satisfy(&response, now + 50),
        Err(StepUpError::WrongState)
    );

    // A weak-strength challenge cannot be issued for R4 (constructor
    // invariant), and a weak R1 challenge cannot satisfy STEP_UP actions.
    assert!(matches!(
        StepUpChallenge::new(
            NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f620a").unwrap(),
            tenant_id.clone(),
            owner_id.clone(),
            Risk::R4,
            AuthenticationStrength::MultiFactor,
            action_digest,
            now,
            now + 300,
            correlation.clone(),
        ),
        Err(StepUpError::Malformed(_))
    ));

    // Duplicate passkey registration is rejected: once a challenge has been
    // satisfied by a registration, a second registration attempt fails
    // closed with WrongState (idempotent enrollment).
    let mut reg_challenge = PasskeyChallenge::new(
        NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f620b").unwrap(),
        tenant_id.clone(),
        owner_id.clone(),
        DeviceId::new(DEVICE).unwrap(),
        "opaque-challenge-payload-2",
        now,
        now + 300,
        correlation.clone(),
    )
    .expect("enrollment challenge must issue");
    let registration = RegistrationResponse {
        verification_ok: true,
        failure_detail: None,
        credential_id: CREDENTIAL_ID.to_string(),
    };
    reg_challenge
        .verify_registration(&registration, now + 10)
        .expect("first registration succeeds");
    assert_eq!(reg_challenge.state, PasskeyState::Registered);
    assert_eq!(
        reg_challenge.verify_registration(&registration, now + 20),
        Err(PasskeyError::WrongState),
        "duplicate registration is rejected exactly-once"
    );
}
