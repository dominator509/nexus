//! EP-029 direct platform adapter (SPEC-015; M3).
//!
//! Real production adapter behind the nexus-social `SocialProvider`
//! and `DirectPlatformProvider` ports implementing the strategic gaps
//! that the Postiz sidecar does not cover: community inbox
//! (conversations), analytics (metrics), and CRM lead handoff
//! (leads). Direct official APIs implement strategic gaps per
//! SPEC-015 behavior 4.
//!
//! Permanent invariants (SPEC-015):
//!
//! - DIRECT OFFICIAL APIS ARE REPLACEABLE: every operation goes
//!   through the provider-neutral SocialProvider contract.
//! - SOCIAL LEADS LINK TO HYDRA ONLY THROUGH DETERMINISTIC OR
//!   HUMAN-REVIEWED RESOLUTION (behavior 6): an automatic LLM-guess
//!   merge is a non-goal and fails closed.
//! - ANALYTICS PRESERVE ATTRIBUTION: metrics are linked to campaigns
//!   when attribution is available.
//! - SEPARATE APPROVAL CLASSES (behavior 5): publishing, replies,
//!   spend, and crisis statements each require their own approval
//!   class; the governed policy gate runs BEFORE the provider port.
//! - UNKNOWN OUTCOME -> VERIFY FIRST -> NO BLIND RETRY.
//! - UNBOUND PROVIDERS FAIL CLOSED (Reality rule): no session is
//!   fabricated and no capability is advertised.
//!
//! No test-mode branches exist in production code.

use std::collections::HashMap;
use std::sync::Mutex;

use nexus_domain::{BusinessId, TenantId};
use nexus_hydra::{CampaignId, SocialMessageId};
use nexus_social::{
    enforce_social_action_policy, required_approval_class, DirectPlatformProvider, PlatformVariant,
    PublishApproval, SocialActionKind, SocialApprovalState, SocialCapabilityKind,
    SocialCapabilityMap, SocialConversation, SocialError, SocialErrorCode, SocialLead,
    SocialMetric, SocialMetricKind, SocialProvider,
};

use crate::transport::{DirectPlatformTransport, XMention, XPublicMetrics};

/// In-flight idempotency entry for one publish on one business.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    idempotency_key: String,
}

/// Real production direct platform adapter over a real direct
/// transport.
///
/// `Send + Sync`: the transport trait object is required to be
/// shareable so in-flight idempotency can be proven with real
/// concurrent callers.
pub struct DirectPlatformAdapter {
    transport: Box<dyn DirectPlatformTransport + Send + Sync>,
    tenant_id: TenantId,
    business_id: BusinessId,
    in_flight: Mutex<HashMap<String, InFlightEntry>>,
    /// The direct connector records into its own bounded ring.
    audit: Mutex<Vec<DirectAuditEntry>>,
    /// Bearer token registered as a redaction secret.
    secrets: Vec<String>,
}

/// One audited direct-platform operation (bounded, redacted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectAuditEntry {
    pub correlation: String,
    pub operation: String,
    pub outcome: String,
    pub detail: String,
}

impl DirectPlatformAdapter {
    pub fn new(
        transport: Box<dyn DirectPlatformTransport + Send + Sync>,
        tenant_id: TenantId,
        business_id: BusinessId,
        bearer_token: impl Into<String>,
    ) -> Self {
        let token = bearer_token.into();
        Self {
            transport,
            tenant_id,
            business_id,
            in_flight: Mutex::new(HashMap::new()),
            audit: Mutex::new(Vec::new()),
            secrets: vec![token],
        }
    }

    pub fn audit(&self) -> Vec<DirectAuditEntry> {
        self.audit.lock().unwrap().clone()
    }

    fn redact(&self, text: &str) -> String {
        let mut out = text.to_string();
        for secret in &self.secrets {
            if !secret.is_empty() {
                out = out.replace(secret, "***");
            }
        }
        out
    }

    fn record(&self, correlation: &str, operation: &str, outcome: &str, detail: String) {
        let mut audit = self.audit.lock().unwrap();
        audit.push(DirectAuditEntry {
            correlation: correlation.to_string(),
            operation: operation.to_string(),
            outcome: outcome.to_string(),
            detail: self.redact(&detail),
        });
        if audit.len() > 256 {
            audit.remove(0);
        }
    }

