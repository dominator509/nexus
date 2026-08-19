//! EP-028 canonical Hydra business-control vocabulary (SPEC-015 terms
//! are vocabulary locked: HydraBinding, BusinessContext,
//! CustomerReference, Campaign, SocialAccount, SocialMessage,
//! LeadHandoff, Attribution, CEOBrief; a new synonym requires an ADR
//! and schema update).
//!
//! Permanent invariants (SPEC-015):
//! - Hydra remains the CRM canonical source; Nexus stores references
//!   and cross-domain projections, never duplicated truth.
//! - Nexus accesses Hydra ONLY through authenticated MCP, REST, and
//!   durable events; there is no direct-database access channel.
//! - Business agents receive a single business scope unless
//!   explicitly authorized for portfolio-level reads.
//! - A social identity is linked to a Hydra person only through
//!   deterministic or human-reviewed identity resolution (never an
//!   automatic LLM guess).
//! - Paid-ad budget changes and public crisis responses require human
//!   approval.
//! - CEO briefs combine permitted CRM, social, communications,
//!   finance, and operational sources with provenance and data
//!   freshness.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{HydraError, HydraErrorCode};

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, HydraError> {
                let value = value.into();
                if value.is_empty() || value.len() > 128 {
                    return Err(HydraError::new(
                        HydraErrorCode::Validation,
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

typed_id!(HydraBindingId);
typed_id!(CustomerReferenceId);
typed_id!(CampaignId);
typed_id!(SocialAccountId);
typed_id!(SocialMessageId);
typed_id!(LeadHandoffId);
typed_id!(AttributionId);
typed_id!(CeoBriefId);
typed_id!(HydraActionId);

/// Error returned when a vocabulary string is not a known canonical
/// class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydraVocabularyError(pub String);

impl fmt::Display for HydraVocabularyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown canonical hydra class: {}", self.0)
    }
}

impl std::error::Error for HydraVocabularyError {}

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
            type Err = HydraVocabularyError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(HydraVocabularyError(other.to_string())),
                }
            }
        }

        impl TryFrom<&str> for $name {
            type Error = HydraVocabularyError;
            fn try_from(s: &str) -> Result<Self, Self::Error> {
                s.parse()
            }
        }
    };
}

vocabulary_enum! {
    /// Authorized Nexus-to-Hydra access channel (SPEC-015 behavior 2:
    /// authenticated MCP, REST, and durable events only). There is NO
    /// direct-database variant; direct Hydra database access is
    /// forbidden by the spec.
    HydraAccessChannel {
        MCP = "MCP",
        REST = "REST",
        DurableEvent = "DURABLE_EVENT",
    }
}

vocabulary_enum! {
    /// Business scope granted to an agent (SPEC-015 behavior 3: a
    /// single business scope unless explicitly authorized for
    /// portfolio-level reads).
    BusinessScope {
        SingleBusiness = "SINGLE_BUSINESS",
        Portfolio = "PORTFOLIO",
    }
}

vocabulary_enum! {
    /// Social identity resolution class (SPEC-015 behavior 6: linked
    /// to a Hydra person only through deterministic or human-reviewed
    /// resolution; automatic LLM-guess merge is a non-goal).
    IdentityResolutionClass {
        Deterministic = "DETERMINISTIC",
        HumanReviewed = "HUMAN_REVIEWED",
        Unlinked = "UNLINKED",
    }
}

vocabulary_enum! {
    /// Hydra action kind (SPEC-015 vocabulary; paid-ad budget changes
    /// and public crisis responses REQUIRE human approval).
    HydraActionKind {
        ReadContext = "READ_CONTEXT",
        ProposeUpdate = "PROPOSE_UPDATE",
        ExecuteUpdate = "EXECUTE_UPDATE",
        PaidAdBudgetChange = "PAID_AD_BUDGET_CHANGE",
        PublicCrisisResponse = "PUBLIC_CRISIS_RESPONSE",
        SocialMessagePublish = "SOCIAL_MESSAGE_PUBLISH",
        LeadHandoff = "LEAD_HANDOFF",
    }
}

vocabulary_enum! {
    /// Hydra action state ladder (PROPOSED != EXECUTED; the fallback
    /// is read-only context and proposal generation until execution
    /// capabilities advertise certified availability).
    HydraActionState {
        Proposed = "PROPOSED",
        Submitted = "SUBMITTED",
        Executed = "EXECUTED",
        Failed = "FAILED",
        Cancelled = "CANCELLED",
    }
}

vocabulary_enum! {
    /// Hydra capability kind (SPEC-015 required tests: capability and
    /// event contract; cross-business isolation; social publish;
    /// lead handoff; attribution reconciliation; CEO brief).
    HydraCapabilityKind {
        ReadContext = "READ_CONTEXT",
        ProposeUpdate = "PROPOSE_UPDATE",
        ExecuteUpdate = "EXECUTE_UPDATE",
        ConsumeEvents = "CONSUME_EVENTS",
        SocialPublish = "SOCIAL_PUBLISH",
        AttributionReconcile = "ATTRIBUTION_RECONCILE",
        CeoBrief = "CEO_BRIEF",
    }
}

vocabulary_enum! {
    /// Social message state ladder (SPEC-015 behavior 5: platform
    /// variants, calendar, approvals, inbox, moderation, analytics,
    /// listening, attribution, CRM handoff; APPROVED != PUBLISHED -
    /// blind social auto-replies are a non-goal).
    SocialMessageState {
        Draft = "DRAFT",
        Scheduled = "SCHEDULED",
        PendingApproval = "PENDING_APPROVAL",
        Approved = "APPROVED",
        Published = "PUBLISHED",
        Failed = "FAILED",
        Cancelled = "CANCELLED",
        Archived = "ARCHIVED",
    }
}

