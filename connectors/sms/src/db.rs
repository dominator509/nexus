//! EP-032 SMS SQL service adapters (M3): real clients for the
//! DOCUMENTED Gammu SMSD SQL backend (docs.gammu.org "SQL Service" /
//! "SMSD Database Structure").
//!
//! The daemon reads outgoing messages from the `outbox` table
//! (documented `find_outbox_sms_id` / `create_outbox`), submits them
//! through the modem, inserts the result into `sentitems`
//! (documented `add_sent_info` with status
//! `SendingOK`/`SendingOKNoReport`/`SendingError`/`Error`), deletes
//! the outbox row (documented `delete_outbox`), and - when a real
//! SMS-STATUS-REPORT arrives - updates `sentitems` to
//! `DeliveryOK`/`DeliveryFailed`/`DeliveryPending`/`DeliveryUnknown`
//! with `DeliveryDateTime` (documented `save_inbox_sms_select` +
//! `save_inbox_sms_update_delivered`).
//!
//! Both adapters implement the same documented table semantics:
//! `submit` inserts the outbox row exactly as `create_outbox` does;
//! `status` reads the provider-observed state from `outbox` first
//! (still queued => Reserved) and from `sentitems` after submission.

use nexus_notifications::{NotificationError, NotificationErrorCode};

use crate::gateway::SmsProviderState;

/// One provider-observed status row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmsDbStatusRow {
    pub state: SmsProviderState,
    pub delivery_date_time: Option<String>,
    pub status_error: Option<i32>,
}

/// The documented SQL service surface used by the connector.
pub trait SmsDb {
    /// Enqueue a message into `outbox` per the documented
    /// `create_outbox` shape. Returns the outbox row id.
    fn submit(
        &mut self,
        destination: &str,
        text: &str,
        creator_id: &str,
        delivery_report: bool,
    ) -> Result<i64, NotificationError>;

    /// Observe the provider state for a message id: `outbox` row
    /// (queued) or `sentitems` row (post-submission lifecycle).
    /// Returns None when neither table has the message.
    fn status(&mut self, id: &str) -> Result<Option<SmsDbStatusRow>, NotificationError>;

    /// Stable provider name for telemetry.
    fn provider_name(&self) -> &'static str;
}

/// Map a documented SMSD `Status` column value to the provider state.
fn map_status(status: &str) -> Result<SmsProviderState, NotificationError> {
    SmsProviderState::parse_documented(status).ok_or_else(|| {
        NotificationError::new(
            NotificationErrorCode::External,
            format!("unexpected provider status value {status:?}"),
            None,
            None,
            None,
            Some("gammu-smsd sentitems.Status".to_string()),
        )
    })
}

/// SQLite backend adapter (DBI `sqlite3` driver). Used by the
/// controlled fixture against the REAL gammu-smsd daemon and by
/// production deployments that select the SQLite service backend.
#[derive(Debug)]
pub struct SqliteSmsDb {
    conn: rusqlite::Connection,
}

impl SqliteSmsDb {
    pub fn open(path: &str) -> Result<Self, NotificationError> {
        let conn = rusqlite::Connection::open(path).map_err(|e| {
            NotificationError::new(
                NotificationErrorCode::Unavailable,
                format!("cannot open gammu smsd sqlite backend: {e}"),
                None,
                None,
                None,
                Some("gammu-smsd sqlite".to_string()),
            )
        })?;
        Ok(Self { conn })
    }

    /// For in-memory tests only (TESTING.md test zone).
    pub fn open_in_memory() -> Result<Self, NotificationError> {
        let conn = rusqlite::Connection::open_in_memory().map_err(|e| {
            NotificationError::new(
                NotificationErrorCode::Internal,
                format!("cannot open in-memory sms db: {e}"),
                None,
                None,
                None,
                None,
            )
        })?;
        Ok(Self { conn })
    }
}

