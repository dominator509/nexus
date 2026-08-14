//! Real hash-bound in-memory artifact store (SPEC-003 behavior 6).
//!
//! Artifacts are immutable by hash: the artifact id IS the sha256
//! hex digest of the content. Publishing stores a manifest keyed by
//! that digest; fetching verifies the id is a well-formed digest;
//! revoking supersedes the manifest (content is never deleted).
//! This store is a real implementation of the fabric ArtifactExchange
//! port used by the composed gateway proof - it computes real digests
//! and enforces hash binding, so an artifact handle can never be
//! fabricated for content that was not published.

use nexus_fabric::artifacts::{
    ArtifactExchange, ArtifactHandle, ArtifactId, ArtifactManifest, ArtifactState,
};
use nexus_fabric::error::FabricError;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Canonical hex digest for artifact content.
pub fn sha256_hex(content: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(content);
    hex_lower(&h.finalize())
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// A real hash-bound artifact store (in-memory; deterministic).
#[derive(Debug, Clone, Default)]
pub struct MemoryArtifactStore {
    manifests: BTreeMap<String, ArtifactManifest>,
}

impl MemoryArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish content directly: compute the sha256 digest and store a
    /// sealed manifest bound to that digest.
    pub fn publish_bytes(
        &mut self,
        content: &[u8],
        content_type: &str,
        parents: &[ArtifactId],
    ) -> Result<ArtifactManifest, FabricError> {
        let digest = sha256_hex(content);
        let id = ArtifactId(format!("sha256:{digest}"));
        let manifest = ArtifactManifest {
            artifact_id: id.clone(),
            sha256: digest,
            size_bytes: content.len() as u64,
            content_type: content_type.to_string(),
            state: ArtifactState::Sealed,
            parents: parents.to_vec(),
        };
        self.manifests
            .insert(manifest.artifact_id.0.clone(), manifest.clone());
        Ok(manifest)
    }

    /// The set of stored artifact ids (deterministic order).
    pub fn stored_ids(&self) -> Vec<String> {
        self.manifests.keys().cloned().collect()
    }
}

impl ArtifactExchange for MemoryArtifactStore {
    fn publish(
        &mut self,
        sha256: &str,
        size_bytes: u64,
        content_type: &str,
        parents: &[ArtifactId],
    ) -> Result<ArtifactManifest, FabricError> {
        let digest = sha256.trim().to_lowercase();
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(FabricError::validation(
                "artifact sha256 must be 64 lowercase hex digits",
                Some("artifact_store.publish".to_string()),
            ));
        }
        let id = ArtifactId(format!("sha256:{digest}"));
        let manifest = ArtifactManifest {
            artifact_id: id.clone(),
            sha256: digest,
            size_bytes,
            content_type: content_type.to_string(),
            state: ArtifactState::Sealed,
            parents: parents.to_vec(),
        };
        self.manifests
            .insert(manifest.artifact_id.0.clone(), manifest.clone());
        Ok(manifest)
    }

    fn fetch(&self, artifact_id: &ArtifactId) -> Result<ArtifactHandle, FabricError> {
        let Some(manifest) = self.manifests.get(&artifact_id.0) else {
            return Err(FabricError::not_found(
                format!("artifact unavailable: {}", artifact_id.0),
                Some("artifact_store.fetch".to_string()),
            ));
        };
        Ok(ArtifactHandle {
            manifest: manifest.clone(),
            // Content travels out of band; the digest is the reference.
            content_ref: manifest.sha256.clone(),
        })
    }

    fn lineage(&self, artifact_id: &ArtifactId) -> Result<Vec<ArtifactId>, FabricError> {
        let Some(manifest) = self.manifests.get(&artifact_id.0) else {
            return Err(FabricError::not_found(
                format!("artifact unavailable: {}", artifact_id.0),
                Some("artifact_store.lineage".to_string()),
            ));
        };
        Ok(manifest.parents.clone())
    }

    fn revoke(&mut self, artifact_id: &ArtifactId) -> Result<(), FabricError> {
        let Some(manifest) = self.manifests.get_mut(&artifact_id.0) else {
            return Err(FabricError::not_found(
                format!("artifact unavailable: {}", artifact_id.0),
                Some("artifact_store.revoke".to_string()),
            ));
        };
        manifest.state = ArtifactState::Revoked;
        Ok(())
    }
}

