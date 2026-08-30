//! PostgreSQL idempotent consumer inbox (SPEC-023 behavior 4, RX-005
//! AUD-008).
//!
//! Concrete `InboxRepository` on real PostgreSQL. The inbox is the
//! deduplication ledger: `record_delivery` inserts the (consumer,
//! event_id) pair exactly once and reports whether this was the first
//! sighting, so NATS replay cannot create duplicate logical effects.
//! `fetch_new` returns deliveries awaiting processing (`NEW` and
//! `FAILED` - bounded retry) for one consumer; `DONE` and in-flight
//! `PROCESSING` rows are excluded.

use nexus_events::{EventError, EventErrorCode, InboxRecord, InboxRepository, InboxStatus};

use crate::unit_of_work::PgUnitOfWork;

/// PostgreSQL implementation of the idempotent inbox port.
pub struct PgInboxRepository<'a> {
    uow: &'a PgUnitOfWork,
}

impl<'a> PgInboxRepository<'a> {
    /// Bind the repository to a live unit of work.
    pub fn new(uow: &'a PgUnitOfWork) -> Self {
        Self { uow }
    }

    fn parse_status(text: &str) -> Result<InboxStatus, EventError> {
        match text {
            "NEW" => Ok(InboxStatus::New),
            "PROCESSING" => Ok(InboxStatus::Processing),
            "DONE" => Ok(InboxStatus::Done),
            "FAILED" => Ok(InboxStatus::Failed),
            other => Err(EventError::new(
                EventErrorCode::Invariant,
                format!("corrupt inbox status: {other}"),
            )),
        }
    }

    fn map_row(row: &postgres::Row) -> Result<InboxRecord, EventError> {
        Ok(InboxRecord {
            consumer: row.get("consumer"),
            event_id: row.get("event_id"),
            status: Self::parse_status(&row.get::<_, String>("status"))?,
            attempts: row.get::<_, i32>("attempts") as u32,
        })
    }
}

impl InboxRepository for PgInboxRepository<'_> {
    fn record_delivery(&self, consumer: &str, event_id: &str) -> Result<bool, EventError> {
        self.uow.with_tx(|tx| {
            let n = tx
                .execute(
                    "INSERT INTO inbox (consumer, event_id, status, attempts)
                     VALUES ($1, $2, 'NEW', 0)
                     ON CONFLICT (consumer, event_id) DO NOTHING",
                    &[&consumer, &event_id],
                )
                .map_err(|e| {
                    EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("postgres inbox record_delivery: {e}"),
                    )
                })?;
            Ok(n == 1)
        })
    }

    fn mark_done(&self, consumer: &str, event_id: &str) -> Result<(), EventError> {
        self.uow.with_tx(|tx| {
            let n = tx
                .execute(
                    "UPDATE inbox SET status = 'DONE', updated_at = now()
                     WHERE consumer = $1 AND event_id = $2",
                    &[&consumer, &event_id],
                )
                .map_err(|e| {
                    EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("postgres inbox mark_done: {e}"),
                    )
                })?;
            if n == 0 {
                return Err(EventError::new(
                    EventErrorCode::Conflict,
                    "inbox delivery not found",
                ));
            }
            Ok(())
        })
    }

    fn mark_failed(&self, consumer: &str, event_id: &str, reason: &str) -> Result<(), EventError> {
        self.uow.with_tx(|tx| {
            let n = tx
                .execute(
                    "UPDATE inbox SET status = 'FAILED', attempts = attempts + 1,
                            last_error = $3, updated_at = now()
                     WHERE consumer = $1 AND event_id = $2",
                    &[&consumer, &event_id, &reason],
                )
                .map_err(|e| {
                    EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("postgres inbox mark_failed: {e}"),
                    )
                })?;
            if n == 0 {
                return Err(EventError::new(
                    EventErrorCode::Conflict,
                    "inbox delivery not found",
                ));
            }
            Ok(())
        })
    }

    fn fetch_new(&self, consumer: &str, limit: u32) -> Result<Vec<InboxRecord>, EventError> {
        self.uow.with_tx(|tx| {
            let rows = tx
                .query(
                    "SELECT consumer, event_id, status, attempts
                     FROM inbox
                     WHERE consumer = $1 AND status IN ('NEW', 'FAILED')
                     ORDER BY created_at
                     LIMIT $2",
                    &[&consumer, &(limit as i64)],
                )
                .map_err(|e| {
                    EventError::new(
                        EventErrorCode::ExternalProvider,
                        format!("postgres inbox fetch_new: {e}"),
                    )
                })?;
            let mut out = Vec::with_capacity(rows.len());
            for row in rows {
                out.push(Self::map_row(&row)?);
            }
            Ok(out)
        })
    }
}
