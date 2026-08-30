//! PostgreSQL transactional outbox (SPEC-023 behavior 1, RX-005
//! AUD-008).
//!
//! Concrete `OutboxRepository` on real PostgreSQL. The repository is
//! bound to the same `PgUnitOfWork` as the domain repositories, so
//! `append` shares the transaction with the state change: a domain write
//! and its outbox insert commit or roll back together (the port takes no
//! transaction argument because atomicity is expressed by sharing one
//! unit of work instance).
//!
//! Publisher semantics: `fetch_pending` returns rows awaiting
//! publication (`PENDING` and `FAILED`, oldest first) so failed rows are
//! retried with bounded attempts; `PUBLISHING` rows are in flight by
//! another worker and are excluded. `mark_published`/`mark_failed` are
//! idempotent for the target row and fail closed (Conflict) when the row
//! does not exist.

use nexus_events::{
    EventEnvelope, EventError, EventErrorCode, OutboxRecord, OutboxRepository, OutboxStatus,
};
use uuid::Uuid;

use crate::unit_of_work::PgUnitOfWork;

/// PostgreSQL implementation of the transactional outbox port.
pub struct PgOutboxRepository<'a> {
    uow: &'a PgUnitOfWork,
}

impl<'a> PgOutboxRepository<'a> {
    /// Bind the repository to a live unit of work. `append` shares that
    /// transaction with the domain repositories (atomicity).
    pub fn new(uow: &'a PgUnitOfWork) -> Self {
        Self { uow }
    }

    fn parse_status(text: &str) -> Result<OutboxStatus, EventError> {
        match text {
            "PENDING" => Ok(OutboxStatus::Pending),
            "PUBLISHING" => Ok(OutboxStatus::Publishing),
            "PUBLISHED" => Ok(OutboxStatus::Published),
            "FAILED" => Ok(OutboxStatus::Failed),
            other => Err(EventError::new(
                EventErrorCode::Invariant,
                format!("corrupt outbox status: {other}"),
            )),
        }
    }

    fn map_row(row: &postgres::Row) -> Result<OutboxRecord, EventError> {
        let envelope: serde_json::Value = row.get("envelope");
        let envelope: EventEnvelope = serde_json::from_value(envelope).map_err(|e| {
            EventError::new(
                EventErrorCode::Invariant,
                format!("corrupt outbox envelope: {e}"),
            )
        })?;
        let status = Self::parse_status(&row.get::<_, String>("status"))?;
        Ok(OutboxRecord {
            outbox_id: row.get("outbox_id"),
            envelope,
            status,
            attempts: row.get::<_, i32>("attempts") as u32,
            last_error: row.get("last_error"),
        })
    }
}

impl OutboxRepository for PgOutboxRepository<'_> {
    fn append(&self, envelope: &EventEnvelope) -> Result<OutboxRecord, EventError> {
        let outbox_id = Uuid::new_v4().to_string();
        let json = serde_json::to_value(envelope).map_err(|e| {
            EventError::new(
                EventErrorCode::Validation,
                format!("outbox envelope serialize: {e}"),
            )
        })?;
        self.uow.with_tx(|tx| {
            tx.execute(
                "INSERT INTO outbox (outbox_id, envelope, status, attempts)
                 VALUES ($1, $2::jsonb, 'PENDING', 0)",
                &[&outbox_id, &json],
            )
            .map_err(|e| {
                EventError::new(
                    EventErrorCode::ExternalProvider,
                    format!("postgres outbox append: {e}"),
                )
            })?;
            Ok(OutboxRecord {
                outbox_id,
                envelope: envelope.clone(),
                status: OutboxStatus::Pending,
                attempts: 0,
                last_error: None,
            })
        })
    }

    fn fetch_pending(&self, limit: u32) -> Result<Vec<OutboxRecord>, EventError> {
        self.uow.with_tx(|tx| {
            let rows = tx
                .query(
                    "SELECT outbox_id, envelope, status, attempts, last_error
                     FROM outbox
                     WHERE status IN ('PENDING', 'FAILED')
                     ORDER BY created_at
                     LIMIT $1",
                    &[&(limit as i64)],
                )
                .map_err(|e| {
                    EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("postgres outbox fetch_pending: {e}"),
                    )
                })?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                out.push(Self::map_row(&row)?);
            }
            Ok(out)
        })
    }

    fn mark_publishing(&self, outbox_id: &str) -> Result<(), EventError> {
        self.uow.with_tx(|tx| {
            let n = tx
                .execute(
                    "UPDATE outbox SET status = 'PUBLISHING', updated_at = now()
                     WHERE outbox_id = $1",
                    &[&outbox_id],
                )
                .map_err(|e| {
                    EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("postgres outbox mark_publishing: {e}"),
                    )
                })?;
            if n == 0 {
                return Err(EventError::new(
                    EventErrorCode::Conflict,
                    "outbox record not found",
                ));
            }
            Ok(())
        })
    }

    fn mark_published(&self, outbox_id: &str) -> Result<(), EventError> {
        self.uow.with_tx(|tx| {
            let n = tx
                .execute(
                    "UPDATE outbox SET status = 'PUBLISHED', updated_at = now()
                     WHERE outbox_id = $1",
                    &[&outbox_id],
                )
                .map_err(|e| {
                    EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("postgres outbox mark_published: {e}"),
                    )
                })?;
            if n == 0 {
                return Err(EventError::new(
                    EventErrorCode::Conflict,
                    "outbox record not found",
                ));
            }
            Ok(())
        })
    }

    fn mark_failed(&self, outbox_id: &str, reason: &str) -> Result<(), EventError> {
        self.uow.with_tx(|tx| {
            let n = tx
                .execute(
                    "UPDATE outbox SET status = 'FAILED', attempts = attempts + 1,
                            last_error = $2, updated_at = now()
                     WHERE outbox_id = $1",
                    &[&outbox_id, &reason],
                )
                .map_err(|e| {
                    EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("postgres outbox mark_failed: {e}"),
                    )
                })?;
            if n == 0 {
                return Err(EventError::new(
                    EventErrorCode::Conflict,
                    "outbox record not found",
                ));
            }
            Ok(())
        })
    }
}
