//! EP-032 notification vocabulary (SPEC-014 terms are vocabulary
//! locked: Notification, Channel, DeliveryReceipt; a new synonym
//! requires an ADR and schema update).
//!
//! Permanent invariants (owner directive, EP-032):
//! - Channel classes come from nexus-domain `NotificationChannel`
//!   (MOBILE_PUSH, DESKTOP, SPEAKER, SMS, EMAIL, PHONE, WATCH, CAR);
//!   they are never redefined here.
//! - Privacy classes come from nexus-domain `Privacy`; they are never
//!   redefined here.
//! - Urgency classes are owned here (LOW/NORMAL/HIGH/CRITICAL) and
//!   match `schemas/notification-envelope.schema.json` exactly.
//! - Typed IDs validate in `new` AND in serde deserialization, so a
//!   malformed wire value can never bypass the contract check (fail
//!   closed, never bypass).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{NotificationError, NotificationErrorCode};

macro_rules! vocabulary_enum {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $text:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            /// Canonical wire string for this class.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = NotificationError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(NotificationError::new(
                        NotificationErrorCode::Vocabulary,
                        format!("unknown {} class: {}", stringify!($name), other),
                        None,
                        None,
                        None,
                        None,
                    )),
                }
            }
        }
    };
}

vocabulary_enum! {
    /// Notification urgency (SPEC-014; schema `notification-envelope`).
    NotificationUrgency {
        Low = "LOW",
        Normal = "NORMAL",
        High = "HIGH",
        Critical = "CRITICAL",
    }
}

vocabulary_enum! {
    /// Delivery lifecycle state of a notification attempt (SPEC-014
    /// receipts; EP-032-owned).
    DeliveryState {
        Pending = "PENDING",
        Sending = "SENDING",
        Delivered = "DELIVERED",
        Failed = "FAILED",
        Expired = "EXPIRED",
        Escalated = "ESCALATED",
    }
}

vocabulary_enum! {
    /// Escalation stage within the configured fallback chain
    /// (SPEC-014; EP-032-owned). Stages advance in order; a delivery
    /// is never duplicated at a later stage.
    EscalationStage {
        Primary = "PRIMARY",
        Secondary = "SECONDARY",
        Tertiary = "TERTIARY",
        Final = "FINAL",
    }
}

macro_rules! typed_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            /// Construct from a validated 1..=128 character identifier.
            pub fn new(value: impl Into<String>) -> Result<Self, NotificationError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(NotificationError::new(
                        NotificationErrorCode::Validation,
                        format!("{} must be 1..=128 characters", stringify!($name)),
                        None,
                        None,
                        None,
                        None,
                    ));
                }
                Ok(Self(value))
            }

            /// The canonical string form.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = NotificationError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(&self.0)
            }
        }

        // Deserialization must run the same contract check as `new`;
        // otherwise a malformed wire value could construct an invalid
        // id through serde (fail closed, never bypass).
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }
    };
}

typed_id!(
    /// Notification identifier (SPEC-014; schema `notification-envelope`).
    NotificationId
);
typed_id!(
    /// Delivery receipt identifier (SPEC-014; EP-032-owned).
    DeliveryReceiptId
);

/// SMS destination telephone number (SPEC-014 behavior 6; EP-032-owned).
///
/// Canonical E.164-ish form: digits with an optional single leading
/// `+` country-code marker; whitespace, dashes, dots, and parens are
/// stripped exactly as the repository's canonical number
/// normalization. This is the provider-neutral notification value
/// object for the SMS channel; channel providers consume it and never
/// invent their own dial-string grammar. GSM 03.40 destination
/// encoding is digit-oriented (BCD), so alphabetic content is
/// rejected before it can reach a provider boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SmsDestination(String);

impl SmsDestination {
    /// Normalize an SMS destination: strip spaces, dashes, dots,
    /// parens; keep a single leading `+`; reject empty/whitespace-only
    /// values and alphabetic content.
    pub fn new(value: impl Into<String>) -> Result<Self, NotificationError> {
        let raw = value.into();
        let normalized: String = raw
            .chars()
            .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '.' | '(' | ')'))
            .collect();
        let digits: String = normalized
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '+')
            .collect();
        if digits.is_empty() || digits.len() > 16 || digits != normalized {
            return Err(NotificationError::validation(format!(
                "invalid SMS destination {raw:?} (must normalize to <=16 digits with optional leading +)"
            )));
        }
        // A '+' is only valid as a single leading country-code marker.
        let plus_count = digits.chars().filter(|c| *c == '+').count();
        if plus_count > 1 || (plus_count == 1 && !digits.starts_with('+')) {
            return Err(NotificationError::validation(format!(
                "invalid SMS destination {raw:?} (malformed '+' placement)"
            )));
        }
        if digits.starts_with('+') && digits.len() < 8 {
            return Err(NotificationError::validation(format!(
                "invalid SMS destination {raw:?} (too short after normalization)"
            )));
        }
        if !digits.starts_with('+') && digits.len() < 7 {
            return Err(NotificationError::validation(format!(
                "invalid SMS destination {raw:?} (too short after normalization)"
            )));
        }
        Ok(Self(digits))
    }

    /// The canonical E.164-ish string form (digits, optional leading `+`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Deserialization must run the same normalization as `new`; otherwise