impl SmsDb for SqliteSmsDb {
    fn submit(
        &mut self,
        destination: &str,
        text: &str,
        creator_id: &str,
        delivery_report: bool,
    ) -> Result<i64, NotificationError> {
        // Documented `create_outbox` columns (SQLite dialect);
        // SenderID left blank so any SMSD instance can claim it.
        let report = if delivery_report { "yes" } else { "no" };
        self.conn
            .execute(
                "INSERT INTO outbox
                 (CreatorID, SenderID, DeliveryReport, MultiPart,
                  DestinationNumber, TextDecoded, Coding, Class)
                 VALUES (?1, '', ?2, 'false', ?3, ?4,
                         'Default_No_Compression', -1)",
                rusqlite::params![creator_id, report, destination, text],
            )
            .map_err(|e| {
                NotificationError::new(
                    NotificationErrorCode::External,
                    format!("gammu smsd outbox insert failed: {e}"),
                    None,
                    None,
                    None,
                    Some("gammu-smsd outbox".to_string()),
                )
            })?;
        Ok(self.conn.last_insert_rowid())
    }

    fn status(&mut self, id: &str) -> Result<Option<SmsDbStatusRow>, NotificationError> {
        // Still queued: the outbox row exists with Status 'Reserved'.
        let queued: Option<String> = self
            .conn
            .query_row(
                "SELECT Status FROM outbox WHERE ID = ?1",
                rusqlite::params![id],
                |row| row.get(0),
            )
            .ok();
        if let Some(status) = queued {
            return Ok(Some(SmsDbStatusRow {
                state: map_status(&status)?,
                delivery_date_time: None,
                status_error: None,
            }));
        }
        // Submitted: the daemon moved the message to sentitems.
        let row = self
            .conn
            .query_row(
                "SELECT Status, StatusError, DeliveryDateTime
                 FROM sentitems WHERE ID = ?1 AND SequencePosition = 1",
                rusqlite::params![id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i32>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .ok();
        match row {
            Some((status, status_error, delivery_date_time)) => Ok(Some(SmsDbStatusRow {
                state: map_status(&status)?,
                delivery_date_time,
                status_error: (status_error >= 0).then_some(status_error),
            })),
            None => Ok(None),
        }
    }

    fn provider_name(&self) -> &'static str {
        "gammu-smsd-sqlite"
    }
}

/// PostgreSQL backend adapter (documented `native_pgsql` driver).
/// Production path: the repository's locked PostgreSQL component hosts
/// the SMSD database, and the daemon is configured with
/// `Service = SQL, Driver = native_pgsql`.
pub struct PostgresSmsDb {
    client: postgres::Client,
}

// postgres::Client does not implement Debug; the manual impl is
// redaction-safe and never exposes the connection string or
// credentials (secrets stay out of telemetry).
impl std::fmt::Debug for PostgresSmsDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresSmsDb")
            .field("backend", &"gammu-smsd-postgres")
            .finish_non_exhaustive()
    }
}

impl PostgresSmsDb {
    pub fn connect(config: &str) -> Result<Self, NotificationError> {
        let client = postgres::Client::connect(config, postgres::NoTls).map_err(|e| {
            NotificationError::new(
                NotificationErrorCode::Unavailable,
                format!("cannot connect to gammu smsd postgres backend: {e}"),
                None,
                None,
                None,
                Some("gammu-smsd postgres".to_string()),
            )
        })?;
        Ok(Self { client })
    }
}

impl SmsDb for PostgresSmsDb {
    fn submit(
        &mut self,
        destination: &str,
        text: &str,
        creator_id: &str,
        delivery_report: bool,
    ) -> Result<i64, NotificationError> {
        // Documented `create_outbox` columns (PostgreSQL dialect).
        let report = if delivery_report { "yes" } else { "no" };
        let row = self
            .client
            .query_one(
                "INSERT INTO outbox
                 (CreatorID, SenderID, DeliveryReport, MultiPart,
                  DestinationNumber, TextDecoded, Coding, Class)
                 VALUES ($1, '', $2, 'false', $3, $4,
                         'Default_No_Compression', -1)
                 RETURNING ID",
                &[&creator_id, &report, &destination, &text],
            )
            .map_err(|e| {
                NotificationError::new(
                    NotificationErrorCode::External,
                    format!("gammu smsd outbox insert failed: {e}"),
                    None,
                    None,
                    None,
                    Some("gammu-smsd outbox".to_string()),
                )
            })?;
        Ok(row.get(0))
    }

