//! Change-feed capability port (SPEC-022 canonical term
//! `ChangeCursor`).
//!
//! A change-feed capability exposes events or changes-since semantics
//! for state reconciliation (SPEC-022 behavior 2). Cursors are opaque
//! tokens with canonical IDs, versions, correlation, and cursor
//! semantics (SPEC-022 behavior 8).

use serde::{Deserialize, Serialize};

use crate::context::InvocationContext;
use crate::error::CapabilityError;

/// Opaque change cursor (SPEC-022 canonical term `ChangeCursor`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeCursor {
    /// Capability key the cursor belongs to.
    pub capability_id: String,
    /// Opaque cursor value (never interpreted by clients).
    pub cursor: String,
}

/// One canonical change-feed event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeEvent {
    /// Canonical event ID (SPEC-022 behavior 8).
    pub event_id: String,
    /// Event type advertised by the capability descriptor.
    pub event_type: String,
    /// Canonical payload.
    pub payload: serde_json::Value,
}

/// A batch of change-feed events plus the next cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeBatch {
    /// Capability key.
    pub capability_id: String,
    /// Events since the previous cursor.
    pub events: Vec<ChangeEvent>,
    /// Next cursor for continued consumption.
    pub next_cursor: ChangeCursor,
}

/// Provider-neutral change-feed port (SPEC-022).
pub trait ChangeFeedCapability {
    /// Read events since an optional cursor.
    fn changes_since(
        &self,
        capability_id: String,
        cursor: Option<ChangeCursor>,
        context: InvocationContext,
    ) -> Result<ChangeBatch, CapabilityError>;
}