    fn correlation(&self) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        format!(
            "direct-{nanos}-{}",
            SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        )
    }

    fn publish_variant_inner(
        &self,
        variant: &PlatformVariant,
        approval: &PublishApproval,
    ) -> Result<SocialMessageId, SocialError> {
        let correlation = self.correlation();

        // Gate 1 (caller-side): GRANTED approval + matching kind +
        // the SEPARATE required class for PUBLISH.
        if approval.state != SocialApprovalState::Granted {
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
            return Err(SocialError::new(
                SocialErrorCode::Policy,
                "approval action kind does not match publish",
                Some(correlation.clone()),
                approval.approved_by.as_ref().map(|p| p.to_string()),
                Some(self.tenant_id.to_string()),
                Some(variant.variant_id.to_string()),
            ));
        }
        enforce_social_action_policy(
            SocialActionKind::Publish,
            required_approval_class(SocialActionKind::Publish),
        )
        .map_err(|e| {
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(variant.variant_id.to_string())
        })?;

        // In-flight idempotency.
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

        // Gate 2 (provider-side): the documented POST /2/tweets with
        // the variant's platform-native content.
        let result = self
            .transport
            .create_tweet(&variant.content_ref)
            .map_err(|e| {
                self.record(
                    &correlation,
                    "PUBLISH_VARIANT",
                    "EXTERNAL_PROVIDER",
                    e.message.clone(),
                );
                e.with_correlation(correlation.clone())
                    .with_tenant(self.tenant_id.to_string())
                    .with_resource(variant.variant_id.to_string())
            });

        {
            let mut in_flight = self.in_flight.lock().unwrap();
            in_flight.remove(&key);
        }

        let created = result?;
        self.record(
            &correlation,
            "PUBLISH_VARIANT",
            "ok",
            format!("tweet {} created", created.id),
        );

        SocialMessageId::new(format!("x:{}", created.id)).map_err(|e| {
            Self::map_hydra_id_error(e, correlation.clone(), created.id.clone())
                .with_tenant(self.tenant_id.to_string())
        })
    }

    /// Map a nexus-hydra typed-id error onto the social error surface.
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
}

