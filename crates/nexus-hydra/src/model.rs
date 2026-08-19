//! EP-028 provider-neutral Hydra value objects (SPEC-015).
//!
//! Hydra remains the CRM canonical source; these objects are Nexus-side
//! references and projections. They never duplicate Hydra truth and
//! never become a second CRM (non-goal: duplicating Hydra CDM).

use std::collections::BTreeSet;

use nexus_domain::{BusinessId, CorrelationId, PersonId, TenantId};
use serde::{Deserialize, Serialize};

use crate::error::{HydraError, HydraErrorCode};
use crate::vocabulary::{
    AttributionId, BusinessScope, CampaignId, CampaignState, CeoBriefId, CeoBriefSourceClass,
    CustomerReferenceId, HydraAccessChannel, HydraBindingId, IdentityResolutionClass,
    LeadHandoffId, LeadHandoffState, SocialAccountId, SocialMessageId, SocialMessageState,
};

/// Explicit business-to-Hydra tenant binding (node contract acceptance
/// obligation 3). Every Hydra access is scoped to exactly one business
/// and one tenant through this binding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydraBusinessBinding {
    pub binding_id: HydraBindingId,
    pub tenant_id: TenantId,
    pub business_id: BusinessId,
    /// Authorized access channels. Only authenticated MCP, REST, and
    /// durable events exist (SPEC-015 behavior 2); there is no
    /// direct-database channel.
    pub channels: BTreeSet<HydraAccessChannel>,
    /// True while the binding is active; an inactive binding must fail
    /// closed.
    pub active: bool,
}

impl HydraBusinessBinding {
    pub fn new(
        binding_id: HydraBindingId,
        tenant_id: TenantId,
        business_id: BusinessId,
        channels: BTreeSet<HydraAccessChannel>,
    ) -> Self {
        Self {
            binding_id,
            tenant_id,
            business_id,
            channels,
            active: true,
        }
    }

    pub fn with_channels(mut self, channels: BTreeSet<HydraAccessChannel>) -> Self {
        self.channels = channels;
        self
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn permits(&self, channel: HydraAccessChannel) -> bool {
        self.active && self.channels.contains(&channel)
    }
}

/// Business context carried on every Hydra request (SPEC-015 behavior
/// 3: a single business scope unless explicitly authorized for
/// portfolio-level reads).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessContext {
    pub tenant_id: TenantId,
    pub principal_id: PersonId,
    /// Single-business scope by default; PORTFOLIO only when explicitly
    /// authorized. The scope is explicit so cross-business isolation is
    /// never accidental.
    pub scope: BusinessScope,
    /// The authorized single business, required for
    /// `BusinessScope::SingleBusiness`.
    pub business_id: Option<BusinessId>,
    pub correlation: Option<CorrelationId>,
}

impl BusinessContext {
    pub fn single(tenant_id: TenantId, principal_id: PersonId, business_id: BusinessId) -> Self {
        Self {
            tenant_id,
            principal_id,
            scope: BusinessScope::SingleBusiness,
            business_id: Some(business_id),
            correlation: None,
        }
    }

    pub fn portfolio(tenant_id: TenantId, principal_id: PersonId) -> Self {
        Self {
            tenant_id,
            principal_id,
            scope: BusinessScope::Portfolio,
            business_id: None,
            correlation: None,
        }
    }

    pub fn with_correlation(mut self, correlation: CorrelationId) -> Self {
        self.correlation = Some(correlation);
        self
    }

