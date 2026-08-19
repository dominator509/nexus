//! EP-029 provider-neutral social command center value objects
//! (SPEC-015).
//!
//! SPEC-015 canonical terms (Campaign, SocialAccount, SocialMessage,
//! LeadHandoff, Attribution, CustomerReference) are vocabulary locked
//! and owned by nexus-hydra; this crate imports them and composes
//! EP-029-owned objects around them. It never redefines a locked term.

use nexus_domain::{BusinessId, PersonId, TenantId};
use nexus_hydra::{CampaignId, IdentityResolutionClass, SocialAccountId, SocialMessageId};
use serde::{Deserialize, Serialize};

use crate::error::{SocialError, SocialErrorCode};
use crate::vocabulary::{
    CampaignObjective, PlatformVariantId, PublishApprovalId, SocialActionKind, SocialApprovalState,
    SocialConversationId, SocialConversationState, SocialLeadId, SocialLeadState, SocialMetricId,
    SocialMetricKind,
};

/// Platform-native content variant (SPEC-015 behavior 5). Every
/// variant belongs to exactly one campaign and preserves that
/// campaign's single objective. `content_ref` references content;
/// free-form provider payloads never become domain contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformVariant {
    pub variant_id: PlatformVariantId,
    pub campaign_id: CampaignId,
    /// Provider-neutral platform label (e.g. "linkedin", "instagram").
    pub platform: String,
    /// The campaign objective this variant preserves. All variants of
    /// one campaign must carry the SAME objective (invariant).
    pub objective: CampaignObjective,
    /// Reference to the platform-native content. Content itself never
    /// becomes a domain contract.
    pub content_ref: String,
    /// Calendar RFC3339 timestamp when the variant is scheduled, if
    /// scheduled.
    pub scheduled_at: Option<String>,
    /// The underlying social message this variant renders.
    pub message_id: SocialMessageId,
}

impl PlatformVariant {
    pub fn new(
        variant_id: PlatformVariantId,
        campaign_id: CampaignId,
        platform: impl Into<String>,
        objective: CampaignObjective,
        content_ref: impl Into<String>,
        message_id: SocialMessageId,
    ) -> Self {
        Self {
            variant_id,
            campaign_id,
            platform: platform.into(),
            objective,
            content_ref: content_ref.into(),
            scheduled_at: None,
            message_id,
        }
    }

    pub fn with_scheduled_at(mut self, scheduled_at: impl Into<String>) -> Self {
        self.scheduled_at = Some(scheduled_at.into());
        self
    }
}

