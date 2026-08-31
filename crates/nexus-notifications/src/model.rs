//! EP-032 notification model (SPEC-014).
//!
//! The canonical `NotificationEnvelope` mirrors
//! `schemas/notification-envelope.schema.json` field-for-field and is
//! the single wire contract for every channel provider. Delivery
//! policy, privacy routing, escalation policy, and delivery receipts
//! encode acceptance obligations 1-4.
//!
//! Permanent invariants:
//! - The envelope is validated at construction: title <= 160 chars,
//!   summary <= 1000 chars, at least one channel, all classes
//!   canonical, non-empty correlation. A malformed envelope cannot
//!   reach a provider (fail closed).
//! - PrivacyRouting never routes SENSITIVE-or-higher content to
//!   shared-room channels (SPEAKER, CAR).
//! - EscalationPolicy never duplicates a channel across stages.
//! - A DeliveryReceipt always carries the notification id and
//!   correlation; a receipt is the ONLY delivery authority
//!   (SENT != DELIVERED, SPEC-014).

use nexus_domain::{CorrelationId, NotificationChannel, PersonId, Privacy};

use crate::error::{NotificationError, NotificationErrorCode};
use crate::vocabulary::{DeliveryReceiptId, DeliveryState, NotificationId, NotificationUrgency};

/// Canonical notification envelope (schema `notification-envelope`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NotificationEnvelope {
    pub notification_id: NotificationId,
    pub person_id: PersonId,
    pub urgency: NotificationUrgency,
    pub privacy: Privacy,
    pub title: String,
    pub summary: String,
    pub channels: Vec<NotificationChannel>,
    pub expires_at: String,
    pub correlation_id: CorrelationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_ref: Option<String>,
}

impl NotificationEnvelope {
    /// Build an envelope, validating the schema bounds exactly as
    /// `schemas/notification-envelope.schema.json` requires.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        notification_id: NotificationId,
        person_id: PersonId,
        urgency: NotificationUrgency,
        privacy: Privacy,
        title: impl Into<String>,
        summary: impl Into<String>,
        channels: Vec<NotificationChannel>,
        expires_at: impl Into<String>,
        correlation_id: CorrelationId,
        action_ref: Option<String>,
    ) -> Result<Self, NotificationError> {
        let title = title.into();
        let summary = summary.into();
        let expires_at = expires_at.into();
        if title.is_empty() || title.chars().count() > 160 {
            return Err(NotificationError::validation(
                "title must be 1..=160 characters",
            ));
        }
        if summary.is_empty() || summary.chars().count() > 1000 {
            return Err(NotificationError::validation(
                "summary must be 1..=1000 characters",
            ));
        }
        if channels.is_empty() {
            return Err(NotificationError::validation(
                "at least one channel is required",
            ));
        }
        if expires_at.is_empty() {
            return Err(NotificationError::validation(
                "expires_at must be a non-empty RFC3339 timestamp",
            ));
        }
        Ok(Self {
            notification_id,
            person_id,
            urgency,
            privacy,
            title,
            summary,
            channels,
            expires_at,
            correlation_id,
            action_ref,
        })
    }

    /// The person this notification is addressed to.
    pub fn recipient(&self) -> PersonId {
        self.person_id.clone()
    }
}

/// Delivery-time context the policy must evaluate to enforce its
/// declared fields: quiet hours, presence, acknowledgement, and
/// expiry. A default context is fail-closed: quiet hours unknown is
/// not quiet hours, but a REQUIRED presence/acknowledgement that is
/// not proven is DENIED.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryContext {
    /// Current unix time in milliseconds (RFC3339-comparable).
    pub now_ms: u64,
    /// Whether the recipient is inside quiet hours.
    pub in_quiet_hours: bool,
    /// Whether the recipient's presence is proven (required when
    /// `require_presence`).
    pub present: bool,
    /// Whether the notification has been acknowledged (required when
    /// `require_acknowledgement`).
    pub acknowledged: bool,
}

/// Current unix time in milliseconds.
fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Parse the RFC3339 UTC timestamp used by the envelope schema
/// (YYYY-MM-DDTHH:MM:SSZ) into unix milliseconds. Malformed input
/// yields None; callers fail closed on unparsable expiry.
pub(crate) fn rfc3339_utc_to_ms(s: &str) -> Option<u64> {
    // Strict 20-char shape: 2026-08-21T12:00:00Z
    if s.len() != 20 || !s.ends_with('Z') {
        return None;
    }
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' || b[16] != b':' {
        return None;
    }
    let year: i64 = s[0..4].parse().ok()?;
    let month: u32 = s[5..7].parse().ok()?;
    let day: u32 = s[8..10].parse().ok()?;
    let hour: u32 = s[11..13].parse().ok()?;
    let minute: u32 = s[14..16].parse().ok()?;
    let second: u32 = s[17..19].parse().ok()?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=60).contains(&second)
    {
        return None;
    }
    // Days from civil algorithm (Howard Hinnant) - valid for all
    // supported years; leap seconds are clamped to 59.
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = month as i64 + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let secs = days * 86400 + hour as i64 * 3600 + minute as i64 * 60 + second.min(59) as i64;
    Some(secs as u64 * 1000)
}

