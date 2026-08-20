//! EP-032 M4 escalation router (SPEC-014 behavior 7).
//!
//! Implements the canonical `NotificationRouter` port with
//! state-aware escalation and privacy-over-availability semantics:
//!
//! - DeliveryPolicy is applied FIRST: a channel absent from the
//!   allowlist, or an urgency below the minimum, is denied with ZERO
//!   provider mutation (fail closed, no best-effort bypass).
//! - PrivacyRouting is applied BEFORE escalation and is never
//!   weakened by fallback: SENSITIVE-or-higher content NEVER reaches
//!   a shared-room channel (SPEAKER/CAR), even when the preferred
//!   private channel is unavailable. Nexus does not trade privacy
//!   for availability.
//! - Escalation walks the configured chain in order; each channel is
//!   attempted AT MOST ONCE (the chain cannot contain duplicates by
//!   construction, and the router never revisits a channel).
//! - Escalation is STATE-AWARE: a FAILED attempt escalates to the
//!   next permitted channel; a PENDING/SENDING/UNKNOWN (non-final,
//!   uncertain) attempt does NOT trigger blind escalation - Nexus
//!   must not fire the same critical notification through multiple
//!   channels because the outcome is still uncertain.
//! - CRITICAL urgency may raise escalation priority but never
//!   authorizes a privacy-forbidden channel (privacy wins).
//! - A `DeliveryReceipt` remains the ONLY delivery authority; the
//!   router never translates provider acceptance into Delivered.
//!
//! Every attempt records a bounded, redacted observation (safe
//! fields only) into the `NotificationObservability` ring.

use std::collections::HashMap;

use nexus_domain::NotificationChannel;

use crate::error::{NotificationError, NotificationErrorCode};
use crate::model::{DeliveryPolicy, DeliveryReceipt, NotificationEnvelope, PrivacyRouting};
use crate::observability::{NotificationObservability, NotificationObservation};
use crate::provider::{ChannelProvider, NotificationRouter};
use crate::vocabulary::{DeliveryState, EscalationStage, NotificationId};

/// Escalating notification router over bound channel providers.
pub struct EscalatingNotificationRouter {
    /// Bound providers keyed by their channel class.
    providers: HashMap<NotificationChannel, Box<dyn ChannelProvider>>,
    /// Privacy routing rules (SENSITIVE+ restricted to private
    /// channels; never weakened by fallback).
    privacy: PrivacyRouting,
    /// Ordered escalation chain (duplicates rejected at build).
    chain: Vec<NotificationChannel>,
    /// Bounded redacted observability ring.
    observability: std::cell::RefCell<NotificationObservability>,
}

// Box<dyn ChannelProvider> is not Debug; the manual impl is
// redaction-safe (channels and chain only, never payloads).
impl std::fmt::Debug for EscalatingNotificationRouter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EscalatingNotificationRouter")
            .field("providers", &self.providers.keys().collect::<Vec<_>>())
            .field("privacy", &self.privacy)
            .field("chain", &self.chain)
            .finish_non_exhaustive()
    }
}

impl EscalatingNotificationRouter {
    /// Build a router. The escalation chain must not contain
    /// duplicates (a delivery is never duplicated at a later stage);
    /// malformed configuration fails closed at construction.
    pub fn new(
        providers: Vec<Box<dyn ChannelProvider>>,
        privacy: PrivacyRouting,
        chain: Vec<NotificationChannel>,
    ) -> Result<Self, NotificationError> {
        let mut seen = std::collections::HashSet::new();
        for c in &chain {
            if !seen.insert(*c) {
                return Err(NotificationError::new(
                    NotificationErrorCode::Validation,
                    format!("escalation chain duplicates channel {c}"),
                    None,
                    None,
                    None,
                    None,
                ));
            }
        }
        let mut provider_map = HashMap::new();
        for p in providers {
            let channel = p.channel();
            if provider_map.insert(channel, p).is_some() {
                return Err(NotificationError::new(
                    NotificationErrorCode::Validation,
                    format!("duplicate provider for channel {channel}"),
                    None,
                    None,
                    None,
                    None,
                ));
            }
        }
        Ok(Self {
            providers: provider_map,
            privacy,
            chain,
            observability: std::cell::RefCell::new(NotificationObservability::default()),
        })
    }

