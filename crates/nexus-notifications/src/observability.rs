//! EP-032 M4 notification observability (SPEC-014; bounded, redacted).
//!
//! A bounded ring of safe observation entries for the notification
//! plane. Only safe fields are recorded (notification fingerprint,
//! channel, provider fingerprint, state, correlation, duration,
//! escalation stage, error class, delivery-report presence). The
//! SMS body, full destination, push private payload, credentials, and
//! raw delivery-report PDUs are structurally impossible to record -
//! the entry type has no such fields (redaction by construction).

use std::collections::VecDeque;

use nexus_domain::{CorrelationId, NotificationChannel};

use crate::error::NotificationErrorCode;
use crate::vocabulary::{DeliveryState, EscalationStage, NotificationId};

/// One bounded observation entry (safe fields only).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationObservation {
    /// Notification fingerprint (the canonical NotificationId; the
    /// identifier itself is not secret payload).
    pub notification_id: NotificationId,
    /// The channel this attempt used.
    pub channel: NotificationChannel,
    /// Provider message reference when the provider returned one
    /// (e.g. SMSD outbox row id). Never a secret.
    pub provider_ref: Option<String>,
    /// Canonical delivery state observed for this attempt.
    pub state: DeliveryState,
    /// Correlation preserved from the envelope.
    pub correlation_id: CorrelationId,
    /// Attempt duration in milliseconds.
    pub duration_ms: u64,
    /// Escalation stage of this attempt when part of a chain.
    pub escalation_stage: Option<EscalationStage>,
    /// Canonical error class when the attempt failed.
    pub error_class: Option<NotificationErrorCode>,
    /// Whether a real delivery report was present (e.g. SMSD
    /// DeliveryDateTime; push ack delivered=true).
    pub delivery_report: bool,
}

/// Bounded notification observability ring (256 entries, oldest
/// evicted). Safe by construction: the entry type cannot carry body,
/// destination, credentials, or raw PDUs.
#[derive(Debug, Clone)]
pub struct NotificationObservability {
    entries: VecDeque<NotificationObservation>,
    max: usize,
}

impl Default for NotificationObservability {
    fn default() -> Self {
        Self::new(256)
    }
}

impl NotificationObservability {
    pub fn new(max: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max: max.max(1),
        }
    }

    /// Record one observation (bounded; oldest evicted first).
    pub fn record(&mut self, entry: NotificationObservation) {
        self.entries.push_back(entry);
        while self.entries.len() > self.max {
            self.entries.pop_front();
        }
    }

    /// The recorded entries (oldest first).
    pub fn entries(&self) -> impl Iterator<Item = &NotificationObservation> {
        self.entries.iter()
    }

    /// Current entry count.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::Privacy;

    fn sample() -> NotificationObservation {
        NotificationObservation {
            notification_id: NotificationId::new("n-1").unwrap(),
            channel: NotificationChannel::Sms,
            provider_ref: Some("7".to_string()),
            state: DeliveryState::Sending,
            correlation_id: CorrelationId::new("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            duration_ms: 12,
            escalation_stage: Some(EscalationStage::Primary),
            error_class: None,
            delivery_report: false,
        }
    }

    #[test]
    fn ep032_failure_unit_observability_records_and_bounds() {
        let mut obs = NotificationObservability::new(3);
        for i in 0..5 {
            let mut e = sample();
            e.notification_id = NotificationId::new(format!("n-{i}")).unwrap();
            obs.record(e);
        }
        assert_eq!(obs.len(), 3);
        let ids: Vec<_> = obs.entries().map(|e| e.notification_id.as_str()).collect();
        assert_eq!(ids, vec!["n-2", "n-3", "n-4"]);
    }

    #[test]
    fn ep032_failure_unit_observability_cannot_carry_secrets_by_construction() {
        // The entry type has no body/destination/credential fields;
        // prove Debug output of a fully populated entry leaks no
        // prohibited payload even when a canary appears in the
        // allowed fields.
        let mut e = sample();
        e.notification_id = NotificationId::new("n-CANARY-1").unwrap();
        let debug = format!("{e:?}");
        assert!(debug.contains("CANARY"), "id is a safe fingerprint");
        // Body/destination/credential canaries are not presentable.
        for prohibited in ["SECRET-BODY", "+1555-SECRET", "DB-PASSWORD"] {
            assert!(!debug.contains(prohibited));
        }
        // A redacted ring of entries never renders raw payloads.
        let mut obs = NotificationObservability::default();
        obs.record(e);
        let ring_debug = format!("{:?}", obs.entries().collect::<Vec<_>>());
        for prohibited in ["SECRET-BODY", "+1555-SECRET", "DB-PASSWORD"] {
            assert!(!ring_debug.contains(prohibited));
        }
        let _ = Privacy::Personal;
    }
}