impl Default for DeliveryContext {
    fn default() -> Self {
        Self {
            now_ms: now_unix_ms(),
            in_quiet_hours: false,
            present: false,
            acknowledged: false,
        }
    }
}

/// Delivery policy: person, urgency, privacy, presence, availability,
/// quiet hours, and acknowledgement determine delivery (acceptance
/// obligation 1).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryPolicy {
    /// Minimum urgency that is allowed to deliver.
    pub min_urgency: NotificationUrgency,
    /// Channels explicitly permitted; empty means "no channel is
    /// permitted" (fail closed), never "everything".
    pub allowed_channels: Vec<NotificationChannel>,
    /// Whether quiet hours suppress delivery (true) or only downgrade
    /// it (false).
    pub quiet_hours_suppress: bool,
    /// Whether an acknowledgement is required before a delivery is
    /// considered complete.
    pub require_acknowledgement: bool,
    /// Whether presence is required for delivery.
    pub require_presence: bool,
}

impl DeliveryPolicy {
    /// Policy gate: urgency must meet the minimum, the channel must be
    /// explicitly permitted, quiet hours must not suppress, and a
    /// required acknowledgement/presence must be proven in the context.
    /// Any unproven requirement is denied (fail closed).
    pub fn allows(
        &self,
        urgency: NotificationUrgency,
        channel: NotificationChannel,
        ctx: &DeliveryContext,
    ) -> bool {
        if urgency < self.min_urgency {
            return false;
        }
        if !self.allowed_channels.contains(&channel) {
            return false;
        }
        if self.quiet_hours_suppress && ctx.in_quiet_hours {
            return false;
        }
        if self.require_acknowledgement && !ctx.acknowledged {
            return false;
        }
        if self.require_presence && !ctx.present {
            return false;
        }
        true
    }

    /// Whether the policy permits ANY channel at this urgency.
    pub fn allows_any(&self, urgency: NotificationUrgency, ctx: &DeliveryContext) -> bool {
        self.allowed_channels
            .iter()
            .any(|c| self.allows(urgency, *c, ctx))
    }
}

/// Privacy routing: sensitive shared-room responses route privately
/// (acceptance obligation 2).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyRouting {
    /// Channels that are considered shared-room / ambient and must
    /// never carry SENSITIVE-or-higher content.
    pub shared_room_channels: Vec<NotificationChannel>,
    /// Channels that are considered private and may carry sensitive
    /// content.
    pub private_channels: Vec<NotificationChannel>,
}

impl PrivacyRouting {
    /// Filter candidate channels for an envelope: SENSITIVE-or-higher
    /// privacy is restricted to private channels; shared-room channels
    /// are removed. Lower privacy keeps the candidate set unchanged.
    pub fn route(
        &self,
        privacy: Privacy,
        candidates: &[NotificationChannel],
    ) -> Vec<NotificationChannel> {
        if is_sensitive_or_higher(privacy) {
            candidates
                .iter()
                .copied()
                .filter(|c| self.private_channels.contains(c))
                .collect()
        } else {
            candidates.to_vec()
        }
    }
}

/// Whether a privacy class is SENSITIVE or higher (SPEC-001 class
/// order: PUBLIC < HOUSEHOLD < PERSONAL < SENSITIVE <
/// BUSINESS_CONFIDENTIAL < SECURITY < SECRET). Implemented with an
/// explicit rank because the canonical vocabulary enum does not derive
/// ordering.
fn is_sensitive_or_higher(privacy: Privacy) -> bool {
    matches!(
        privacy,
        Privacy::Sensitive | Privacy::BusinessConfidential | Privacy::Security | Privacy::Secret
    )
}

/// Escalation policy: failures escalate across configured channels
/// WITHOUT duplication (acceptance obligation 3).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EscalationPolicy {
    /// Ordered fallback chain. A channel appears at most once across
    /// the whole chain.
    pub chain: Vec<NotificationChannel>,
}

impl EscalationPolicy {
    /// Build a chain, rejecting duplicate channels (a delivery is
    /// never duplicated at a later stage).
    pub fn new(chain: Vec<NotificationChannel>) -> Result<Self, NotificationError> {
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
        Ok(Self { chain })
    }

    /// The channel to escalate to after `current`, or None when the
    /// chain is exhausted.
    pub fn next_after(&self, current: NotificationChannel) -> Option<NotificationChannel> {
        let pos = self.chain.iter().position(|c| *c == current)?;
        self.chain.get(pos + 1).copied()
    }

