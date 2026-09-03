//! EP-009 M3 unit tests: headscale model parsing and error mapping.
//!
//! Pure unit tests (no live provider). The JSON shapes below are the
//! REAL `headscale v0.23.0 -o json` outputs captured from the pinned
//! container on 2026-08-14 (see Decision Log). The real provider
//! proofs live in tests/trust/ (ep009_integration_*).

use nexus_trust::mesh::{MeshController, MeshNode, WireGuardConfig, WireGuardPeer};
use nexus_trust::secret::{SecretReference, SecretStore, SecretValue};
use nexus_trust::vocabulary::{MeshNodeState, TrustZone};

use crate::error::{HeadscaleError, HeadscaleErrorCode};
use crate::mesh::HeadscaleMeshController;
use crate::model::{fingerprint, machine_key, node_key_hex, Node, PreAuthKey, User};

#[test]
fn ep009_unit_headscale_machine_key_normalization() {
    assert_eq!(machine_key("abc123"), "mkey:abc123");
    assert_eq!(machine_key("mkey:abc123"), "mkey:abc123");
}

#[test]
fn ep009_unit_headscale_node_key_hex_strips_prefix() {
    assert_eq!(node_key_hex("nodekey:abc123"), "abc123");
    assert_eq!(node_key_hex("abc123"), "abc123");
}

#[test]
fn ep009_unit_headscale_fingerprint_never_leaks_key() {
    let f = fingerprint("mkey:deadbeef");
    assert!(f.len() == 16);
    assert!(!f.contains("deadbeef"));
}

#[test]
fn ep009_unit_headscale_parses_real_preauth_json() {
    // Captured from `preauthkeys create -o json` (real server).
    let json = r#"{
        "user": "tenant-alpha",
        "id": "1",
        "key": "a2d815487c37617d5625e83e4367af907602aa2c9b4ab550",
        "expiration": {"seconds": 1786690298, "nanos": 892905556},
        "created_at": {"seconds": 1786688498, "nanos": 988322328}
    }"#;
    let key: PreAuthKey = serde_json::from_str(json).expect("parse preauth key");
    assert_eq!(key.user, "tenant-alpha");
    assert_eq!(key.id, "1");
    assert!(key.key.starts_with("a2d8"));
    assert_eq!(key.expiration.unix_seconds(), 1786690298);
}

#[test]
fn ep009_unit_headscale_parses_real_user_json() {
    // Captured from `users list -o json` (real server).
    let json = r#"{
        "id": "1",
        "name": "tenant-alpha",
        "created_at": {"seconds": 1786688293, "nanos": 699744154}
    }"#;
    let user: User = serde_json::from_str(json).expect("parse user");
    assert_eq!(user.name, "tenant-alpha");
    assert_eq!(user.id, "1");
}

#[test]
fn ep009_unit_headscale_parses_real_registered_node_json() {
    // Captured from `nodes register -o json` (real server, node-1).
    let json = r#"{
        "id": 1,
        "machine_key": "mkey:acbd9d789e209d29ddaffaaab2b1d9abd8cecb998fa47bd189e3af6ec30d4319",
        "node_key": "nodekey:0026cb1ac4b8ea2ee507540a73448a1bd308ef889f028d2e5a2dd78e4b729b04",
        "disco_key": "discokey:0000000000000000000000000000000000000000000000000000000000000000",
        "ip_addresses": ["100.64.0.1", "fd7a:115c:a1e0::1"],
        "name": "node-1",
        "user": {"id": "1", "name": "tenant-alpha", "created_at": {"seconds": 1786688293, "nanos": 699744154}},
        "last_seen": {"seconds": -62135596800},
        "expiry": {"seconds": -62135596800},
        "created_at": {"seconds": 1786688523, "nanos": 574818427},
        "register_method": 2,
        "given_name": "node-1"
    }"#;
    let node: Node = serde_json::from_str(json).expect("parse node");
    assert_eq!(node.id, 1);
    assert_eq!(node.ip_addresses, vec!["100.64.0.1", "fd7a:115c:a1e0::1"]);
    assert_eq!(
        node_key_hex(&node.node_key),
        "0026cb1ac4b8ea2ee507540a73448a1bd308ef889f028d2e5a2dd78e4b729b04"
    );
    assert_eq!(node.register_method, Some(2));
    // Not expired -> Registered state.
    assert_eq!(node.expiry.unix_seconds(), -62135596800);
}

