//! Nexus private mesh: Headscale-compatible adapter (EP-009 M3).
//!
//! `nexus-headscale` implements the nexus-trust `MeshController` port
//! against a REAL pinned headscale CLI (v0.23.0) talking gRPC to a
//! real Headscale server. Raw WireGuard and standard mTLS paths
//! coexist (EP-009 acceptance obligation 4): this adapter produces
//! WireGuard configs from real provider allocations; mTLS is proven by
//! the pki milestone.
//!
//! Dependency direction (SPEC-001): this crate may import only
//! `nexus-trust` (the contract crate) plus serde/serde_json. It must
//! never import provider runtimes or HTTP stacks; the headscale CLI is
//! the transport boundary.

#![forbid(unsafe_code)]

pub mod error;
pub mod mesh;
pub mod model;

pub use error::{HeadscaleError, HeadscaleErrorCode};
pub use mesh::HeadscaleMeshController;

#[cfg(test)]
mod lib_tests;
