//! Repository set composition root (EP-004).
//!
//! A `RepositorySet` hands out the repository ports for one tenant inside
//! one unit of work, so callers cannot accidentally mix transactions or
//! cross tenant boundaries.
//!
//! The accessors return `&mut dyn` (RX-005 AUD-007): repository port
//! methods take `&mut self`, so an immutable `&dyn` handle would make the
//! composition root unusable. The concrete PostgreSQL set owns the active
//! transaction and hands out each repository bound to it.

use nexus_domain::TenantId;

use crate::error::DataError;
use crate::ports::{MemoryRepository, VectorRepository, WorldGraphRepository};

/// Composition root for tenant-scoped repositories.
pub trait RepositorySet {
    /// Memory repository bound to the active unit of work.
    fn memory(&mut self) -> Result<&mut dyn MemoryRepository, DataError>;

    /// World graph repository bound to the active unit of work.
    fn world_graph(&mut self) -> Result<&mut dyn WorldGraphRepository, DataError>;

    /// Vector repository bound to the active unit of work.
    fn vector(&mut self) -> Result<&mut dyn VectorRepository, DataError>;

    /// The tenant this set is scoped to.
    fn tenant(&self) -> TenantId;
}
