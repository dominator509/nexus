//! PostgreSQL unit of work (EP-004 M3, RX-005 AUD-007).
//!
//! Owns the `postgres::Client` and manages the transaction lifecycle
//! explicitly (BEGIN/COMMIT/ROLLBACK). Fail-closed: dropping without
//! commit rolls back. Owning the client removes the self-referential
//! lifetime a `Transaction<'a>` wrapper would impose and lets repository
//! adapters share the live connection through `with_tx`.

use std::cell::RefCell;

use nexus_data::{DataError, DataErrorCode, UnitOfWork};
use postgres::Client;

/// A unit of work that owns its PostgreSQL connection.
///
/// `begin` starts a real transaction on the wire. `commit`/`rollback`
/// finish it. Repository adapters run one operation at a time through
/// `with_tx`, which borrows the client mutably and fails closed once the
/// unit of work is finished.
pub struct PgUnitOfWork {
    client: RefCell<Option<Client>>,
}

impl PgUnitOfWork {
    /// Open a connection and begin a transaction.
    pub fn begin(client: Client) -> Result<Self, DataError> {
        let mut client = client;
        client
            .simple_query("BEGIN")
            .map_err(|e| {
                DataError::new(
                    DataErrorCode::ExternalProvider,
                    format!("postgres begin: {e}"),
                )
            })?;
        Ok(Self {
            client: RefCell::new(Some(client)),
        })
    }

    /// Run one operation against the live connection (inside the
    /// transaction). Fail-closed: after `commit`/`rollback` the slot is
    /// empty and any further use is a `Conflict` error. The closure's
    /// error type is free (`E: From<DataError>`) so repository adapters
    /// can surface their own typed errors (e.g. `EventError`).
    pub(crate) fn with_tx<T, E>(
        &self,
        f: impl FnOnce(&mut Client) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<DataError>,
    {
        let mut guard = self.client.borrow_mut();
        let client = guard
            .as_mut()
            .ok_or_else(|| DataError::new(DataErrorCode::Conflict, "unit of work not begun"))?;
        f(client)
    }

    /// Whether a live transaction is currently held (fail-closed gate for
    /// the repository set accessors).
    pub(crate) fn transaction_present(&self) -> bool {
        self.client.borrow().is_some()
    }
}

impl UnitOfWork for PgUnitOfWork {
    fn begin(&mut self) -> Result<(), DataError> {
        // The transaction is begun in `Self::begin`; this satisfies the
        // port contract that a unit of work can be (re)started. Re-begin
        // on an active transaction is a no-op.
        Ok(())
    }

    fn commit(&mut self) -> Result<(), DataError> {
        let mut client = self
            .client
            .borrow_mut()
            .take()
            .ok_or_else(|| DataError::new(DataErrorCode::Conflict, "unit of work not begun"))?;
        client.simple_query("COMMIT").map_err(|e| {
            DataError::new(
                DataErrorCode::ExternalProvider,
                format!("postgres commit: {e}"),
            )
        })?;
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), DataError> {
        if let Some(mut client) = self.client.borrow_mut().take() {
            client.simple_query("ROLLBACK").map_err(|e| {
                DataError::new(
                    DataErrorCode::ExternalProvider,
                    format!("postgres rollback: {e}"),
                )
            })?;
        }
        Ok(())
    }
}