    /// The first channel in the chain.
    pub fn first(&self) -> Option<NotificationChannel> {
        self.chain.first().copied()
    }
}

/// Delivery receipt: the ONLY delivery authority (acceptance
/// obligation 4; SENT != DELIVERED).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryReceipt {
    pub id: DeliveryReceiptId,
    pub notification_id: NotificationId,
    pub channel: NotificationChannel,
    pub state: DeliveryState,
    pub correlation_id: CorrelationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivered_at_ms: Option<u64>,
}

impl DeliveryReceipt {
    pub fn new(
        id: DeliveryReceiptId,
        notification_id: NotificationId,
        channel: NotificationChannel,
        state: DeliveryState,
        correlation_id: CorrelationId,
        provider_ref: Option<String>,
        delivered_at_ms: Option<u64>,
    ) -> Self {
        Self {
            id,
            notification_id,
            channel,
            state,
            correlation_id,
            provider_ref,
            delivered_at_ms,
        }
    }

    /// A receipt proves delivery only in the Delivered state.
    pub fn is_delivered(&self) -> bool {
        self.state == DeliveryState::Delivered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_envelope(
        channels: Vec<NotificationChannel>,
        privacy: Privacy,
    ) -> Result<NotificationEnvelope, NotificationError> {
        NotificationEnvelope::new(
            NotificationId::new("n-1").unwrap(),
            PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            NotificationUrgency::High,
            privacy,
            "Suspicious sign-in",
            "A new device signed in to your account.",
            channels,
            "2026-08-21T12:00:00Z",
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
        )
    }

    #[test]
    fn ep032_unit_envelope_constructs_valid() {
        let env =
            sample_envelope(vec![NotificationChannel::MobilePush], Privacy::Personal).unwrap();
        assert_eq!(
            env.recipient().as_str(),
            "018f0f6f-9c1e-7b6e-8000-000000000001"
        );
        assert_eq!(env.urgency, NotificationUrgency::High);
        assert_eq!(env.channels, vec![NotificationChannel::MobilePush]);
    }

    #[test]
    fn ep032_unit_envelope_rejects_empty_channels() {
        let err = sample_envelope(vec![], Privacy::Personal).unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Validation);
    }

    #[test]
    fn ep032_unit_envelope_rejects_title_summary_bounds() {
        let long_title = "t".repeat(161);
        let err = NotificationEnvelope::new(
            NotificationId::new("n-1").unwrap(),
            PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            NotificationUrgency::Normal,
            Privacy::Personal,
            long_title,
            "summary",
            vec![NotificationChannel::Desktop],
            "2026-08-21T12:00:00Z",
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Validation);

        let long_summary = "s".repeat(1001);
        let err = NotificationEnvelope::new(
            NotificationId::new("n-1").unwrap(),
            PersonId::new("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap(),
            NotificationUrgency::Normal,
            Privacy::Personal,
            "title",
            long_summary,
            vec![NotificationChannel::Desktop],
            "2026-08-21T12:00:00Z",
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
        )
        .unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Validation);
    }

    #[test]
    fn ep032_unit_envelope_serde_matches_schema_field_names() {
        let env = sample_envelope(vec![NotificationChannel::Sms], Privacy::Sensitive).unwrap();
        let json = serde_json::to_value(&env).unwrap();
        let obj = json.as_object().unwrap();
        for required in [
            "notification_id",
            "person_id",
            "urgency",
            "privacy",
            "title",
            "summary",
            "channels",
            "expires_at",
            "correlation_id",
        ] {
            assert!(
                obj.contains_key(required),
                "missing schema field {required}"
            );
        }
        // Deny unknown fields on deserialize (schema additionalProperties: false).
        let mut wire = serde_json::to_value(&env).unwrap();
        wire.as_object_mut()
            .unwrap()
            .insert("bogus".to_string(), serde_json::json!(1));
        let res: Result<NotificationEnvelope, _> = serde_json::from_value(wire);
        assert!(res.is_err());
    }

    #[test]
    fn ep032_unit_privacy_routing_sensitive_never_shared_room() {
        let routing = PrivacyRouting {
            shared_room_channels: vec![NotificationChannel::Speaker, NotificationChannel::Car],
            private_channels: vec![
                NotificationChannel::MobilePush,
                NotificationChannel::Desktop,
                NotificationChannel::Watch,
            ],
        };
        let candidates = vec![
            NotificationChannel::Speaker,
            NotificationChannel::Car,
            NotificationChannel::MobilePush,
            NotificationChannel::Desktop,
        ];
        // Sensitive content: shared-room channels removed.
        let routed = routing.route(Privacy::Sensitive, &candidates);
        assert_eq!(
            routed,
            vec![
                NotificationChannel::MobilePush,
                NotificationChannel::Desktop
            ]
        );
        // Public content: candidate set unchanged.
        let routed = routing.route(Privacy::Public, &candidates);
        assert_eq!(routed, candidates);
    }