vocabulary_enum! {
    /// Campaign state (SPEC-015 vocabulary: leads, campaigns, revenue
    /// attribution).
    CampaignState {
        Draft = "DRAFT",
        Active = "ACTIVE",
        Paused = "PAUSED",
        Completed = "COMPLETED",
        Cancelled = "CANCELLED",
    }
}

vocabulary_enum! {
    /// CEO brief source class (SPEC-015 behavior 7: permitted CRM,
    /// social, communications, finance, and operational sources).
    CeoBriefSourceClass {
        Crm = "CRM",
        Social = "SOCIAL",
        Communications = "COMMUNICATIONS",
        Finance = "FINANCE",
        Operational = "OPERATIONAL",
    }
}

vocabulary_enum! {
    /// Lead handoff state (SPEC-015 vocabulary; CRM handoff).
    LeadHandoffState {
        Pending = "PENDING",
        InProgress = "IN_PROGRESS",
        HandedOff = "HANDED_OFF",
        Failed = "FAILED",
        Cancelled = "CANCELLED",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ep028_unit_typed_ids_validate_and_reject() {
        let id = HydraBindingId::new("hydra-binding-1").unwrap();
        assert_eq!(id.as_str(), "hydra-binding-1");
        assert!(HydraBindingId::new("").is_err());
        assert!(HydraBindingId::new("x".repeat(129)).is_err());
        assert!(CustomerReferenceId::new("c").is_ok());
        assert!(CampaignId::new("c").is_ok());
        assert!(SocialAccountId::new("s").is_ok());
        assert!(SocialMessageId::new("s").is_ok());
        assert!(LeadHandoffId::new("l").is_ok());
        assert!(AttributionId::new("a").is_ok());
        assert!(CeoBriefId::new("b").is_ok());
        assert!(HydraActionId::new("a").is_ok());
    }

    #[test]
    fn ep028_unit_typed_ids_serde_cannot_bypass_validation() {
        // Deserialization must run the same checks as `new`.
        let json = "\"\"";
        let res: Result<HydraBindingId, _> = serde_json::from_str(json);
        assert!(res.is_err());
        let json = "\"valid-id\"";
        let id: HydraBindingId = serde_json::from_str(json).unwrap();
        assert_eq!(id.as_str(), "valid-id");
    }

    #[test]
    fn ep028_unit_access_channels_have_no_direct_database_variant() {
        // SPEC-015 behavior 2: authenticated MCP/REST/durable events
        // only. The absence of a DIRECT_DATABASE variant is structural.
        for ch in [
            HydraAccessChannel::MCP,
            HydraAccessChannel::REST,
            HydraAccessChannel::DurableEvent,
        ] {
            assert!(matches!(
                ch,
                HydraAccessChannel::MCP
                    | HydraAccessChannel::REST
                    | HydraAccessChannel::DurableEvent
            ));
        }
        assert_eq!(HydraAccessChannel::MCP.as_str(), "MCP");
        assert_eq!(HydraAccessChannel::REST.as_str(), "REST");
        assert_eq!(HydraAccessChannel::DurableEvent.as_str(), "DURABLE_EVENT");
        // A fabricated direct-database channel must be rejected.
        assert!("DIRECT_DATABASE".parse::<HydraAccessChannel>().is_err());
    }

    #[test]
    fn ep028_unit_vocabulary_wire_spelling_locked() {
        assert_eq!(BusinessScope::SingleBusiness.as_str(), "SINGLE_BUSINESS");
        assert_eq!(BusinessScope::Portfolio.as_str(), "PORTFOLIO");
        assert_eq!(
            IdentityResolutionClass::HumanReviewed.as_str(),
            "HUMAN_REVIEWED"
        );
        assert_eq!(
            HydraActionKind::PaidAdBudgetChange.as_str(),
            "PAID_AD_BUDGET_CHANGE"
        );
        assert_eq!(HydraActionState::Proposed.as_str(), "PROPOSED");
        assert_eq!(HydraActionState::Executed.as_str(), "EXECUTED");
        assert_eq!(
            SocialMessageState::PendingApproval.as_str(),
            "PENDING_APPROVAL"
        );
        assert_eq!(SocialMessageState::Published.as_str(), "PUBLISHED");
        assert_eq!(CeoBriefSourceClass::Operational.as_str(), "OPERATIONAL");
        let json = serde_json::to_string(&HydraCapabilityKind::ReadContext).unwrap();
        assert_eq!(json, "\"READ_CONTEXT\"");
    }

    #[test]
    fn ep028_unit_vocabulary_rejects_unknown() {
        assert_eq!(
            "FABRICATED".parse::<BusinessScope>(),
            Err(HydraVocabularyError("FABRICATED".to_string()))
        );
        let res: Result<IdentityResolutionClass, _> = serde_json::from_str("\"LLM_GUESS\"");
        assert!(res.is_err());
        let res: Result<SocialMessageState, _> = serde_json::from_str("\"AUTO_REPLIED\"");
        assert!(res.is_err());
    }

    #[test]
    fn ep028_unit_vocabulary_roundtrips_serde() {
        let scope = BusinessScope::Portfolio;
        let json = serde_json::to_string(&scope).unwrap();
        let back: BusinessScope = serde_json::from_str(&json).unwrap();
        assert_eq!(back, BusinessScope::Portfolio);
    }
}
