//! PostgreSQL world graph repository (EP-004 M3, RX-005 AUD-007).
//!
//! Implements the adjacency-table fallback doctrine (INV-015): recursive
//! queries over `world_graph_edges`, no dedicated graph database. Every
//! read and write is tenant-scoped at the SQL level AND behind the RLS
//! policy from migration 003.

use nexus_data::{
    DataError, DataErrorCode, PostgresWorldGraphRepository, WorldGraphRepository,
};
use nexus_domain::{NexusId, TenantId};
use postgres::Client;
use uuid::Uuid;

use crate::unit_of_work::PgUnitOfWork;

/// PostgreSQL implementation of the world graph repository port.
pub struct PgWorldGraphRepository<'a> {
    uow: &'a PgUnitOfWork,
}

impl<'a> PgWorldGraphRepository<'a> {
    /// Bind the repository to a live unit of work.
    pub fn new(uow: &'a PgUnitOfWork) -> Self {
        Self { uow }
    }

    fn set_tenant(tx: &mut Client, tenant: &TenantId) -> Result<(), DataError> {
        tx.execute(
            "SELECT set_config('app.tenant_id', $1, true)",
            &[&tenant.as_str()],
        )
        .map_err(|e| {
            DataError::new(
                DataErrorCode::ExternalProvider,
                format!("postgres set tenant: {e}"),
            )
        })?;
        Ok(())
    }
}

impl WorldGraphRepository for PgWorldGraphRepository<'_> {
    fn neighbors(&mut self, tenant: TenantId, node: NexusId) -> Result<Vec<NexusId>, DataError> {
        let node_uuid = Uuid::parse_str(node.as_str())
            .map_err(|e| DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}")))?;
        let tenant_uuid = Uuid::parse_str(tenant.as_str())
            .map_err(|e| DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}")))?;
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            let rows = tx
                .query(
                    "SELECT DISTINCT to_node FROM world_graph_edges
                     WHERE tenant_id = $1 AND from_node = $2 ORDER BY to_node",
                    &[&tenant_uuid, &node_uuid],
                )
                .map_err(|e| {
                    DataError::new(
                        DataErrorCode::ExternalProvider,
                        format!("postgres neighbors: {e}"),
                    )
                })?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let id: Uuid = row.get(0);
                out.push(
                    NexusId::new(id.to_string())
                        .map_err(|e| DataError::new(DataErrorCode::Invariant, e.to_string()))?,
                );
            }
            Ok(out)
        })
    }

    fn follow(&mut self, tenant: TenantId, from: NexusId, to: NexusId) -> Result<bool, DataError> {
        let from_uuid = Uuid::parse_str(from.as_str())
            .map_err(|e| DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}")))?;
        let to_uuid = Uuid::parse_str(to.as_str())
            .map_err(|e| DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}")))?;
        let tenant_uuid = Uuid::parse_str(tenant.as_str())
            .map_err(|e| DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}")))?;
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            let row = tx
                .query_one(
                    "SELECT EXISTS(
                        SELECT 1 FROM world_graph_edges
                        WHERE tenant_id = $1 AND from_node = $2 AND to_node = $3
                     )",
                    &[&tenant_uuid, &from_uuid, &to_uuid],
                )
                .map_err(|e| {
                    DataError::new(
                        DataErrorCode::ExternalProvider,
                        format!("postgres follow: {e}"),
                    )
                })?;
            let exists: bool = row.get(0);
            Ok(exists)
        })
    }

    fn walk(
        &mut self,
        tenant: TenantId,
        start: NexusId,
        depth: u8,
    ) -> Result<Vec<NexusId>, DataError> {
        if depth == 0 {
            return Ok(vec![start]);
        }
        let start_uuid = Uuid::parse_str(start.as_str())
            .map_err(|e| DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}")))?;
        let tenant_uuid = Uuid::parse_str(tenant.as_str())
            .map_err(|e| DataError::new(DataErrorCode::Invariant, format!("corrupt id: {e}")))?;
        self.uow.with_tx(|tx| {
            Self::set_tenant(tx, &tenant)?;
            let rows = tx
                .query(
                    "WITH RECURSIVE walk(node, hops) AS (
                        SELECT $2::uuid, 0
                        UNION ALL
                        SELECT e.to_node, w.hops + 1
                        FROM world_graph_edges e
                        JOIN walk w ON e.from_node = w.node
                        WHERE e.tenant_id = $1 AND w.hops < $3
                     )
                     SELECT DISTINCT node FROM walk ORDER BY node",
                    &[&tenant_uuid, &start_uuid, &i32::from(depth)],
                )
                .map_err(|e| {
                    DataError::new(
                        DataErrorCode::ExternalProvider,
                        format!("postgres walk: {e}"),
                    )
                })?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                let id: Uuid = row.get(0);
                out.push(
                    NexusId::new(id.to_string())
                        .map_err(|e| DataError::new(DataErrorCode::Invariant, e.to_string()))?,
                );
            }
            Ok(out)
        })
    }
}

impl PostgresWorldGraphRepository for PgWorldGraphRepository<'_> {}