    fn status(&mut self, id: &str) -> Result<Option<SmsDbStatusRow>, NotificationError> {
        let queued: Option<String> = self
            .client
            .query_opt("SELECT Status FROM outbox WHERE ID = $1", &[&id])
            .map_err(|e| {
                NotificationError::new(
                    NotificationErrorCode::External,
                    format!("gammu smsd outbox read failed: {e}"),
                    None,
                    None,
                    None,
                    Some("gammu-smsd outbox".to_string()),
                )
            })?
            .map(|row| row.get(0));
        if let Some(status) = queued {
            return Ok(Some(SmsDbStatusRow {
                state: map_status(&status)?,
                delivery_date_time: None,
                status_error: None,
            }));
        }
        let row = self
            .client
            .query_opt(
                "SELECT Status, StatusError, DeliveryDateTime
                 FROM sentitems WHERE ID = $1 AND SequencePosition = 1",
                &[&id],
            )
            .map_err(|e| {
                NotificationError::new(
                    NotificationErrorCode::External,
                    format!("gammu smsd sentitems read failed: {e}"),
                    None,
                    None,
                    None,
                    Some("gammu-smsd sentitems".to_string()),
                )
            })?;
        match row {
            Some(row) => {
                let status: String = row.get(0);
                let status_error: i32 = row.get(1);
                let delivery_date_time: Option<String> = row.get(2);
                Ok(Some(SmsDbStatusRow {
                    state: map_status(&status)?,
                    delivery_date_time,
                    status_error: (status_error >= 0).then_some(status_error),
                }))
            }
            None => Ok(None),
        }
    }