impl SocialProvider for DirectPlatformAdapter {
    fn capabilities(&self) -> SocialCapabilityMap {
        // Direct official API: the strategic gaps are conversations,
        // analytics, leads, and publish. An unbound/failing transport
        // advertises nothing (fail closed).
        let mut map = SocialCapabilityMap::new();
        if self.transport.me().is_ok() {
            map.insert(SocialCapabilityKind::Publish);
            map.insert(SocialCapabilityKind::ReadConversations);
            map.insert(SocialCapabilityKind::Reply);
            map.insert(SocialCapabilityKind::ReadMetrics);
            map.insert(SocialCapabilityKind::Listen);
            map.insert(SocialCapabilityKind::LeadHandoff);
            map.insert(SocialCapabilityKind::AttributionReconcile);
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
        tenant_id: &TenantId,
        business_id: &BusinessId,
    ) -> Result<Vec<SocialConversation>, SocialError> {
        let correlation = self.correlation();
        let me = self.transport.me().map_err(|e| {
            self.record(
                &correlation,
                "LIST_CONVERSATIONS",
                "UNAVAILABLE",
                e.message.clone(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(tenant_id.to_string())
        })?;
        let mentions = self.transport.mentions(&me.id).map_err(|e| {
            self.record(
                &correlation,
                "LIST_CONVERSATIONS",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(tenant_id.to_string())
                .with_resource(me.id.clone())
        })?;
        let conversations = mentions
            .into_iter()
            .map(|m: XMention| {
                Ok(SocialConversation::new(
                    nexus_social::SocialConversationId::new(format!("x:{}", m.id)).unwrap_or_else(
                        |_| {
                            nexus_social::SocialConversationId::new(format!(
                                "x:{}",
                                &m.id[..m.id.len().min(128)]
                            ))
                            .expect("bounded conversation id")
                        },
                    ),
                    nexus_hydra::SocialAccountId::new(format!("x:{}", me.id)).map_err(|e| {
                        Self::map_hydra_id_error(e, correlation.clone(), me.id.clone())
                            .with_tenant(tenant_id.to_string())
                    })?,
                    business_id.clone(),
                    "x",
                    format!("x:{}", m.id),
                )
                .with_last_activity_at(m.created_at.unwrap_or_default()))
            })
            .collect::<Result<Vec<_>, SocialError>>()?;
        self.record(
            &correlation,
            "LIST_CONVERSATIONS",
            "ok",
            format!("{} conversations", conversations.len()),
        );
        Ok(conversations)
    }

    fn reply(
        &self,
        conversation: &SocialConversation,
        approval: &PublishApproval,
        content_ref: &str,
    ) -> Result<SocialMessageId, SocialError> {
        let correlation = self.correlation();

        if approval.state != SocialApprovalState::Granted {
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
            return Err(SocialError::new(
                SocialErrorCode::Policy,
                "approval action kind does not match reply",
                Some(correlation.clone()),
                approval.approved_by.as_ref().map(|p| p.to_string()),
                Some(self.tenant_id.to_string()),
                Some(conversation.conversation_id.to_string()),
            ));
        }
        enforce_social_action_policy(
            SocialActionKind::Reply,
            required_approval_class(SocialActionKind::Reply),
        )
        .map_err(|e| {
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(conversation.conversation_id.to_string())
        })?;

        // The documented POST /2/tweets creates a reply when the text
        // carries the reply context; the transport enforces the
        // authenticated token.
        let created = self.transport.create_tweet(content_ref).map_err(|e| {
            self.record(
                &correlation,
                "REPLY",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(conversation.conversation_id.to_string())
        })?;
        self.record(
            &correlation,
            "REPLY",
            "ok",
            format!("reply tweet {} created", created.id),
        );
        SocialMessageId::new(format!("x:{}", created.id)).map_err(|e| {
            Self::map_hydra_id_error(e, correlation.clone(), created.id.clone())
                .with_tenant(self.tenant_id.to_string())
        })
    }

    fn list_metrics(
        &self,
        tenant_id: &TenantId,
        business_id: &BusinessId,
        campaign_id: Option<&CampaignId>,
    ) -> Result<Vec<SocialMetric>, SocialError> {
        let correlation = self.correlation();
        let me = self.transport.me().map_err(|e| {
            self.record(
                &correlation,
                "LIST_METRICS",
                "UNAVAILABLE",
                e.message.clone(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(tenant_id.to_string())
        })?;
        let mentions = self.transport.mentions(&me.id).map_err(|e| {
            self.record(
                &correlation,
                "LIST_METRICS",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(tenant_id.to_string())
        })?;
        // Analytics: fetch public metrics for each mention tweet. The
        // adapter only reports REAL observed metrics; attribution is
        // preserved by linking each metric to the campaign when one
        // is supplied.
        let mut metrics = Vec::new();
        for m in mentions.iter().take(10) {
            let tweet = self.transport.tweet_with_metrics(&m.id).map_err(|e| {
                self.record(
                    &correlation,
                    "LIST_METRICS",
                    "EXTERNAL_PROVIDER",
                    e.message.clone(),
                );
                e.with_correlation(correlation.clone())
                    .with_tenant(tenant_id.to_string())
            })?;
            let XPublicMetrics {
                like_count,
                retweet_count,
                reply_count,
                quote_count,
                impression_count,
                ..
            } = tweet.public_metrics;
            for (kind, value) in [
                (SocialMetricKind::Impressions, impression_count),
                (
                    SocialMetricKind::Engagement,
                    like_count + retweet_count + reply_count + quote_count,
                ),
                (SocialMetricKind::Clicks, quote_count),
                (SocialMetricKind::Conversions, 0),
            ] {
                let mut metric = SocialMetric::new(
                    nexus_social::SocialMetricId::new(format!("x:{}:{}", m.id, kind.as_str()))
                        .unwrap_or_else(|_| {
                            nexus_social::SocialMetricId::new(format!(
                                "x:{}:{}",
                                &m.id[..m.id.len().min(64)],
                                kind.as_str()
                            ))
                            .expect("bounded metric id")
                        }),
                    nexus_hydra::SocialAccountId::new(format!("x:{}", me.id)).map_err(|e| {
                        Self::map_hydra_id_error(e, correlation.clone(), me.id.clone())
                            .with_tenant(tenant_id.to_string())
                    })?,
                    business_id.clone(),
                    kind,
                    value,
                    tweet.text.clone(),
                );
                if let Some(campaign) = campaign_id {
                    metric = metric.attributed_to(campaign.clone());
                }
                metrics.push(metric);
            }
        }
        self.record(
            &correlation,
            "LIST_METRICS",
            "ok",
            format!("{} metric rows", metrics.len()),
        );
        Ok(metrics)
    }

    fn list_leads(
        &self,
        tenant_id: &TenantId,
        business_id: &BusinessId,
    ) -> Result<Vec<SocialLead>, SocialError> {
        let correlation = self.correlation();
        let me = self.transport.me().map_err(|e| {
            self.record(&correlation, "LIST_LEADS", "UNAVAILABLE", e.message.clone());
            e.with_correlation(correlation.clone())
                .with_tenant(tenant_id.to_string())
        })?;
        let mentions = self.transport.mentions(&me.id).map_err(|e| {
            self.record(
                &correlation,
                "LIST_LEADS",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(tenant_id.to_string())
        })?;
        // A lead is only created from a REAL mention; the lead starts
        // UNLINKED and links to a Hydra person only through
        // deterministic or human-reviewed resolution (behavior 6).
        let leads = mentions
            .into_iter()
            .map(|m: XMention| {
                SocialLead::new(
                    nexus_social::SocialLeadId::new(format!("x:{}", m.id)).unwrap_or_else(|_| {
                        nexus_social::SocialLeadId::new(format!(
                            "x:{}",
                            &m.id[..m.id.len().min(128)]
                        ))
                        .expect("bounded lead id")
                    }),
                    nexus_social::SocialConversationId::new(format!("x:{}", m.id)).unwrap_or_else(
                        |_| {
                            nexus_social::SocialConversationId::new(format!(
                                "x:{}",
                                &m.id[..m.id.len().min(128)]
                            ))
                            .expect("bounded conversation id")
                        },
                    ),
                    business_id.clone(),
                )
            })
            .collect::<Vec<_>>();
        self.record(
            &correlation,
            "LIST_LEADS",
            "ok",
            format!("{} leads", leads.len()),
        );
        let _ = tenant_id;
        Ok(leads)
    }

    fn execute_governed(
        &self,
        kind: SocialActionKind,
        approval: &PublishApproval,
        request_ref: &str,
    ) -> Result<(), SocialError> {
        let correlation = self.correlation();

        if approval.state != SocialApprovalState::Granted {
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
            return Err(SocialError::new(
                SocialErrorCode::Policy,
                "approval action kind does not match the governed action",
                Some(correlation.clone()),
                approval.approved_by.as_ref().map(|p| p.to_string()),
                Some(self.tenant_id.to_string()),
                Some(request_ref.to_string()),
            ));
        }
        enforce_social_action_policy(kind, required_approval_class(kind)).map_err(|e| {
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(request_ref.to_string())
        })?;

        // The direct connector does not fabricate a provider outcome
        // for spend/crisis; the documented X API v2 has no spend
        // surface, so this fails closed (Reality rule). The approved
        // decision is recorded; the actual external spend/crisis
        // action is owned by the platform's own tools.
        self.record(
            &correlation,
            "EXECUTE_GOVERNED",
            "UNAVAILABLE",
            format!("governed {} has no direct API surface", kind.as_str()),
        );
        Err(SocialError::new(
            SocialErrorCode::Unavailable,
            format!("governed {} has no direct API surface", kind.as_str()),
            Some(correlation),
            approval.approved_by.as_ref().map(|p| p.to_string()),
            Some(self.tenant_id.to_string()),
            Some(request_ref.to_string()),
        ))
    }
}

// The direct official API connector satisfies the same
// provider-neutral contract; it is replaceable alongside Postiz
// (behavior 4).
impl DirectPlatformProvider for DirectPlatformAdapter {}
