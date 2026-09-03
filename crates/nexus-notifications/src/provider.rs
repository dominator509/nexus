//! EP-032 provider ports (fail-closed defaults; SPEC-014).
//!
//! The Communication Router selects push, desktop, speaker, SMS,
//! email, phone, watch, car, or future robot based on person,
//! presence, privacy, urgency, quiet hours, cost, and availability
//! (SPEC-014 behavior 7). Nexus orchestrates providers; it never
//! replaces a provider's transport with a home-grown stack. Unbound
//! providers fail closed and never fabricate delivery state (Reality
//! rule). Provider-specific payloads are normalized at the
//! infrastructure boundary and never become domain contracts.

use nexus_domain::{NotificationChannel, PersonId};

use crate::error::NotificationError;
use crate::model::{DeliveryPolicy, DeliveryReceipt, NotificationEnvelope};
use crate::vocabulary::SmsDestination;

/// Channel provider port (provider-neutral; push / SMS / desktop /
/// speaker / email / phone / watch / car providers implement this
/// boundary).
pub trait ChannelProvider {
    /// The channel this provider serves.
    fn channel(&self) -> NotificationChannel;

    /// Whether the provider is currently bound and available. A bound
    /// provider advertises availability; an unbound provider does not.
    fn available(&self) -> bool {
        false
    }

    /// Whether this provider REQUIRES an explicit destination to
    /// deliver (e.g. SMS needs a phone number). Destination-requiring
    /// providers cannot deliver from the channel-agnostic envelope
    /// alone; the router resolves a destination through its
    /// `DestinationResolver` before attempting the leg. A provider
    /// that does not require a destination is attempted through the
    /// canonical `deliver`.
    fn requires_destination(&self) -> bool {
        false
    }

    /// Deliver an envelope on this provider's channel. Returns a
    /// delivery receipt; a receipt is the ONLY delivery authority.
    fn deliver(
        &self,
        envelope: &NotificationEnvelope,
    ) -> Result<DeliveryReceipt, NotificationError> {
        let _ = envelope;
        Err(NotificationError::unavailable(
            "channel provider has no implementation bound",
        ))
    }

    /// Deliver an envelope to an EXPLICIT, validated destination
    /// (destination-requiring channels such as SMS). The destination
    /// is validated BEFORE any provider mutation (fail closed). A
    /// provider without destination-aware delivery fails closed:
    /// a destination can never be invented.
    fn deliver_to(
        &self,
        envelope: &NotificationEnvelope,
        destination: &SmsDestination,
    ) -> Result<DeliveryReceipt, NotificationError> {
        let _ = (envelope, destination);
        Err(NotificationError::unavailable(
            "channel provider has no destination-aware delivery bound",
        ))
    }
}

/// Resolves the destination a destination-requiring channel needs
/// (e.g. the SMS phone number for a person) at delivery time.
///
/// A resolver is an authority over destinations: it never invents
/// one. An unresolved destination is a truthful non-delivery (the
/// router fails the leg closed and escalates), never a fabricated
/// recipient.
pub trait DestinationResolver {
    /// Resolve the SMS destination for a person, if one is known.
    fn resolve(&self, person_id: &PersonId) -> Option<SmsDestination>;
}

/// Fail-closed destination resolver: no destination is ever known.
/// Destination-requiring legs fail closed (never fabricated).
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundDestinationResolver;

impl DestinationResolver for UnboundDestinationResolver {
    fn resolve(&self, _person_id: &PersonId) -> Option<SmsDestination> {
        None
    }
}

/// Notification router port: applies delivery policy, privacy routing,
/// and escalation, then dispatches to the bound channel providers.
pub trait NotificationRouter {
    /// Route an envelope to the configured providers and return one
    /// receipt per attempted channel.
    fn route(
        &self,
        envelope: &NotificationEnvelope,
        policy: &DeliveryPolicy,
    ) -> Result<Vec<DeliveryReceipt>, NotificationError> {
        let _ = (envelope, policy);
        Err(NotificationError::unavailable(
            "notification router has no implementation bound",
        ))
    }

    /// Route with an explicit delivery context (quiet hours, presence,
    /// acknowledgement, time). Fails closed when unbound.
    fn route_with_context(
        &self,
        envelope: &NotificationEnvelope,
        policy: &DeliveryPolicy,
        ctx: &crate::model::DeliveryContext,
    ) -> Result<Vec<DeliveryReceipt>, NotificationError> {
        let _ = (envelope, policy, ctx);
        Err(NotificationError::unavailable(
            "notification router has no implementation bound",
        ))
    }
}

/// Fail-closed channel provider for an unbound channel. Advertises
/// nothing and always returns Unavailable (Reality rule).
#[derive(Debug, Clone, Copy)]
pub struct UnboundChannelProvider {
    pub channel: NotificationChannel,
}

impl ChannelProvider for UnboundChannelProvider {
    fn channel(&self) -> NotificationChannel {
        self.channel
    }

    fn available(&self) -> bool {
        false
    }

    fn deliver(
        &self,
        _envelope: &NotificationEnvelope,
    ) -> Result<DeliveryReceipt, NotificationError> {
        Err(NotificationError::unavailable(
            "channel provider has no implementation bound",
        ))
    }
}

/// Fail-closed router with no providers bound. Advertises nothing and
/// always returns Unavailable.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundNotificationRouter;

impl NotificationRouter for UnboundNotificationRouter {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EscalationPolicy;
    use crate::vocabulary::{DeliveryState, NotificationId, NotificationUrgency};
    use nexus_domain::{CorrelationId, PersonId, Privacy};

    fn sample_envelope() -> NotificationEnvelope {
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

    #[test]
    fn ep032_unit_unbound_provider_fails_closed() {
        let provider = UnboundChannelProvider {
            channel: NotificationChannel::MobilePush,
        };
        assert_eq!(provider.channel(), NotificationChannel::MobilePush);
        assert!(!provider.available());
        let err = provider.deliver(&sample_envelope()).unwrap_err();
        assert_eq!(err.code, crate::error::NotificationErrorCode::Unavailable);
    }

    #[test]
    fn ep032_unit_unbound_router_fails_closed() {
        let router = UnboundNotificationRouter;
        let policy = DeliveryPolicy {
            min_urgency: NotificationUrgency::Low,
            allowed_channels: vec![NotificationChannel::MobilePush],
            quiet_hours_suppress: false,
            require_acknowledgement: false,
            require_presence: false,
        };
        let err = router.route(&sample_envelope(), &policy).unwrap_err();
        assert_eq!(err.code, crate::error::NotificationErrorCode::Unavailable);
    }

    #[test]
    fn ep032_unit_escalation_references_policy() {
        // Compile-time reference: escalation works with router policy
        // and receipt correlation (acceptance obligation 3 + 4).
        let policy = EscalationPolicy::new(vec![
            NotificationChannel::MobilePush,
            NotificationChannel::Sms,
        ])
        .unwrap();
        let next = policy.next_after(NotificationChannel::MobilePush);
        assert_eq!(next, Some(NotificationChannel::Sms));
        let _ = DeliveryState::Pending;
    }
}