    /// Validate the scope invariant: a single-business scope must name
    /// exactly one business; a portfolio scope must not name one.
    pub fn validate(&self) -> Result<(), HydraError> {
        match self.scope {
            BusinessScope::SingleBusiness => {
                if self.business_id.is_none() {
                    return Err(HydraError::validation(
                        "single-business scope requires a business_id",
                    ));
                }
            }
            BusinessScope::Portfolio => {
                if self.business_id.is_some() {
                    return Err(HydraError::validation(
                        "portfolio scope must not name a single business",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Nexus-side reference to a Hydra CRM customer (SPEC-015 behavior 1:
/// Hydra remains canonical; Nexus stores references, never duplicated
/// truth). Identity linking is only through deterministic or
/// human-reviewed resolution (behavior 6); an LLM guess never merges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomerReference {
    pub customer_reference_id: CustomerReferenceId,
    pub business_id: BusinessId,
    /// The referenced Hydra person/account id. This is a REFERENCE,
    /// not a copy of Hydra truth.
    pub hydra_person_id: PersonId,
    pub resolution: IdentityResolutionClass,
}

impl CustomerReference {
    pub fn new(
        customer_reference_id: CustomerReferenceId,
        business_id: BusinessId,
        hydra_person_id: PersonId,
        resolution: IdentityResolutionClass,
    ) -> Self {
        Self {
            customer_reference_id,
            business_id,
            hydra_person_id,
            resolution,
        }
    }

    /// A reference is mergeable only through deterministic or
    /// human-reviewed resolution. Automatic LLM-guess merges are a
    /// non-goal and fail closed.
    pub fn mergeable(&self) -> bool {
        matches!(
            self.resolution,
            IdentityResolutionClass::Deterministic | IdentityResolutionClass::HumanReviewed
        )
    }
}

/// Campaign (SPEC-015 vocabulary: leads, campaigns, revenue
/// attribution).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Campaign {
    pub campaign_id: CampaignId,
    pub business_id: BusinessId,
    pub name: String,
    pub state: CampaignState,
}

impl Campaign {
    pub fn new(campaign_id: CampaignId, business_id: BusinessId, name: impl Into<String>) -> Self {
        Self {
            campaign_id,
            business_id,
            name: name.into(),
            state: CampaignState::Draft,
        }
    }
}

/// Social account (SPEC-015 behavior 4/6: Postiz is an isolated sidecar
/// for scheduling and connector breadth; a social identity links to a
/// Hydra person only through deterministic or human-reviewed
/// resolution).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialAccount {
    pub account_id: SocialAccountId,
    pub business_id: BusinessId,
    /// Platform name (platform-native variants per behavior 5). This is
    /// a provider-neutral label, not a vocabulary-locked class.
    pub platform: String,
    /// Optional link to the Hydra person when resolution has been
    /// deterministic or human-reviewed; otherwise UNLINKED.
    pub identity_resolution: IdentityResolutionClass,
    pub hydra_person_id: Option<PersonId>,
}

impl SocialAccount {
    pub fn new(
        account_id: SocialAccountId,
        business_id: BusinessId,
        platform: impl Into<String>,
    ) -> Self {
        Self {
            account_id,
            business_id,
            platform: platform.into(),
            identity_resolution: IdentityResolutionClass::Unlinked,
            hydra_person_id: None,
        }
    }

    pub fn with_link(
        mut self,
        resolution: IdentityResolutionClass,
        hydra_person_id: PersonId,
    ) -> Result<Self, HydraError> {
        if !matches!(
            resolution,
            IdentityResolutionClass::Deterministic | IdentityResolutionClass::HumanReviewed
        ) {
            return Err(HydraError::policy(
                "social identity links require deterministic or human-reviewed resolution",
            ));
        }
        self.identity_resolution = resolution;
        self.hydra_person_id = Some(hydra_person_id);
        Ok(self)
    }
}

/// Social message (SPEC-015 behavior 5: platform-native variants,
/// calendar, approvals, inbox, moderation, analytics, listening,
/// attribution, CRM handoff). APPROVED != PUBLISHED; blind social
/// auto-replies are a non-goal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialMessage {
    pub message_id: SocialMessageId,
    pub account_id: SocialAccountId,
    pub state: SocialMessageState,
    /// Calendar timestamp (RFC3339) when scheduled, if scheduled.
    pub scheduled_at: Option<String>,
    /// Platform-native variant key when present.
    pub variant: Option<String>,
    /// Reference to the message content. Content never becomes a
    /// domain contract (free-form provider payloads are normalized at
    /// the infrastructure boundary).
    pub content_ref: String,
}

impl SocialMessage {
    pub fn new(
        message_id: SocialMessageId,
        account_id: SocialAccountId,
        content_ref: impl Into<String>,
    ) -> Self {
        Self {
            message_id,
            account_id,
            state: SocialMessageState::Draft,
            scheduled_at: None,
            variant: None,
            content_ref: content_ref.into(),
        }
    }

    pub fn request_approval(&mut self) -> Result<(), HydraError> {
        match self.state {
            SocialMessageState::Draft | SocialMessageState::Scheduled => {
                self.state = SocialMessageState::PendingApproval;
                Ok(())
            }
            _ => Err(HydraError::new(
                HydraErrorCode::Conflict,
                "only draft or scheduled messages may request approval",
                None,
                None,
                None,
                None,
            )),
        }
    }

    pub fn approve(&mut self) -> Result<(), HydraError> {
        match self.state {
            SocialMessageState::PendingApproval => {
                self.state = SocialMessageState::Approved;
                Ok(())
            }
            _ => Err(HydraError::new(
                HydraErrorCode::Conflict,
                "only a pending-approval message may be approved",
                None,
                None,
                None,
                None,
            )),
        }
    }

    pub fn publish(&mut self) -> Result<(), HydraError> {
        match self.state {
            SocialMessageState::Approved => {
                self.state = SocialMessageState::Published;
                Ok(())
            }
            _ => Err(HydraError::new(
                HydraErrorCode::Conflict,
                "only an approved message may be published (APPROVED != PUBLISHED)",
                None,
                None,
                None,
                None,
            )),
        }
    }
}

/// Lead handoff (SPEC-015 vocabulary; CRM handoff).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeadHandoff {
    pub handoff_id: LeadHandoffId,
    pub business_id: BusinessId,
    pub customer_reference_id: CustomerReferenceId,
    pub state: LeadHandoffState,
    pub correlation: Option<CorrelationId>,
}

impl LeadHandoff {
    pub fn new(
        handoff_id: LeadHandoffId,
        business_id: BusinessId,
        customer_reference_id: CustomerReferenceId,
    ) -> Self {
        Self {
            handoff_id,
            business_id,
            customer_reference_id,
            state: LeadHandoffState::Pending,
            correlation: None,
        }
    }
}

/// Revenue attribution (SPEC-015 vocabulary; attribution
/// reconciliation is a required test).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attribution {
    pub attribution_id: AttributionId,
    pub business_id: BusinessId,
    pub campaign_id: CampaignId,
    pub customer_reference_id: Option<CustomerReferenceId>,
    /// Canonical revenue amount (minor units as string to avoid float
    /// drift; provider-neutral).
    pub amount_minor: Option<String>,
    pub currency: Option<String>,
}

impl Attribution {
    pub fn new(
        attribution_id: AttributionId,
        business_id: BusinessId,
        campaign_id: CampaignId,
    ) -> Self {
        Self {
            attribution_id,
            business_id,
            campaign_id,
            customer_reference_id: None,
            amount_minor: None,
            currency: None,
        }
    }
}

/// CEO brief source with provenance and data freshness (SPEC-015
/// behavior 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CeoBriefSource {
    pub source_class: CeoBriefSourceClass,
    /// Source reference (e.g. binding/capability/projection id).
    pub source_ref: String,
    /// RFC3339 timestamp of the source observation (data freshness).
    pub observed_at: String,
}

impl CeoBriefSource {
    pub fn new(
        source_class: CeoBriefSourceClass,
        source_ref: impl Into<String>,
        observed_at: impl Into<String>,
    ) -> Self {
        Self {
            source_class,
            source_ref: source_ref.into(),
            observed_at: observed_at.into(),
        }
    }
}

/// CEO brief (SPEC-015 behavior 7: combines permitted CRM, social,
/// communications, finance, and operational sources with provenance
/// and data freshness; permission-filtered).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CeoBrief {
    pub brief_id: CeoBriefId,
    pub business_id: BusinessId,
    /// RFC3339 timestamp when the brief was generated.
    pub generated_at: String,
    pub sources: Vec<CeoBriefSource>,
}

