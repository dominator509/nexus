//! Provider-neutral repository ports (SPEC-002, EP-004).
//!
//! These traits are the replacement boundary: PostgreSQL is the initial
//! implementation (INV-004), pgvector is one retrieval index rather than
//! the source of truth (SPEC-002 behavior 2), and a future graph engine is
//! a projection behind `WorldGraphRepository` (INV-015). Domain and
//! application code depend on these ports, never on a provider crate.

use nexus_domain::{NexusId, TenantId};

use crate::error::DataError;
use crate::memory::{MemoryCandidate, MemoryProposal, MemoryQuery, MemoryRecord};

/// Memory repository port (SPEC-002 behavior 5-6).
///
/// Writes are proposals evaluated by policy before they become canonical
/// facts. Reads always enforce tenant isolation and authorization filters.
pub trait MemoryRepository {
    /// Stage a memory proposal for policy evaluation.
    fn propose(&mut self, tenant: TenantId, proposal: MemoryProposal) -> Result<(), DataError>;

    /// Promote a proposal to an active canonical fact after policy approval.
    fn activate(&mut self, tenant: TenantId, memory_id: NexusId) -> Result<(), DataError>;

    /// Retrieve a single record by id within the tenant boundary.
    fn get(&mut self, tenant: TenantId, memory_id: NexusId) -> Result<MemoryRecord, DataError>;

    /// Run a hybrid retrieval query.
    fn query(
        &mut self,
        tenant: TenantId,
        query: &MemoryQuery,
    ) -> Result<Vec<MemoryCandidate>, DataError>;

    /// Mark a record deleted (retention or explicit deletion).
    fn delete(&mut self, tenant: TenantId, memory_id: NexusId) -> Result<(), DataError>;

    /// Supersede `old_id` with a new record (SPEC-002 supersession).
    fn supersede(
        &mut self,
        tenant: TenantId,
        old_id: NexusId,
        new_record: MemoryRecord,
    ) -> Result<(), DataError>;
}

/// World graph repository port (SPEC-002 behavior 7, INV-015).
///
/// The graph is a projection over canonical state. A future graph engine
/// implements this same contract; callers never import a graph vendor SDK.
pub trait WorldGraphRepository {
    /// Resolve typed entity neighbors for a node.
    fn neighbors(&mut self, tenant: TenantId, node: NexusId) -> Result<Vec<NexusId>, DataError>;

    /// Follow a directed edge between two typed nodes.
    fn follow(&mut self, tenant: TenantId, from: NexusId, to: NexusId) -> Result<bool, DataError>;

    /// Walk up to `depth` hops from a node.
    fn walk(
        &mut self,
        tenant: TenantId,
        start: NexusId,
        depth: u8,
    ) -> Result<Vec<NexusId>, DataError>;
}

/// PostgreSQL world graph repository (SPEC-002 behavior 7).
///
/// Marker/contract for the initial PostgreSQL implementation: recursive
/// queries and adjacency tables only (EP-004 fallback doctrine). No
/// dedicated graph database.
pub trait PostgresWorldGraphRepository: WorldGraphRepository {}

/// Vector retrieval port (SPEC-002 behavior 2).
///
/// pgvector stores initial embeddings; the embedding model and dimensions
/// are versioned per row. This index is a retrieval aid, never the source
/// of truth.
pub trait VectorRepository {
    /// Store or update a vector for a record.
    fn upsert_vector(
        &mut self,
        tenant: TenantId,
        memory_id: NexusId,
        embedding: Vec<f32>,
    ) -> Result<(), DataError>;

    /// Nearest-neighbor candidates by embedding.
    fn nearest(
        &mut self,
        tenant: TenantId,
        embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<MemoryCandidate>, DataError>;

    /// Remove a vector (deletion workflow).
    fn remove(&mut self, tenant: TenantId, memory_id: NexusId) -> Result<(), DataError>;
}
