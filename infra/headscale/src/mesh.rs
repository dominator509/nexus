//! Headscale-compatible `MeshController` adapter (EP-009 M3).
//!
//! Implements the nexus-trust `MeshController` port against a REAL
//! pinned `headscale` CLI (v0.23.0) that talks gRPC to a real
//! Headscale server. The CLI is the provider's own admin client; this
//! adapter invokes it as a subprocess (mirroring the M2 SOPS store
//! pattern), parses the real `-o json` output, and maps results onto
//! the contract types.
//!
//! TRANSPORT (real, observed): the CLI connects to the server gRPC
//! endpoint with `HEADSCALE_CLI_ADDRESS` + `HEADSCALE_CLI_API_KEY`
//! over TLS (server must present a certificate; `cli.insecure: true`
//! in the CLI config enables InsecureSkipVerify for the test/self-host
//! path). API keys are created by the server operator via
//! `headscale apikeys create`. The API key is accepted in-memory and
//! NEVER logged, serialized, or emitted in telemetry.
//!
//! SECRET HANDLING: machine keys, node keys, and preauth keys are
//! provider protocol identifiers, not bearer credentials for Nexus
//! itself; they are still never logged and never appear in evidence
//! (fingerprints only).

use std::process::Command;

use nexus_trust::mesh::{MeshController, MeshNode, WireGuardConfig, WireGuardPeer};
use nexus_trust::vocabulary::{MeshNodeState, TrustZone};
use nexus_trust::TrustError;

use crate::error::{HeadscaleError, HeadscaleErrorCode};
use crate::model::{fingerprint, machine_key, node_key_hex, Node, PreAuthKey, User};

/// A real Headscale mesh controller adapter.
#[derive(Debug, Clone)]
pub struct HeadscaleMeshController {
    /// Path to the pinned headscale CLI binary.
    binary: String,
    /// Path to a headscale CLI config file (HEADSCALE_CONFIG).
    config_file: String,
    /// Server gRPC address (host:port).
    address: String,
    /// API key (held in memory; Debug is redacted via manual impl).
    api_key: RedactedKey,
}

/// API key wrapper with redacted Debug.
#[derive(Clone)]
struct RedactedKey(String);

impl std::fmt::Debug for RedactedKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RedactedKey(<redacted>)")
    }
}

