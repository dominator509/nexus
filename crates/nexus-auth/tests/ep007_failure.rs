//! EP-007 M4 forced-failure tests for `nexus-auth` (real failure mechanisms).
//!
//! These tests prove the domain layer fails safely under malformed input,
//! denied permission, expired/corrupted tokens, inverted windows, and
//! unauthorized states. They use the REAL public API and REAL construction
//! invariants - no mocks, no stubs (TESTING.md). Test names begin with
//! `ep007_failure_` per the EP-007 milestone contract.

use nexus_auth::{
    AuthenticationStrength, GrantFlow, OidcClient, OidcError, StepUpChallenge, StepUpError,
    TokenClaims, TokenValidationOutcome, TokenValidator,
};
use nexus_domain::{CorrelationId, NexusId, PrincipalType, Risk, TenantId};

fn tenant() -> TenantId {
    TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a01").unwrap()
}

fn corr() -> CorrelationId {
    CorrelationId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6a02").unwrap()
}

fn pid() -> NexusId {
    NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6101").unwrap()
}

fn valid_claims() -> TokenClaims {
    TokenClaims {
        token_class: nexus_auth::TokenClass::Access,
        subject: "user-1".into(),
        issuer: "https://auth.nexus.local/realms/nexus".into(),
        audiences: vec!["nexus-app".into()],
        scopes: vec!["openid".into(), "nexus.read".into()],
        not_before_unix_s: 1000,
        expires_at_unix_s: 9000,
        device_id: None,
        strength: AuthenticationStrength::MultiFactor,
        signature_verified: true,
        correlation: corr(),
        tenant_id: tenant(),
        principal_type: PrincipalType::Human,
    }
}

fn validator() -> TokenValidator {
    TokenValidator::new(
        "https://auth.nexus.local/realms/nexus",
        "nexus-app",
        vec!["openid".into(), "nexus.read".into()],
        None,
    )
    .unwrap()
}

// --------------------------------------------------------------------------
// Token validator: every dimension must fail closed
// --------------------------------------------------------------------------

#[test]
fn ep007_failure_token_with_unverified_signature_is_rejected() {
    let mut claims = valid_claims();
    claims.signature_verified = false;
    assert!(matches!(
        validator().validate(&claims, 5000),
        TokenValidationOutcome::Rejected(OidcError::BadSignature)
    ));
}

#[test]
fn ep007_failure_token_with_wrong_issuer_is_rejected() {
    let mut claims = valid_claims();
    claims.issuer = "https://evil.example/realms/nexus".into();
    assert!(matches!(
        validator().validate(&claims, 5000),
        TokenValidationOutcome::Rejected(OidcError::IssuerMismatch)
    ));
}

#[test]
fn ep007_failure_token_with_wrong_audience_is_rejected() {
    let mut claims = valid_claims();
    claims.audiences = vec!["evil-client".into()];
    assert!(matches!(
        validator().validate(&claims, 5000),
        TokenValidationOutcome::Rejected(OidcError::AudienceMismatch)
    ));
}

#[test]
fn ep007_failure_expired_token_is_rejected() {
    let claims = valid_claims(); // expires_at 9000
    assert!(matches!(
        validator().validate(&claims, 9000),
        TokenValidationOutcome::Rejected(OidcError::InvalidTime)
    ));
    assert!(matches!(
        validator().validate(&claims, 9001),
        TokenValidationOutcome::Rejected(OidcError::InvalidTime)
    ));
}

#[test]
fn ep007_failure_token_before_not_before_is_rejected() {
    let claims = valid_claims(); // not_before 1000
    assert!(matches!(
        validator().validate(&claims, 999),
        TokenValidationOutcome::Rejected(OidcError::InvalidTime)
    ));
}

#[test]
fn ep007_failure_token_missing_required_scope_is_rejected() {
    let mut claims = valid_claims();
    claims.scopes = vec!["openid".into()]; // nexus.read missing
    assert!(matches!(
        validator().validate(&claims, 5000),
        TokenValidationOutcome::Rejected(OidcError::MissingScope)
    ));
}

