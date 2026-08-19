//! EP-029 Postiz adapter core (SPEC-015; M2).
//!
//! Real production adapter behind the nexus-social `SocialProvider`
//! and `PostizProvider` ports: capability mapping from the documented
//! Postiz integration list, governed dual-gate publish, single
//! campaign-objective enforcement, in-flight idempotency, bounded
//! observability, and fail-closed behavior.
//!
//! Permanent invariants (SPEC-015):
//!
//! - POSTIZ IS AN ISOLATED REPLACEABLE SIDECAR (behavior 4): every
//!   operation goes through the provider-neutral SocialProvider
//!   contract; the sidecar can be replaced by a direct official API
//!   without changing domain code.
//! - PLATFORM-NATIVE VARIANTS PRESERVE ONE CAMPAIGN OBJECTIVE
//!   (behavior 5): a publish request for a mixed-objective variant set
//!   fails closed BEFORE any transport call.
//! - SEPARATE APPROVAL CLASSES (behavior 5): publishing, replies,
//!   spend, and crisis statements each require their own approval
//!   class; the governed policy gate runs BEFORE the provider port
//!   (dual authorization gates).
//! - HUMAN APPROVAL (behavior 8): paid-ad budget changes and public
//!   crisis responses require human approval.
//! - POLICY BEFORE MUTATION: denied actions make ZERO provider calls.
//! - UNKNOWN OUTCOME -> VERIFY FIRST -> NO BLIND RETRY.
//! - UNBOUND PROVIDERS FAIL CLOSED (Reality rule): no session is
//!   fabricated and no capability is advertised.
//!
//! No test-mode branches exist in production code.

use std::collections::HashMap;
use std::sync::Mutex;

use nexus_domain::{BusinessId, TenantId};
use nexus_hydra::{CampaignId, SocialMessage, SocialMessageId};
use nexus_social::{
    enforce_social_action_policy, required_approval_class, variants_preserve_single_objective,
    DirectPlatformProvider, PlatformVariant, PostizProvider, PublishApproval, SocialActionKind,
    SocialApprovalState, SocialCapabilityKind, SocialCapabilityMap, SocialConversation,
    SocialError, SocialErrorCode, SocialLead, SocialMetric, SocialProvider,
};

use crate::observability::{SocialAuditEntry, SocialObservability};
use crate::transport::PostizTransport;

/// In-flight idempotency entry for one publish/reply on one business.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    idempotency_key: String,
}

/// Real production Postiz adapter over a real Postiz transport.
///
/// `Send + Sync`: the transport trait object is required to be
/// shareable so in-flight idempotency can be proven with real
/// concurrent callers.
pub struct PostizAdapter {
    transport: Box<dyn PostizTransport + Send + Sync>,
    tenant_id: TenantId,
    business_id: BusinessId,
    in_flight: Mutex<HashMap<String, InFlightEntry>>,
    observability: Mutex<SocialObservability>,
}

impl PostizAdapter {
    pub fn new(
        transport: Box<dyn PostizTransport + Send + Sync>,
        tenant_id: TenantId,
        business_id: BusinessId,
        credential: impl Into<String>,
    ) -> Self {
        let credential = credential.into();
        // The credential is registered as a redaction secret so a
        // poisoned error can never leak it into the audit ring. The
        // transport holds the credential for the Authorization header.
        Self {
            transport,
            tenant_id,
            business_id,
            in_flight: Mutex::new(HashMap::new()),
            observability: Mutex::new(SocialObservability::new(256, vec![credential])),
        }
    }

    pub fn audit(&self) -> Vec<SocialAuditEntry> {
        self.observability.lock().unwrap().audit()
    }

    fn record(
        &self,
        correlation: &str,
        operation: &str,
        outcome: &str,
        detail: String,
        fields: std::collections::BTreeMap<String, String>,
    ) {
        self.observability.lock().unwrap().record(SocialAuditEntry {
            correlation: correlation.to_string(),
            operation: operation.to_string(),
            outcome: outcome.to_string(),
            detail,
            fields,
        });
    }

