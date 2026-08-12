//! Repository set composition root (EP-004).
//!
//! A `RepositorySet` hands out the repository ports for one tenant inside
//! one unit of work, so callers cannot accidentally mix transactions or
//! cross tenant boundaries.

use nexus_domain::TenantId;

use crate::error::DataError;
use crate::ports::{MemoryRepository, VectorRepository, WorldGraphRepository};

/// Composition root for tenant-scoped repositories.
pub trait RepositorySet {
    /// Memory repository bound to the active unit of work.
    fn memory(&self) -> Result<&dyn MemoryRepository, DataError>;

    /// World graph repository bound to the active unit of work.
    fn world_graph(&self) -> Result<&dyn WorldGraphRepository, DataError>;

    /// Vector repository bound to the active unit of work.
    fn vector(&self) -> Result<&dyn VectorRepository, DataError>;

    /// The tenant this set is scoped to.
    fn tenant(&self) -> TenantId;
}