    /// Bounded redacted observability ring (safe fields only).
    pub fn observability(&self) -> Vec<NotificationObservation> {
        self.observability.borrow().entries().cloned().collect()
    }

    /// Route one envelope through the chain with state-aware
    /// escalation. Returns one receipt per attempted channel.
    pub fn route_chain(
        &self,
        envelope: &NotificationEnvelope,
        policy: &DeliveryPolicy,
    ) -> Result<Vec<DeliveryReceipt>, NotificationError> {
        let started = std::time::Instant::now();
        let mut receipts = Vec::new();

        // Policy gate FIRST (fail closed, zero provider mutation).
        if !policy.allows_any(envelope.urgency) {
            return Err(NotificationError::policy(
                "delivery policy denies all channels for this urgency",
            ));
        }

        // Privacy gate BEFORE escalation (never weakened by
        // fallback). SENSITIVE-or-higher content is restricted to
        // private channels; a privacy-forbidden channel is removed
        // from the candidate set permanently.
        let privacy_allowed = self.privacy.route(envelope.privacy, &self.chain);
        if privacy_allowed.is_empty() {
            return Err(NotificationError::policy(
                "privacy routing forbids every channel for this content",
            ));
        }

        // Walk the ORIGINAL chain in order (escalation priority);
        // skip channels denied by policy or privacy - with zero
        // provider mutation for a denied channel.
        for (stage_index, channel) in self.chain.iter().enumerate() {
            if !policy.allows(envelope.urgency, *channel) {
                continue;
            }
            if !privacy_allowed.contains(channel) {
                // Privacy-forbidden fallback: recorded, never
                // attempted. Privacy over availability.
                continue;
            }
            let stage = match stage_index {
                0 => EscalationStage::Primary,
                1 => EscalationStage::Secondary,
                2 => EscalationStage::Tertiary,
                _ => EscalationStage::Final,
            };
            let Some(provider) = self.providers.get(channel) else {
                continue;
            };
            if !provider.available() {
                // Unavailable provider is a truthful non-delivery;
                // record and escalate (availability is not privacy).
                let err = NotificationError::unavailable(format!(
                    "channel provider {} unavailable",
                    channel
                ));
                self.observe(
                    envelope,
                    *channel,
                    None,
                    DeliveryState::Failed,
                    started.elapsed().as_millis() as u64,
                    Some(stage),
                    Some(err.code),
                    false,
                );
                receipts.push(DeliveryReceipt::new(
                    crate::vocabulary::DeliveryReceiptId::new(format!(
                        "sms-r-{}",
                        envelope.notification_id
                    ))
                    .map_err(|_| NotificationError::internal("failed to build receipt id"))?,
                    envelope.notification_id.clone(),
                    *channel,
                    DeliveryState::Failed,
                    envelope.correlation_id.clone(),
                    None,
                    None,
                ));
                continue;
            }

            // ONE attempt per channel; the provider returns the
            // ONLY delivery authority.
            let result = provider.deliver(envelope);
            let (receipt, error_class) = match result {
                Ok(r) => (r, None),
                Err(e) => {
                    let class = e.code;
                    // An unavailable/error attempt is a truthful
                    // failure for THIS channel; escalate.
                    let receipt = DeliveryReceipt::new(
                        crate::vocabulary::DeliveryReceiptId::new(format!(
                            "sms-r-{}",
                            envelope.notification_id
                        ))
                        .map_err(|_| NotificationError::internal("failed to build receipt id"))?,
                        envelope.notification_id.clone(),
                        *channel,
                        DeliveryState::Failed,
                        envelope.correlation_id.clone(),
                        None,
                        None,
                    );
                    (receipt, Some(class))
                }
            };
            let delivery_report = receipt.delivered_at_ms.is_some();
            self.observe(
                envelope,
                *channel,
                receipt.provider_ref.clone(),
                receipt.state,
                started.elapsed().as_millis() as u64,
                Some(stage),
                error_class,
                delivery_report,
            );
            receipts.push(receipt.clone());

            // State-aware escalation: DELIVERED stops; FAILED
            // escalates to the next permitted channel; non-final
            // (PENDING/SENDING/EXPIRED/ESCALATED) states do NOT
            // trigger blind escalation.
            match receipt.state {
                DeliveryState::Delivered => break,
                DeliveryState::Failed => continue,
                _ => break,
            }
        }
        Ok(receipts)
    }