// a malformed wire destination could reach a provider through serde
// (fail closed, never bypass).
impl<'de> Deserialize<'de> for SmsDestination {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for SmsDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep032_unit_urgency_parses_all_classes() {
        for (text, expected) in [
            ("LOW", NotificationUrgency::Low),
            ("NORMAL", NotificationUrgency::Normal),
            ("HIGH", NotificationUrgency::High),
            ("CRITICAL", NotificationUrgency::Critical),
        ] {
            assert_eq!(text.parse::<NotificationUrgency>().unwrap(), expected);
            assert_eq!(expected.as_str(), text);
        }
    }

    #[test]
    fn ep032_unit_urgency_rejects_unknown() {
        assert!("URGENT".parse::<NotificationUrgency>().is_err());
        assert!("low".parse::<NotificationUrgency>().is_err());
        assert!("".parse::<NotificationUrgency>().is_err());
    }

    #[test]
    fn ep032_unit_urgency_serde_roundtrip_and_rejects_unknown() {
        let json = serde_json::to_string(&NotificationUrgency::Critical).unwrap();
        assert_eq!(json, "\"CRITICAL\"");
        let back: NotificationUrgency = serde_json::from_str(&json).unwrap();
        assert_eq!(back, NotificationUrgency::Critical);
        let res: Result<NotificationUrgency, _> = serde_json::from_str("\"PANIC\"");
        assert!(res.is_err());
    }

    #[test]
    fn ep032_unit_delivery_state_and_escalation_stage_parse() {
        assert_eq!(
            "DELIVERED".parse::<DeliveryState>().unwrap(),
            DeliveryState::Delivered
        );
        assert_eq!(DeliveryState::Escalated.as_str(), "ESCALATED");
        assert!("LOST".parse::<DeliveryState>().is_err());
        assert_eq!(
            "SECONDARY".parse::<EscalationStage>().unwrap(),
            EscalationStage::Secondary
        );
        assert!("NINTH".parse::<EscalationStage>().is_err());
    }

    #[test]
    fn ep032_unit_ids_validate_in_new() {
        let id = NotificationId::new("n-123").unwrap();
        assert_eq!(id.as_str(), "n-123");
        assert!(NotificationId::new("").is_err());
        assert!(NotificationId::new("x".repeat(129)).is_err());
        let rid = DeliveryReceiptId::new("r-1").unwrap();
        assert_eq!(rid.as_str(), "r-1");
        assert!(DeliveryReceiptId::new("").is_err());
    }

    #[test]
    fn ep032_unit_ids_validate_in_serde() {
        // A malformed wire value must fail deserialization, never
        // bypass the contract check.
        let json = serde_json::to_string(&NotificationId::new("n-1").unwrap()).unwrap();
        let back: NotificationId = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "n-1");
        let res: Result<NotificationId, _> = serde_json::from_str("\"\"");
        assert!(res.is_err());
        let res: Result<NotificationId, _> = serde_json::from_str("\"x\"");
        assert!(res.is_ok());
    }

    #[test]
    fn ep032_unit_sms_destination_normalizes_in_new() {
        let e164 = "+15551234567";
        assert_eq!(
            SmsDestination::new("+1 (555) 123-4567").unwrap().as_str(),
            e164
        );
        assert_eq!(
            SmsDestination::new("+1.555.123.4567").unwrap().as_str(),
            e164
        );
        assert_eq!(
            SmsDestination::new("15551234567").unwrap().as_str(),
            "15551234567"
        );
        // Alphabetic content rejected (GSM 03.40 BCD destination).
        assert!(SmsDestination::new("").is_err());
        assert!(SmsDestination::new("   ").is_err());
        assert!(SmsDestination::new("abc").is_err());
        assert!(SmsDestination::new("+12").is_err());
        assert!(SmsDestination::new("123").is_err());
        assert!(SmsDestination::new("+1555123456712345678").is_err());
        assert!(SmsDestination::new("1+5551234567").is_err());
        assert!(SmsDestination::new("++15551234567").is_err());
        assert!(SmsDestination::new("+1555a1234567").is_err());
    }

    #[test]
    fn ep032_unit_sms_destination_validates_in_serde() {
        // Serde must enforce the exact same invariants as `new`
        // (anti-bypass: a malformed wire destination can never reach
        // a provider).
        let good = serde_json::to_string(&SmsDestination::new("+15551234567").unwrap()).unwrap();
        assert_eq!(good, "\"+15551234567\"");
        let back: SmsDestination = serde_json::from_str(&good).unwrap();
        assert_eq!(back.as_str(), "+15551234567");
        let bad: Result<SmsDestination, _> = serde_json::from_str("\"not-a-number\"");
        assert!(bad.is_err());
        let bad: Result<SmsDestination, _> = serde_json::from_str("\"\"");
        assert!(bad.is_err());
        let bad: Result<SmsDestination, _> = serde_json::from_str("\"+1 (555) 123-4567\"");
        // Normalization still applies on the wire: punctuation is
        // accepted and normalized, alphabetic content is rejected.
        assert!(bad.is_ok());
        assert_eq!(bad.unwrap().as_str(), "+15551234567");
    }

    #[test]
    fn ep032_unit_vocabulary_no_vendor_brand_leaks() {
        // Acceptance obligation: no provider brand in canonical names.
        let all = [
            NotificationUrgency::Low.as_str(),
            DeliveryState::Pending.as_str(),
            EscalationStage::Primary.as_str(),
        ];
        for value in all {
            let lower = value.to_ascii_lowercase();
            for brand in [
                "pushover",
                "twilio",
                "onesignal",
                "firebase",
                "apns",
                "slack",
                "telegram",
            ] {
                assert!(
                    !lower.contains(brand),
                    "canonical class {value} leaks provider brand {brand}"
                );
            }
        }
    }
}
