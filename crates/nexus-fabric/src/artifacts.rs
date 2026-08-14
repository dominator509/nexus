//! Artifact exchange contract (SPEC-003 required behavior 6).
//!
//! Artifacts are immutable by hash; new versions create new manifests
//! and preserve lineage.

use crate::error::FabricError;
use serde::{Deserialize, Serialize};

/// Artifact identifier (content-hash bound).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactId(pub String);

/// Artifact lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArtifactState {
    Sealed,
    Superseded,
    Revoked,
}

impl ArtifactState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sealed => "SEALED",
            Self::Superseded => "SUPERSEDED",
            Self::Revoked => "REVOKED",
        }
    }
}

impl std::str::FromStr for ArtifactState {
    type Err = crate::vocabulary::FabricVocabularyError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "SEALED" => Ok(Self::Sealed),
            "SUPERSEDED" => Ok(Self::Superseded),
            "REVOKED" => Ok(Self::Revoked),
            other => Err(crate::vocabulary::FabricVocabularyError::unknown(
                "ArtifactState",
                other,
            )),
        }
    }
}

/// Artifact manifest (SPEC-003 canonical term; immutable by hash).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactManifest {
    pub artifact_id: ArtifactId,
    pub sha256: String,
    pub size_bytes: u64,
    pub content_type: String,
    pub state: ArtifactState,
    /// Lineage: ids of the artifacts this one derives from.
    pub parents: Vec<ArtifactId>,
}

/// A fetched artifact (metadata + content reference).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactHandle {
    pub manifest: ArtifactManifest,
    /// Content reference; raw bytes travel out of band.
    pub content_ref: String,
}

/// Provider-neutral artifact exchange port.
pub trait ArtifactExchange {
    /// Publish content by sha256; the manifest is sealed on success.
    fn publish(
        &mut self,
        sha256: &str,
        size_bytes: u64,
        content_type: &str,
        parents: &[ArtifactId],
    ) -> Result<ArtifactManifest, FabricError>;
    /// Fetch a manifest by id.
    fn fetch(&self, artifact_id: &ArtifactId) -> Result<ArtifactHandle, FabricError>;
    /// Lineage of an artifact (ancestors).
    fn lineage(&self, artifact_id: &ArtifactId) -> Result<Vec<ArtifactId>, FabricError>;
    /// Revoke an artifact (supersedes it; content is never deleted).
    fn revoke(&mut self, artifact_id: &ArtifactId) -> Result<(), FabricError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep012_unit_artifact_state_round_trip() {
        for (wire, expected) in [
            ("SEALED", ArtifactState::Sealed),
            ("SUPERSEDED", ArtifactState::Superseded),
            ("REVOKED", ArtifactState::Revoked),
        ] {
            assert_eq!(wire.parse::<ArtifactState>().unwrap(), expected);
            assert_eq!(expected.as_str(), wire);
        }
        assert!("DRAFT".parse::<ArtifactState>().is_err());
    }

    #[test]
    fn ep012_unit_artifact_manifest_round_trip() {
        let manifest = ArtifactManifest {
            artifact_id: ArtifactId("art-1".into()),
            sha256: "abc123".into(),
            size_bytes: 42,
            content_type: "application/json".into(),
            state: ArtifactState::Sealed,
            parents: vec![ArtifactId("art-0".into())],
        };
        let json = serde_json::to_value(&manifest).unwrap();
        let back: ArtifactManifest = serde_json::from_value(json).unwrap();
        assert_eq!(back.sha256, "abc123");
        assert_eq!(back.parents.len(), 1);
    }
}
