//! EP-032 SMS channel provider adapter (M3).
//!
//! Implements the nexus-notifications `ChannelProvider` port for the
//! SMS channel over a bound Gammu SMSD gateway.
//!
//! Deterministic invariants (SPEC-014; EP-032):
//! - `available()` is true ONLY when a gateway is bound (Reality
//!   rule). Unbound providers advertise nothing and fail closed.
//! - `deliver()` fails closed WITHOUT a destination: the canonical
//!   envelope is channel-agnostic and carries no phone number, so
//!   fabricating a destination is forbidden. `deliver_to()` is the
//!   typed production entry point (destination is validated BEFORE
//!   any provider mutation).
//! - Every delivery returns a `DeliveryReceipt` carrying the
//!   notification id and correlation (acceptance obligation 4;
//!   SENT != DELIVERED, a receipt is the ONLY delivery authority).
//! - `DeliveryState::Delivered` requires the provider's authoritative
//!   `DeliveryOK` state WITH `DeliveryDateTime` (a real delivery
//!   report). `SendingOK`/`SendingOKNoReport`/`DeliveryPending`/
//!   `DeliveryUnknown` map to `Sending`; `SendingError`/`Error`/
//!   `DeliveryFailed` map to `Failed`. Never fabricated.
//! - A duplicate notification id is rejected with Conflict (bounded
//!   recent-delivery ring; idempotency).
//! - Sensitive payload content (SMS body, full destination) is never
//!   logged or embedded in errors/telemetry (redaction-safe).

use std::collections::VecDeque;

use nexus_domain::NotificationChannel;
use nexus_notifications::{
    ChannelProvider, DeliveryReceipt, DeliveryReceiptId, DeliveryState, NotificationEnvelope,
    NotificationError, NotificationErrorCode, NotificationId, SmsDestination,
};

use crate::gateway::{SmsGateway, SmsProviderState};

/// Error type returned by the SMS channel provider (SPEC-006 codes).
pub type SmsChannelProviderError = NotificationError;

/// SMS channel provider over a bound Gammu SMSD gateway.
#[derive(Debug)]
pub struct SmsChannelProvider<T> {
    gateway: Option<std::cell::RefCell<T>>,
    /// Bounded ring of recently delivered notification ids
    /// (idempotency; oldest evicted first).
    recent: std::cell::RefCell<VecDeque<NotificationId>>,
    max_recent: usize,
}

impl<T> SmsChannelProvider<T> {
    pub fn new(gateway: T) -> Self {
        Self {
            gateway: Some(std::cell::RefCell::new(gateway)),
            recent: std::cell::RefCell::new(VecDeque::new()),
            max_recent: 256,
        }
    }

    pub fn unbound() -> Self {
        Self {
            gateway: None,
            recent: std::cell::RefCell::new(VecDeque::new()),
            max_recent: 256,
        }
    }

    /// Reject a delivery that was already attempted (idempotency).
    fn record_recent(&self, id: &NotificationId) -> Result<(), NotificationError> {
        let mut ring = self.recent.borrow_mut();
        if ring.contains(id) {
            return Err(NotificationError::new(
                NotificationErrorCode::Conflict,
                format!("notification {id} already delivered"),
                None,
                None,
                None,
                Some("NotificationId".to_string()),
            ));
        }
        ring.push_back(id.clone());
        while ring.len() > self.max_recent {
            ring.pop_front();
        }
        Ok(())
    }
}

impl<T: SmsGateway> SmsChannelProvider<T> {
    /// Build the redaction-safe receipt for an observed provider
    /// status. The provider reference is a numeric outbox id; the
    /// body and full destination never appear.
    fn receipt_for(
        &self,
        envelope: &NotificationEnvelope,
        status: &crate::gateway::SmsProviderStatus,
    ) -> Result<DeliveryReceipt, NotificationError> {
        let state = match status.state {
            SmsProviderState::Reserved => DeliveryState::Pending,
            SmsProviderState::SendingOk
            | SmsProviderState::SendingOkNoReport
            | SmsProviderState::DeliveryPending
            | SmsProviderState::DeliveryUnknown => DeliveryState::Sending,
            // Authoritative delivered state ONLY with a real delivery
            // report recorded by the provider (DeliveryDateTime).
            SmsProviderState::DeliveryOk if status.delivered_at.is_some() => {
                DeliveryState::Delivered
            }
            SmsProviderState::DeliveryOk => DeliveryState::Sending,
            SmsProviderState::SendingError
            | SmsProviderState::Error
            | SmsProviderState::DeliveryFailed => DeliveryState::Failed,
        };
        let receipt_id = DeliveryReceiptId::new(format!("sms-{}", envelope.notification_id))
            .map_err(|_| {
                NotificationError::new(
                    NotificationErrorCode::Internal,
                    "failed to build receipt id",
                    Some(envelope.correlation_id.as_str().to_string()),
                    None,
                    None,
                    Some("DeliveryReceiptId".to_string()),
                )
            })?;
        Ok(DeliveryReceipt::new(
            receipt_id,
            envelope.notification_id.clone(),
            self.channel(),
            state,
            envelope.correlation_id.clone(),
            Some(status.provider_ref.as_str().to_string()),
            None,
        ))
    }
}