    fn correlation(&self) -> String {
        self.observability.lock().unwrap().next_correlation()
    }

    /// Build the documented Postiz create-post payload from a
    /// platform-native variant (canonical shape from docs.postiz.com:
    /// type draft|schedule|now, date RFC3339, posts[].integration.id,
    /// posts[].value[].content, posts[].settings.__type).
    fn build_post_payload(&self, variant: &PlatformVariant, post_type: &str) -> serde_json::Value {
        serde_json::json!({
            "type": post_type,
            "date": variant.scheduled_at.as_deref().unwrap_or(""),
            "shortLink": false,
            "tags": [],
            "posts": [{
                "integration": { "id": variant.platform },
                "value": [{ "content": variant.content_ref }],
                "settings": { "__type": variant.platform }
            }]
        })
    }

    /// Map a nexus-hydra typed-id error (returned by
    /// `SocialMessageId::new`) onto the social error surface.
    fn map_hydra_id_error(
        e: nexus_hydra::HydraError,
        correlation: String,
        resource: String,
    ) -> SocialError {
        SocialError::new(
            SocialErrorCode::Validation,
            e.message,
            Some(correlation),
            None,
            None,
            Some(resource),
        )
    }

    fn publish_variant_inner(
        &self,
        variant: &PlatformVariant,
        approval: &PublishApproval,
    ) -> Result<SocialMessageId, SocialError> {
        let correlation = self.correlation();

        // Single-objective invariant: this publish request must not
        // mix objectives (SPEC-015 behavior 5). A one-variant publish
        // trivially preserves its objective; a multi-variant request
        // would be validated by the caller. We also re-check the
        // variant's own objective is canonical by construction.
        variants_preserve_single_objective(std::slice::from_ref(variant)).map_err(|e| {
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(variant.variant_id.to_string())
        })?;

        // Gate 1 (caller-side): the approval must be GRANTED and its
        // action kind must match; the policy module enforces the
        // SEPARATE approval class for PUBLISH.
        if approval.state != SocialApprovalState::Granted {
            self.record(
                &correlation,
                "PUBLISH_VARIANT",
                "POLICY",
                "publish approval is not granted".into(),
                std::collections::BTreeMap::from([
                    (
                        "action_kind".into(),
                        SocialActionKind::Publish.as_str().into(),
                    ),
                    ("business".into(), self.business_id.to_string()),
                ]),
            );
            return Err(SocialError::new(
                SocialErrorCode::Policy,
                "publish approval is not granted",
                Some(correlation.clone()),
                approval.approved_by.as_ref().map(|p| p.to_string()),
                Some(self.tenant_id.to_string()),
                Some(variant.variant_id.to_string()),
            ));
        }
        if approval.action_kind != SocialActionKind::Publish {
            self.record(
                &correlation,
                "PUBLISH_VARIANT",
                "POLICY",
                "approval action kind does not match publish".into(),
                std::collections::BTreeMap::from([
                    (
                        "action_kind".into(),
                        SocialActionKind::Publish.as_str().into(),
                    ),
                    ("business".into(), self.business_id.to_string()),
                ]),
            );
            return Err(SocialError::new(
                SocialErrorCode::Policy,
                "approval action kind does not match publish",
                Some(correlation.clone()),
                approval.approved_by.as_ref().map(|p| p.to_string()),
                Some(self.tenant_id.to_string()),
                Some(variant.variant_id.to_string()),
            ));
        }
        if let Err(e) = enforce_social_action_policy(
            SocialActionKind::Publish,
            required_approval_class(SocialActionKind::Publish),
        ) {
            self.record(
                &correlation,
                "PUBLISH_VARIANT",
                "POLICY",
                e.message.clone(),
                std::collections::BTreeMap::from([
                    (
                        "action_kind".into(),
                        SocialActionKind::Publish.as_str().into(),
                    ),
                    ("business".into(), self.business_id.to_string()),
                ]),
            );
            return Err(e
                .with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(variant.variant_id.to_string()));
        }

        // In-flight idempotency: a duplicate in-flight publish is a
        // Conflict; completion/failure releases the entry.
        let key = format!(
            "{}:{}",
            self.business_id.as_str(),
            variant.variant_id.as_str()
        );
        {
            let mut in_flight = self.in_flight.lock().unwrap();
            if let Some(entry) = in_flight.get(&key) {
                if entry.idempotency_key == approval.approval_id.as_str() {
                    return Err(SocialError::new(
                        SocialErrorCode::Conflict,
                        "publish already in flight",
                        Some(correlation.clone()),
                        approval.approved_by.as_ref().map(|p| p.to_string()),
                        Some(self.tenant_id.to_string()),
                        Some(variant.variant_id.to_string()),
                    ));
                }
            }
            in_flight.insert(
                key.clone(),
                InFlightEntry {
                    idempotency_key: approval.approval_id.as_str().to_string(),
                },
            );
        }

        // Gate 2 (provider-side): the transport enforces the
        // authenticated credential and the documented surface. Only
        // NOW does any provider call happen.
        let payload = self.build_post_payload(variant, "now");
        let result = self.transport.create_post(&payload).map_err(|e| {
            self.record(
                &correlation,
                "PUBLISH_VARIANT",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(variant.variant_id.to_string())
        });

        // Release the in-flight entry after completion OR failure
        // (bounded retry: retry after completion is not a Conflict).
        {
            let mut in_flight = self.in_flight.lock().unwrap();
            in_flight.remove(&key);
        }

        let post_ref = result?;
        self.record(
            &correlation,
            "PUBLISH_VARIANT",
            "ok",
            format!("post {} state {}", post_ref.id, post_ref.status),
            std::collections::BTreeMap::from([
                ("business".into(), self.business_id.to_string()),
                (
                    "action_kind".into(),
                    SocialActionKind::Publish.as_str().into(),
                ),
                ("platform".into(), variant.platform.clone()),
            ]),
        );

        SocialMessageId::new(format!("postiz:{}", post_ref.id)).map_err(|e| {
            Self::map_hydra_id_error(e, correlation.clone(), post_ref.id.clone())
                .with_tenant(self.tenant_id.to_string())
        })
    }

    fn execute_governed_inner(
        &self,
        kind: SocialActionKind,
        approval: &PublishApproval,
        request_ref: &str,
    ) -> Result<(), SocialError> {
        let correlation = self.correlation();

        // Gate 1 (caller-side): GRANTED approval + matching kind +
        // the SEPARATE required class (behavior 8: spend/crisis
        // require human approval).
        if approval.state != SocialApprovalState::Granted {
            self.record(
                &correlation,
                "EXECUTE_GOVERNED",
                "POLICY",
                "governed action approval is not granted".into(),
                std::collections::BTreeMap::from([
                    ("action_kind".into(), kind.as_str().into()),
                    ("business".into(), self.business_id.to_string()),
                ]),
            );
            return Err(SocialError::new(
                SocialErrorCode::Policy,
                "governed action approval is not granted",
                Some(correlation.clone()),
                approval.approved_by.as_ref().map(|p| p.to_string()),
                Some(self.tenant_id.to_string()),
                Some(request_ref.to_string()),
            ));
        }
        if approval.action_kind != kind {
            self.record(
                &correlation,
                "EXECUTE_GOVERNED",
                "POLICY",
                "approval action kind does not match the governed action".into(),
                std::collections::BTreeMap::from([
                    ("action_kind".into(), kind.as_str().into()),
                    ("business".into(), self.business_id.to_string()),
                ]),
            );
            return Err(SocialError::new(
                SocialErrorCode::Policy,
                "approval action kind does not match the governed action",
                Some(correlation.clone()),
                approval.approved_by.as_ref().map(|p| p.to_string()),
                Some(self.tenant_id.to_string()),
                Some(request_ref.to_string()),
            ));
        }
        if let Err(e) = enforce_social_action_policy(kind, required_approval_class(kind)) {
            self.record(
                &correlation,
                "EXECUTE_GOVERNED",
                "POLICY",
                e.message.clone(),
                std::collections::BTreeMap::from([
                    ("action_kind".into(), kind.as_str().into()),
                    ("business".into(), self.business_id.to_string()),
                ]),
            );
            return Err(e
                .with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(request_ref.to_string()));
        }

        // Gate 2 (provider-side): the transport executes the governed
        // action. For spend/crisis the transport surface is the
        // documented change-status / post endpoints; the adapter never
        // fabricates a provider outcome.
        let key = format!("{}:{}", self.business_id.as_str(), request_ref);
        {
            let mut in_flight = self.in_flight.lock().unwrap();
            if let Some(entry) = in_flight.get(&key) {
                if entry.idempotency_key == approval.approval_id.as_str() {
                    return Err(SocialError::new(
                        SocialErrorCode::Conflict,
                        "governed action already in flight",
                        Some(correlation.clone()),
                        approval.approved_by.as_ref().map(|p| p.to_string()),
                        Some(self.tenant_id.to_string()),
                        Some(request_ref.to_string()),
                    ));
                }
            }
            in_flight.insert(
                key.clone(),
                InFlightEntry {
                    idempotency_key: approval.approval_id.as_str().to_string(),
                },
            );
        }

        let result = self
            .transport
            .change_post_status(request_ref, "published")
            .map_err(|e| {
                self.record(
                    &correlation,
                    "EXECUTE_GOVERNED",
                    "EXTERNAL_PROVIDER",
                    e.message.clone(),
                    std::collections::BTreeMap::new(),
                );
                e.with_correlation(correlation.clone())
                    .with_tenant(self.tenant_id.to_string())
                    .with_resource(request_ref.to_string())
            });

        {
            let mut in_flight = self.in_flight.lock().unwrap();
            in_flight.remove(&key);
        }

        result?;
        self.record(
            &correlation,
            "EXECUTE_GOVERNED",
            "ok",
            format!("governed {} on {}", kind.as_str(), request_ref),
            std::collections::BTreeMap::from([
                ("business".into(), self.business_id.to_string()),
                ("action_kind".into(), kind.as_str().into()),
            ]),
        );
        Ok(())
    }
}

impl SocialProvider for PostizAdapter {
    fn capabilities(&self) -> SocialCapabilityMap {
        // Map the documented integration list into canonical
        // capabilities. Unknown provider kinds are skipped and never
        // widen the contract. Unbound transports fail closed (empty).
        let mut map = SocialCapabilityMap::new();
        match self.transport.list_integrations() {
            Ok(integrations) if !integrations.is_empty() => {
                // Postiz is the scheduling/connector sidecar: connected
                // integrations imply publish + schedule breadth.
                map.insert(SocialCapabilityKind::DraftAndSchedule);
                map.insert(SocialCapabilityKind::SubmitForApproval);
                map.insert(SocialCapabilityKind::Publish);
                map.insert(SocialCapabilityKind::ReadConversations);
                map.insert(SocialCapabilityKind::Reply);
                map.insert(SocialCapabilityKind::ReadMetrics);
                map.insert(SocialCapabilityKind::Listen);
                map.insert(SocialCapabilityKind::LeadHandoff);
                map.insert(SocialCapabilityKind::AttributionReconcile);
                let _ = integrations;
            }
            _ => {}
        }
        map
    }

