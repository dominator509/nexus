//! EP-029 canonical social command center vocabulary (SPEC-015).
//!
//! SPEC-015 canonical terms (HydraBinding, BusinessContext,
//! CustomerReference, Campaign, SocialAccount, SocialMessage,
//! LeadHandoff, Attribution, CEOBrief) are vocabulary locked and owned
//! by nexus-hydra (EP-028); this crate imports them rather than
//! redefining them. EP-029 owns the social-command-center vocabulary:
//! platform variants, conversations, leads, metrics, approvals, and
//! campaign objectives.
//!
//! Permanent invariants (SPEC-015):
//! - Postiz is an isolated AGPL sidecar for scheduling and connector
//!   breadth; direct official APIs implement strategic gaps.
//! - Social content supports platform-native variants, calendar,
//!   approvals, inbox, moderation, analytics, listening, attribution,
//!   and CRM handoff.
//! - Platform-native content variants preserve ONE campaign objective.
//! - Publishing, replies, spend, and crisis statements use SEPARATE
//!   approval classes.
//! - A social identity links to a Hydra person only through
//!   deterministic or human-reviewed resolution (never an automatic
//!   LLM guess).
//! - Social leads link to Hydra; analytics preserve attribution.
//! - Blind social auto-replies are a non-goal.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{SocialError, SocialErrorCode};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, SocialError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(SocialError::new(
                        SocialErrorCode::Validation,
                        concat!(stringify!($name), " must be 1..=128 characters"),
                        None,
                        None,
                        None,
                        None,
                    ));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        // Deserialization must run the same contract check as `new`;
        // otherwise a malformed wire value could construct an invalid
        // id through serde (fail closed, never bypass).
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::new(raw).map_err(serde::de::Error::custom)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

typed_id!(PlatformVariantId);
typed_id!(SocialConversationId);
typed_id!(SocialLeadId);
typed_id!(SocialMetricId);
typed_id!(PublishApprovalId);

/// Error returned when a vocabulary string is not a known canonical
/// class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocialVocabularyError(pub String);

impl fmt::Display for SocialVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown canonical social class: {}", self.0)
    }
}

impl std::error::Error for SocialVocabularyError {}

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
            type Err = SocialVocabularyError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(SocialVocabularyError(other.to_string())),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = SocialVocabularyError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

vocabulary_enum! {
    /// Social action kind (SPEC-015 behavior 5: publishing, replies,
    /// spend, and crisis statements use SEPARATE approval classes;
    /// behavior 8: paid-ad budget changes and public crisis responses
    /// require human approval).
    SocialActionKind {
        Publish = "PUBLISH",
        Reply = "REPLY",
        SpendChange = "SPEND_CHANGE",
        CrisisStatement = "CRISIS_STATEMENT",
    }
}

vocabulary_enum! {
    /// Conversation state (inbox, moderation; blind auto-replies are a
    /// non-goal).
    SocialConversationState {
        Open = "OPEN",
        PendingModeration = "PENDING_MODERATION",
        Archived = "ARCHIVED",
        Closed = "CLOSED",
    }
}

vocabulary_enum! {
    /// Social lead state (CRM lead handoff; a lead links to a Hydra
    /// person only through deterministic or human-reviewed
    /// resolution).
    SocialLeadState {
        New = "NEW",
        Qualified = "QUALIFIED",
        HandedOff = "HANDED_OFF",
        Failed = "FAILED",
        Cancelled = "CANCELLED",
    }
}

vocabulary_enum! {
    /// Social metric kind (analytics; attribution is preserved by
    /// linking metrics to campaigns).
    SocialMetricKind {
        Impressions = "IMPRESSIONS",
        Reach = "REACH",
        Engagement = "ENGAGEMENT",
        Clicks = "CLICKS",
        Conversions = "CONVERSIONS",
        Followers = "FOLLOWERS",
    }
}

vocabulary_enum! {
    /// Campaign objective (SPEC-015 behavior 5: platform-native
    /// content variants preserve ONE campaign objective).
    CampaignObjective {
        Awareness = "AWARENESS",
        Engagement = "ENGAGEMENT",
        Traffic = "TRAFFIC",
        Leads = "LEADS",
        Sales = "SALES",
        AppInstalls = "APP_INSTALLS",
    }
}

