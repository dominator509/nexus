//! EP-009 M3 unit tests: headscale model parsing and error mapping.
//!
//! Pure unit tests (no live provider). The JSON shapes below are the
//! REAL `headscale v0.23.0 -o json` outputs captured from the pinned
//! container on 2026-08-14 (see Decision Log). The real provider
//! proofs live in tests/trust/ (ep009_integration_*).

use nexus_trust::mesh::{MeshNode, WireGuardConfig, WireGuardPeer};
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