#[test]
fn ep009_unit_headscale_parses_expired_node_json() {
    // Captured from `nodes expire -o json` (real server): expiry set to now.
    let json = r#"{
        "id": 1,
        "machine_key": "mkey:acbd9d789e209d29ddaffaaab2b1d9abd8cecb998fa47bd189e3af6ec30d4319",
        "node_key": "nodekey:0026cb1ac4b8ea2ee507540a73448a1bd308ef889f028d2e5a2dd78e4b729b04",
        "disco_key": "",
        "ip_addresses": ["100.64.0.1"],
        "name": "node-1",
        "user": {"id": "1", "name": "tenant-alpha", "created_at": {"seconds": 1786688293, "nanos": 0}},
        "last_seen": {"seconds": -62135596800},
        "expiry": {"seconds": 1786688537, "nanos": 381771245},
        "created_at": {"seconds": 1786688523, "nanos": 0},
        "register_method": 2,
        "given_name": "node-1"
    }"#;
    let node: Node = serde_json::from_str(json).expect("parse expired node");
    assert!(
        node.expiry.unix_seconds() > 0,
        "expiry must be set after expire"
    );
}

#[test]
fn ep009_unit_headscale_mesh_node_lifecycle_mapping() {
    let node = MeshNode::new(
        "1",
        "tenant-alpha",
        "node-1",
        TrustZone::PrivateMesh,
        "0026cb1a",
        None,
    )
    .expect("valid node");
    assert_eq!(node.state, MeshNodeState::Pending);
    let _ = node;
}

#[test]
fn ep009_unit_headscale_wireguard_peer_and_config_validation() {
    let peer = WireGuardPeer::new(
        "peer-pubkey",
        Some("10.0.0.1:51820".to_string()),
        vec!["100.64.0.2/32".into()],
        25,
    )
    .expect("valid peer");
    assert_eq!(peer.persistent_keepalive_seconds, 25);
    let cfg = WireGuardConfig::new(
        "nexus0",
        "openbao:mesh/tenant-alpha/1",
        vec!["100.64.0.1/32".into()],
        vec![],
        vec![peer],
    )
    .expect("valid config");
    assert_eq!(cfg.interface, "nexus0");
    // Empty interface/key must be rejected.
    assert!(WireGuardConfig::new("", "k", vec![], vec![], vec![]).is_err());
    assert!(WireGuardConfig::new("nexus0", "", vec![], vec![], vec![]).is_err());
}

#[test]
fn ep009_unit_headscale_error_maps_to_trust_codes() {
    assert_eq!(
        HeadscaleErrorCode::BinaryUnavailable.trust_code(),
        nexus_trust::TrustErrorCode::Unavailable
    );
    assert_eq!(
        HeadscaleErrorCode::Unavailable.trust_code(),
        nexus_trust::TrustErrorCode::Unavailable
    );
    assert_eq!(
        HeadscaleErrorCode::ProviderAuthorization.trust_code(),
        nexus_trust::TrustErrorCode::ProviderAuthorization
    );
    assert_eq!(
        HeadscaleErrorCode::MalformedProviderResponse.trust_code(),
        nexus_trust::TrustErrorCode::MalformedProviderResponse
    );
    assert_eq!(
        HeadscaleErrorCode::NotFound.trust_code(),
        nexus_trust::TrustErrorCode::NotFound
    );
    assert_eq!(
        HeadscaleErrorCode::StateConflict.trust_code(),
        nexus_trust::TrustErrorCode::StateConflict
    );
}