impl HeadscaleMeshController {
    /// Construct the adapter.
    ///
    /// `config_file` must be a headscale CLI config (the adapter writes
    /// nothing to disk itself; the operator provisions the config and
    /// API key out-of-band).
    pub fn new(
        binary: impl Into<String>,
        config_file: impl Into<String>,
        address: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            binary: binary.into(),
            config_file: config_file.into(),
            address: address.into(),
            api_key: RedactedKey(api_key.into()),
        }
    }

    /// Run a headscale CLI subcommand; returns stdout on success.
    fn run(&self, args: &[&str]) -> Result<String, HeadscaleError> {
        let out = Command::new(&self.binary)
            .args(args)
            .env("HEADSCALE_CONFIG", &self.config_file)
            .env("HEADSCALE_CLI_ADDRESS", &self.address)
            .env("HEADSCALE_CLI_API_KEY", &self.api_key.0)
            .env("CI", "true")
            .output()
            .map_err(|e| {
                HeadscaleError::new(
                    HeadscaleErrorCode::BinaryUnavailable,
                    format!("cannot run headscale CLI: {e}"),
                )
            })?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let code = if stderr.contains("invalid token")
                || stderr.contains("permission")
                || stderr.contains("unauthorized")
                || stderr.contains("failed to validate")
            {
                HeadscaleErrorCode::ProviderAuthorization
            } else if stderr.contains("Could not connect")
                || stderr.contains("context deadline exceeded")
            {
                HeadscaleErrorCode::Unavailable
            } else {
                HeadscaleErrorCode::MalformedProviderResponse
            };
            Err(HeadscaleError::new(code, "headscale CLI failed (redacted)"))
        }
    }

    /// Ensure a tenant (headscale user) exists. Idempotent.
    fn ensure_user(&self, tenant_id: &str) -> Result<(), HeadscaleError> {
        // `users create` succeeds on first call and fails with exit 1
        // "user already exists" on subsequent calls. Both are success
        // for idempotent registration.
        let out = Command::new(&self.binary)
            .args(["users", "create", tenant_id])
            .env("HEADSCALE_CONFIG", &self.config_file)
            .env("HEADSCALE_CLI_ADDRESS", &self.address)
            .env("HEADSCALE_CLI_API_KEY", &self.api_key.0)
            .env("CI", "true")
            .output()
            .map_err(|e| {
                HeadscaleError::new(
                    HeadscaleErrorCode::BinaryUnavailable,
                    format!("cannot run headscale CLI: {e}"),
                )
            })?;
        if out.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("already exists") {
            return Ok(());
        }
        let code = if stderr.contains("invalid token")
            || stderr.contains("permission")
            || stderr.contains("unauthorized")
            || stderr.contains("failed to validate")
        {
            HeadscaleErrorCode::ProviderAuthorization
        } else if stderr.contains("Could not connect")
            || stderr.contains("context deadline exceeded")
        {
            HeadscaleErrorCode::Unavailable
        } else {
            HeadscaleErrorCode::MalformedProviderResponse
        };
        Err(HeadscaleError::new(
            code,
            "headscale users create failed (redacted)",
        ))
    }

    /// Create a pending node with a fresh machine key, then register it.
    fn create_and_register(&self, node: &MeshNode) -> Result<Node, HeadscaleError> {
        // Machine key: 32 random bytes hex, canonical mkey: prefix.
        let mkey = fresh_machine_key();
        self.ensure_user(&node.tenant_id)?;
        let created = self.run(&[
            "debug",
            "create-node",
            "--user",
            &node.tenant_id,
            "--name",
            &node.name,
            "--key",
            &mkey,
            "-o",
            "json",
        ])?;
        let pending: Node = serde_json::from_str(&created).map_err(|e| {
            HeadscaleError::new(
                HeadscaleErrorCode::MalformedProviderResponse,
                format!("cannot parse create-node output: {e}"),
            )
        })?;
        let registered = self.run(&[
            "nodes",
            "register",
            "--user",
            &node.tenant_id,
            "--key",
            &mkey,
            "-o",
            "json",
        ])?;
        let reg: Node = serde_json::from_str(&registered).map_err(|e| {
            HeadscaleError::new(
                HeadscaleErrorCode::MalformedProviderResponse,
                format!("cannot parse register output: {e}"),
            )
        })?;
        let _ = pending;
        Ok(reg)
    }
}

/// Generate a fresh machine key: `mkey:` + 64 hex chars (32 bytes).
fn fresh_machine_key() -> String {
    use std::fs;
    let mut buf = [0u8; 32];
    if let Ok(f) = fs::File::open("/dev/urandom") {
        use std::io::Read;
        let mut f = f;
        let _ = f.read_exact(&mut buf);
    }
    let hex: String = buf.iter().map(|b| format!("{b:02x}")).collect();
    machine_key(&hex)
}

/// Map a headscale node record onto the contract MeshNode.
fn to_mesh_node(n: &Node, tenant_id: &str) -> MeshNode {
    let name = if n.name.is_empty() {
        n.given_name.clone()
    } else {
        n.name.clone()
    };
    let state = if n.expiry.unix_seconds() > 0 {
        MeshNodeState::Revoked
    } else {
        MeshNodeState::Registered
    };
    MeshNode {
        node_id: n.id.to_string(),
        tenant_id: tenant_id.to_string(),
        name,
        zone: TrustZone::PrivateMesh,
        wireguard_public_key: node_key_hex(&n.node_key).to_string(),
        endpoint: None,
        state,
        last_seen_unix_s: n.last_seen.unix_seconds(),
    }
}

impl MeshController for HeadscaleMeshController {
    fn register_node(&self, node: MeshNode) -> Result<(), TrustError> {
        let reg = self
            .create_and_register(&node)
            .map_err(HeadscaleError::into_trust)?;
        if reg.ip_addresses.is_empty() {
            return Err(HeadscaleError::new(
                HeadscaleErrorCode::StateConflict,
                format!(
                    "node registered but no IP allocated (fingerprint {})",
                    fingerprint(&node.name)
                ),
            )
            .into_trust());
        }
        Ok(())
    }