impl<T: SmsGateway> SmsChannelProvider<T> {
    /// Deliver an envelope to an explicit, validated destination
    /// through the bound gateway. The destination is normalized and
    /// validated BEFORE any provider mutation (fail closed).
    pub fn deliver_to(
        &self,
        envelope: &NotificationEnvelope,
        destination: &SmsDestination,
    ) -> Result<DeliveryReceipt, NotificationError> {
        let Some(gateway) = &self.gateway else {
            return Err(NotificationError::unavailable(
                "sms channel provider has no gateway bound",
            ));
        };
        // Body bound: single SMS part per the documented
        // `outbox.TextDecoded` varchar(160) column. Fail closed
        // rather than silently truncate.
        if envelope.summary.is_empty() || envelope.summary.chars().count() > 160 {
            return Err(NotificationError::validation(
                "sms body must be 1..=160 characters",
            ));
        }
        // Idempotency: the same notification is never delivered twice.
        self.record_recent(&envelope.notification_id)?;

        let mut gateway = gateway.borrow_mut();
        let provider_ref = gateway.submit(
            destination,
            &envelope.summary,
            envelope.notification_id.as_str(),
        )?;
        // Observe the provider-owned queue state: the message is now
        // in the daemon's outbox (Reserved => Pending). The receipt
        // reflects ONLY the observed provider state.
        let status = gateway.status(&provider_ref)?;
        self.receipt_for(envelope, &status)
    }

    /// Observe the current provider state of a previously submitted
    /// message and return an updated receipt.
    pub fn refresh(
        &self,
        envelope: &NotificationEnvelope,
        provider_ref: &crate::gateway::SmsProviderRef,
    ) -> Result<DeliveryReceipt, NotificationError> {
        let Some(gateway) = &self.gateway else {
            return Err(NotificationError::unavailable(
                "sms channel provider has no gateway bound",
            ));
        };
        let mut gateway = gateway.borrow_mut();
        let status = gateway.status(provider_ref)?;
        self.receipt_for(envelope, &status)
    }
}

impl<T: SmsGateway> ChannelProvider for SmsChannelProvider<T> {
    fn channel(&self) -> NotificationChannel {
        NotificationChannel::Sms
    }

    fn available(&self) -> bool {
        self.gateway.is_some()
    }

