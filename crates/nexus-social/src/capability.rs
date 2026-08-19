//! EP-029 social capability advertisement (SPEC-015; reality rule).
//!
//! A provider advertises only capabilities it actually holds.
//! Unbound/uncertified providers advertise nothing (fail closed), and
//! an unadvertised capability is UNAVAILABLE. Unknown provider
//! capability kinds are skipped at the infrastructure boundary and
//! never widen the contract vocabulary.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::vocabulary::SocialVocabularyError;

/// Social capability kind (SPEC-015 behavior 5: platform-native
/// variants, calendar, approvals, inbox, moderation, analytics,
/// listening, attribution, CRM handoff).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SocialCapabilityKind {
    /// Draft and schedule platform-native variants (calendar).
    DraftAndSchedule,
    /// Submit content for approval.
    SubmitForApproval,
    /// Publish an approved message through a certified account.
    Publish,
    /// Read the inbox / conversations (community, moderation).
    ReadConversations,
    /// Reply under governance (never blind auto-replies).
    Reply,
    /// Read analytics metrics.
    ReadMetrics,
    /// Read listening signals.
    Listen,
    /// CRM lead handoff to Hydra.
    LeadHandoff,
    /// Attribution reconciliation.
    AttributionReconcile,
}

impl SocialCapabilityKind {
    /// Canonical wire string for this class.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DraftAndSchedule => "DRAFT_AND_SCHEDULE",
            Self::SubmitForApproval => "SUBMIT_FOR_APPROVAL",
            Self::Publish => "PUBLISH",
            Self::ReadConversations => "READ_CONVERSATIONS",
            Self::Reply => "REPLY",
            Self::ReadMetrics => "READ_METRICS",
            Self::Listen => "LISTEN",
            Self::LeadHandoff => "LEAD_HANDOFF",
            Self::AttributionReconcile => "ATTRIBUTION_RECONCILE",
        }
    }
}

impl std::fmt::Display for SocialCapabilityKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SocialCapabilityKind {
    type Err = SocialVocabularyError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DRAFT_AND_SCHEDULE" => Ok(Self::DraftAndSchedule),
            "SUBMIT_FOR_APPROVAL" => Ok(Self::SubmitForApproval),
            "PUBLISH" => Ok(Self::Publish),
            "READ_CONVERSATIONS" => Ok(Self::ReadConversations),
            "REPLY" => Ok(Self::Reply),
            "READ_METRICS" => Ok(Self::ReadMetrics),
            "LISTEN" => Ok(Self::Listen),
            "LEAD_HANDOFF" => Ok(Self::LeadHandoff),
            "ATTRIBUTION_RECONCILE" => Ok(Self::AttributionReconcile),
            other => Err(SocialVocabularyError(other.to_string())),
        }
    }
}

/// Fail-closed capability map: empty by default, advertises nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialCapabilityMap {
    kinds: BTreeSet<SocialCapabilityKind>,
}

impl SocialCapabilityMap {
    pub fn new() -> Self {
        Self {
            kinds: BTreeSet::new(),
        }
    }

    pub fn insert(&mut self, kind: SocialCapabilityKind) {
        self.kinds.insert(kind);
    }

    pub fn contains(&self, kind: SocialCapabilityKind) -> bool {
        self.kinds.contains(&kind)
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    pub fn kinds(&self) -> impl Iterator<Item = SocialCapabilityKind> + '_ {
        self.kinds.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep029_unit_capability_map_fails_closed() {
        let map = SocialCapabilityMap::new();
        assert!(map.is_empty());
        assert!(!map.contains(SocialCapabilityKind::Publish));
        let mut map = map;
        map.insert(SocialCapabilityKind::ReadMetrics);
        assert!(map.contains(SocialCapabilityKind::ReadMetrics));
        assert!(!map.contains(SocialCapabilityKind::Publish));
    }

    #[test]
    fn ep029_unit_capability_kind_wire_spelling_locked() {
        assert_eq!(
            SocialCapabilityKind::DraftAndSchedule.as_str(),
            "DRAFT_AND_SCHEDULE"
        );
        assert_eq!(SocialCapabilityKind::Publish.as_str(), "PUBLISH");
        assert_eq!(
            SocialCapabilityKind::AttributionReconcile.as_str(),
            "ATTRIBUTION_RECONCILE"
        );
        assert_eq!(
            "FABRICATED".parse::<SocialCapabilityKind>(),
            Err(SocialVocabularyError("FABRICATED".to_string()))
        );
    }

    #[test]
    fn ep029_unit_capability_map_serde_roundtrip() {
        let mut map = SocialCapabilityMap::new();
        map.insert(SocialCapabilityKind::Reply);
        map.insert(SocialCapabilityKind::Listen);
        let json = serde_json::to_string(&map).unwrap();
        let back: SocialCapabilityMap = serde_json::from_str(&json).unwrap();
        assert_eq!(back, map);
    }
}
