//! Unit of work transaction boundary (SPEC-006, EP-004).
//!
//! A unit of work groups repository mutations into one transaction with
//! explicit commit or rollback. The port is provider-neutral; PostgreSQL
//! implements it with a real transaction (EP-004 M3).

use crate::error::DataError;

/// Unit of work port.
///
/// Implementations must be fail-closed: dropping without commit rolls back.
pub trait UnitOfWork {
    /// Begin the transaction.
    fn begin(&mut self) -> Result<(), DataError>;

    /// Commit all mutations in this unit of work.
    fn commit(&mut self) -> Result<(), DataError>;

    /// Roll back all mutations in this unit of work.
    fn rollback(&mut self) -> Result<(), DataError>;
}
