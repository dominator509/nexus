//! PostgreSQL repository set (EP-004 M3, RX-005 AUD-007).
//!
//! Tenant-scoped composition root binding all three repositories to one
//! unit of work. The set owns the transaction slot; repositories borrow it
//! for the duration of one operation via `with_tx`.

use nexus_data::{
    DataError, DataErrorCode, MemoryRepository, RepositorySet, VectorRepository,
    WorldGraphRepository,
};
use nexus_domain::TenantId;

use crate::memory::PgMemoryRepository;
use crate::unit_of_work::PgUnitOfWork;
use crate::vector::PgVectorRepository;
use crate::world_graph::PgWorldGraphRepository;

/// PostgreSQL repository set for one tenant inside one unit of work.
pub struct PgRepositorySet<'a> {
    uow: &'a PgUnitOfWork,
    tenant: TenantId,
    memory: PgMemoryRepository<'a>,
    world_graph: PgWorldGraphRepository<'a>,
    vector: PgVectorRepository<'a>,
}

impl<'a> PgRepositorySet<'a> {
    /// Bind a repository set to a live unit of work and tenant.
    pub fn new(uow: &'a PgUnitOfWork, tenant: TenantId) -> Self {
        Self {
            uow,
            tenant,
            memory: PgMemoryRepository::new(uow),
            world_graph: PgWorldGraphRepository::new(uow),
            vector: PgVectorRepository::new(uow),
        }
    }
}

impl RepositorySet for PgRepositorySet<'_> {
    fn memory(&mut self) -> Result<&mut dyn MemoryRepository, DataError> {
        if self.uow_transaction_present() {
            Ok(&mut self.memory)
        } else {
            Err(DataError::new(
                DataErrorCode::Conflict,
                "unit of work not begun",
            ))
        }
    }

    fn world_graph(&mut self) -> Result<&mut dyn WorldGraphRepository, DataError> {
        if self.uow_transaction_present() {
            Ok(&mut self.world_graph)
        } else {
            Err(DataError::new(
                DataErrorCode::Conflict,
                "unit of work not begun",
            ))
        }
    }

    fn vector(&mut self) -> Result<&mut dyn VectorRepository, DataError> {
        if self.uow_transaction_present() {
            Ok(&mut self.vector)
        } else {
            Err(DataError::new(
                DataErrorCode::Conflict,
                "unit of work not begun",
            ))
        }
    }

    fn tenant(&self) -> TenantId {
        self.tenant.clone()
    }
}

impl PgRepositorySet<'_> {
    /// Whether the bound unit of work still holds a live transaction.
    fn uow_transaction_present(&self) -> bool {
        self.uow.transaction_present()
    }
}