impl CeoBrief {
    pub fn new(
        brief_id: CeoBriefId,
        business_id: BusinessId,
        generated_at: impl Into<String>,
    ) -> Self {
        Self {
            brief_id,
            business_id,
            generated_at: generated_at.into(),
            sources: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: CeoBriefSource) -> Self {
        self.sources.push(source);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{BusinessId, PersonId, TenantId};
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

    fn binding() -> HydraBusinessBinding {
        HydraBusinessBinding::new(
            HydraBindingId::new("binding-1").unwrap(),
            tenant(),
            business(),
            BTreeSet::from([HydraAccessChannel::REST, HydraAccessChannel::DurableEvent]),
        )
    }

    #[test]
    fn ep028_unit_binding_explicit_tenant_and_business() {
        let b = binding();
        assert_eq!(b.tenant_id, tenant());
        assert_eq!(b.business_id, business());
        assert!(b.active);
        assert!(b.permits(HydraAccessChannel::REST));
        assert!(b.permits(HydraAccessChannel::DurableEvent));
        assert!(!b.permits(HydraAccessChannel::MCP));
    }

    #[test]
    fn ep028_unit_binding_inactive_fails_closed() {
        let mut b = binding();
        b.deactivate();
        assert!(!b.permits(HydraAccessChannel::REST));
        assert!(!b.permits(HydraAccessChannel::MCP));
    }

    #[test]
    fn ep028_unit_business_context_single_requires_business() {
        let ctx = BusinessContext::single(tenant(), person(), business());
        assert!(ctx.validate().is_ok());
        let bad = BusinessContext {
            tenant_id: tenant(),
            principal_id: person(),
            scope: BusinessScope::SingleBusiness,
            business_id: None,
            correlation: None,
        };
        assert!(bad.validate().is_err());
        let portfolio = BusinessContext::portfolio(tenant(), person());
        assert!(portfolio.validate().is_ok());
        let bad_portfolio = BusinessContext {
            tenant_id: tenant(),
            principal_id: person(),
            scope: BusinessScope::Portfolio,
            business_id: Some(business()),
            correlation: None,
        };
        assert!(bad_portfolio.validate().is_err());
    }

    #[test]
    fn ep028_unit_customer_reference_is_reference_not_truth() {
        let c = CustomerReference::new(
            CustomerReferenceId::new("cust-1").unwrap(),
            business(),
            person(),
            IdentityResolutionClass::Deterministic,
        );
        // Reference carries the Hydra person id, never a copied CRM
        // record, and is mergeable only via owned resolution classes.
        assert_eq!(c.hydra_person_id, person());
        assert!(c.mergeable());
        let unlinked = CustomerReference::new(
            CustomerReferenceId::new("cust-2").unwrap(),
            business(),
            person(),
            IdentityResolutionClass::Unlinked,
        );
        assert!(!unlinked.mergeable());
    }

    #[test]
    fn ep028_unit_social_identity_link_requires_owned_resolution() {
        let acct = SocialAccount::new(SocialAccountId::new("acct-1").unwrap(), business(), "x");
        assert!(acct
            .clone()
            .with_link(IdentityResolutionClass::HumanReviewed, person())
            .is_ok());
        // LLM-guess merges are a non-goal and must fail closed.
        let res = acct.with_link(IdentityResolutionClass::Unlinked, person());
        assert!(res.is_err());
    }

    #[test]
    fn ep028_unit_social_message_approval_ladder() {
        let mut msg = SocialMessage::new(
            SocialMessageId::new("msg-1").unwrap(),
            SocialAccountId::new("acct-1").unwrap(),
            "ref-1",
        );
        msg.request_approval().unwrap();
        assert_eq!(msg.state, SocialMessageState::PendingApproval);
        // Publish without approval must fail closed.
        assert!(msg.publish().is_err());
        msg.approve().unwrap();
        assert_eq!(msg.state, SocialMessageState::Approved);
        msg.publish().unwrap();
        assert_eq!(msg.state, SocialMessageState::Published);
        // Re-publish is a conflict.
        assert!(msg.publish().is_err());
    }

    #[test]
    fn ep028_unit_objects_roundtrip_serde() {
        let b = binding();
        let json = serde_json::to_string(&b).unwrap();
        let back: HydraBusinessBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(back, b);

        let brief = CeoBrief::new(
            CeoBriefId::new("brief-1").unwrap(),
            business(),
            "2026-08-19T00:00:00Z",
        )
        .with_source(CeoBriefSource::new(
            CeoBriefSourceClass::Finance,
            "finance-projection-1",
            "2026-08-19T00:00:00Z",
        ));
        let json = serde_json::to_string(&brief).unwrap();
        let back: CeoBrief = serde_json::from_str(&json).unwrap();
        assert_eq!(back, brief);
        assert_eq!(back.sources.len(), 1);
        assert_eq!(back.sources[0].source_class, CeoBriefSourceClass::Finance);
    }
}
