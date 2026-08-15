//! Agent artifact exchange (SPEC-010; ADR-024).
//!
//! Nexus owns artifacts. Artifacts are immutable by content hash with
//! full provenance lineage; a new version is a new artifact, never a
//! mutation of an existing one. The canonical persisted manifest is
//! nexus-fabric `ArtifactManifest`; this type is the task-bound
//! exchange record.

use crate::error::AgentsError;
use nexus_domain::{ArtifactId, TaskId};
use serde::{Deserialize, Serialize};

/// An artifact produced by an agent task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentArtifact {
    pub artifact_id: ArtifactId,
    pub task_id: TaskId,
    pub name: String,
    /// Immutable content hash (sha256 hex). Identity of the artifact.
    pub content_hash: String,
    /// Lineage: ids of artifacts this one derives from.
    pub provenance: Vec<ArtifactId>,
    pub content_type: String,
    pub created_at_epoch_ms: u64,
}

impl AgentArtifact {
    /// Canonical invariants. Fails closed on empty name or hash.
    pub fn validate(&self) -> Result<(), AgentsError> {
        if self.name.is_empty() {
            return Err(AgentsError::validation(
                "artifact name must not be empty",
                Some("agent-artifact".into()),
            ));
        }
        if self.content_hash.is_empty() || self.content_hash.len() != 64 {
            return Err(AgentsError::validation(
                "artifact content_hash must be a 64-char sha256 hex",
                Some("agent-artifact".into()),
            ));
        }
        Ok(())
    }
}
