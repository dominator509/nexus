//! EP-029 provider ports (node contract public interfaces).
//!
//! Provider-neutral, versioned, and fail-closed: an unbound provider
//! returns Unavailable and never fabricates social state. Provider
//! implementations live in connectors/postiz and connectors/social-direct
//! (M2+); M1 owns the ports. Postiz is an isolated replaceable sidecar
//! (SPEC-015 behavior 4); direct official APIs implement strategic
//! gaps through the same provider-neutral contract.

use nexus_domain::{BusinessId, TenantId};
use nexus_hydra::{CampaignId, SocialMessage, SocialMessageId};

use crate::capability::SocialCapabilityMap;
use crate::error::SocialError;
use crate::model::{
    PlatformVariant, PublishApproval, SocialConversation, SocialLead, SocialMetric,
};
use crate::vocabulary::SocialActionKind;

/// Publish a platform-native variant after its approval gate passes.
///
/// The caller MUST have passed the policy gate (policy module); the
/// provider enforces the approval again (defense in depth). A variant
/// is published only through a certified account; unbound providers
/// fail closed.
pub trait SocialProvider {
    /// The capabilities this provider actually advertises. Unbound
    /// and uncertified providers advertise nothing (fail closed).
    fn capabilities(&self) -> SocialCapabilityMap;

    /// Publish an approved platform-native variant.
    fn publish_variant(
        &self,
        variant: &PlatformVariant,
        approval: &PublishApproval,
    ) -> Result<SocialMessageId, SocialError>;

    /// List conversations for an account (inbox / moderation).
    fn list_conversations(
        &self,
        tenant_id: &TenantId,
        business_id: &BusinessId,
    ) -> Result<Vec<SocialConversation>, SocialError>;

    /// Reply to a conversation under governance. The caller must have
    /// passed the REPLY approval class gate; blind auto-replies are a
    /// non-goal.
    fn reply(
        &self,
        conversation: &SocialConversation,
        approval: &PublishApproval,
        content_ref: &str,
    ) -> Result<SocialMessageId, SocialError>;

    /// Read analytics metrics for an account, optionally attributed
    /// to a campaign.
    fn list_metrics(
        &self,
        tenant_id: &TenantId,
        business_id: &BusinessId,
        campaign_id: Option<&CampaignId>,
    ) -> Result<Vec<SocialMetric>, SocialError>;

    /// List leads (CRM handoff source).
    fn list_leads(
        &self,
        tenant_id: &TenantId,
        business_id: &BusinessId,
    ) -> Result<Vec<SocialLead>, SocialError>;

    /// Execute a governed action kind (spend change, crisis
    /// statement) after its approval gate passes.
    fn execute_governed(
        &self,
        kind: SocialActionKind,
        approval: &PublishApproval,
        request_ref: &str,
    ) -> Result<(), SocialError>;
}

/// Postiz sidecar provider (SPEC-015 behavior 4: Postiz is an
/// isolated AGPL sidecar for scheduling and connector breadth). The
/// sidecar is replaceable: every operation goes through the same
/// provider-neutral SocialProvider contract.
pub trait PostizProvider: SocialProvider {
    /// Schedule a message through the sidecar calendar.
    fn schedule(
        &self,
        message: &SocialMessage,
        scheduled_at: &str,
    ) -> Result<SocialMessageId, SocialError>;
}

/// Direct official API provider (SPEC-015 behavior 4: direct official
/// APIs implement strategic gaps). Replaceable through the same
/// provider-neutral contract.
pub trait DirectPlatformProvider: SocialProvider {}

/// Fail-closed unbound provider. Every operation returns Unavailable;
/// it never fabricates conversations, metrics, leads, or publish
/// results.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundSocialProvider;

impl SocialProvider for UnboundSocialProvider {
    fn capabilities(&self) -> SocialCapabilityMap {
        SocialCapabilityMap::new()
    }