    fn deliver(
        &self,
        _envelope: &NotificationEnvelope,
    ) -> Result<DeliveryReceipt, NotificationError> {
        // The canonical envelope carries no SMS destination; a
        // destination cannot be invented. Fail closed (never
        // fabricate a recipient).
        Err(NotificationError::validation(
            "sms delivery requires an explicit destination (deliver_to)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::{SmsProviderRef, SmsProviderStatus};
    use nexus_domain::{CorrelationId, PersonId, Privacy};
    use nexus_notifications::{NotificationUrgency, UnboundChannelProvider};

    fn envelope(id: &str) -> NotificationEnvelope {
        NotificationEnvelope::new(
            NotificationId::new(id).unwrap(),
            PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            NotificationUrgency::High,
            Privacy::Personal,
            "Suspicious sign-in",
            "A new device signed in to your account.",
            vec![NotificationChannel::Sms],
            "2026-08-21T12:00:00Z",
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
        )
        .unwrap()
    }

    fn dest() -> SmsDestination {
        SmsDestination::new("+15551234567").unwrap()
    }

    /// In-memory gateway double that records provider mutations.
    /// Unit tests control the peer only; the production
    /// GammuSmsdGateway + real daemon are exercised by the
    /// integration suite (TESTING.md test zones).
    #[derive(Debug, Default)]
    struct FakeGateway {
        submissions: std::cell::RefCell<Vec<(String, String)>>,
        state: std::cell::RefCell<SmsProviderState>,
        delivered_at: std::cell::RefCell<Option<String>>,
    }

    impl SmsGateway for FakeGateway {
        fn submit(
            &mut self,
            _destination: &SmsDestination,
            text: &str,
            notification_id: &str,
        ) -> Result<SmsProviderRef, NotificationError> {
            self.submissions
                .borrow_mut()
                .push((notification_id.to_string(), text.to_string()));
            Ok(SmsProviderRef("7".to_string()))
        }

        fn status(
            &mut self,
            provider_ref: &SmsProviderRef,
        ) -> Result<SmsProviderStatus, NotificationError> {
            Ok(SmsProviderStatus {
                provider_ref: provider_ref.clone(),
                state: *self.state.borrow(),
                delivered_at: self.delivered_at.borrow().clone(),
                status_error: None,
            })
        }
    }

    fn provider_with(state: SmsProviderState) -> SmsChannelProvider<FakeGateway> {
        let g = FakeGateway {
            state: std::cell::RefCell::new(state),
            ..Default::default()
        };
        SmsChannelProvider::new(g)
    }

    #[test]
    fn ep032_unit_sms_provider_available_only_when_bound() {
        let bound = provider_with(SmsProviderState::Reserved);
        assert!(bound.available());
        let unbound = SmsChannelProvider::<FakeGateway>::unbound();
        assert!(!unbound.available());
        // The contract's fail-closed default also advertises nothing.
        let default = UnboundChannelProvider {
            channel: NotificationChannel::Sms,
        };
        assert!(!default.available());
        assert_eq!(bound.channel(), NotificationChannel::Sms);
    }

    #[test]
    fn ep032_unit_sms_provider_unbound_fails_closed() {
        let provider = SmsChannelProvider::<FakeGateway>::unbound();
        let err = provider.deliver_to(&envelope("n-1"), &dest()).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Unavailable);
        let err = provider.deliver(&envelope("n-1")).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Validation);
    }

    #[test]
    fn ep032_unit_sms_provider_requires_explicit_destination() {
        // The canonical deliver() cannot fabricate a recipient.
        let provider = provider_with(SmsProviderState::Reserved);
        let err = provider.deliver(&envelope("n-1")).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Validation);
        // Zero provider mutation from a denied delivery.
        assert!(provider
            .gateway
            .unwrap()
            .borrow()
            .submissions
            .borrow()
            .is_empty());
    }