vocabulary_enum! {
    /// Publish approval state (separate approval classes per
    /// SPEC-015 behavior 5; APPROVED != PUBLISHED).
    SocialApprovalState {
        Pending = "PENDING",
        Granted = "GRANTED",
        Denied = "DENIED",
        Revoked = "REVOKED",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep029_unit_typed_ids_validate_and_reject() {
        let id = PlatformVariantId::new("variant-1").unwrap();
        assert_eq!(id.as_str(), "variant-1");
        assert!(PlatformVariantId::new("").is_err());
        assert!(PlatformVariantId::new("x".repeat(129)).is_err());
        assert!(SocialConversationId::new("c").is_ok());
        assert!(SocialLeadId::new("l").is_ok());
        assert!(SocialMetricId::new("m").is_ok());
        assert!(PublishApprovalId::new("p").is_ok());
    }

    #[test]
    fn ep029_unit_typed_ids_serde_cannot_bypass_validation() {
        let json = "\"\"";
        let res: Result<PlatformVariantId, _> = serde_json::from_str(json);
        assert!(res.is_err());
        let json = "\"valid-id\"";
        let id: PlatformVariantId = serde_json::from_str(json).unwrap();
        assert_eq!(id.as_str(), "valid-id");
    }

    #[test]
    fn ep029_unit_vocabulary_wire_spelling_locked() {
        assert_eq!(SocialActionKind::Publish.as_str(), "PUBLISH");
        assert_eq!(SocialActionKind::Reply.as_str(), "REPLY");
        assert_eq!(SocialActionKind::SpendChange.as_str(), "SPEND_CHANGE");
        assert_eq!(
            SocialActionKind::CrisisStatement.as_str(),
            "CRISIS_STATEMENT"
        );
        assert_eq!(
            SocialConversationState::PendingModeration.as_str(),
            "PENDING_MODERATION"
        );
        assert_eq!(SocialLeadState::HandedOff.as_str(), "HANDED_OFF");
        assert_eq!(SocialMetricKind::Conversions.as_str(), "CONVERSIONS");
        assert_eq!(CampaignObjective::AppInstalls.as_str(), "APP_INSTALLS");
        assert_eq!(SocialApprovalState::Pending.as_str(), "PENDING");
        let json = serde_json::to_string(&SocialActionKind::CrisisStatement).unwrap();
        assert_eq!(json, "\"CRISIS_STATEMENT\"");
    }

    #[test]
    fn ep029_unit_vocabulary_rejects_unknown() {
        assert_eq!(
            "FABRICATED".parse::<SocialActionKind>(),
            Err(SocialVocabularyError("FABRICATED".to_string()))
        );
        let res: Result<CampaignObjective, _> = serde_json::from_str("\"LLM_GUESS\"");
        assert!(res.is_err());
        let res: Result<SocialConversationState, _> = serde_json::from_str("\"AUTO_REPLIED\"");
        assert!(res.is_err());
    }

    #[test]
    fn ep029_unit_vocabulary_roundtrips_serde() {
        let kind = SocialActionKind::SpendChange;
        let json = serde_json::to_string(&kind).unwrap();
        let back: SocialActionKind = serde_json::from_str(&json).unwrap();
        assert_eq!(back, SocialActionKind::SpendChange);
    }

    #[test]
    fn ep029_unit_action_kinds_are_separate_and_locked() {
        // Publishing, replies, spend, and crisis statements must use
        // SEPARATE approval classes (SPEC-015 behavior 5). The wire
        // spelling is locked and each kind is distinct.
        let kinds = [
            SocialActionKind::Publish,
            SocialActionKind::Reply,
            SocialActionKind::SpendChange,
            SocialActionKind::CrisisStatement,
        ];
        let mut spelled: Vec<&str> = kinds.iter().map(|k| k.as_str()).collect();
        spelled.sort_unstable();
        spelled.dedup();
        assert_eq!(spelled.len(), 4, "four distinct action kinds");
    }
}