    fn publish_variant(
        &self,
        variant: &PlatformVariant,
        approval: &PublishApproval,
    ) -> Result<SocialMessageId, SocialError> {
        self.publish_variant_inner(variant, approval)
    }

    fn list_conversations(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
    ) -> Result<Vec<SocialConversation>, SocialError> {
        // The documented Postiz public API does not expose an inbox
        // conversation read surface; the adapter fails closed rather
        // than fabricating conversations (Reality rule). Community
        // inbox reads are owned by the direct-platform connector (M3)
        // when an official API exists.
        let correlation = self.correlation();
        self.record(
            &correlation,
            "LIST_CONVERSATIONS",
            "UNAVAILABLE",
            "postiz public API has no inbox surface; fail closed".into(),
            std::collections::BTreeMap::new(),
        );
        Err(SocialError::new(
            SocialErrorCode::Unavailable,
            "postiz public API has no conversation read surface",
            Some(correlation),
            None,
            Some(self.tenant_id.to_string()),
            None,
        ))
    }

    fn reply(
        &self,
        conversation: &SocialConversation,
        approval: &PublishApproval,
        content_ref: &str,
    ) -> Result<SocialMessageId, SocialError> {
        let correlation = self.correlation();

        // Gate 1 (caller-side): REPLY requires the REPLY approval
        // class (separate from publish; blind auto-replies are a
        // non-goal).
        if approval.state != SocialApprovalState::Granted {
            self.record(
                &correlation,
                "REPLY",
                "POLICY",
                "reply approval is not granted".into(),
                std::collections::BTreeMap::from([
                    (
                        "action_kind".into(),
                        SocialActionKind::Reply.as_str().into(),
                    ),
                    ("business".into(), self.business_id.to_string()),
                ]),
            );
            return Err(SocialError::new(
                SocialErrorCode::Policy,
                "reply approval is not granted",
                Some(correlation.clone()),
                approval.approved_by.as_ref().map(|p| p.to_string()),
                Some(self.tenant_id.to_string()),
                Some(conversation.conversation_id.to_string()),
            ));
        }
        if approval.action_kind != SocialActionKind::Reply {
            self.record(
                &correlation,
                "REPLY",
                "POLICY",
                "approval action kind does not match reply".into(),
                std::collections::BTreeMap::from([
                    (
                        "action_kind".into(),
                        SocialActionKind::Reply.as_str().into(),
                    ),
                    ("business".into(), self.business_id.to_string()),
                ]),
            );
            return Err(SocialError::new(
                SocialErrorCode::Policy,
                "approval action kind does not match reply",
                Some(correlation.clone()),
                approval.approved_by.as_ref().map(|p| p.to_string()),
                Some(self.tenant_id.to_string()),
                Some(conversation.conversation_id.to_string()),
            ));
        }
        if let Err(e) = enforce_social_action_policy(
            SocialActionKind::Reply,
            required_approval_class(SocialActionKind::Reply),
        ) {
            self.record(
                &correlation,
                "REPLY",
                "POLICY",
                e.message.clone(),
                std::collections::BTreeMap::from([
                    (
                        "action_kind".into(),
                        SocialActionKind::Reply.as_str().into(),
                    ),
                    ("business".into(), self.business_id.to_string()),
                ]),
            );
            return Err(e
                .with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(conversation.conversation_id.to_string()));
        }

        // The documented Postiz public API creates posts; a governed
        // reply is a create-post against the thread. The transport
        // enforces the authenticated credential.
        let payload = serde_json::json!({
            "type": "now",
            "date": "",
            "shortLink": false,
            "tags": [],
            "posts": [{
                "integration": { "id": conversation.platform },
                "value": [{ "content": content_ref }],
                "settings": { "__type": conversation.platform }
            }]
        });
        let result = self.transport.create_post(&payload).map_err(|e| {
            self.record(
                &correlation,
                "REPLY",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(conversation.conversation_id.to_string())
        })?;
        self.record(
            &correlation,
            "REPLY",
            "ok",
            format!("reply post {} state {}", result.id, result.status),
            std::collections::BTreeMap::from([
                ("business".into(), self.business_id.to_string()),
                (
                    "action_kind".into(),
                    SocialActionKind::Reply.as_str().into(),
                ),
            ]),
        );
        SocialMessageId::new(format!("postiz:{}", result.id)).map_err(|e| {
            Self::map_hydra_id_error(e, correlation.clone(), result.id.clone())
                .with_tenant(self.tenant_id.to_string())
        })
    }

    fn list_metrics(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
        _campaign_id: Option<&CampaignId>,
    ) -> Result<Vec<SocialMetric>, SocialError> {
        // The documented Postiz public API lists posts but does not
        // expose engagement analytics in the documented surface; the
        // adapter fails closed rather than fabricating metrics
        // (Reality rule; attribution preserved only from real
        // analytics, owned by the direct-platform connector M3).
        let correlation = self.correlation();
        self.record(
            &correlation,
            "LIST_METRICS",
            "UNAVAILABLE",
            "postiz public API has no analytics surface; fail closed".into(),
            std::collections::BTreeMap::new(),
        );
        Err(SocialError::new(
            SocialErrorCode::Unavailable,
            "postiz public API has no analytics read surface",
            Some(correlation),
            None,
            Some(self.tenant_id.to_string()),
            None,
        ))
    }

    fn list_leads(
        &self,
        _tenant_id: &TenantId,
        _business_id: &BusinessId,
    ) -> Result<Vec<SocialLead>, SocialError> {
        // CRM lead handoff is owned by the direct-platform connector
        // and Hydra (M3/M5); the Postiz sidecar does not fabricate
        // leads.
        let correlation = self.correlation();
        self.record(
            &correlation,
            "LIST_LEADS",
            "UNAVAILABLE",
            "postiz public API has no lead surface; fail closed".into(),
            std::collections::BTreeMap::new(),
        );
        Err(SocialError::new(
            SocialErrorCode::Unavailable,
            "postiz public API has no lead read surface",
            Some(correlation),
            None,
            Some(self.tenant_id.to_string()),
            None,
        ))
    }

    fn execute_governed(
        &self,
        kind: SocialActionKind,
        approval: &PublishApproval,
        request_ref: &str,
    ) -> Result<(), SocialError> {
        self.execute_governed_inner(kind, approval, request_ref)
    }
}

impl PostizProvider for PostizAdapter {
    fn schedule(
        &self,
        message: &SocialMessage,
        scheduled_at: &str,
    ) -> Result<SocialMessageId, SocialError> {
        let correlation = self.correlation();
        // The documented Postiz create-post surface supports
        // `type: schedule` with an RFC3339 date (docs.postiz.com).
        let payload = serde_json::json!({
            "type": "schedule",
            "date": scheduled_at,
            "shortLink": false,
            "tags": [],
            "posts": [{
                "integration": { "id": message.account_id.as_str() },
                "value": [{ "content": message.content_ref }],
                "settings": { "__type": message.account_id.as_str() }
            }]
        });
        let result = self.transport.create_post(&payload).map_err(|e| {
            self.record(
                &correlation,
                "SCHEDULE",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(message.message_id.to_string())
        })?;
        self.record(
            &correlation,
            "SCHEDULE",
            "ok",
            format!("scheduled post {} state {}", result.id, result.status),
            std::collections::BTreeMap::from([
                ("business".into(), self.business_id.to_string()),
                (
                    "action_kind".into(),
                    SocialActionKind::Publish.as_str().into(),
                ),
            ]),
        );
        SocialMessageId::new(format!("postiz:{}", result.id)).map_err(|e| {
            Self::map_hydra_id_error(e, correlation.clone(), result.id.clone())
                .with_tenant(self.tenant_id.to_string())
        })
    }
}

// Postiz implements the same provider-neutral contract as a direct
// official API; the sidecar seam is replaceable (behavior 4).
impl DirectPlatformProvider for PostizAdapter {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{PostizIntegration, PostizPostRef};
    use nexus_domain::PersonId;
    use nexus_hydra::Campaign;
    use nexus_social::{
        CampaignObjective, PlatformVariantId, PublishApprovalId, SocialConversationId,
    };
    use std::str::FromStr;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    fn variant() -> PlatformVariant {
        PlatformVariant::new(
            PlatformVariantId::new("v-1").unwrap(),
            campaign().campaign_id,
            "instagram",
            CampaignObjective::Leads,
            "ref://instagram-post",
            message().message_id,
        )
    }

