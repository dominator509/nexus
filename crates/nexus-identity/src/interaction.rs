//! Interaction context (SPEC-001 requirement 5, EP-003).
//!
//! Every interface entry creates an `InteractionContext` carrying the
//! authenticated principal, device, channel, room, presence evidence,
//! privacy classification, correlation, and objective references.

use nexus_domain::{CorrelationId, DeviceId, NexusId, ObjectiveId, TaskId};
use serde::{Deserialize, Serialize};

use crate::presence::PresenceEvidence;
use crate::principal::Principal;
use crate::privacy::PrivacyContext;

/// The authenticated, tenant-scoped context of one interaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InteractionContext {
    /// Authenticated principal.
    pub principal: Principal,
    /// Device that originated the interaction, when known.
    pub device_id: Option<DeviceId>,
    /// Free-form channel label (voice, chat, api, ...).
    pub channel: Option<String>,
    /// Room reference, when the interaction happens in a physical room.
    pub room_id: Option<NexusId>,
    /// Presence evidence observed during this interaction.
    pub presence_evidence: Vec<PresenceEvidence>,
    /// Privacy classification for this interaction.
    pub privacy: PrivacyContext,
    /// Correlation identifier for the request chain.
    pub correlation_id: CorrelationId,
    /// Objective references currently in scope.
    pub objective_ids: Vec<ObjectiveId>,
    /// Task reference, when the interaction is inside a task.
    pub task_id: Option<TaskId>,
}

impl InteractionContext {
    /// Construct an interaction context.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        principal: Principal,
        device_id: Option<DeviceId>,
        channel: Option<String>,
        room_id: Option<NexusId>,
        presence_evidence: Vec<PresenceEvidence>,
        privacy: PrivacyContext,
        correlation_id: CorrelationId,
        objective_ids: Vec<ObjectiveId>,
        task_id: Option<TaskId>,
    ) -> Self {
        Self {
            principal,
            device_id,
            channel,
            room_id,
            presence_evidence,
            privacy,
            correlation_id,
            objective_ids,
            task_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{NexusId, PrincipalType, Privacy, TenantId};

    const VALID_ID: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6071";
    const TENANT: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6072";
    const CORR: &str = "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6073";

    #[test]
    fn ep003_unit_interaction_context_constructs() {
        let principal = Principal::new(
            NexusId::new(VALID_ID).unwrap(),
            PrincipalType::Human,
            TenantId::new(TENANT).unwrap(),
        );
        let ctx = InteractionContext::new(
            principal,
            None,
            Some("voice".to_string()),
            None,
            vec![],
            PrivacyContext::new(Privacy::Personal, false),
            CorrelationId::new(CORR).unwrap(),
            vec![],
            None,
        );
        assert_eq!(ctx.channel.as_deref(), Some("voice"));
        assert_eq!(ctx.principal.principal_type(), PrincipalType::Human);
    }

    #[test]
    fn ep003_unit_interaction_context_serde_roundtrip() {
        let principal = Principal::new(
            NexusId::new(VALID_ID).unwrap(),
            PrincipalType::Service,
            TenantId::new(TENANT).unwrap(),
        );
        let ctx = InteractionContext::new(
            principal,
            None,
            None,
            None,
            vec![],
            PrivacyContext::new(Privacy::Household, true),
            CorrelationId::new(CORR).unwrap(),
            vec![ObjectiveId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6074").unwrap()],
            None,
        );
        let json = serde_json::to_string(&ctx).unwrap();
        let back: InteractionContext = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ctx);
        assert_eq!(back.objective_ids.len(), 1);
    }
}