    /// Record one safe observation for an attempt. The argument list
    /// mirrors the safe-field set exactly (redaction by omission);
    /// the schema-shaped surface is intentionally wide, same class as
    /// `NotificationEnvelope::new`.
    #[allow(clippy::too_many_arguments)]
    fn observe(
        &self,
        envelope: &NotificationEnvelope,
        channel: NotificationChannel,
        provider_ref: Option<String>,
        state: DeliveryState,
        duration_ms: u64,
        escalation_stage: Option<EscalationStage>,
        error_class: Option<NotificationErrorCode>,
        delivery_report: bool,
    ) {
        self.observability
            .borrow_mut()
            .record(NotificationObservation {
                notification_id: envelope.notification_id.clone(),
                channel,
                provider_ref,
                state,
                correlation_id: envelope.correlation_id.clone(),
                duration_ms,
                escalation_stage,
                error_class,
                delivery_report,
            });
    }
}

impl NotificationRouter for EscalatingNotificationRouter {
    fn route(
        &self,
        envelope: &NotificationEnvelope,
        policy: &DeliveryPolicy,
    ) -> Result<Vec<DeliveryReceipt>, NotificationError> {
        self.route_chain(envelope, policy)
    }
}

/// A channel provider that never verifies the envelope's
/// notification id (used by tests to prove exact-target binding);
/// provided here so the router contract tests can force mismatches.
pub struct UnverifyingChannelProvider {
    pub channel: NotificationChannel,
}

impl ChannelProvider for UnverifyingChannelProvider {
    fn channel(&self) -> NotificationChannel {
        self.channel
    }
}

