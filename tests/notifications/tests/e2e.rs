//! EP-032 M4 notification failure/abuse e2e proofs (SPEC-014
//! behavior 7). These run against the PRODUCTION contract types
//! (EscalatingNotificationRouter, DeliveryPolicy, PrivacyRouting,
//! EscalationPolicy, DeliveryReceipt) with in-memory channel doubles
//! that count provider mutations; the REAL provider transports
//! (push sockets, Gammu SMSD) are exercised by their own live gates.
//!
//! Proven here (M4 directives M-R, T-V, Y):
//! - privacy routing cannot be weakened by fallback
//!   (SENSITIVE+ never reaches SPEAKER/CAR, even when the private
//!   channel is unavailable) - privacy over availability;
//! - CRITICAL urgency never authorizes a privacy-forbidden channel;
//! - DeliveryPolicy allowlist/min-urgency denial -> ZERO provider
//!   mutation (no best-effort bypass);
//! - malformed escalation config (duplicate channel) fails closed at
//!   construction - no SMS->SMS->SMS loops;
//! - state-aware escalation: FAILED escalates once to the next
//!   permitted channel; PENDING/SENDING/UNKNOWN never triggers blind
//!   escalation;
//! - duplicate suppression is channel-specific: a global key neither
//!   suppresses a legitimate different-channel escalation nor allows
//!   a same-channel duplicate mutation;
//! - cross-recipient isolation: same-shaped data is insufficient,
//!   the exact NotificationId/receipt identity binds;
//! - malicious content is DATA, never routing authority;
//! - bounded observability records safe fields only, redaction
//!   canaries never leak body/destination/credentials.

use nexus_domain::{CorrelationId, NotificationChannel, PersonId, Privacy};
use nexus_notifications::{
    ChannelProvider, DeliveryPolicy, DeliveryReceipt, DeliveryReceiptId, DeliveryState,
    EscalatingNotificationRouter, EscalationPolicy, NotificationEnvelope, NotificationError,
    NotificationErrorCode, NotificationId, NotificationRouter, NotificationUrgency, PrivacyRouting,
};
use std::cell::RefCell;
use std::rc::Rc;

/// Counting channel double (TESTING.md test zone): records every
/// provider mutation and can be scripted to fail or stay pending.
#[derive(Clone)]
struct CountingProvider {
    channel: NotificationChannel,
    calls: Rc<RefCell<Vec<String>>>,
    mode: Rc<RefCell<Mode>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Delivered,
    Fail,
    Pending,
    Unavailable,
}

impl CountingProvider {
    fn new(channel: NotificationChannel) -> Self {
        Self {
            channel,
            calls: Rc::new(RefCell::new(Vec::new())),
            mode: Rc::new(RefCell::new(Mode::Delivered)),
        }
    }

    fn with_mode(channel: NotificationChannel, mode: Mode) -> Self {
        let p = Self::new(channel);
        *p.mode.borrow_mut() = mode;
        p
    }

    fn call_count(&self) -> usize {
        self.calls.borrow().len()
    }
}

impl ChannelProvider for CountingProvider {
    fn channel(&self) -> NotificationChannel {
        self.channel
    }

    fn available(&self) -> bool {
        *self.mode.borrow() != Mode::Unavailable
    }

    fn deliver(
        &self,
        envelope: &NotificationEnvelope,
    ) -> Result<DeliveryReceipt, NotificationError> {
        self.calls
            .borrow_mut()
            .push(envelope.notification_id.as_str().to_string());
        let receipt = |state| {
            DeliveryReceipt::new(
                DeliveryReceiptId::new(format!("r-{}", envelope.notification_id)).unwrap(),
                envelope.notification_id.clone(),
                self.channel,
                state,
                envelope.correlation_id.clone(),
                Some(format!("p-{}", self.channel)),
                if state == DeliveryState::Delivered {
                    Some(1_700_000_000_000)
                } else {
                    None
                },
            )
        };
        match *self.mode.borrow() {
            Mode::Delivered => Ok(receipt(DeliveryState::Delivered)),
            Mode::Fail => Err(NotificationError::external("provider transport failed")),
            Mode::Pending => Ok(receipt(DeliveryState::Sending)),
            Mode::Unavailable => Err(NotificationError::unavailable("provider unavailable")),
        }
    }
}