    fn publish_variant(
        &self,
        _variant: &PlatformVariant,
        _approval: &PublishApproval,
    ) -> Result<SocialMessageId, SocialError> {
        Err(SocialError::unavailable("no social provider bound"))
    }

    fn list_conversations(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
    ) -> Result<Vec<SocialConversation>, SocialError> {
        Err(SocialError::unavailable("no social provider bound"))
    }

    fn reply(
        &self,
        _conversation: &SocialConversation,
        _approval: &PublishApproval,
        _content_ref: &str,
    ) -> Result<SocialMessageId, SocialError> {
        Err(SocialError::unavailable("no social provider bound"))
    }

    fn list_metrics(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
        _campaign_id: Option<&CampaignId>,
    ) -> Result<Vec<SocialMetric>, SocialError> {
        Err(SocialError::unavailable("no social provider bound"))
    }

    fn list_leads(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
    ) -> Result<Vec<SocialLead>, SocialError> {
        Err(SocialError::unavailable("no social provider bound"))
    }

    fn execute_governed(
        &self,
        _kind: SocialActionKind,
        _approval: &PublishApproval,
        _request_ref: &str,
    ) -> Result<(), SocialError> {
        Err(SocialError::unavailable("no social provider bound"))
    }
}

/// Fail-closed unbound Postiz sidecar provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundPostizProvider;

impl SocialProvider for UnboundPostizProvider {
    fn capabilities(&self) -> SocialCapabilityMap {
        SocialCapabilityMap::new()
    }

    fn publish_variant(
        &self,
        _variant: &PlatformVariant,
        _approval: &PublishApproval,
    ) -> Result<SocialMessageId, SocialError> {
        Err(SocialError::unavailable("no Postiz provider bound"))
    }

    fn list_conversations(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
    ) -> Result<Vec<SocialConversation>, SocialError> {
        Err(SocialError::unavailable("no Postiz provider bound"))
    }

    fn reply(
        &self,
        _conversation: &SocialConversation,
        _approval: &PublishApproval,
        _content_ref: &str,
    ) -> Result<SocialMessageId, SocialError> {
        Err(SocialError::unavailable("no Postiz provider bound"))
    }

    fn list_metrics(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
        _campaign_id: Option<&CampaignId>,
    ) -> Result<Vec<SocialMetric>, SocialError> {
        Err(SocialError::unavailable("no Postiz provider bound"))
    }

    fn list_leads(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
    ) -> Result<Vec<SocialLead>, SocialError> {
        Err(SocialError::unavailable("no Postiz provider bound"))
    }

    fn execute_governed(
        &self,
        _kind: SocialActionKind,
        _approval: &PublishApproval,
        _request_ref: &str,
    ) -> Result<(), SocialError> {
        Err(SocialError::unavailable("no Postiz provider bound"))
    }
}

impl PostizProvider for UnboundPostizProvider {
    fn schedule(
        &self,
        _message: &SocialMessage,
        _scheduled_at: &str,
    ) -> Result<SocialMessageId, SocialError> {
        Err(SocialError::unavailable("no Postiz provider bound"))
    }
}

/// Fail-closed unbound direct platform provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundDirectPlatformProvider;

impl SocialProvider for UnboundDirectPlatformProvider {
    fn capabilities(&self) -> SocialCapabilityMap {
        SocialCapabilityMap::new()
    }

    fn publish_variant(
        &self,
        _variant: &PlatformVariant,
        _approval: &PublishApproval,
    ) -> Result<SocialMessageId, SocialError> {
        Err(SocialError::unavailable(
            "no direct platform provider bound",
        ))
    }

    fn list_conversations(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
    ) -> Result<Vec<SocialConversation>, SocialError> {
        Err(SocialError::unavailable(
            "no direct platform provider bound",
        ))
    }