#[test]
fn ep007_failure_token_with_wrong_device_context_is_rejected() {
    let mut claims = valid_claims();
    claims.device_id = Some("device-other".into());
    let strict = TokenValidator::new(
        "https://auth.nexus.local/realms/nexus",
        "nexus-app",
        vec!["openid".into(), "nexus.read".into()],
        Some("device-7".into()),
    )
    .unwrap();
    assert!(matches!(
        strict.validate(&claims, 5000),
        TokenValidationOutcome::Rejected(OidcError::DeviceContextMismatch)
    ));
}

// --------------------------------------------------------------------------
// OidcClient: malformed and unauthorized states
// --------------------------------------------------------------------------

#[test]
fn ep007_failure_oidc_client_rejects_empty_client_id() {
    assert!(matches!(
        OidcClient::new(
            "",
            "https://auth.nexus.local",
            "https://app.nexus.local/callback",
            vec![GrantFlow::AuthorizationCode],
            vec!["openid".into()]
        ),
        Err(OidcError::Malformed(_))
    ));
}

#[test]
fn ep007_failure_oidc_client_rejects_insecure_issuer() {
    assert!(matches!(
        OidcClient::new(
            "nexus-app",
            "http://auth.nexus.local",
            "https://app.nexus.local/callback",
            vec![GrantFlow::AuthorizationCode],
            vec!["openid".into()]
        ),
        Err(OidcError::Malformed(_))
    ));
}

#[test]
fn ep007_failure_oidc_client_rejects_empty_flow_list() {
    assert!(matches!(
        OidcClient::new(
            "nexus-app",
            "https://auth.nexus.local",
            "https://app.nexus.local/callback",
            vec![],
            vec!["openid".into()]
        ),
        Err(OidcError::Malformed(_))
    ));
}

#[test]
fn ep007_failure_oidc_client_rejects_empty_required_scope() {
    assert!(matches!(
        OidcClient::new(
            "nexus-app",
            "https://auth.nexus.local",
            "https://app.nexus.local/callback",
            vec![GrantFlow::AuthorizationCode],
            vec!["".into()]
        ),
        Err(OidcError::Malformed(_))
    ));
}

#[test]
fn ep007_failure_oidc_client_does_not_permit_disallowed_flow() {
    let client = OidcClient::new(
        "nexus-app",
        "https://auth.nexus.local",
        "https://app.nexus.local/callback",
        vec![GrantFlow::AuthorizationCode, GrantFlow::RefreshToken],
        vec!["openid".into()],
    )
    .unwrap();
    // Human client must never permit the client-credentials flow.
    assert!(!client.permits(GrantFlow::ClientCredentials));
}

// --------------------------------------------------------------------------
// Step-up: R3/R4 must require STEP_UP; inverted windows rejected
// --------------------------------------------------------------------------

#[test]
fn ep007_failure_step_up_r3_requires_step_up_strength() {
    assert!(matches!(
        StepUpChallenge::new(
            pid(),
            tenant(),
            pid(),
            Risk::R3,
            AuthenticationStrength::SingleFactor,
            "challenge-payload",
            1000,
            9000,
            corr()
        ),
        Err(StepUpError::Malformed(_))
    ));
}

#[test]
fn ep007_failure_step_up_inverted_window_is_rejected() {
    assert!(matches!(
        StepUpChallenge::new(
            pid(),
            tenant(),
            pid(),
            Risk::R4,
            AuthenticationStrength::StepUp,
            "challenge-payload",
            9000,
            1000,
            corr()
        ),
        Err(StepUpError::Malformed(_))
    ));
}

// --------------------------------------------------------------------------
// Vocabulary: canonical enum ladder never weakens
// --------------------------------------------------------------------------

#[test]
fn ep007_failure_strength_ladder_orders_are_canonical() {
    use std::cmp::Ordering;
    assert_eq!(
        AuthenticationStrength::None.cmp(&AuthenticationStrength::SingleFactor),
        Ordering::Less
    );
    assert_eq!(
        AuthenticationStrength::SingleFactor.cmp(&AuthenticationStrength::MultiFactor),
        Ordering::Less
    );
    assert_eq!(
        AuthenticationStrength::MultiFactor.cmp(&AuthenticationStrength::StepUp),
        Ordering::Less
    );
}