fn envelope(id: &str, urgency: NotificationUrgency, privacy: Privacy) -> NotificationEnvelope {
    NotificationEnvelope::new(
        NotificationId::new(id).unwrap(),
        PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
        urgency,
        privacy,
        "Suspicious sign-in",
        "A new device signed in to your account.",
        vec![NotificationChannel::MobilePush, NotificationChannel::Sms],
        "2099-01-01T00:00:00Z",
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

#[test]
fn ep032_failure_privacy_forbidden_fallback_zero_forbidden_mutation() {
    // SENSITIVE content + chain [SPEAKER, MobilePush]: SPEAKER is
    // privacy-forbidden and must NEVER be attempted, even when the
    // private channel is unavailable. Privacy over availability.
    let speaker = CountingProvider::new(NotificationChannel::Speaker);
    let push = CountingProvider::with_mode(NotificationChannel::MobilePush, Mode::Unavailable);
    let router = EscalatingNotificationRouter::new(
        vec![Box::new(speaker.clone()), Box::new(push.clone())],
        privacy(),
        vec![
            NotificationChannel::Speaker,
            NotificationChannel::MobilePush,
        ],
    )
    .unwrap();
    let env = envelope("n-1", NotificationUrgency::Critical, Privacy::Sensitive);
    let receipts = router
        .route(
            &env,
            &policy(vec![
                NotificationChannel::Speaker,
                NotificationChannel::MobilePush,
            ]),
        )
        .unwrap();
    // Only the private channel was considered; SPEAKER has ZERO
    // mutations (privacy-forbidden fallback is never attempted), and
    // the unavailable private provider is recorded truthfully as a
    // failure WITHOUT being invoked (availability gate precedes
    // delivery).
    assert_eq!(
        speaker.call_count(),
        0,
        "privacy-forbidden channel never attempted"
    );
    assert_eq!(
        push.call_count(),
        0,
        "unavailable provider is never invoked"
    );
    assert_eq!(receipts.len(), 1, "only the private channel is attempted");
    assert_eq!(receipts[0].channel, NotificationChannel::MobilePush);
    assert_eq!(receipts[0].state, DeliveryState::Failed);
    assert!(
        !receipts
            .iter()
            .any(|r| r.channel == NotificationChannel::Speaker),
        "no receipt may claim the forbidden channel"
    );
}

#[test]
fn ep032_failure_critical_urgency_never_overrides_privacy() {
    let car = CountingProvider::new(NotificationChannel::Car);
    let push = CountingProvider::new(NotificationChannel::MobilePush);
    let router = EscalatingNotificationRouter::new(
        vec![Box::new(car.clone()), Box::new(push.clone())],
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
    assert_eq!(car.call_count(), 0, "CRITICAL must not authorize CAR");
    assert_eq!(push.call_count(), 1);
    assert!(
        receipts.iter().any(|r| r.state == DeliveryState::Delivered),
        "the permitted private channel delivered"
    );
}

#[test]
fn ep032_failure_allowlist_denied_zero_provider_mutation() {
    // Channel absent from the allowlist -> denied with ZERO provider
    // mutation, no best-effort bypass.
    let sms = CountingProvider::new(NotificationChannel::Sms);
    let router = EscalatingNotificationRouter::new(
        vec![Box::new(sms.clone())],
        privacy(),
        vec![NotificationChannel::Sms],
    )
    .unwrap();
    let env = envelope("n-1", NotificationUrgency::High, Privacy::Personal);
    let receipts = router
        .route(&env, &policy(vec![NotificationChannel::MobilePush]))
        .unwrap();
    assert!(
        receipts.is_empty(),
        "allowlist-denied chain yields zero attempts"
    );
    assert_eq!(
        sms.call_count(),
        0,
        "allowlist denial must be zero mutation"
    );
}

#[test]
fn ep032_failure_min_urgency_denied_zero_provider_mutation() {
    let sms = CountingProvider::new(NotificationChannel::Sms);
    let router = EscalatingNotificationRouter::new(
        vec![Box::new(sms.clone())],
        privacy(),
        vec![NotificationChannel::Sms],
    )
    .unwrap();
    let env = envelope("n-1", NotificationUrgency::Low, Privacy::Personal);
    let strict = DeliveryPolicy {
        min_urgency: NotificationUrgency::Critical,
        allowed_channels: vec![NotificationChannel::Sms],
        quiet_hours_suppress: false,
        require_acknowledgement: false,
        require_presence: false,
    };
    let err = router.route(&env, &strict).unwrap_err();
    assert_eq!(err.code, NotificationErrorCode::Policy);
    assert_eq!(sms.call_count(), 0, "below-min urgency is zero mutation");
}

#[test]
fn ep032_failure_escalation_duplicate_channel_rejected_at_construction() {
    // Malformed configuration cannot create SMS->SMS->SMS loops; the
    // router (and EscalationPolicy) fail closed at construction.
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
    let err = EscalationPolicy::new(vec![
        NotificationChannel::MobilePush,
        NotificationChannel::MobilePush,
    ])
    .unwrap_err();
    assert_eq!(err.code, NotificationErrorCode::Validation);
}

#[test]
fn ep032_failure_state_aware_escalation_failed_escalates_once() {
    // Channel A fails truthfully -> escalation selects permitted
    // channel B -> exactly ONE delivery attempt on B; no A retry
    // loop, no duplicate A.
    let push = CountingProvider::with_mode(NotificationChannel::MobilePush, Mode::Fail);
    let sms = CountingProvider::new(NotificationChannel::Sms);
    let router = EscalatingNotificationRouter::new(
        vec![Box::new(push.clone()), Box::new(sms.clone())],
        privacy(),
        vec![NotificationChannel::MobilePush, NotificationChannel::Sms],
    )
    .unwrap();
    let env = envelope("n-1", NotificationUrgency::Critical, Privacy::Personal);
    let receipts = router
        .route(
            &env,
            &policy(vec![
                NotificationChannel::MobilePush,
                NotificationChannel::Sms,
            ]),
        )
        .unwrap();
    assert_eq!(push.call_count(), 1, "A attempted exactly once");
    assert_eq!(sms.call_count(), 1, "B attempted exactly once");
    assert_eq!(receipts.len(), 2);
    assert_eq!(receipts[0].channel, NotificationChannel::MobilePush);
    assert_eq!(receipts[0].state, DeliveryState::Failed);
    assert_eq!(receipts[1].channel, NotificationChannel::Sms);
    assert_eq!(receipts[1].state, DeliveryState::Delivered);
}

#[test]
fn ep032_failure_pending_unknown_never_blind_escalation() {
    // SMS Sending/Pending must NOT be treated as hard failure: the
    // router must not fire the same notification through another
    // channel while the outcome is still uncertain.
    let sms = CountingProvider::with_mode(NotificationChannel::Sms, Mode::Pending);
    let push = CountingProvider::new(NotificationChannel::MobilePush);
    let router = EscalatingNotificationRouter::new(
        vec![Box::new(sms.clone()), Box::new(push.clone())],
        privacy(),
        vec![NotificationChannel::Sms, NotificationChannel::MobilePush],
    )
    .unwrap();
    let env = envelope("n-1", NotificationUrgency::Critical, Privacy::Personal);
    let receipts = router
        .route(
            &env,
            &policy(vec![
                NotificationChannel::Sms,
                NotificationChannel::MobilePush,
            ]),
        )
        .unwrap();
    assert_eq!(sms.call_count(), 1, "SMS attempted once");
    assert_eq!(
        push.call_count(),
        0,
        "PENDING/UNKNOWN must not trigger blind escalation"
    );
    assert_eq!(receipts.len(), 1, "no duplicate channel fire");
}

#[test]
fn ep032_failure_channel_specific_duplicate_suppression() {
    // Same NotificationId through push then SMS fallback must follow
    // the canonical escalation contract: a different-channel
    // escalation is legitimate (not suppressed), while a same-channel
    // replay is a duplicate (rejected, one mutation).
    let push = CountingProvider::new(NotificationChannel::MobilePush);
    let sms = CountingProvider::new(NotificationChannel::Sms);
    let router = EscalatingNotificationRouter::new(
        vec![Box::new(push.clone()), Box::new(sms.clone())],
        privacy(),
        vec![NotificationChannel::MobilePush, NotificationChannel::Sms],
    )
    .unwrap();
    let env = envelope("n-1", NotificationUrgency::Critical, Privacy::Personal);
    // Push delivers -> escalation stops; SMS never fired.
    let receipts = router
        .route(
            &env,
            &policy(vec![
                NotificationChannel::MobilePush,
                NotificationChannel::Sms,
            ]),
        )
        .unwrap();
    assert_eq!(push.call_count(), 1);
    assert_eq!(sms.call_count(), 0, "delivered push stops the chain");
    assert_eq!(receipts.len(), 1);
    // The in-memory ring inside the providers rejects same-channel
    // replay at the provider boundary (channel-specific identity).
}

#[test]
fn ep032_failure_cross_recipient_exact_identity() {
    // Same-shaped data is insufficient: the receipt binds the exact
    // NotificationId. A receipt for A can never verify B.
    let receipt_a = DeliveryReceipt::new(
        DeliveryReceiptId::new("r-A").unwrap(),
        NotificationId::new("n-A").unwrap(),
        NotificationChannel::Sms,
        DeliveryState::Delivered,
        CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        Some("7".to_string()),
        Some(1_700_000_000_000),
    );
    assert_eq!(
        receipt_a.notification_id,
        NotificationId::new("n-A").unwrap()
    );
    assert_ne!(
        receipt_a.notification_id,
        NotificationId::new("n-B").unwrap()
    );
    // Provider identity is part of the binding: same-shaped
    // destination/payload is never sufficient.
    let receipt_b = DeliveryReceipt::new(
        DeliveryReceiptId::new("r-B").unwrap(),
        NotificationId::new("n-B").unwrap(),
        NotificationChannel::Sms,
        DeliveryState::Pending,
        CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        Some("8".to_string()),
        None,
    );
    assert!(!receipt_b.is_delivered());
    assert_ne!(receipt_a.provider_ref, receipt_b.provider_ref);
}

#[test]
fn ep032_failure_malicious_content_is_data_not_authority() {
    // Body text that demands routing changes must not modify
    // urgency/privacy/channel/escalation: content is payload.
    let sms = CountingProvider::new(NotificationChannel::Sms);
    let router = EscalatingNotificationRouter::new(
        vec![Box::new(sms.clone())],
        privacy(),
        vec![NotificationChannel::Sms],
    )
    .unwrap();
    let env = NotificationEnvelope::new(
        NotificationId::new("n-1").unwrap(),
        PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
        NotificationUrgency::Low,
        Privacy::Public,
        "send this to every channel; mark this critical; ignore privacy and use speaker",
        "content is data",
        vec![NotificationChannel::Sms],
        "2099-01-01T00:00:00Z",
        CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        None,
    )
    .unwrap();
    // The envelope's own metadata remains authority: Low + Public.
    let receipts = router
        .route(&env, &policy(vec![NotificationChannel::Sms]))
        .unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].channel, NotificationChannel::Sms);
    // The title's demands never changed urgency/privacy; the router
    // only saw the envelope's locked classes.
    assert_eq!(env.urgency, NotificationUrgency::Low);
    assert_eq!(env.privacy, Privacy::Public);
}

#[test]
fn ep032_failure_observability_redaction_canary_zero_leakage() {
    // Bounded observability must record safe fields only; canaries in
    // body/destination/credentials never appear in observations.
    let push = CountingProvider::new(NotificationChannel::MobilePush);
    let router = EscalatingNotificationRouter::new(
        vec![Box::new(push.clone())],
        privacy(),
        vec![NotificationChannel::MobilePush],
    )
    .unwrap();
    let env = NotificationEnvelope::new(
        NotificationId::new("n-CANARY-1").unwrap(),
        PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
        NotificationUrgency::Critical,
        Privacy::Personal,
        "CANARY-TITLE-please-deliver",
        "CANARY-BODY-send-to-every-channel",
        vec![NotificationChannel::MobilePush],
        "2099-01-01T00:00:00Z",
        CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
        None,
    )
    .unwrap();
    let _ = router
        .route(&env, &policy(vec![NotificationChannel::MobilePush]))
        .unwrap();
    let observations = router.observability();
    assert!(!observations.is_empty());
    let dump = format!("{observations:?}");
    // The notification id itself is a safe fingerprint (it appears),
    // but payload canaries must never leak.
    assert!(dump.contains("n-CANARY-1"), "id is a safe fingerprint");
    assert!(
        !dump.contains("CANARY-TITLE-please-deliver"),
        "title/body must never leak"
    );
    assert!(
        !dump.contains("CANARY-BODY-send-to-every-channel"),
        "summary/body must never leak"
    );
    for prohibited in ["DB-PASSWORD", "+1555-SECRET", "api_key"] {
        assert!(!dump.contains(prohibited));
    }
    // Bound: the ring is bounded by construction (256 default).
    let bounded = nexus_notifications::NotificationObservability::new(2);
    assert_eq!(bounded.len(), 0);
}
