//! Serde models for the REAL headscale v0.23.0 CLI JSON output.
//!
//! Field names and shapes were captured live from the pinned
//! `headscale/headscale:0.23.0` container on 2026-08-14 (see Decision
//! Log). The CLI talks gRPC to the server; `-o json` emits the v1
//! proto messages as JSON. These structs parse that exact output.
//!
//! Secret-handling rule: machine keys / node keys / preauth keys are
//! identifiers used by the provider protocol. They are NEVER logged by
//! this adapter and never serialized into evidence; the adapter maps
//! them to fingerprints where they leave the provider boundary.

use serde::{Deserialize, Serialize};

/// A protobuf-style timestamp (`{"seconds": N, "nanos": N}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtoTimestamp {
    pub seconds: i64,
    #[serde(default)]
    pub nanos: i32,
}

impl ProtoTimestamp {
    /// Unix seconds (negative epoch means "never" per protobuf default).
    pub fn unix_seconds(&self) -> i64 {
        self.seconds
    }
}

/// Headscale user record (`headscale users list -o json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub created_at: ProtoTimestamp,
}

/// Headscale pre-auth key record
/// (`headscale preauthkeys create -o json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreAuthKey {
    #[serde(default)]
    pub user: String,
    pub id: String,
    pub key: String,
    pub expiration: ProtoTimestamp,
    pub created_at: ProtoTimestamp,
}

/// Headscale node record (`headscale nodes list -o json`,
/// `headscale nodes register -o json`, `headscale debug create-node
/// -o json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Node {
    /// Numeric node id. `debug create-node` output omits it; the
    /// register/list responses always include it.
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub machine_key: String,
    #[serde(default)]
    pub node_key: String,
    #[serde(default)]
    pub disco_key: String,
    #[serde(default)]
    pub ip_addresses: Vec<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub given_name: String,
    #[serde(default)]
    pub user: Option<User>,
    pub last_seen: ProtoTimestamp,
    pub expiry: ProtoTimestamp,
    pub created_at: ProtoTimestamp,
    #[serde(default)]
    pub register_method: Option<i32>,
    #[serde(default)]
    pub online: Option<bool>,
}

/// Machine key with the canonical `mkey:` prefix.
pub fn machine_key(value: &str) -> String {
    if value.starts_with("mkey:") {
        value.to_string()
    } else {
        format!("mkey:{value}")
    }
}

/// Strip the `nodekey:` prefix, returning the raw public key hex.
pub fn node_key_hex(value: &str) -> &str {
    value.strip_prefix("nodekey:").unwrap_or(value)
}

/// One-way fingerprint for evidence/telemetry (never the key itself).
pub fn fingerprint(value: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
