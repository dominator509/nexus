//! EP-032 SMS gateway port (M3): the documented Gammu SMSD interface.
//!
//! The gateway boundary is provider-neutral: an implementation
//! submits a message to the daemon's outbox queue and observes the
//! daemon-written provider state (outbox `Reserved` -> sentitems
//! status lifecycle, `DeliveryDateTime` from a real delivery report).
//!
//! Provider state semantics are locked to the DOCUMENTED Gammu SMSD
//! status vocabulary (SMSD Database Structure, Gammu >= 1.38.5):
//! - `Reserved`            : enqueued, not yet submitted (outbox row)
//! - `SendingOK`           : submitted to network, awaiting report
//! - `SendingOKNoReport`   : submitted, no delivery report requested
//! - `SendingError`        : modem/network submission failed
//! - `Error`               : other processing error
//! - `DeliveryOK`          : real delivery report, success
//! - `DeliveryFailed`      : real delivery report, failure
//! - `DeliveryPending`     : real delivery report, pending
//! - `DeliveryUnknown`     : real delivery report, unknown status
//!
//! Only `DeliveryOK` WITH a recorded `DeliveryDateTime` is an
//! authoritative delivered state. Everything else is queued, in
//! flight, or failed - never fabricated into success.

use nexus_notifications::{NotificationError, SmsDestination};

use crate::db::{SmsDb, SmsDbStatusRow};

/// Provider-side identity of a submitted SMS: the documented outbox
/// row ID. Carried in the `DeliveryReceipt.provider_ref`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SmsProviderRef(pub String);

impl SmsProviderRef {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SmsProviderRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Provider-observed state of a submitted message (documented SMSD
/// status vocabulary). This is the ONLY evidence the connector maps
/// into canonical `DeliveryState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SmsProviderState {
    /// Enqueued in outbox; the daemon has not submitted it yet.
    #[default]
    Reserved,
    /// Submitted to the network; waiting for a delivery report.
    SendingOk,
    /// Submitted; no delivery report requested (no delivery proof).
    SendingOkNoReport,
    /// Real delivery report reported success (authoritative).
    DeliveryOk,
    /// Real delivery report reported failure.
    DeliveryFailed,
    /// Real delivery report announced pending delivery.
    DeliveryPending,
    /// Real delivery report returned unknown status.
    DeliveryUnknown,
    /// Modem/network submission failed.
    SendingError,
    /// Other daemon processing error.
    Error,
}

impl SmsProviderState {
    /// Parse the documented outbox/sentitems `Status` column value.
    pub fn parse_documented(value: &str) -> Option<Self> {
        match value {
            "Reserved" => Some(Self::Reserved),
            "SendingOK" => Some(Self::SendingOk),
            "SendingOKNoReport" => Some(Self::SendingOkNoReport),
            "DeliveryOK" => Some(Self::DeliveryOk),
            "DeliveryFailed" => Some(Self::DeliveryFailed),
            "DeliveryPending" => Some(Self::DeliveryPending),
            "DeliveryUnknown" => Some(Self::DeliveryUnknown),
            "SendingError" => Some(Self::SendingError),
            "Error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Provider-observed status of one submitted message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmsProviderStatus {
    pub provider_ref: SmsProviderRef,
    pub state: SmsProviderState,
    /// `sentitems.DeliveryDateTime` - set only when a real delivery
    /// report arrived (documented field).
    pub delivered_at: Option<String>,
    /// `sentitems.StatusError` - TP-Status from the delivery report
    /// (GSM 03.40 section 9.2.3.15), -1 when unset.
    pub status_error: Option<i32>,
}

/// The SMS gateway boundary: submit to the daemon's outbox queue and
/// observe daemon-written provider state. Implementations speak the
/// documented Gammu SMSD SQL service through a real `SmsDb`.
#[derive(Debug)]
pub struct GammuSmsdGateway<D> {
    db: D,
    /// Identifier recorded in the outbox `CreatorID` column so the
    /// provider-side message identity binds to the notification.
    creator_prefix: String,
}

impl<D: SmsDb> GammuSmsdGateway<D> {
    pub fn new(db: D, creator_prefix: impl Into<String>) -> Self {
        Self {
            db,
            creator_prefix: creator_prefix.into(),
        }
    }

    /// The creator id recorded on the outbox row for a notification.
    pub fn creator_for(&self, notification_id: &str) -> String {
        format!("{}{}", self.creator_prefix, notification_id)
    }

    /// Submit a message to the daemon's outbox queue (documented
    /// `create_outbox` shape). Returns the provider message reference
    /// (outbox row id). `DeliveryReport=yes` requests a delivery
    /// report from the network so the daemon can record the real
    /// delivered state.
    pub fn submit(
        &mut self,
        destination: &SmsDestination,
        text: &str,
        notification_id: &str,
    ) -> Result<SmsProviderRef, NotificationError> {
        let id = self.db.submit(
            destination.as_str(),
            text,
            &self.creator_for(notification_id),
            true,
        )?;
        Ok(SmsProviderRef(id.to_string()))
    }

    /// Observe the daemon-written provider state for a message.
    pub fn status(
        &mut self,
        provider_ref: &SmsProviderRef,
    ) -> Result<SmsProviderStatus, NotificationError> {
        let row: SmsDbStatusRow = self.db.status(&provider_ref.0)?.ok_or_else(|| {
            NotificationError::external(format!(
                "provider message {} not found in outbox/sentitems",
                provider_ref.as_str()
            ))
        })?;
        Ok(SmsProviderStatus {
            provider_ref: provider_ref.clone(),
            state: row.state,
            delivered_at: row.delivery_date_time,
            status_error: row.status_error,
        })
    }
}

/// The connector's gateway boundary (provider-neutral; unit tests use
/// an in-memory double; production/fixture use `GammuSmsdGateway`).
pub trait SmsGateway {
    fn submit(
        &mut self,
        destination: &SmsDestination,
        text: &str,
        notification_id: &str,
    ) -> Result<SmsProviderRef, NotificationError>;
    fn status(
        &mut self,
        provider_ref: &SmsProviderRef,
    ) -> Result<SmsProviderStatus, NotificationError>;
}

impl<D: SmsDb> SmsGateway for GammuSmsdGateway<D> {
    fn submit(
        &mut self,
        destination: &SmsDestination,
        text: &str,
        notification_id: &str,
    ) -> Result<SmsProviderRef, NotificationError> {
        GammuSmsdGateway::submit(self, destination, text, notification_id)
    }

    fn status(
        &mut self,
        provider_ref: &SmsProviderRef,
    ) -> Result<SmsProviderStatus, NotificationError> {
        GammuSmsdGateway::status(self, provider_ref)
    }
}