    fn provider_name(&self) -> &'static str {
        "gammu-smsd-postgres"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal documented schema (Gammu SMSD Database Structure;
    /// version 17 matches the Ubuntu noble package schema). Table
    /// definitions are copied from the package's sqlite.sql, which is
    /// the authoritative runtime schema for the pinned runtime.
    const SCHEMA: &str = r#"
        CREATE TABLE gammu (Version INTEGER NOT NULL DEFAULT 0 PRIMARY KEY);
        INSERT INTO gammu (Version) VALUES (17);
        CREATE TABLE outbox (
            UpdatedInDB NUMERIC NOT NULL DEFAULT (datetime('now','localtime')),
            InsertIntoDB NUMERIC NOT NULL DEFAULT (datetime('now','localtime')),
            SendingDateTime NUMERIC NOT NULL DEFAULT (datetime('now','localtime')),
            SendBefore time NOT NULL DEFAULT '23:59:59',
            SendAfter time NOT NULL DEFAULT '00:00:00',
            SendDays INTEGER NOT NULL DEFAULT 127,
            Text TEXT, DestinationNumber TEXT NOT NULL DEFAULT '',
            Coding TEXT NOT NULL DEFAULT 'Default_No_Compression',
            UDH TEXT, Class INTEGER DEFAULT -1,
            TextDecoded TEXT NOT NULL DEFAULT '',
            ID INTEGER PRIMARY KEY AUTOINCREMENT,
            MultiPart TEXT NOT NULL DEFAULT 'false',
            RelativeValidity INTEGER DEFAULT -1,
            SenderID TEXT,
            SendingTimeOut NUMERIC NOT NULL DEFAULT (datetime('now','localtime')),
            DeliveryReport TEXT DEFAULT 'default',
            CreatorID TEXT NOT NULL, Retries INTEGER DEFAULT 0,
            Priority INTEGER DEFAULT 0,
            Status TEXT NOT NULL DEFAULT 'Reserved',
            StatusCode INTEGER NOT NULL DEFAULT -1
        );
        CREATE TABLE sentitems (
            UpdatedInDB NUMERIC NOT NULL DEFAULT (datetime('now','localtime')),
            InsertIntoDB NUMERIC NOT NULL DEFAULT (datetime('now','localtime')),
            SendingDateTime NUMERIC NOT NULL DEFAULT (datetime('now','localtime')),
            DeliveryDateTime NUMERIC NULL,
            Text TEXT NOT NULL, DestinationNumber TEXT NOT NULL DEFAULT '',
            Coding TEXT NOT NULL DEFAULT 'Default_No_Compression',
            UDH TEXT NOT NULL, SMSCNumber TEXT NOT NULL DEFAULT '',
            Class INTEGER NOT NULL DEFAULT -1,
            TextDecoded TEXT NOT NULL DEFAULT '',
            ID INTEGER, SenderID TEXT NOT NULL,
            SequencePosition INTEGER NOT NULL DEFAULT 1,
            Status TEXT NOT NULL DEFAULT 'SendingOK',
            StatusError INTEGER NOT NULL DEFAULT -1,
            TPMR INTEGER NOT NULL DEFAULT -1,
            RelativeValidity INTEGER NOT NULL DEFAULT -1,
            CreatorID TEXT NOT NULL,
            StatusCode INTEGER NOT NULL DEFAULT -1,
            PRIMARY KEY (ID, SequencePosition)
        );
    "#;

    fn mem_db() -> SqliteSmsDb {
        let db = SqliteSmsDb::open_in_memory().unwrap();
        db.conn.execute_batch(SCHEMA).unwrap();
        db
    }

    #[test]
    fn ep032_unit_sms_db_submit_writes_documented_outbox_row() {
        let mut db = mem_db();
        let id = db
            .submit("+15551234567", "hello", "nexus:n-1", true)
            .unwrap();
        assert_eq!(id, 1);
        let row: (String, String, String, String, String) = db
            .conn
            .query_row(
                "SELECT CreatorID, DestinationNumber, TextDecoded, DeliveryReport, Status
                 FROM outbox WHERE ID = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .unwrap();
        assert_eq!(row.0, "nexus:n-1");
        assert_eq!(row.1, "+15551234567");
        assert_eq!(row.2, "hello");
        assert_eq!(row.3, "yes");
        assert_eq!(row.4, "Reserved");
    }

    #[test]
    fn ep032_unit_sms_db_status_reserved_while_queued() {
        let mut db = mem_db();
        let id = db
            .submit("+15551234567", "hello", "nexus:n-1", true)
            .unwrap();
        let status = db.status(&id.to_string()).unwrap().unwrap();
        assert_eq!(status.state, SmsProviderState::Reserved);
        assert_eq!(status.delivery_date_time, None);
    }

    #[test]
    fn ep032_unit_sms_db_status_maps_documented_sentitems_states() {
        let mut db = mem_db();
        db.submit("+15551234567", "hello", "nexus:n-1", true)
            .unwrap();
        // Simulate the daemon: outbox row consumed, sentitems row
        // written by the documented add_sent_info path.
        db.conn
            .execute("DELETE FROM outbox WHERE ID = 1", [])
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sentitems (ID, SequencePosition, Status, CreatorID, SenderID, TPMR, Text, UDH)
                 VALUES (1, 1, 'SendingOK', 'nexus:n-1', '', 7, 'hello', '')",
                [],
            )
            .unwrap();
        let status = db.status("1").unwrap().unwrap();
        assert_eq!(status.state, SmsProviderState::SendingOk);

        // Delivery report arrives: documented save_inbox_sms_update_delivered.
        db.conn
            .execute(
                "UPDATE sentitems SET Status = 'DeliveryOK', StatusError = 0,
                        DeliveryDateTime = datetime('now','localtime')
                 WHERE ID = 1 AND TPMR = 7",
                [],
            )
            .unwrap();
        let status = db.status("1").unwrap().unwrap();
        assert_eq!(status.state, SmsProviderState::DeliveryOk);
        assert!(status.delivery_date_time.is_some());
        assert_eq!(status.status_error, Some(0));
    }

    #[test]
    fn ep032_unit_sms_db_status_unknown_id_returns_none() {
        let mut db = mem_db();
        assert!(db.status("999").unwrap().is_none());
    }

    #[test]
    fn ep032_unit_sms_db_rejects_unknown_documented_status() {
        let mut db = mem_db();
        db.submit("+15551234567", "hello", "nexus:n-1", true)
            .unwrap();
        db.conn
            .execute("DELETE FROM outbox WHERE ID = 1", [])
            .unwrap();
        db.conn
            .execute(
                "INSERT INTO sentitems (ID, SequencePosition, Status, CreatorID, SenderID, TPMR, Text, UDH)
                 VALUES (1, 1, 'FAKE', 'nexus:n-1', '', 0, 'hello', '')",
                [],
            )
            .unwrap();
        let err = db.status("1").unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::External);
    }

    #[test]
    fn ep032_unit_sms_db_unopenable_path_fails_closed() {
        let err = SqliteSmsDb::open("/nonexistent/dir/smsd.db").unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Unavailable);
    }
}