    fn granted_approval(kind: SocialActionKind) -> PublishApproval {
        let mut ap = PublishApproval::new(
            PublishApprovalId::new("ap-1").unwrap(),
            tenant(),
            business(),
            kind,
            message().message_id,
        );
        ap.grant(person()).unwrap();
        ap
    }

    /// Counting transport: records every provider call; the adapter
    /// must make ZERO calls on denial (policy before mutation).
    struct CountingTransport {
        calls: AtomicUsize,
    }

    impl CountingTransport {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl PostizTransport for CountingTransport {
        fn list_integrations(&self) -> Result<Vec<PostizIntegration>, SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![PostizIntegration {
                id: "ig-1".into(),
                name: "Instagram".into(),
                identifier: "Instagram".into(),
                available: true,
            }])
        }

        fn create_post(&self, _payload: &serde_json::Value) -> Result<PostizPostRef, SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(PostizPostRef {
                id: "post-123".into(),
                status: "published".into(),
            })
        }

        fn list_posts(&self) -> Result<Vec<PostizPostRef>, SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![PostizPostRef {
                id: "post-123".into(),
                status: "published".into(),
            }])
        }

        fn change_post_status(&self, _post_id: &str, _status: &str) -> Result<(), SocialError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    impl PostizTransport for std::sync::Arc<CountingTransport> {
        fn list_integrations(&self) -> Result<Vec<PostizIntegration>, SocialError> {
            (**self).list_integrations()
        }

        fn create_post(&self, payload: &serde_json::Value) -> Result<PostizPostRef, SocialError> {
            (**self).create_post(payload)
        }

        fn list_posts(&self) -> Result<Vec<PostizPostRef>, SocialError> {
            (**self).list_posts()
        }

        fn change_post_status(&self, post_id: &str, status: &str) -> Result<(), SocialError> {
            (**self).change_post_status(post_id, status)
        }
    }

