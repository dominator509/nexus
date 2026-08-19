//! EP-028 Hydra business-control contracts (SPEC-015).
//!
//! Provider-neutral contract layer for the authenticated Nexus-to-Hydra
//! capability, context, action, event, identity, and business binding
//! seam. Hydra remains the CRM canonical source; this crate defines
//! references, projections, capabilities, actions, and event consumers
//! that later provider/runtime milestones (connectors/hydra) implement.
//!
//! Dependency direction: this crate depends only on nexus-domain
//! (typed ids + canonical vocabulary) and serde. Provider
//! implementations never appear here.

pub mod action;
pub mod capability;
pub mod context;
pub mod error;
pub mod events;
pub mod model;
pub mod provider;
pub mod vocabulary;

pub use action::{
    enforce_hydra_action_policy, hydra_action_governed, requires_human_approval,
    HydraActionRequest, HydraActionSink,
};
pub use capability::HydraCapabilityMap;
pub use context::HydraContextProjection;
pub use error::{HydraError, HydraErrorCode};
pub use events::{HydraEventConsumer, HydraEventEnvelope};
pub use model::{
    Attribution, BusinessContext, Campaign, CeoBrief, CeoBriefSource, CustomerReference,
    HydraBusinessBinding, LeadHandoff, SocialAccount, SocialMessage,
};
pub use provider::{HydraActionResult, HydraProvider, UnboundHydraProvider};
pub use vocabulary::{
    BusinessScope, CampaignId, CampaignState, CeoBriefId, CeoBriefSourceClass, CustomerReferenceId,
    HydraAccessChannel, HydraActionId, HydraActionKind, HydraActionState, HydraBindingId,
    HydraCapabilityKind, IdentityResolutionClass, LeadHandoffId, LeadHandoffState, SocialAccountId,
    SocialMessageId, SocialMessageState,
};
