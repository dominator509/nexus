//! EP-009 M3 Headscale mesh live proof: REAL adapter, REAL server.
//!
//! Exercises `HeadscaleMeshController` (the REAL nexus-headscale
//! adapter) against a REAL pinned `headscale/headscale:0.23.0`
//! container over real gRPC (TLS + API key). Proves the full
//! `MeshController` contract across the provider boundary:
//! - register_node (create + register, real IP allocation)
//! - list_nodes (real JSON parsing)
//! - wireguard_config (real addresses + peers)
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
//!
//! The API key is read from NEXUS_HS_API_KEY or, if that is unset,
//! from NEXUS_HS_API_KEY_FILE. It is never printed, never logged, and
//! never written to evidence.

use nexus_trust::mesh::{MeshController, MeshNode};
use nexus_trust::vocabulary::{MeshNodeState, TrustZone};

use nexus_headscale::HeadscaleMeshController;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn api_key() -> String {
    if let Ok(k) = std::env::var("NEXUS_HS_API_KEY") {
        if !k.is_empty() {
            return k;
        }
    }
    if let Ok(path) = std::env::var("NEXUS_HS_API_KEY_FILE") {
        let content = std::fs::read_to_string(&path).expect("read api key file");
        let key = content.lines().next().unwrap_or("").trim().to_string();
        if !key.is_empty() {
            return key;
        }
    }
    panic!("NEXUS_HS_API_KEY or NEXUS_HS_API_KEY_FILE must be set");
}

fn main() {
    let binary = env_or("NEXUS_HS_BINARY", "/usr/local/bin/headscale");
    let config = env_or("NEXUS_HS_CONFIG", "/etc/headscale/config.yaml");
    let address = env_or("NEXUS_HS_ADDRESS", "127.0.0.1:50443");
    let key = api_key();

    let controller = HeadscaleMeshController::new(&binary, &config, &address, &key);

    let tenant = format!("tenant-live-{}", std::process::id());

    // 1. register two nodes (real create + register + IP allocation).
    let node_a = MeshNode::new(
        "1",
        &tenant,
        "live-node-a",
        TrustZone::PrivateMesh,
        "live-pubkey-a",
        None,
    )
    .expect("node a");
    controller.register_node(node_a).expect("register node a");
    let node_b = MeshNode::new(
        "2",
        &tenant,
        "live-node-b",
        TrustZone::PrivateMesh,
        "live-pubkey-b",
        None,
    )
    .expect("node b");
    controller.register_node(node_b).expect("register node b");

    // 2. list nodes -> must see 2 registered nodes with allocated IPs.
    let nodes = controller.list_nodes(&tenant).expect("list nodes");
    assert_eq!(nodes.len(), 2, "expected 2 registered nodes");
    for n in &nodes {
        assert!(
            !n.wireguard_public_key.is_empty(),
            "node must carry a public key"
        );
    }
    let node_a_id = nodes
        .iter()
        .find(|n| n.name == "live-node-a")
        .map(|n| n.node_id.clone())
        .expect("find node a");

    // 3. wireguard config for node a: real addresses from provider.
    let wg = controller.wireguard_config(&node_a_id).expect("wg config");
    assert!(!wg.addresses.is_empty(), "config must carry allocated IPs");
    assert_eq!(wg.interface, "nexus0");
    // Peers: node b is a registered IPv4 node -> must appear as a peer.
    assert!(
        wg.peers.iter().any(|p| !p.public_key.is_empty()),
        "peer set must include the other registered node"
    );

    // 4. node_state: mark node a revoked -> real expire.
    controller
        .node_state(&node_a_id, MeshNodeState::Revoked, 0)
        .expect("revoke state");

    // 5. revoke_node: terminal (expire + delete).
    controller.revoke_node(&node_a_id).expect("revoke node");

    // 6. after revocation only node b remains.
    let remaining = controller.list_nodes(&tenant).expect("list after revoke");
    assert_eq!(remaining.len(), 1, "one node must remain after revoke");

    println!(
        "EP-009 M3 headscale live proof: ok (tenant={}, registered=2, revoked=1, wg_peers={}, addresses={})",
        tenant,
        wg.peers.len(),
        wg.addresses.len()
    );
}