    fn reply(
        &self,
        _conversation: &SocialConversation,
        _approval: &PublishApproval,
        _content_ref: &str,
    ) -> Result<SocialMessageId, SocialError> {
        Err(SocialError::unavailable(
            "no direct platform provider bound",
        ))
    }

    fn list_metrics(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
        _campaign_id: Option<&CampaignId>,
    ) -> Result<Vec<SocialMetric>, SocialError> {
        Err(SocialError::unavailable(
            "no direct platform provider bound",
        ))
    }

    fn list_leads(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
    ) -> Result<Vec<SocialLead>, SocialError> {
        Err(SocialError::unavailable(
            "no direct platform provider bound",
        ))
    }

    fn execute_governed(
        &self,
        _kind: SocialActionKind,
        _approval: &PublishApproval,
        _request_ref: &str,
    ) -> Result<(), SocialError> {
        Err(SocialError::unavailable(
            "no direct platform provider bound",
        ))
    }
}

impl DirectPlatformProvider for UnboundDirectPlatformProvider {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SocialErrorCode;
    use crate::model::PublishApproval;
    use crate::vocabulary::{PublishApprovalId, SocialConversationId, SocialLeadId};
    use nexus_domain::PersonId;
    use nexus_hydra::{Campaign, IdentityResolutionClass, SocialAccount};
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
            nexus_hydra::SocialAccountId::new("acct-1").unwrap(),
            "ref://content-1",
        )
    }

    fn approval() -> PublishApproval {
        PublishApproval::new(
            PublishApprovalId::new("ap-1").unwrap(),
            tenant(),
            business(),
            SocialActionKind::Publish,
            message().message_id,
        )
    }

    #[test]
    fn ep029_unit_unbound_provider_fails_closed() {
        let provider = UnboundSocialProvider;
        assert!(provider.capabilities().is_empty());
        let variant = PlatformVariant::new(
            crate::vocabulary::PlatformVariantId::new("v-1").unwrap(),
            campaign().campaign_id,
            "linkedin",
            crate::vocabulary::CampaignObjective::Leads,
            "ref://a",
            message().message_id,
        );
        let err = provider.publish_variant(&variant, &approval()).unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Unavailable);
        let err = provider
            .list_conversations(&tenant(), &business())
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Unavailable);
    }

    #[test]
    fn ep029_unit_unbound_postiz_fails_closed() {
        let provider = UnboundPostizProvider;
        assert!(provider.capabilities().is_empty());
        let err = provider
            .schedule(&message(), "2026-08-20T00:00:00Z")
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Unavailable);
        let err = provider
            .list_metrics(&tenant(), &business(), None)
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Unavailable);
    }

    #[test]
    fn ep029_unit_unbound_direct_platform_fails_closed() {
        let provider = UnboundDirectPlatformProvider;
        assert!(provider.capabilities().is_empty());
        let conv = SocialConversation::new(
            SocialConversationId::new("conv-1").unwrap(),
            nexus_hydra::SocialAccountId::new("acct-1").unwrap(),
            business(),
            "instagram",
            "thread-1",
        );
        let err = provider
            .reply(&conv, &approval(), "ref://reply")
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Unavailable);
        let err = provider
            .execute_governed(SocialActionKind::SpendChange, &approval(), "ref://budget-1")
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Unavailable);
    }

    #[test]
    fn ep029_unit_social_lead_links_to_hydra_person() {
        // A lead links to a Hydra person only through deterministic or
        // human-reviewed resolution; the person id is a REFERENCE.
        let lead = SocialLead::new(
            SocialLeadId::new("lead-1").unwrap(),
            SocialConversationId::new("conv-1").unwrap(),
            business(),
        );
        let linked = lead
            .with_link(IdentityResolutionClass::HumanReviewed, person())
            .unwrap();
        assert_eq!(linked.hydra_person_id, Some(person()));
        let _ = SocialAccount::new(
            nexus_hydra::SocialAccountId::new("acct-1").unwrap(),
            business(),
            "x",
        );
    }
}