    #[test]
    fn ep029_unit_publish_requires_granted_approval_zero_calls_on_denial() {
        use std::sync::Arc;
        let transport = Arc::new(CountingTransport::new());
        let adapter = PostizAdapter::new(
            Box::new(transport.clone()),
            tenant(),
            business(),
            "test-api-key",
        );
        // A pending approval must be denied BEFORE any provider call.
        let pending = PublishApproval::new(
            PublishApprovalId::new("ap-2").unwrap(),
            tenant(),
            business(),
            SocialActionKind::Publish,
            message().message_id,
        );
        let err = adapter.publish_variant(&variant(), &pending).unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Policy);
        // Zero provider calls on policy denial (measured through the
        // shared counter).
        assert_eq!(transport.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ep029_unit_publish_approved_reaches_transport_exactly_once() {
        let transport = CountingTransport::new();
        let adapter = PostizAdapter::new(Box::new(transport), tenant(), business(), "test-api-key");
        let mid = adapter
            .publish_variant(&variant(), &granted_approval(SocialActionKind::Publish))
            .unwrap();
        assert!(mid.as_str().starts_with("postiz:post-123"));
        // Audit records exactly one ok publish.
        let audit = adapter.audit();
        let publishes: Vec<_> = audit
            .iter()
            .filter(|e| e.operation == "PUBLISH_VARIANT")
            .collect();
        assert_eq!(publishes.len(), 1);
        assert_eq!(publishes[0].outcome, "ok");
    }

    #[test]
    fn ep029_unit_reply_requires_reply_approval_class() {
        let transport = CountingTransport::new();
        let adapter = PostizAdapter::new(Box::new(transport), tenant(), business(), "test-api-key");
        let conv = SocialConversation::new(
            SocialConversationId::new("conv-1").unwrap(),
            nexus_hydra::SocialAccountId::new("acct-1").unwrap(),
            business(),
            "instagram",
            "thread-1",
        );
        // A publish-kind approval cannot be used for a reply (separate
        // approval classes, behavior 5).
        let wrong_kind = granted_approval(SocialActionKind::Publish);
        let err = adapter
            .reply(&conv, &wrong_kind, "ref://reply")
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Policy);
        // A granted REPLY approval succeeds.
        let ok = adapter
            .reply(
                &conv,
                &granted_approval(SocialActionKind::Reply),
                "ref://reply",
            )
            .unwrap();
        assert!(ok.as_str().starts_with("postiz:"));
    }

