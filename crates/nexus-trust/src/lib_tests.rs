//! EP-009 M1 unit tests: trust contracts and vocabulary.
//!
//! Proves construction, validation, serialization, vocabulary rejection,
//! redaction, and dependency-direction constraints of the nexus-trust
//! contracts (SPEC-005 behaviors 5-7; SPEC-020).

use nexus_auth::AuthenticationStrength;
use nexus_domain::{CapabilityClass, PrincipalType, Risk, TenantId};
use nexus_identity::{DeviceKind, Principal, TrustLevel};

use crate::bootstrap::BootstrapBundle;
use crate::device::{DeviceSecretReference, DeviceSecretStoreError, DeviceSecretValue};
use crate::error::{TrustError, TrustErrorCode};
use crate::mesh::{MeshNode, WireGuardConfig, WireGuardPeer};
use crate::pki::{Certificate, CertificateAuthorityError, ServiceIdentity};
use crate::secret::{SecretReference, SecretReferenceError, SecretValue};
use crate::token::{CapabilityToken, CapabilityTokenIssuerError};
use crate::vocabulary::{
    CertificateState, MeshNodeState, SecretState, ServiceIdentityState, TokenState, TrustZone,
};

fn tenant() -> TenantId {
    TenantId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
}

fn principal() -> Principal {
    Principal::new(
        nexus_domain::NexusId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        PrincipalType::Human,
        tenant(),
    )
}

// ---------------------------------------------------------------------------
// SecretReference + SecretValue
// ---------------------------------------------------------------------------

#[test]
fn ep009_unit_secret_reference_roundtrip() {
    let reference = SecretReference::new("openbao", "db/password", Some("v3".to_string())).unwrap();
    assert_eq!(reference.store, "openbao");
    assert_eq!(reference.key, "db/password");
    assert_eq!(reference.version.as_deref(), Some("v3"));
    let text = reference.to_string();
    assert_eq!(text, "openbao:db/password@v3");
    let json = serde_json::to_string(&reference).unwrap();
    let back: SecretReference = serde_json::from_str(&json).unwrap();
    assert_eq!(back, reference);
}

#[test]
fn ep009_unit_secret_reference_rejects_empty() {
    assert_eq!(
        SecretReference::new("", "k", None).unwrap_err(),
        SecretReferenceError::EmptyStore
    );
    assert_eq!(
        SecretReference::new("s", " ", None).unwrap_err(),
        SecretReferenceError::EmptyKey
    );
}

#[test]
fn ep009_unit_secret_value_never_leaks_in_debug() {
    let value = SecretValue::new(vec![b's', b'e', b'c', b'r', b'e', b't']);
    let debug = format!("{value:?}");
    assert!(
        !debug.contains("secret"),
        "Debug must not contain the value: {debug}"
    );
    assert!(
        debug.contains("6"),
        "Debug should still show the length: {debug}"
    );
    // Serialization is redacted; deserialization fails closed.
    let json = serde_json::to_string(&value).unwrap();
    assert!(
        !json.contains("secret"),
        "serialized must be redacted: {json}"
    );
    assert!(serde_json::from_str::<SecretValue>(&json).is_err());
}

// ---------------------------------------------------------------------------
// Vocabulary
// ---------------------------------------------------------------------------

#[test]
fn ep009_unit_vocabulary_roundtrip() {
    assert_eq!(TrustZone::PrivateMesh.as_str(), "PRIVATE_MESH");
    assert_eq!("LOCAL".parse::<TrustZone>().unwrap(), TrustZone::Local);
    assert_eq!(SecretState::Rotating.as_str(), "ROTATING");
    assert_eq!(CertificateState::Revoked.as_str(), "REVOKED");
    assert_eq!(ServiceIdentityState::Suspended.as_str(), "SUSPENDED");
    assert_eq!(MeshNodeState::Online.as_str(), "ONLINE");
    assert_eq!(TokenState::Expired.as_str(), "EXPIRED");
    let back: TrustZone = serde_json::from_str("\"GUEST\"").unwrap();
    assert_eq!(back, TrustZone::Guest);
}

#[test]
fn ep009_unit_vocabulary_rejects_unknown() {
    assert!("INTERNET".parse::<TrustZone>().is_err());
    assert!("ACTIVE_ALWAYS".parse::<TokenState>().is_err());
    assert!("NODE".parse::<MeshNodeState>().is_err());
}

// ---------------------------------------------------------------------------
// Capability token
// ---------------------------------------------------------------------------

#[test]
fn ep009_unit_token_is_short_lived_and_scoped() {
    let token = CapabilityToken::new(
        "tok-1",
        "nexus-worker",
        tenant().as_str(),
        "task:0190",
        "task:complete",
        principal().principal_id.as_str(),
        100,
        200,
    )
    .unwrap();
    assert!(token.is_usable_at(150));
    assert!(
        !token.is_usable_at(200),
        "token must never be usable at expiry"
    );
    assert!(!token.is_usable_at(250));
    assert!(token.covers(
        "nexus-worker",
        "task:0190",
        "task:complete",
        tenant().as_str(),
        principal().principal_id.as_str()
    ));
    assert!(!token.covers(
        "nexus-worker",
        "task:0190",
        "task:delete",
        tenant().as_str(),
        principal().principal_id.as_str()
    ));
    assert!(!token.covers(
        "nexus-worker",
        "task:0190",
        "task:complete",
        "other-tenant",
        principal().principal_id.as_str()
    ));
}

#[test]
fn ep009_unit_token_rejects_inverted_or_empty() {
    assert_eq!(
        CapabilityToken::new("t", "a", "ten", "r", "action", "actor", 200, 100).unwrap_err(),
        CapabilityTokenIssuerError::InvertedTimes
    );
    assert_eq!(
        CapabilityToken::new("t", "a", "ten", "r", "action", " ", 100, 200).unwrap_err(),
        CapabilityTokenIssuerError::EmptyField
    );
}