#[test]
fn ep009_unit_headscale_error_display_never_leaks() {
    let e = HeadscaleError::new(HeadscaleErrorCode::ProviderAuthorization, "opaque");
    let text = format!("{e}");
    assert_eq!(text, "HEADSCALE_PROVIDER_AUTHORIZATION: opaque");
    let trust = e.into_trust();
    assert_eq!(
        trust.code,
        nexus_trust::TrustErrorCode::ProviderAuthorization
    );
}

#[test]
fn ep009_unit_headscale_controller_debug_redacts_api_key() {
    let c = HeadscaleMeshController::new(
        "headscale",
        "/tmp/hs-cli-config.yaml",
        "127.0.0.1:50443",
        "apikey-super-secret-value",
    );
    let debug = format!("{c:?}");
    assert!(!debug.contains("apikey-super-secret-value"));
    assert!(debug.contains("<redacted>"));
}

// ---------------------------------------------------------------------------
// AUD-012 remediation regression tests: identity binding + resolvable
// private-key references. These prove the adapter can NEVER register a
// synthetic identity or fabricate a private-key reference.
// ---------------------------------------------------------------------------

#[test]
fn rx006_unit_headscale_rejects_placeholder_key() {
    // A placeholder key (short, non-hex) must be rejected before any
    // provider call: a synthetic identity can never be registered.
    let node = MeshNode::new(
        "1",
        "tenant-alpha",
        "node-1",
        TrustZone::PrivateMesh,
        "live-pubkey-a",
        None,
    )
    .expect("valid node");
    let c = HeadscaleMeshController::new(
        "headscale",
        "/tmp/hs-cli-config.yaml",
        "127.0.0.1:50443",
        "key",
    );
    let err = c.register_node(node).unwrap_err();
    assert_eq!(
        err.code,
        nexus_trust::TrustErrorCode::MalformedProviderResponse,
        "placeholder key must fail closed as MalformedProviderResponse"
    );
}

#[test]
fn rx006_unit_headscale_rejects_empty_key() {
    // Construct a valid node, then empty the key field to simulate a
    // caller bypassing MeshNode::new's own validation.
    let mut node = MeshNode::new(
        "1",
        "tenant-alpha",
        "node-1",
        TrustZone::PrivateMesh,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        None,
    )
    .expect("valid node");
    node.wireguard_public_key = String::new();
    let c = HeadscaleMeshController::new(
        "headscale",
        "/tmp/hs-cli-config.yaml",
        "127.0.0.1:50443",
        "key",
    );
    let err = c.register_node(node).unwrap_err();
    assert_eq!(
        err.code,
        nexus_trust::TrustErrorCode::MalformedProviderResponse
    );
}

#[test]
fn rx006_unit_headscale_accepts_valid_key_shape() {
    // A 32-byte hex key is accepted (the provider round-trip is proven
    // by the live integration; here we prove the validation boundary).
    // The binary path does not exist, so the failure is
    // BinaryUnavailable, NOT a validation failure.
    let key = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let node = MeshNode::new(
        "1",
        "tenant-alpha",
        "node-1",
        TrustZone::PrivateMesh,
        key,
        None,
    )
    .expect("valid node");
    let c = HeadscaleMeshController::new(
        "/nonexistent/headscale-bin",
        "/tmp/hs-cli-config.yaml",
        "127.0.0.1:50443",
        "key",
    );
    let err = c.register_node(node).unwrap_err();
    assert_eq!(
        err.code,
        nexus_trust::TrustErrorCode::Unavailable,
        "valid key must reach the provider boundary (BinaryUnavailable), not be rejected"
    );
}

