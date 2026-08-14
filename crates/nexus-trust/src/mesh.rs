//! Private mesh control (SPEC-005 behavior 7).
//!
//! The Nexus private mesh uses Headscale-compatible WireGuard and mTLS
//! to protect node communication. `MeshController` is the
//! provider-neutral port: it registers nodes, exposes per-node WireGuard
//! configuration (Headscale-compatible), reports node state, and
//! revokes membership. Raw WireGuard and standard mTLS paths coexist
//! (EP-009 acceptance obligation 4).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::TrustError;
use crate::vocabulary::{MeshNodeState, TrustZone};

/// A private mesh node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshNode {
    /// Node identifier.
    pub node_id: String,
    /// Tenant boundary.
    pub tenant_id: String,
    /// Canonical node name.
    pub name: String,
    /// Trust zone the node belongs to.
    pub zone: TrustZone,
    /// WireGuard public key (Base64).
    pub wireguard_public_key: String,
    /// WireGuard endpoint (host:port), if known.
    pub endpoint: Option<String>,
    /// Current node state.
    pub state: MeshNodeState,
    /// Last seen time, unix seconds (0 = never).
    pub last_seen_unix_s: i64,
}

impl MeshNode {
    /// Construct a node; rejects empty fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_id: impl Into<String>,
        tenant_id: impl Into<String>,
        name: impl Into<String>,
        zone: TrustZone,
        wireguard_public_key: impl Into<String>,
        endpoint: Option<String>,
    ) -> Result<Self, MeshControllerError> {
        let node_id = node_id.into();
        let tenant_id = tenant_id.into();
        let name = name.into();
        let wireguard_public_key = wireguard_public_key.into();
        if node_id.trim().is_empty()
            || tenant_id.trim().is_empty()
            || name.trim().is_empty()
            || wireguard_public_key.trim().is_empty()
        {
            return Err(MeshControllerError::EmptyField);
        }
        Ok(Self {
            node_id,
            tenant_id,
            name,
            zone,
            wireguard_public_key,
            endpoint,
            state: MeshNodeState::Pending,
            last_seen_unix_s: 0,
        })
    }
}

/// A WireGuard interface configuration (Headscale-compatible).
///
/// The config carries the node's own private key reference (never the
/// key material inline) plus the peer set. Raw WireGuard and
/// Headscale-compatible paths both consume this record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireGuardConfig {
    /// Interface name, e.g. `nexus0`.
    pub interface: String,
    /// Reference to the node's private key in the secret store.
    pub private_key_reference: String,
    /// Assigned address CIDRs.
    pub addresses: Vec<String>,
    /// DNS servers for the mesh.
    pub dns: Vec<String>,
    /// Peer public keys with endpoints and allowed IPs.
    pub peers: Vec<WireGuardPeer>,
}

impl WireGuardConfig {
    /// Construct a config; rejects empty interface/key reference.
    pub fn new(
        interface: impl Into<String>,
        private_key_reference: impl Into<String>,
        addresses: Vec<String>,
        dns: Vec<String>,
        peers: Vec<WireGuardPeer>,
    ) -> Result<Self, MeshControllerError> {
        let interface = interface.into();
        let private_key_reference = private_key_reference.into();
        if interface.trim().is_empty() || private_key_reference.trim().is_empty() {
            return Err(MeshControllerError::EmptyField);
        }
        Ok(Self {
            interface,
            private_key_reference,
            addresses,
            dns,
            peers,
        })
    }
}

/// One WireGuard peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireGuardPeer {
    /// Peer public key (Base64).
    pub public_key: String,
    /// Peer endpoint (host:port), if known.
    pub endpoint: Option<String>,
    /// Allowed IPs for this peer.
    pub allowed_ips: Vec<String>,
    /// Persistent keepalive seconds (0 = none).
    pub persistent_keepalive_seconds: u16,
}

impl WireGuardPeer {
    /// Construct a peer; rejects empty public key.
    pub fn new(
        public_key: impl Into<String>,
        endpoint: Option<String>,
        allowed_ips: Vec<String>,
        persistent_keepalive_seconds: u16,
    ) -> Result<Self, MeshControllerError> {
        let public_key = public_key.into();
        if public_key.trim().is_empty() {
            return Err(MeshControllerError::EmptyField);
        }
        Ok(Self {
            public_key,
            endpoint,
            allowed_ips,
            persistent_keepalive_seconds,
        })
    }
}

/// Provider-neutral private mesh controller port.
pub trait MeshController {
    /// Register a node into the mesh (Pending -> Registered).
    fn register_node(&self, node: MeshNode) -> Result<(), TrustError>;
    /// Fetch the WireGuard config for a registered node.
    fn wireguard_config(&self, node_id: &str) -> Result<WireGuardConfig, TrustError>;
    /// Report a node state observation.
    fn node_state(
        &self,
        node_id: &str,
        state: MeshNodeState,
        now_unix_s: i64,
    ) -> Result<(), TrustError>;
    /// List all nodes in a tenant.
    fn list_nodes(&self, tenant_id: &str) -> Result<Vec<MeshNode>, TrustError>;
    /// Revoke a node from the mesh (terminal).
    fn revoke_node(&self, node_id: &str) -> Result<(), TrustError>;
}

/// Mesh construction errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshControllerError {
    /// A required field was empty/whitespace.
    EmptyField,
}

impl fmt::Display for MeshControllerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mesh fields must not be empty")
    }
}

impl std::error::Error for MeshControllerError {}