/// Validate the single-objective invariant across a campaign's
/// variants (SPEC-015 behavior 5: platform-native content variants
/// preserve ONE campaign objective). A mixed-objective set fails
/// closed.
pub fn variants_preserve_single_objective(variants: &[PlatformVariant]) -> Result<(), SocialError> {
    let mut seen: Option<CampaignObjective> = None;
    let mut campaign: Option<CampaignId> = None;
    for v in variants {
        match campaign {
            None => campaign = Some(v.campaign_id.clone()),
            Some(c) if c != v.campaign_id => {
                return Err(SocialError::validation(
                    "variants belong to different campaigns",
                ));
            }
            _ => {}
        }
        match seen {
            None => seen = Some(v.objective),
            Some(o) if o != v.objective => {
                return Err(SocialError::new(
                    SocialErrorCode::Validation,
                    "variants of one campaign must preserve a single objective",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// Social conversation (inbox, moderation; SPEC-015 behavior 5).
/// Blind social auto-replies are a non-goal: replies require an
/// approval of the REPLY class before any provider mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialConversation {
    pub conversation_id: SocialConversationId,
    pub account_id: SocialAccountId,
    pub business_id: BusinessId,
    /// Provider-neutral platform label.
    pub platform: String,
    pub state: SocialConversationState,
    /// Reference to the thread (provider-side id or durable ref).
    pub thread_ref: String,
    /// RFC3339 timestamp of the last observed activity.
    pub last_activity_at: Option<String>,
    /// Participant references (provider-side user ids/refs).
    pub participants: Vec<String>,
}

impl SocialConversation {
    pub fn new(
        conversation_id: SocialConversationId,
        account_id: SocialAccountId,
        business_id: BusinessId,
        platform: impl Into<String>,
        thread_ref: impl Into<String>,
    ) -> Self {
        Self {
            conversation_id,
            account_id,
            business_id,
            platform: platform.into(),
            state: SocialConversationState::Open,
            thread_ref: thread_ref.into(),
            last_activity_at: None,
            participants: Vec::new(),
        }
    }

    pub fn with_last_activity_at(mut self, at: impl Into<String>) -> Self {
        self.last_activity_at = Some(at.into());
        self
    }
}

/// Social lead (SPEC-015 behavior 5/6: CRM handoff; a lead links to a
/// Hydra person only through deterministic or human-reviewed
/// resolution). `hydra_person_id` is a REFERENCE, never a copy of
/// Hydra truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialLead {
    pub lead_id: SocialLeadId,
    pub conversation_id: SocialConversationId,
    pub business_id: BusinessId,
    pub state: SocialLeadState,
    pub resolution: IdentityResolutionClass,
    pub hydra_person_id: Option<PersonId>,
    /// Optional campaign attribution for this lead.
    pub campaign_id: Option<CampaignId>,
}

impl SocialLead {
    pub fn new(
        lead_id: SocialLeadId,
        conversation_id: SocialConversationId,
        business_id: BusinessId,
    ) -> Self {
        Self {
            lead_id,
            conversation_id,
            business_id,
            state: SocialLeadState::New,
            resolution: IdentityResolutionClass::Unlinked,
            hydra_person_id: None,
            campaign_id: None,
        }
    }

    /// Link to a Hydra person. Only deterministic or human-reviewed
    /// resolution is permitted (SPEC-015 behavior 6); an automatic
    /// LLM-guess merge is a non-goal and fails closed.
    pub fn with_link(
        mut self,
        resolution: IdentityResolutionClass,
        hydra_person_id: PersonId,
    ) -> Result<Self, SocialError> {
        if !matches!(
            resolution,
            IdentityResolutionClass::Deterministic | IdentityResolutionClass::HumanReviewed
        ) {
            return Err(SocialError::policy(
                "social lead links require deterministic or human-reviewed resolution",
            ));
        }
        self.resolution = resolution;
        self.hydra_person_id = Some(hydra_person_id);
        Ok(self)
    }
}

/// Social metric (analytics; SPEC-015 required test: attribution
/// reconciliation). Attribution is preserved by linking a metric to a
/// campaign. Values are u64 counts (no float drift).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialMetric {
    pub metric_id: SocialMetricId,
    pub account_id: SocialAccountId,
    pub business_id: BusinessId,
    pub kind: SocialMetricKind,
    pub value: u64,
    /// RFC3339 observation timestamp.
    pub observed_at: String,
    /// Attribution: the campaign this metric is attributed to.
    pub campaign_id: Option<CampaignId>,
}

impl SocialMetric {
    pub fn new(
        metric_id: SocialMetricId,
        account_id: SocialAccountId,
        business_id: BusinessId,
        kind: SocialMetricKind,
        value: u64,
        observed_at: impl Into<String>,
    ) -> Self {
        Self {
            metric_id,
            account_id,
            business_id,
            kind,
            value,
            observed_at: observed_at.into(),
            campaign_id: None,
        }
    }

    pub fn attributed_to(mut self, campaign_id: CampaignId) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }
}

/// Publish approval (SPEC-015 behavior 5: publishing, replies, spend,
/// and crisis statements use SEPARATE approval classes; behavior 8:
/// paid-ad budget changes and public crisis responses require human
/// approval). APPROVED/GRANTED != PUBLISHED.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishApproval {
    pub approval_id: PublishApprovalId,
    pub tenant_id: TenantId,
    pub business_id: BusinessId,
    pub action_kind: SocialActionKind,
    pub state: SocialApprovalState,
    /// The approved social message (or reply/message being governed).
    pub message_id: SocialMessageId,
    /// The approver principal reference, set when granted.
    pub approved_by: Option<PersonId>,
}

impl PublishApproval {
    pub fn new(
        approval_id: PublishApprovalId,
        tenant_id: TenantId,
        business_id: BusinessId,
        action_kind: SocialActionKind,
        message_id: SocialMessageId,
    ) -> Self {
        Self {
            approval_id,
            tenant_id,
            business_id,
            action_kind,
            state: SocialApprovalState::Pending,
            message_id,
            approved_by: None,
        }
    }

    pub fn grant(&mut self, approver: PersonId) -> Result<(), SocialError> {
        match self.state {
            SocialApprovalState::Pending => {
                self.state = SocialApprovalState::Granted;
                self.approved_by = Some(approver);
                Ok(())
            }
            _ => Err(SocialError::new(
                SocialErrorCode::Conflict,
                "only a pending approval may be granted",
                None,
                None,
                None,
                None,
            )),
        }
    }

    pub fn deny(&mut self) -> Result<(), SocialError> {
        match self.state {
            SocialApprovalState::Pending => {
                self.state = SocialApprovalState::Denied;
                Ok(())
            }
            _ => Err(SocialError::new(
                SocialErrorCode::Conflict,
                "only a pending approval may be denied",
                None,
                None,
                None,
                None,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::PersonId;
    use nexus_hydra::{
        Campaign, CampaignState, CustomerReference, SocialAccount, SocialMessage,
        SocialMessageState,
    };
    use std::str::FromStr;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    fn person() -> PersonId {
        PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap()
    }

    fn business() -> BusinessId {
        BusinessId::from_str("018f0f6f-9c1e-7b6e-8000-000000000003").unwrap()
    }

    fn campaign() -> Campaign {
        Campaign::new(CampaignId::new("campaign-1").unwrap(), business(), "launch")
    }

    fn message() -> SocialMessage {
        SocialMessage::new(
            SocialMessageId::new("msg-1").unwrap(),
            SocialAccountId::new("acct-1").unwrap(),
            "ref://content-1",
        )
    }

    #[test]
    fn ep029_unit_variant_construction_and_serde() {
        let v = PlatformVariant::new(
            PlatformVariantId::new("v-1").unwrap(),
            campaign().campaign_id,
            "linkedin",
            CampaignObjective::Awareness,
            "ref://linkedin-post",
            message().message_id,
        );
        assert_eq!(v.objective, CampaignObjective::Awareness);
        assert_eq!(v.scheduled_at, None);
        let json = serde_json::to_string(&v).unwrap();
        let back: PlatformVariant = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn ep029_unit_variants_preserve_single_objective() {
        let cid = campaign().campaign_id;
        let mid = message().message_id;
        let a = PlatformVariant::new(
            PlatformVariantId::new("v-1").unwrap(),
            cid.clone(),
            "linkedin",
            CampaignObjective::Leads,
            "ref://a",
            mid.clone(),
        );
        let b = PlatformVariant::new(
            PlatformVariantId::new("v-2").unwrap(),
            cid.clone(),
            "instagram",
            CampaignObjective::Leads,
            "ref://b",
            mid.clone(),
        );
        assert!(variants_preserve_single_objective(&[a.clone(), b]).is_ok());

        let c = PlatformVariant::new(
            PlatformVariantId::new("v-3").unwrap(),
            cid.clone(),
            "x",
            CampaignObjective::Awareness,
            "ref://c",
            mid.clone(),
        );
        let err = variants_preserve_single_objective(&[a, c]).unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Validation);
    }

    #[test]
    fn ep029_unit_lead_links_only_deterministic_or_human_reviewed() {
        let lead = SocialLead::new(
            SocialLeadId::new("lead-1").unwrap(),
            SocialConversationId::new("conv-1").unwrap(),
            business(),
        );
        assert_eq!(lead.resolution, IdentityResolutionClass::Unlinked);
        let ok = lead
            .clone()
            .with_link(IdentityResolutionClass::Deterministic, person());
        assert!(ok.is_ok());
        let err = lead
            .with_link(IdentityResolutionClass::Unlinked, person())
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Policy);
    }

    #[test]
    fn ep029_unit_metric_attribution_preserved() {
        let m = SocialMetric::new(
            SocialMetricId::new("m-1").unwrap(),
            SocialAccountId::new("acct-1").unwrap(),
            business(),
            SocialMetricKind::Clicks,
            42,
            "2026-08-19T00:00:00Z",
        )
        .attributed_to(campaign().campaign_id);
        assert!(m.campaign_id.is_some());
        let json = serde_json::to_string(&m).unwrap();
        let back: SocialMetric = serde_json::from_str(&json).unwrap();
        assert_eq!(back.campaign_id, m.campaign_id);
    }

    #[test]
    fn ep029_unit_approval_grant_deny_ladder() {
        let mut ap = PublishApproval::new(
            PublishApprovalId::new("ap-1").unwrap(),
            tenant(),
            business(),
            SocialActionKind::Publish,
            message().message_id,
        );
        assert_eq!(ap.state, SocialApprovalState::Pending);
        ap.grant(person()).unwrap();
        assert_eq!(ap.state, SocialApprovalState::Granted);
        // Granting a granted approval conflicts.
        assert!(ap.grant(person()).is_err());
        // Denying a granted approval conflicts (revocation is explicit).
        assert!(ap.deny().is_err());
    }

    #[test]
    fn ep029_unit_conversation_and_message_state_imports() {
        // Locked SPEC-015 terms come from nexus-hydra unchanged.
        let conv = SocialConversation::new(
            SocialConversationId::new("conv-1").unwrap(),
            SocialAccountId::new("acct-1").unwrap(),
            business(),
            "instagram",
            "provider-thread-1",
        );
        assert_eq!(conv.state, SocialConversationState::Open);
        assert!(SocialMessageState::Approved != SocialMessageState::Published);
        let c = campaign();
        assert_eq!(c.state, CampaignState::Draft);
        let _ = SocialAccount::new(SocialAccountId::new("a").unwrap(), business(), "x");
        let _ = CustomerReference::new(
            nexus_hydra::CustomerReferenceId::new("cr-1").unwrap(),
            business(),
            person(),
            IdentityResolutionClass::Deterministic,
        );
        assert!(conv.participants.is_empty());
    }
}