    fn wireguard_config(&self, node_id: &str) -> Result<WireGuardConfig, TrustError> {
        // Resolve the node's tenant by listing all users/nodes: the
        // contract gives us only the node id, so find it first.
        let list = self
            .run(&["nodes", "list", "-o", "json"])
            .map_err(HeadscaleError::into_trust)?;
        let nodes: Vec<Node> = serde_json::from_str(&list).map_err(|e| {
            HeadscaleError::new(
                HeadscaleErrorCode::MalformedProviderResponse,
                format!("cannot parse node list: {e}"),
            )
            .into_trust()
        })?;
        let target = nodes
            .iter()
            .find(|n| n.id.to_string() == node_id)
            .ok_or_else(|| {
                HeadscaleError::new(
                    HeadscaleErrorCode::NotFound,
                    format!("node {node_id} not found"),
                )
                .into_trust()
            })?;
        let tenant = target
            .user
            .as_ref()
            .map(|u| u.name.clone())
            .unwrap_or_default();
        let peers: Vec<WireGuardPeer> = nodes
            .iter()
            .filter(|n| {
                n.id.to_string() != node_id && n.ip_addresses.iter().any(|ip| ip.contains('.'))
            })
            .map(|n| {
                WireGuardPeer::new(
                    node_key_hex(&n.node_key).to_string(),
                    None,
                    n.ip_addresses.clone(),
                    0,
                )
                .unwrap_or_else(|_| WireGuardPeer::new("", None, vec![], 0).expect("peer"))
            })
            .collect();
        let addresses = target.ip_addresses.clone();
        WireGuardConfig::new(
            "nexus0",
            format!("openbao:mesh/{tenant}/{node_id}"),
            addresses,
            vec![],
            peers,
        )
        .map_err(|e| {
            HeadscaleError::new(
                HeadscaleErrorCode::MalformedProviderResponse,
                format!("cannot build wireguard config: {e}"),
            )
            .into_trust()
        })
    }

    fn node_state(
        &self,
        node_id: &str,
        state: MeshNodeState,
        _now_unix_s: i64,
    ) -> Result<(), TrustError> {
        if state == MeshNodeState::Revoked {
            let id = node_id.parse::<u64>().map_err(|_| {
                HeadscaleError::new(
                    HeadscaleErrorCode::MalformedProviderResponse,
                    "node id must be numeric",
                )
                .into_trust()
            })?;
            self.run(&["nodes", "expire", "--identifier", &id.to_string()])
                .map_err(HeadscaleError::into_trust)?;
        }
        Ok(())
    }

    fn list_nodes(&self, tenant_id: &str) -> Result<Vec<MeshNode>, TrustError> {
        let list = self
            .run(&["nodes", "list", "-u", tenant_id, "-o", "json"])
            .map_err(HeadscaleError::into_trust)?;
        if list.trim().is_empty() || list.trim() == "null" {
            return Ok(vec![]);
        }
        let nodes: Vec<Node> = serde_json::from_str(&list).map_err(|e| {
            HeadscaleError::new(
                HeadscaleErrorCode::MalformedProviderResponse,
                format!("cannot parse node list: {e}"),
            )
            .into_trust()
        })?;
        Ok(nodes.iter().map(|n| to_mesh_node(n, tenant_id)).collect())
    }

    fn revoke_node(&self, node_id: &str) -> Result<(), TrustError> {
        let id = node_id.parse::<u64>().map_err(|_| {
            HeadscaleError::new(
                HeadscaleErrorCode::MalformedProviderResponse,
                "node id must be numeric",
            )
            .into_trust()
        })?;
        self.run(&["nodes", "expire", "--identifier", &id.to_string()])
            .map_err(HeadscaleError::into_trust)?;
        self.run(&[
            "nodes",
            "delete",
            "--identifier",
            &id.to_string(),
            "--force",
        ])
        .map_err(HeadscaleError::into_trust)?;
        Ok(())
    }
}

// Re-export helpers used by tests/examples.
pub use crate::model::PreAuthKey as _PreAuthKeyAlias;
pub use crate::model::User as _UserAlias;
#[allow(unused)]
fn _type_asserts(_u: User, _p: PreAuthKey) {}