#[test]
fn ep009_unit_token_revoke_expire_terminal() {
    let mut token =
        CapabilityToken::new("tok-2", "aud", "ten", "res", "act", "actor", 100, 200).unwrap();
    token.revoke();
    assert_eq!(token.state, TokenState::Revoked);
    assert!(!token.is_usable_at(150));
    token.expire();
    assert_eq!(token.state, TokenState::Expired);
}

// ---------------------------------------------------------------------------
// Certificate + ServiceIdentity
// ---------------------------------------------------------------------------

#[test]
fn ep009_unit_certificate_short_lived_window() {
    let cert = Certificate::new(
        "cert-1",
        "svc/worker",
        TrustZone::PrivateMesh,
        100,
        200,
        "openbao:pki/cert-1",
    )
    .unwrap();
    assert!(cert.is_valid_at(150));
    assert!(!cert.is_valid_at(99), "before not_before");
    assert!(!cert.is_valid_at(200), "at not_after");
    let mut revoked = cert.clone();
    revoked.revoke();
    assert!(!revoked.is_valid_at(150));
}

#[test]
fn ep009_unit_certificate_rejects_inverted_or_empty() {
    assert_eq!(
        Certificate::new("c", "s", TrustZone::Local, 200, 100, "ref").unwrap_err(),
        CertificateAuthorityError::InvertedTimes
    );
    assert_eq!(
        Certificate::new("c", " ", TrustZone::Local, 100, 200, "ref").unwrap_err(),
        CertificateAuthorityError::EmptyField
    );
}

#[test]
fn ep009_unit_service_identity_constructs() {
    let identity =
        ServiceIdentity::new("svc-1", tenant().as_str(), "nexus-worker", TrustZone::Local).unwrap();
    assert_eq!(identity.state, ServiceIdentityState::Active);
    assert!(ServiceIdentity::new("", "ten", "name", TrustZone::Guest).is_err());
}

// ---------------------------------------------------------------------------
// Mesh
// ---------------------------------------------------------------------------

#[test]
fn ep009_unit_mesh_node_and_wireguard_config() {
    let node = MeshNode::new(
        "node-1",
        tenant().as_str(),
        "edge-1",
        TrustZone::Local,
        "BASE64PUBKEY",
        Some("10.0.0.1:51820".to_string()),
    )
    .unwrap();
    assert_eq!(node.state, MeshNodeState::Pending);

    let peer = WireGuardPeer::new(
        "PEERKEY",
        Some("10.0.0.2:51820".to_string()),
        vec!["100.64.0.2/32".to_string()],
        25,
    )
    .unwrap();
    let config = WireGuardConfig::new(
        "nexus0",
        "openbao:wg/edge-1",
        vec!["100.64.0.1/32".to_string()],
        vec!["100.64.0.1".to_string()],
        vec![peer],
    )
    .unwrap();
    assert_eq!(config.interface, "nexus0");
    assert!(WireGuardConfig::new("", "ref", vec![], vec![], vec![]).is_err());
    assert!(MeshNode::new("", "t", "n", TrustZone::Public, "K", None).is_err());
}

// ---------------------------------------------------------------------------
// Bootstrap + device stores
// ---------------------------------------------------------------------------

#[test]
fn ep009_unit_bootstrap_bundle_constructs() {
    let reference = SecretReference::new("sops", "age/identity", None).unwrap();
    let bundle = BootstrapBundle::new(
        "config/sops/bootstrap.yaml",
        reference.clone(),
        vec![reference.clone()],
    )
    .unwrap();
    assert_eq!(bundle.sealed_path, "config/sops/bootstrap.yaml");
    assert!(BootstrapBundle::new("", reference, vec![]).is_err());
}

#[test]
fn ep009_unit_device_secret_reference_and_redaction() {
    let reference = DeviceSecretReference::new("device-1", "refresh-token").unwrap();
    assert_eq!(reference.to_string(), "device:device-1:refresh-token");
    assert_eq!(
        DeviceSecretReference::new("", "k").unwrap_err(),
        DeviceSecretStoreError::EmptyDeviceId
    );
    let value = DeviceSecretValue::new(vec![1, 2, 3]);
    let debug = format!("{value:?}");
    assert!(
        !debug.contains("1, 2, 3"),
        "device secret must not leak: {debug}"
    );
    let json = serde_json::to_string(&value).unwrap();
    assert!(json.contains("<redacted>"));
}

// ---------------------------------------------------------------------------
// Canonical error surface + dependency direction
// ---------------------------------------------------------------------------

#[test]
fn ep009_unit_trust_error_typed_codes() {
    let err = TrustError::new(TrustErrorCode::Unavailable, "openbao down");
    assert_eq!(err.code, TrustErrorCode::Unavailable);
    assert_eq!(err.code.as_str(), "UNAVAILABLE");
    assert_eq!(TrustErrorCode::NotFound.as_str(), "NOT_FOUND");
    assert_eq!(TrustErrorCode::StateConflict.as_str(), "STATE_CONFLICT");
}

#[test]
fn ep009_unit_domain_dependencies_are_contract_only() {
    // The trust crate must only import domain/identity/auth + serde. This
    // compiles only because those crates exist; the dependency-direction
    // integration test enforces the tree (tests/dependency_direction.rs).
    let _ = (
        AuthenticationStrength::StepUp,
        CapabilityClass::Administrative,
        Risk::R4,
        TrustLevel::Verified,
        DeviceKind::Desktop,
    );
}