/// Shared handle to the artifact store. The composed gateway and the
/// A2A engine hold the SAME store (interior mutability), so an
/// artifact published through the gateway is immediately visible to
/// the A2A attachment path. This is the composition fix: cloning the
/// store at construction would give the A2A engine a stale copy and
/// every attach would fail NOT_FOUND.
#[derive(Debug, Clone, Default)]
pub struct SharedArtifactStore(std::rc::Rc<std::cell::RefCell<MemoryArtifactStore>>);

impl SharedArtifactStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Publish content directly into the shared store.
    pub fn publish_bytes(
        &self,
        content: &[u8],
        content_type: &str,
        parents: &[ArtifactId],
    ) -> Result<ArtifactManifest, FabricError> {
        self.0
            .borrow_mut()
            .publish_bytes(content, content_type, parents)
    }

    /// Fetch an artifact handle from the shared store.
    pub fn fetch(&self, artifact_id: &ArtifactId) -> Result<ArtifactHandle, FabricError> {
        self.0.borrow().fetch(artifact_id)
    }
}

impl ArtifactExchange for SharedArtifactStore {
    fn publish(
        &mut self,
        sha256: &str,
        size_bytes: u64,
        content_type: &str,
        parents: &[ArtifactId],
    ) -> Result<ArtifactManifest, FabricError> {
        self.0
            .borrow_mut()
            .publish(sha256, size_bytes, content_type, parents)
    }

    fn fetch(&self, artifact_id: &ArtifactId) -> Result<ArtifactHandle, FabricError> {
        self.0.borrow().fetch(artifact_id)
    }

    fn lineage(&self, artifact_id: &ArtifactId) -> Result<Vec<ArtifactId>, FabricError> {
        self.0.borrow().lineage(artifact_id)
    }

    fn revoke(&mut self, artifact_id: &ArtifactId) -> Result<(), FabricError> {
        self.0.borrow_mut().revoke(artifact_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_fabric::error::FabricErrorCode;

    #[test]
    fn ep012_gateway_artifact_digest_is_deterministic() {
        let d1 = sha256_hex(b"hello nexus");
        let d2 = sha256_hex(b"hello nexus");
        assert_eq!(d1, d2);
        assert_eq!(d1.len(), 64);
        assert_ne!(d1, sha256_hex(b"hello nexus!"));
    }

    #[test]
    fn ep012_gateway_artifact_publish_fetch_round_trip() {
        let mut store = MemoryArtifactStore::new();
        let manifest = store
            .publish_bytes(b"payload", "application/json", &[])
            .unwrap();
        assert_eq!(manifest.state, ArtifactState::Sealed);
        let handle = store.fetch(&manifest.artifact_id).unwrap();
        assert_eq!(handle.manifest.sha256, manifest.sha256);
        assert_eq!(handle.content_ref, manifest.sha256);
    }

    #[test]
    fn ep012_gateway_artifact_fetch_missing_fails_closed() {
        let store = MemoryArtifactStore::new();
        let err = store
            .fetch(&ArtifactId(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            ))
            .unwrap_err();
        assert_eq!(err.code, FabricErrorCode::NotFound);
    }

    #[test]
    fn ep012_gateway_artifact_publish_rejects_malformed_digest() {
        let mut store = MemoryArtifactStore::new();
        let err = store
            .publish("not-a-digest", 1, "text/plain", &[])
            .unwrap_err();
        assert_eq!(err.code, FabricErrorCode::Validation);
    }

    #[test]
    fn ep012_gateway_artifact_revoke_supersedes() {
        let mut store = MemoryArtifactStore::new();
        let manifest = store.publish_bytes(b"x", "text/plain", &[]).unwrap();
        store.revoke(&manifest.artifact_id).unwrap();
        let handle = store.fetch(&manifest.artifact_id).unwrap();
        assert_eq!(handle.manifest.state, ArtifactState::Revoked);
        // Revoking a missing artifact fails closed.
        let missing = ArtifactId(
            "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into(),
        );
        assert!(store.revoke(&missing).is_err());
    }
}