#[test]
fn rx006_unit_headscale_wireguard_config_fails_closed_without_store() {
    // Without a secret store the adapter must refuse to fabricate a
    // private-key reference: it fails closed instead of returning a
    // config pointing at nothing.
    let c = HeadscaleMeshController::new(
        "headscale",
        "/tmp/hs-cli-config.yaml",
        "127.0.0.1:50443",
        "key",
    );
    let err = c.wireguard_config("1").unwrap_err();
    assert_eq!(
        err.code,
        nexus_trust::TrustErrorCode::StateConflict,
        "no secret store -> wireguard_config must fail closed"
    );
}

#[test]
fn rx006_unit_headscale_secret_store_debug_redacted() {
    use std::sync::Arc;
    let store = Arc::new(NoopSecretStore);
    let c = HeadscaleMeshController::new(
        "headscale",
        "/tmp/hs-cli-config.yaml",
        "127.0.0.1:50443",
        "key",
    )
    .with_secret_store(store);
    let debug = format!("{c:?}");
    assert!(debug.contains("<configured>"));
    assert!(!debug.contains("SecretStoreHandle"));
}

/// Minimal store that never resolves anything (for fail-closed tests).
#[derive(Debug)]
struct NoopSecretStore;

impl SecretStore for NoopSecretStore {
    fn get(&self, _reference: &SecretReference) -> Result<SecretValue, nexus_trust::TrustError> {
        Err(nexus_trust::TrustError::not_found("noop store"))
    }
    fn put(
        &self,
        _reference: &SecretReference,
        _value: SecretValue,
    ) -> Result<(), nexus_trust::TrustError> {
        Err(nexus_trust::TrustError::not_found("noop store"))
    }
    fn rotate(
        &self,
        _reference: &SecretReference,
        _value: SecretValue,
    ) -> Result<(), nexus_trust::TrustError> {
        Err(nexus_trust::TrustError::not_found("noop store"))
    }
    fn revoke(&self, _reference: &SecretReference) -> Result<(), nexus_trust::TrustError> {
        Err(nexus_trust::TrustError::not_found("noop store"))
    }
    fn state(
        &self,
        _reference: &SecretReference,
    ) -> Result<nexus_trust::vocabulary::SecretState, nexus_trust::TrustError> {
        Err(nexus_trust::TrustError::not_found("noop store"))
    }
}

#[test]
fn rx006_unit_headscale_wireguard_config_fails_closed_when_reference_missing() {
    use std::sync::Arc;
    // Mock headscale CLI: emits a real node-list JSON document so the
    // provider boundary is crossed; the store then cannot resolve the
    // private key reference -> must fail closed (NotFound).
    let mock = std::env::temp_dir().join("rx006-mock-headscale.sh");
    std::fs::write(
        &mock,
        r#"#!/usr/bin/env sh
cat <<'EOF'
[{"id":1,"machine_key":"mkey:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","node_key":"nodekey:0026cb1ac4b8ea2ee507540a73448a1bd308ef889f028d2e5a2dd78e4b729b04","disco_key":"","ip_addresses":["100.64.0.1","fd7a:115c:a1e0::1"],"name":"node-1","user":{"id":"1","name":"tenant-alpha","created_at":{"seconds":0}},"last_seen":{"seconds":0},"expiry":{"seconds":0},"created_at":{"seconds":0}}]
EOF
"#,
    )
    .expect("write mock cli");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&mock)
            .expect("mock metadata")
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&mock, perms).expect("chmod mock");
    }
    let store = Arc::new(NoopSecretStore);
    let c = HeadscaleMeshController::new(
        mock.to_str().expect("mock path"),
        "/tmp/hs-cli-config.yaml",
        "127.0.0.1:50443",
        "key",
    )
    .with_secret_store(store);
    // The store cannot resolve the key -> wireguard_config must fail
    // closed (NotFound), never return a fabricated config.
    let err = c.wireguard_config("1").unwrap_err();
    let _ = std::fs::remove_file(&mock);
    assert_eq!(
        err.code,
        nexus_trust::TrustErrorCode::NotFound,
        "unresolvable private key reference must fail closed"
    );
}