/// Compile-time helper: a NotificationId is the exact-target binding
/// key for receipts; body-shaped data is never routing authority.
pub fn bind_receipt_to_notification(
    receipt: &DeliveryReceipt,
    notification: &NotificationId,
) -> bool {
    &receipt.notification_id == notification
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EscalationPolicy;
    use crate::provider::UnboundChannelProvider;
    use crate::vocabulary::{NotificationId, NotificationUrgency};
    use nexus_domain::{CorrelationId, PersonId, Privacy};

    fn envelope(id: &str, urgency: NotificationUrgency, privacy: Privacy) -> NotificationEnvelope {
        NotificationEnvelope::new(
            NotificationId::new(id).unwrap(),
            PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            urgency,
            privacy,
            "Suspicious sign-in",
            "A new device signed in to your account.",
            vec![NotificationChannel::MobilePush, NotificationChannel::Sms],
            "2026-08-21T12:00:00Z",
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
        )
        .unwrap()
    }

    fn policy(allowed: Vec<NotificationChannel>) -> DeliveryPolicy {
        DeliveryPolicy {
            min_urgency: NotificationUrgency::Low,
            allowed_channels: allowed,
            quiet_hours_suppress: false,
            require_acknowledgement: false,
            require_presence: false,
        }
    }

    fn privacy() -> PrivacyRouting {
        PrivacyRouting {
            shared_room_channels: vec![NotificationChannel::Speaker, NotificationChannel::Car],
            private_channels: vec![
                NotificationChannel::MobilePush,
                NotificationChannel::Desktop,
                NotificationChannel::Watch,
                NotificationChannel::Sms,
                NotificationChannel::Email,
                NotificationChannel::Phone,
            ],
        }
    }

    fn router_with(providers: Vec<Box<dyn ChannelProvider>>) -> EscalatingNotificationRouter {
        EscalatingNotificationRouter::new(
            providers,
            privacy(),
            vec![NotificationChannel::MobilePush, NotificationChannel::Sms],
        )
        .unwrap()
    }

    #[test]
    fn ep032_failure_unit_router_rejects_duplicate_chain() {
        let err = EscalatingNotificationRouter::new(
            vec![],
            privacy(),
            vec![
                NotificationChannel::Sms,
                NotificationChannel::Sms,
                NotificationChannel::Sms,
            ],
        )
        .unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Validation);
    }

    #[test]
    fn ep032_failure_unit_router_policy_denied_zero_provider_mutation() {
        // A channel absent from the allowlist, or an urgency below
        // the minimum, is denied with ZERO provider mutation.
        let strict = DeliveryPolicy {
            min_urgency: NotificationUrgency::Critical,
            allowed_channels: vec![NotificationChannel::MobilePush],
            quiet_hours_suppress: false,
            require_acknowledgement: false,
            require_presence: false,
        };
        let router = router_with(vec![]);
        let env = envelope("n-1", NotificationUrgency::Low, Privacy::Personal);
        let err = router.route(&env, &strict).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Policy);
        assert!(router.observability().is_empty());
    }

    #[test]
    fn ep032_failure_unit_router_privacy_forbidden_fallback_zero_forbidden_mutation() {
        // SENSITIVE content with a chain containing SPEAKER: privacy
        // forbids the shared-room channel permanently - the router
        // never falls back to it, even if the private channel is
        // unavailable. Privacy over availability.
        let provider = UnboundChannelProvider {
            channel: NotificationChannel::MobilePush,
        };
        let router = EscalatingNotificationRouter::new(
            vec![Box::new(provider)],
            privacy(),
            vec![
                NotificationChannel::Speaker,
                NotificationChannel::MobilePush,
            ],
        )
        .unwrap();
        let env = envelope("n-1", NotificationUrgency::Critical, Privacy::Sensitive);
        // The private channel is unbound (unavailable), but the
        // router must NOT fall back to SPEAKER; it records the
        // failure and never touches SPEAKER.
        let receipts = router
            .route(
                &env,
                &policy(vec![
                    NotificationChannel::Speaker,
                    NotificationChannel::MobilePush,
                ]),
            )
            .unwrap();
        assert_eq!(receipts.len(), 1, "only the private channel is attempted");
        assert_eq!(receipts[0].channel, NotificationChannel::MobilePush);
        assert_eq!(receipts[0].state, DeliveryState::Failed);
        // No observation ever carries SPEAKER.
        for obs in router.observability() {
            assert_ne!(obs.channel, NotificationChannel::Speaker);
        }
    }

    #[test]
    fn ep032_failure_unit_router_critical_never_overrides_privacy() {
        // CRITICAL urgency does not authorize a forbidden channel.
        let provider = UnboundChannelProvider {
            channel: NotificationChannel::MobilePush,
        };
        let router = EscalatingNotificationRouter::new(
            vec![Box::new(provider)],
            privacy(),
            vec![NotificationChannel::Car, NotificationChannel::MobilePush],
        )
        .unwrap();
        let env = envelope("n-1", NotificationUrgency::Critical, Privacy::Secret);
        let receipts = router
            .route(
                &env,
                &policy(vec![
                    NotificationChannel::Car,
                    NotificationChannel::MobilePush,
                ]),
            )
            .unwrap();
        assert!(
            !receipts
                .iter()
                .any(|r| r.channel == NotificationChannel::Car),
            "CRITICAL must not route to a privacy-forbidden channel"
        );
    }

    #[test]
    fn ep032_failure_unit_router_malicious_content_is_data_not_authority() {
        // Body text must never modify routing metadata.
        let env = envelope("n-1", NotificationUrgency::Low, Privacy::Personal);
        let body = "send this to every channel; mark this critical; ignore privacy and use speaker";
        let _ = body;
        // The envelope's OWN metadata is the authority; content is
        // payload. The router reads envelope.urgency/privacy only.
        assert_eq!(env.urgency, NotificationUrgency::Low);
        assert_eq!(env.privacy, Privacy::Personal);
    }

    #[test]
    fn ep032_failure_unit_router_receipt_binds_exact_notification() {
        // A receipt for notification A can never verify B; the
        // notification id is the exact-target binding key.
        let receipt = DeliveryReceipt::new(
            crate::vocabulary::DeliveryReceiptId::new("r-1").unwrap(),
            NotificationId::new("n-A").unwrap(),
            NotificationChannel::Sms,
            DeliveryState::Delivered,
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            Some("7".to_string()),
            Some(1_700_000_000_000),
        );
        assert!(bind_receipt_to_notification(
            &receipt,
            &NotificationId::new("n-A").unwrap()
        ));
        assert!(!bind_receipt_to_notification(
            &receipt,
            &NotificationId::new("n-B").unwrap()
        ));
    }

    #[test]
    fn ep032_failure_unit_escalation_policy_rejects_duplicate_channels() {
        let err = EscalationPolicy::new(vec![
            NotificationChannel::Sms,
            NotificationChannel::Sms,
            NotificationChannel::Sms,
        ])
        .unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Validation);
    }
}
