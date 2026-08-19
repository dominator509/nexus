//! EP-029 social command center contracts (SPEC-015).
//!
//! Provider-neutral contract layer for the social command center:
//! Postiz-isolated connector, direct official APIs, content,
//! community, analytics, approvals, CRM lead handoff, and
//! attribution. Postiz remains an isolated replaceable sidecar;
//! direct official APIs implement strategic gaps through the same
//! provider-neutral contract (SPEC-015 behavior 4).
//!
//! SPEC-015 canonical terms (Campaign, SocialAccount, SocialMessage,
//! LeadHandoff, Attribution, CustomerReference, CEOBrief) are
//! vocabulary locked and owned by nexus-hydra (EP-028); this crate
//! imports them and never redefines them. EP-029 owns the social
//! command center vocabulary and ports.
//!
//! Dependency direction: this crate depends only on nexus-domain,
//! nexus-hydra (both contract crates), and serde/serde_json. Provider
//! implementations never appear here (connectors/postiz and
//! connectors/social-direct own them).

pub mod capability;
pub mod error;
pub mod model;
pub mod policy;
pub mod provider;
pub mod vocabulary;

pub use capability::{SocialCapabilityKind, SocialCapabilityMap};
pub use error::{SocialError, SocialErrorCode};
pub use model::{
    variants_preserve_single_objective, PlatformVariant, PublishApproval, SocialConversation,
    SocialLead, SocialMetric,
};
pub use policy::{class_rank, enforce_social_action_policy, required_approval_class};
pub use provider::{
    DirectPlatformProvider, PostizProvider, SocialProvider, UnboundDirectPlatformProvider,
    UnboundPostizProvider, UnboundSocialProvider,
};
pub use vocabulary::{
    CampaignObjective, PlatformVariantId, PublishApprovalId, SocialActionKind, SocialApprovalState,
    SocialConversationId, SocialConversationState, SocialLeadId, SocialLeadState, SocialMetricId,
    SocialMetricKind, SocialVocabularyError,
};