    #[test]
    fn ep032_unit_sms_provider_rejects_malformed_destination_zero_mutation() {
        let provider = provider_with(SmsProviderState::Reserved);
        let bad = SmsDestination::new("not a number at all").unwrap_err();
        assert_eq!(bad.code, NotificationErrorCode::Validation);
        // The gateway was never touched.
        let g = provider.gateway.as_ref().unwrap();
        assert!(g.borrow().submissions.borrow().is_empty());
        // deliver_to with a syntactically valid but semantically
        // empty normalization is impossible (SmsDestination
        // validates); prove the adapter also validates the body
        // bound.
        let long = NotificationEnvelope::new(
            NotificationId::new("n-1").unwrap(),
            PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            NotificationUrgency::High,
            Privacy::Personal,
            "t",
            "x".repeat(161),
            vec![NotificationChannel::Sms],
            "2026-08-21T12:00:00Z",
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
        )
        .unwrap();
        let err = provider.deliver_to(&long, &dest()).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Validation);
        assert!(g.borrow().submissions.borrow().is_empty());
    }

    #[test]
    fn ep032_unit_sms_provider_reserved_is_pending_not_delivered() {
        let provider = provider_with(SmsProviderState::Reserved);
        let receipt = provider.deliver_to(&envelope("n-1"), &dest()).unwrap();
        assert_eq!(receipt.state, DeliveryState::Pending);
        assert!(!receipt.is_delivered());
        assert_eq!(receipt.channel, NotificationChannel::Sms);
        assert_eq!(receipt.provider_ref.as_deref(), Some("7"));
        assert_eq!(
            receipt.correlation_id.as_str(),
            "018f0f6f-9c1e-7b6e-8000-000000000002"
        );
    }

    #[test]
    fn ep032_unit_sms_provider_sendingok_never_delivered() {
        // SENT != DELIVERED: submission without a delivery report is
        // observed as Sending, never Delivered.
        for state in [
            SmsProviderState::SendingOk,
            SmsProviderState::SendingOkNoReport,
            SmsProviderState::DeliveryPending,
            SmsProviderState::DeliveryUnknown,
        ] {
            let provider = provider_with(state);
            let receipt = provider.deliver_to(&envelope("n-x"), &dest()).unwrap();
            assert_eq!(receipt.state, DeliveryState::Sending, "{state:?}");
            assert!(!receipt.is_delivered());
        }
    }

    #[test]
    fn ep032_unit_sms_provider_deliveryok_requires_datetime() {
        // DeliveryOK WITHOUT DeliveryDateTime cannot prove delivery.
        let provider = provider_with(SmsProviderState::DeliveryOk);
        let receipt = provider.deliver_to(&envelope("n-1"), &dest()).unwrap();
        assert_eq!(receipt.state, DeliveryState::Sending);
        assert!(!receipt.is_delivered());
    }

    #[test]
    fn ep032_unit_sms_provider_deliveryok_with_datetime_is_delivered() {
        let g = FakeGateway {
            state: std::cell::RefCell::new(SmsProviderState::DeliveryOk),
            delivered_at: std::cell::RefCell::new(Some("2026-08-20 16:01:47".to_string())),
            ..Default::default()
        };
        let provider = SmsChannelProvider::new(g);
        let receipt = provider.deliver_to(&envelope("n-1"), &dest()).unwrap();
        assert!(receipt.is_delivered());
        assert_eq!(receipt.state, DeliveryState::Delivered);
    }

    #[test]
    fn ep032_unit_sms_provider_failed_states_observed_not_fabricated() {
        for state in [
            SmsProviderState::SendingError,
            SmsProviderState::Error,
            SmsProviderState::DeliveryFailed,
        ] {
            let provider = provider_with(state);
            let receipt = provider.deliver_to(&envelope("n-x"), &dest()).unwrap();
            assert_eq!(receipt.state, DeliveryState::Failed, "{state:?}");
            assert!(!receipt.is_delivered());
        }
    }

    #[test]
    fn ep032_unit_sms_provider_duplicate_rejected_conflict_one_mutation() {
        let provider = provider_with(SmsProviderState::Reserved);
        assert!(provider.deliver_to(&envelope("n-1"), &dest()).is_ok());
        let err = provider.deliver_to(&envelope("n-1"), &dest()).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Conflict);
        // Exactly one provider mutation for the duplicate pair.
        let g = provider.gateway.as_ref().unwrap();
        assert_eq!(g.borrow().submissions.borrow().len(), 1);
    }

    #[test]
    fn ep032_unit_sms_provider_distinct_ids_not_duplicates() {
        let provider = provider_with(SmsProviderState::Reserved);
        assert!(provider.deliver_to(&envelope("n-1"), &dest()).is_ok());
        assert!(provider.deliver_to(&envelope("n-2"), &dest()).is_ok());
        let g = provider.gateway.as_ref().unwrap();
        assert_eq!(g.borrow().submissions.borrow().len(), 2);
    }

    #[test]
    fn ep032_unit_sms_provider_redaction_no_body_no_full_destination() {
        // Errors and receipts must never carry the SMS body or the
        // full destination number (redaction-safe).
        let provider = provider_with(SmsProviderState::SendingError);
        let receipt = provider.deliver_to(&envelope("n-1"), &dest()).unwrap();
        let debug = format!("{receipt:?}");
        assert!(!debug.contains("A new device signed in"));
        assert!(!debug.contains("15551234567"));
        let err = provider.deliver(&envelope("n-1")).unwrap_err();
        assert!(!format!("{err:?}").contains("15551234567"));
        assert!(!format!("{err:?}").contains("A new device signed in"));
    }

    #[test]
    fn ep032_unit_sms_provider_refresh_reflects_later_provider_state() {
        let g = FakeGateway {
            state: std::cell::RefCell::new(SmsProviderState::SendingOk),
            ..Default::default()
        };
        let provider = SmsChannelProvider::new(g);
        let env = envelope("n-1");
        let receipt = provider.deliver_to(&env, &dest()).unwrap();
        assert_eq!(receipt.state, DeliveryState::Sending);
        // Provider later records a real delivery report.
        provider
            .gateway
            .as_ref()
            .unwrap()
            .borrow()
            .state
            .replace(SmsProviderState::DeliveryOk);
        provider
            .gateway
            .as_ref()
            .unwrap()
            .borrow()
            .delivered_at
            .replace(Some("2026-08-20 16:01:47".to_string()));
        let updated = provider
            .refresh(&env, &SmsProviderRef("7".to_string()))
            .unwrap();
        assert!(updated.is_delivered());
    }
}