    #[test]
    fn ep032_unit_escalation_policy_no_duplicates_and_order() {
        let policy = EscalationPolicy::new(vec![
            NotificationChannel::MobilePush,
            NotificationChannel::Sms,
            NotificationChannel::Phone,
        ])
        .unwrap();
        assert_eq!(policy.first(), Some(NotificationChannel::MobilePush));
        assert_eq!(
            policy.next_after(NotificationChannel::MobilePush),
            Some(NotificationChannel::Sms)
        );
        assert_eq!(policy.next_after(NotificationChannel::Phone), None);
        // Duplicate channel rejected.
        let err = EscalationPolicy::new(vec![
            NotificationChannel::MobilePush,
            NotificationChannel::MobilePush,
        ])
        .unwrap_err();
        assert_eq!(err.code, NotificationErrorCode::Validation);
    }

    #[test]
    fn ep032_unit_delivery_policy_fails_closed() {
        let policy = DeliveryPolicy {
            min_urgency: NotificationUrgency::High,
            allowed_channels: vec![NotificationChannel::MobilePush],
            quiet_hours_suppress: true,
            require_acknowledgement: false,
            require_presence: false,
        };
        let ctx = DeliveryContext {
            now_ms: 1_700_000_000_000,
            in_quiet_hours: false,
            present: true,
            acknowledged: true,
        };
        // Low urgency denied.
        assert!(!policy.allows(
            NotificationUrgency::Low,
            NotificationChannel::MobilePush,
            &ctx
        ));
        // Allowed channel at sufficient urgency passes.
        assert!(policy.allows(
            NotificationUrgency::Critical,
            NotificationChannel::MobilePush,
            &ctx
        ));
        // Channel not on allowlist denied (fail closed).
        assert!(!policy.allows(
            NotificationUrgency::Critical,
            NotificationChannel::Sms,
            &ctx
        ));
        assert!(policy.allows_any(NotificationUrgency::Critical, &ctx));
        assert!(!policy.allows_any(NotificationUrgency::Low, &ctx));
    }

    #[test]
    fn ep032_unit_delivery_policy_enforces_quiet_hours_ack_presence() {
        // AUD-018: the policy's declared fields (quiet hours,
        // acknowledgement, presence) are REAL gates, not decoration.
        let policy = DeliveryPolicy {
            min_urgency: NotificationUrgency::Low,
            allowed_channels: vec![NotificationChannel::MobilePush],
            quiet_hours_suppress: true,
            require_acknowledgement: true,
            require_presence: true,
        };
        let base = DeliveryContext {
            now_ms: 1_700_000_000_000,
            in_quiet_hours: false,
            present: true,
            acknowledged: true,
        };
        assert!(policy.allows(
            NotificationUrgency::High,
            NotificationChannel::MobilePush,
            &base
        ));
        // Quiet hours suppress.
        let quiet = DeliveryContext {
            in_quiet_hours: true,
            ..base.clone()
        };
        assert!(!policy.allows(
            NotificationUrgency::High,
            NotificationChannel::MobilePush,
            &quiet
        ));
        // Missing acknowledgement denied (fail closed).
        let unacked = DeliveryContext {
            acknowledged: false,
            ..base.clone()
        };
        assert!(!policy.allows(
            NotificationUrgency::High,
            NotificationChannel::MobilePush,
            &unacked
        ));
        // Missing presence denied (fail closed).
        let absent = DeliveryContext {
            present: false,
            ..base.clone()
        };
        assert!(!policy.allows(
            NotificationUrgency::High,
            NotificationChannel::MobilePush,
            &absent
        ));
    }

    #[test]
    fn ep032_unit_receipt_is_delivery_authority() {
        let receipt = DeliveryReceipt::new(
            DeliveryReceiptId::new("r-1").unwrap(),
            NotificationId::new("n-1").unwrap(),
            NotificationChannel::Email,
            DeliveryState::Delivered,
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            Some("provider-msg-1".to_string()),
            Some(1_700_000_000_000),
        );
        assert!(receipt.is_delivered());
        assert_eq!(
            receipt.correlation_id.as_str(),
            "018f0f6f-9c1e-7b6e-8000-000000000002"
        );
        let pending = DeliveryReceipt::new(
            DeliveryReceiptId::new("r-2").unwrap(),
            NotificationId::new("n-1").unwrap(),
            NotificationChannel::Email,
            DeliveryState::Pending,
            CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            None,
            None,
        );
        assert!(!pending.is_delivered());
    }
}
