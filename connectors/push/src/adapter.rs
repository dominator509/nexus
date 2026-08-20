//! EP-032 push channel provider adapter (M2).
//!
//! Implements the nexus-notifications `ChannelProvider` port for the
//! MOBILE_PUSH channel over a bound push transport.
//!
//! Deterministic invariants (SPEC-014; EP-032):
//! - `available()` is true ONLY when a transport is bound (Reality
//!   rule). Unbound providers advertise nothing and fail closed.
//! - Every delivery returns a `DeliveryReceipt` carrying the
//!   notification id and correlation (acceptance obligation 4;
//!   SENT != DELIVERED, a receipt is the ONLY delivery authority).
//! - An ack with `delivered: false` is OBSERVED as a Failed receipt,
//!   never fabricated into success.
//! - A duplicate notification id is rejected with Conflict (bounded
//!   recent-delivery ring; idempotency).
//! - Sensitive payload content is never logged or embedded in
//!   errors/telemetry (redaction-safe).

use std::collections::VecDeque;

use nexus_domain::NotificationChannel;
use nexus_notifications::{
    ChannelProvider, DeliveryReceipt, DeliveryReceiptId, DeliveryState, NotificationEnvelope,
    NotificationError, NotificationErrorCode, NotificationId,
};

use crate::transport::PushTransport;

/// Error type returned by the push channel provider (SPEC-006 codes).
pub type PushChannelProviderError = NotificationError;

/// Mobile push channel provider over a bound transport.
#[derive(Debug)]
pub struct PushChannelProvider<T> {
    transport: Option<std::cell::RefCell<T>>,
    /// Bounded ring of recently delivered notification ids
    /// (idempotency; oldest evicted first).
    recent: std::cell::RefCell<VecDeque<NotificationId>>,
    max_recent: usize,
}

impl<T> PushChannelProvider<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport: Some(std::cell::RefCell::new(transport)),
            recent: std::cell::RefCell::new(VecDeque::new()),
            max_recent: 256,
        }
    }

    pub fn unbound() -> Self {
        Self {
            transport: None,
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

impl<T: PushTransport> ChannelProvider for PushChannelProvider<T> {
    fn channel(&self) -> NotificationChannel {
        NotificationChannel::MobilePush
    }

    fn available(&self) -> bool {
        self.transport.is_some()
    }

    fn deliver(
        &self,
        envelope: &NotificationEnvelope,
    ) -> Result<DeliveryReceipt, NotificationError> {
        let Some(transport) = &self.transport else {
            return Err(NotificationError::unavailable(
                "push channel provider has no transport bound",
            ));
        };
        // Idempotency: the same notification is never delivered twice.
        self.record_recent(&envelope.notification_id)?;

        let mut transport = transport.borrow_mut();
        let ack = transport.deliver(envelope)?;

        let state = if ack.delivered {
            DeliveryState::Delivered
        } else {
            DeliveryState::Failed
        };
        let receipt_id = DeliveryReceiptId::new(format!("push-{}", envelope.notification_id))
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
            Some(ack.provider_ref),
            ack.delivered_at_ms,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::PushAck;
    use nexus_domain::{CorrelationId, NotificationChannel, PersonId, Privacy};
    use nexus_notifications::{NotificationUrgency, UnboundChannelProvider};

    fn envelope() -> NotificationEnvelope {
        NotificationEnvelope::new(
            NotificationId::new("n-1").unwrap(),
            PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            NotificationUrgency::High,
            Privacy::Personal,
            "Suspicious sign-in",
            "A new device signed in to your account.",
            vec![NotificationChannel::MobilePush],
            "2026-08-21T12:00:00Z",
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
        )
        .unwrap()
    }

    /// In-memory transport double that plays a real provider peer.
    #[derive(Debug)]
    struct FakePeerTransport {
        delivered: bool,
        provider_ref: String,
    }

    impl PushTransport for FakePeerTransport {
        fn deliver(
            &mut self,
            _envelope: &NotificationEnvelope,
        ) -> Result<PushAck, NotificationError> {
            Ok(PushAck {
                provider_ref: self.provider_ref.clone(),
                delivered: self.delivered,
                delivered_at_ms: Some(1_700_000_000_000),
                error: None,
            })
        }
    }

    #[test]
    fn ep032_unit_push_provider_available_only_when_bound() {
        let bound = PushChannelProvider::new(FakePeerTransport {
            delivered: true,
            provider_ref: "p-1".to_string(),
        });
        assert!(bound.available());
        let unbound = PushChannelProvider::<FakePeerTransport>::unbound();
        assert!(!unbound.available());
        // The contract's fail-closed default also advertises nothing.
        let default = UnboundChannelProvider {
            channel: NotificationChannel::MobilePush,
        };
        assert!(!default.available());
    }

    #[test]
    fn ep032_unit_push_provider_unbound_fails_closed() {
        let provider = PushChannelProvider::<FakePeerTransport>::unbound();
        let err = provider.deliver(&envelope()).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Unavailable);
    }

    #[test]
    fn ep032_unit_push_provider_delivered_receipt_with_correlation() {
        let provider = PushChannelProvider::new(FakePeerTransport {
            delivered: true,
            provider_ref: "p-1".to_string(),
        });
        let receipt = provider.deliver(&envelope()).unwrap();
        assert!(receipt.is_delivered());
        assert_eq!(receipt.channel, NotificationChannel::MobilePush);
        assert_eq!(receipt.notification_id.as_str(), "n-1");
        assert_eq!(
            receipt.correlation_id.as_str(),
            "018f0f6f-9c1e-7b6e-8000-000000000002"
        );
        assert_eq!(receipt.provider_ref.as_deref(), Some("p-1"));
    }

    #[test]
    fn ep032_unit_push_provider_failed_ack_observed_not_fabricated() {
        let provider = PushChannelProvider::new(FakePeerTransport {
            delivered: false,
            provider_ref: "p-2".to_string(),
        });
        let receipt = provider.deliver(&envelope()).unwrap();
        assert!(!receipt.is_delivered());
        assert_eq!(receipt.state, DeliveryState::Failed);
        assert_eq!(receipt.provider_ref.as_deref(), Some("p-2"));
    }

    #[test]
    fn ep032_unit_push_provider_duplicate_rejected_conflict() {
        let provider = PushChannelProvider::new(FakePeerTransport {
            delivered: true,
            provider_ref: "p-1".to_string(),
        });
        assert!(provider.deliver(&envelope()).is_ok());
        let err = provider.deliver(&envelope()).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Conflict);
    }

    #[test]
    fn ep032_unit_push_provider_distinct_ids_not_duplicates() {
        let provider = PushChannelProvider::new(FakePeerTransport {
            delivered: true,
            provider_ref: "p-1".to_string(),
        });
        assert!(provider.deliver(&envelope()).is_ok());
        let other = NotificationEnvelope::new(
            NotificationId::new("n-2").unwrap(),
            PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            NotificationUrgency::High,
            Privacy::Personal,
            "Suspicious sign-in",
            "A new device signed in to your account.",
            vec![NotificationChannel::MobilePush],
            "2026-08-21T12:00:00Z",
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
        )
        .unwrap();
        assert!(provider.deliver(&other).is_ok());
    }
}
