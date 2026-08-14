//! Idempotency tracking for retryable commands (SPEC-006 behavior 2).
//!
//! Commands require idempotency keys when transport or workflow
//! retries are possible. The tracker stores the result produced for a
//! key and replays it for a repeated key, which makes retries safe.
//! A key is bound to the capability it was first used with: reusing
//! the same key for a *different* capability is a conflict and is
//! rejected.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use nexus_capabilities::error::{CapabilityError, CapabilityErrorCode};

/// Error produced by the idempotency tracker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdempotencyTrackerError(pub CapabilityError);

impl std::fmt::Display for IdempotencyTrackerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "idempotency tracker: {}", self.0)
    }
}

impl std::error::Error for IdempotencyTrackerError {}

/// One recorded idempotent command result (SPEC-006
/// `IdempotencyRecord`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    /// Client-supplied idempotency key.
    pub key: String,
    /// Capability the record belongs to.
    pub capability_id: String,
    /// Canonical result payload replayed for retries.
    pub result: serde_json::Value,
}

/// Deterministic in-memory idempotency tracker.
///
/// Interior mutability (`Mutex`) lets the tracker implement `&self`
/// methods while remaining shareable across dispatchers and threads.
#[derive(Debug)]
pub struct IdempotencyTracker {
    /// Records by key; BTreeMap gives deterministic iteration order
    /// for tests and diagnostics.
    records: Mutex<BTreeMap<String, IdempotencyRecord>>,
}

impl Default for IdempotencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for IdempotencyTracker {
    fn clone(&self) -> Self {
        let records = self
            .records
            .lock()
            .expect("idempotency lock poisoned")
            .clone();
        Self {
            records: Mutex::new(records),
        }
    }
}

impl IdempotencyTracker {
    /// Create an empty tracker.
    pub fn new() -> Self {
        Self {
            records: Mutex::new(BTreeMap::new()),
        }
    }

    /// Record an idempotent command result. Re-recording the same key
    /// for the same capability is an update (idempotent); the same key
    /// for a different capability is a conflict.
    pub fn record(&self, record: IdempotencyRecord) -> Result<(), IdempotencyTrackerError> {
        let mut records = self.records.lock().expect("idempotency lock poisoned");
        if let Some(existing) = records.get(&record.key)
            && existing.capability_id != record.capability_id
        {
            return Err(IdempotencyTrackerError(CapabilityError::new(
                CapabilityErrorCode::Conflict,
                "idempotency key reused for a different capability",
                None,
                None,
                None,
                None,
            )));
        }
        records.insert(record.key.clone(), record);
        Ok(())
    }

    /// Look up a recorded result by key.
    pub fn get(&self, key: &str) -> Result<Option<IdempotencyRecord>, IdempotencyTrackerError> {
        Ok(self
            .records
            .lock()
            .expect("idempotency lock poisoned")
            .get(key)
            .cloned())
    }

    /// Number of recorded entries (for tests and diagnostics).
    pub fn len(&self) -> usize {
        self.records
            .lock()
            .expect("idempotency lock poisoned")
            .len()
    }

    /// True when no results are recorded.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
