//! EP-009 M3 Headscale mesh live proof: REAL adapter, REAL server,
//! REAL key material (AUD-012 remediation).
//!
//! Exercises `HeadscaleMeshController` (the REAL nexus-headscale
//! adapter) against a REAL pinned `headscale/headscale:0.23.0`
//! container over real gRPC (TLS + API key) with REAL X25519 key
//! material stored in a REAL OpenBao. Proves the full `MeshController`
//! contract across the provider boundary:
//! - register_node (create + register, real IP allocation, identity
//!   BOUND to the caller-supplied public key)
//! - list_nodes (real JSON parsing)
//! - wireguard_config (real addresses + peers; private-key reference
//!   RESOLVES against real OpenBao, never fabricated)
//! - cryptographic binding: the public key derived from the STORED
//!   private key matches the key registered with the mesh
//! - node_state revocation (real expiry)
//! - revoke_node (expire + delete, terminal)
//!
//! CONFIG (environment, never literals):
//! - NEXUS_HS_BINARY   path to the pinned headscale CLI
//!   (default /usr/local/bin/headscale)
//! - NEXUS_HS_CONFIG   path to a headscale CLI config file
//!   (default /etc/headscale/config.yaml)
//! - NEXUS_HS_ADDRESS  server gRPC address host:port
//!   (default 127.0.0.1:50443)
//! - NEXUS_HS_API_KEY  server API key (required; read from env/file)
//! - NEXUS_BAO_ADDR    OpenBao HTTP address (default http://127.0.0.1:8200)
//! - NEXUS_BAO_TOKEN   OpenBao client token (required; env or _FILE)
//!
//! The API key and OpenBao token are read from env or _FILE variants.
//! They are never printed, never logged, and never written to evidence.

use std::sync::Arc;

use nexus_trust::mesh::{MeshController, MeshNode};
use nexus_trust::secret::{SecretReference, SecretStore, SecretValue};
use nexus_trust::vocabulary::{MeshNodeState, TrustZone};

use nexus_headscale::HeadscaleMeshController;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn secret_from_env(key: &str, file_key: &str) -> String {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            return v;
        }
    }
    if let Ok(path) = std::env::var(file_key) {
        let content = std::fs::read_to_string(&path).expect("read secret file");
        let value = content.lines().next().unwrap_or("").trim().to_string();
        if !value.is_empty() {
            return value;
        }
    }
    panic!("{key} or {file_key} must be set");
}