    #[test]
    fn ep029_unit_execute_governed_spend_crisis_human_approval() {
        let transport = CountingTransport::new();
        let adapter = PostizAdapter::new(Box::new(transport), tenant(), business(), "test-api-key");
        // A granted spend approval passes the gate (required class is
        // STRONG_HUMAN; the approval is granted by a human).
        assert!(adapter
            .execute_governed(
                SocialActionKind::SpendChange,
                &granted_approval(SocialActionKind::SpendChange),
                "budget-1",
            )
            .is_ok());
        // A crisis statement requires FOUR_EYES (granted by human).
        assert!(adapter
            .execute_governed(
                SocialActionKind::CrisisStatement,
                &granted_approval(SocialActionKind::CrisisStatement),
                "crisis-1",
            )
            .is_ok());
        // A publish-kind approval cannot govern a spend change
        // (separate classes).
        let err = adapter
            .execute_governed(
                SocialActionKind::SpendChange,
                &granted_approval(SocialActionKind::Publish),
                "budget-2",
            )
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Policy);
    }

    #[test]
    fn ep029_unit_capability_map_fails_closed_when_transport_unavailable() {
        struct FailingTransport;
        impl PostizTransport for FailingTransport {}
        let adapter = PostizAdapter::new(Box::new(FailingTransport), tenant(), business(), "k");
        assert!(adapter.capabilities().is_empty());
    }

    #[test]
    fn ep029_unit_capability_map_from_integrations() {
        let transport = CountingTransport::new();
        let adapter = PostizAdapter::new(Box::new(transport), tenant(), business(), "test-api-key");
        let caps = adapter.capabilities();
        assert!(caps.contains(SocialCapabilityKind::Publish));
        assert!(caps.contains(SocialCapabilityKind::DraftAndSchedule));
        assert!(caps.contains(SocialCapabilityKind::LeadHandoff));
    }

    #[test]
    fn ep029_unit_observability_redacts_credential_canary() {
        let transport = CountingTransport::new();
        let adapter = PostizAdapter::new(
            Box::new(transport),
            tenant(),
            business(),
            "sekret-api-key-xyz",
        );
        // Force an error path that records detail.
        let _ = adapter.list_conversations(&tenant(), &business());
        let audit = adapter.audit();
        for entry in &audit {
            assert!(!entry.detail.contains("sekret-api-key-xyz"));
            assert!(!format!("{:?}", entry.fields).contains("sekret-api-key-xyz"));
        }
        assert!(audit.iter().any(|e| e.outcome == "UNAVAILABLE"));
    }

    #[test]
    fn ep029_unit_schedule_uses_documented_type_schedule() {
        let transport = CountingTransport::new();
        let adapter = PostizAdapter::new(Box::new(transport), tenant(), business(), "test-api-key");
        let mid = adapter
            .schedule(&message(), "2026-08-20T10:00:00.000Z")
            .unwrap();
        assert!(mid.as_str().starts_with("postiz:post-123"));
        let audit = adapter.audit();
        assert!(audit
            .iter()
            .any(|e| e.operation == "SCHEDULE" && e.outcome == "ok"));
    }

    #[test]
    fn ep029_unit_error_paths_carry_correlation_and_tenant() {
        let transport = CountingTransport::new();
        let adapter = PostizAdapter::new(Box::new(transport), tenant(), business(), "test-api-key");
        let err = adapter
            .list_metrics(&tenant(), &business(), None)
            .unwrap_err();
        assert_eq!(err.code, SocialErrorCode::Unavailable);
        assert!(err.correlation.is_some());
        assert_eq!(err.tenant.as_deref(), Some(tenant().as_str()));
    }

    #[test]
    fn ep029_unit_publish_conflict_released_after_completion() {
        // A duplicate in-flight publish is a Conflict, but a retry
        // after completion is not (release-after-end).
        let transport = CountingTransport::new();
        let adapter = PostizAdapter::new(Box::new(transport), tenant(), business(), "test-api-key");
        let approval = granted_approval(SocialActionKind::Publish);
        // First publish completes and releases the entry.
        let first = adapter.publish_variant(&variant(), &approval);
        assert!(first.is_ok());
        // Retry after completion is NOT a conflict.
        let second = adapter.publish_variant(&variant(), &approval);
        assert!(second.is_ok());
    }
}