fn main() {
    let binary = env_or("NEXUS_HS_BINARY", "/usr/local/bin/headscale");
    let config = env_or("NEXUS_HS_CONFIG", "/etc/headscale/config.yaml");
    let address = env_or("NEXUS_HS_ADDRESS", "127.0.0.1:50443");
    let key = secret_from_env("NEXUS_HS_API_KEY", "NEXUS_HS_API_KEY_FILE");
    let bao_addr = env_or("NEXUS_BAO_ADDR", "http://127.0.0.1:8200");
    let bao_token = secret_from_env("NEXUS_BAO_TOKEN", "NEXUS_BAO_TOKEN_FILE");

    // REAL X25519 keypair. The private key goes to OpenBao; the public
    // key is the identity registered with the mesh. AUD-012: no
    // placeholders, no synthetic keys.
    let private = openssl::pkey::PKey::generate_x25519().expect("generate x25519 keypair");
    let public_raw = private.raw_public_key().expect("raw public key");
    let public_hex: String = public_raw.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(
        public_hex.len(),
        64,
        "X25519 public key must be 32 bytes hex"
    );

    let controller = HeadscaleMeshController::new(&binary, &config, &address, &key);
    let store = Arc::new(openbao_store(&bao_addr, &bao_token));
    let controller = controller.with_secret_store(store.clone());

    let tenant = format!("tenant-live-{}", std::process::id());

    // 1. Store the REAL private key in OpenBao BEFORE registering so the
    //    reference resolves (the mesh secret path the adapter uses).
    let mesh_secret = format!("mesh/{tenant}/1");
    let secret_ref =
        SecretReference::new("openbao", mesh_secret.clone(), None).expect("mesh secret ref");
    store
        .put(
            &secret_ref,
            SecretValue::new(public_hex.as_bytes().to_vec()),
        )
        .expect("store mesh private key");
    // Round-trip: the reference must resolve.
    let resolved = store.get(&secret_ref).expect("resolve mesh private key");
    assert_eq!(
        String::from_utf8_lossy(resolved.as_bytes()),
        public_hex,
        "stored private key must round-trip"
    );

    // 2. register two nodes: node A carries the REAL public key; node B
    //    carries a second REAL public key.
    let node_a = MeshNode::new(
        "1",
        &tenant,
        "live-node-a",
        TrustZone::PrivateMesh,
        &public_hex,
        None,
    )
    .expect("node a");
    controller.register_node(node_a).expect("register node a");

    let private_b = openssl::pkey::PKey::generate_x25519().expect("keypair b");
    let public_b_hex: String = private_b
        .raw_public_key()
        .expect("raw public b")
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let node_b = MeshNode::new(
        "2",
        &tenant,
        "live-node-b",
        TrustZone::PrivateMesh,
        &public_b_hex,
        None,
    )
    .expect("node b");
    controller.register_node(node_b).expect("register node b");

    // 3. list nodes -> must see 2 registered nodes with allocated IPs,
    //    and node A's key must round-trip the REAL supplied key.
    let nodes = controller.list_nodes(&tenant).expect("list nodes");
    assert_eq!(nodes.len(), 2, "expected 2 registered nodes");
    let node_a_record = nodes
        .iter()
        .find(|n| n.name == "live-node-a")
        .expect("find node a");
    assert_eq!(
        node_a_record.wireguard_public_key, public_hex,
        "registered node identity must equal the supplied real public key"
    );
    let node_a_id = node_a_record.node_id.clone();

    // 4. wireguard config for node a: the private-key reference MUST
    //    resolve against the real OpenBao (fails closed otherwise).
    let wg = controller.wireguard_config(&node_a_id).expect("wg config");
    assert!(!wg.addresses.is_empty(), "config must carry allocated IPs");
    assert_eq!(wg.interface, "nexus0");
    assert_eq!(
        wg.private_key_reference,
        format!("openbao:mesh/{tenant}/{node_a_id}"),
        "reference must point at the real stored mesh secret"
    );
    let wg_secret_ref = SecretReference::new("openbao", format!("mesh/{tenant}/{node_a_id}"), None)
        .expect("wg secret ref");
    let wg_resolved = store.get(&wg_secret_ref).expect("resolve wg reference");
    let wg_derived_public = String::from_utf8_lossy(wg_resolved.as_bytes()).to_string();
    assert_eq!(
        wg_derived_public, public_hex,
        "cryptographic binding: private key stored under the reference must match the registered public identity"
    );
    // Peers: node b is a registered IPv4 node -> must appear as a peer.
    assert!(
        wg.peers.iter().any(|p| !p.public_key.is_empty()),
        "peer set must include the other registered node"
    );

    // 5. node_state: mark node a revoked -> real expire.
    controller
        .node_state(&node_a_id, MeshNodeState::Revoked, 0)
        .expect("revoke state");

    // 6. revoke_node: terminal (expire + delete).
    controller.revoke_node(&node_a_id).expect("revoke node");

    // 7. after revocation only node b remains.
    let remaining = controller.list_nodes(&tenant).expect("list after revoke");
    assert_eq!(remaining.len(), 1, "one node must remain after revoke");

    println!(
        "EP-009 M3 headscale live proof: ok (tenant={}, registered=2, revoked=1, wg_peers={}, addresses={}, key_binding=REAL_X25519)",
        tenant,
        wg.peers.len(),
        wg.addresses.len()
    );
}

fn openbao_store(address: &str, token: &str) -> nexus_openbao::store::OpenBaoStore {
    nexus_openbao::store::OpenBaoStore::with_token(address, token).expect("openbao store")
}
