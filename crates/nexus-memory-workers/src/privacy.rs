//! Privacy and disclosure workers (SPEC-020, INV-007; EP-016 M2).
//!
//! Privacy is a hard safety boundary. Relevance never overrides
//! disclosure privacy. Shared-room requests exclude private/sensitive
//! memories unless explicitly allowed; the same user on a private
//! channel may include them according to permission. Private answer
//! routing records the routing decision only; actual phone/headphone
//! delivery is owned by a later node and never asserted here.

use crate::permission::{AccessProfile, PermissionFilter};
use crate::purpose::{PurposeLimiter, PurposePolicy};
use crate::util::sensitivity_rank;
use nexus_context::{ContextError, ContextPurpose, FilteredCandidate, PrivacyFilter};
use nexus_data::memory::{MemoryCandidate, Sensitivity};

/// Disclosure context for a request: where the answer will be spoken /
/// shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisclosureContext {
    /// Private phone / headphones / private channel: private memories
    /// may be included according to permission.
    PrivateChannel,
    /// Shared room / shared speaker: only the shared-safe subset may
    /// enter; private/sensitive memories are excluded.
    SharedRoom,
}

/// Private answer routing decision (EP-016 M2; SPEC-020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateRoutingDecision {
    /// Whether the answer must be routed to a private channel.
    pub private_route: bool,
    /// Whether actual phone/headphone delivery happened here.
    pub delivery_owned: bool,
    /// Redacted reason (never sensitive content).
    pub reason: Option<String>,
}

/// Deterministic privacy filter.
///
/// Policy (EP-016 Decision Log):
/// - Permission boundary (tenant, namespace, sensitivity ceiling) is
///   enforced first (already applied by PermissionFilter upstream; this
///   worker re-checks the same profile for defense in depth).
/// - Purpose limitation is a hard constraint.
/// - Shared-room requests: candidates marked private/sensitive above the
///   shared-safe ceiling are `Deny`; they never enter the spoken
///   context. A private channel may include them per permission.
/// - Presence is evidence, not authority: recognized voice never
///   implies full private-memory access; the same permission policy
///   applies.
#[derive(Debug, Clone)]
pub struct DeterministicPrivacyFilter {
    pub access: AccessProfile,
    pub purpose: PurposePolicy,
    pub disclosure: DisclosureContext,
}

impl DeterministicPrivacyFilter {
    pub fn new(
        access: AccessProfile,
        purpose: PurposePolicy,
        disclosure: DisclosureContext,
    ) -> Self {
        Self {
            access,
            purpose,
            disclosure,
        }
    }

    /// The shared-safe sensitivity ceiling: HOUSEHOLD and below may be
    /// spoken in a shared room; anything above is excluded.
    pub const fn shared_safe_ceiling() -> Sensitivity {
        Sensitivity::Household
    }

    /// Filter candidates, preserving order. The returned vector carries
    /// a decision and a redacted reason per candidate.
    pub fn filter_with_disclosure(
        &self,
        tenant_id: &str,
        principal_id: &str,
        _purpose: ContextPurpose,
        candidates: Vec<MemoryCandidate>,
    ) -> Result<Vec<FilteredCandidate>, ContextError> {
        if tenant_id != self.access.tenant_id || principal_id != self.access.principal_id {
            return Err(ContextError::authorization(
                "privacy filter principal/tenant mismatch",
                Some("privacy-filter".into()),
            ));
        }
        let permission = PermissionFilter;
        let mut out = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if !permission.allowed(
                &self.access,
                sensitivity_rank(self.access.max_sensitivity),
                &candidate,
            ) {
                out.push(FilteredCandidate::denied(candidate, "permission boundary"));
                continue;
            }
            if !PurposeLimiter.allowed(&self.purpose, &candidate) {
                out.push(FilteredCandidate::denied(candidate, "purpose limitation"));
                continue;
            }
            if self.disclosure == DisclosureContext::SharedRoom
                && sensitivity_rank(candidate.record.sensitivity)
                    > sensitivity_rank(Self::shared_safe_ceiling())
            {
                out.push(FilteredCandidate::denied(
                    candidate,
                    "shared-room disclosure",
                ));
                continue;
            }
            out.push(FilteredCandidate::allowed(candidate));
        }
        Ok(out)
    }

    /// Private answer routing decision for a shared-room request whose
    /// answer requires sensitive memory. Records the routing decision
    /// only; delivery is NOT OWNED here.
    pub fn routing_decision(&self, requires_sensitive: bool) -> PrivateRoutingDecision {
        if self.disclosure == DisclosureContext::SharedRoom && requires_sensitive {
            PrivateRoutingDecision {
                private_route: true,
                delivery_owned: false,
                reason: Some("shared-room request requires private routing".into()),
            }
        } else {
            PrivateRoutingDecision {
                private_route: false,
                delivery_owned: false,
                reason: None,
            }
        }
    }
}

impl PrivacyFilter for DeterministicPrivacyFilter {
    fn filter(
        &mut self,
        tenant_id: &str,
        principal_id: &str,
        purpose: ContextPurpose,
        candidates: Vec<MemoryCandidate>,
    ) -> Result<Vec<FilteredCandidate>, ContextError> {
        self.filter_with_disclosure(tenant_id, principal_id, purpose, candidates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_context::PrivacyFilterDecision;
    use nexus_data::memory::{MemoryRecord, MemoryStatus, RetentionPolicy, RetentionUnit};
    use nexus_domain::{MemoryType, NexusId, TenantId};

    fn record(namespace: &str, sensitivity: Sensitivity) -> MemoryCandidate {
        MemoryCandidate {
            record: MemoryRecord {
                memory_id: NexusId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7b01").unwrap(),
                tenant_id: TenantId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7b02").unwrap(),
                namespace: namespace.into(),
                memory_type: MemoryType::Episodic,
                content: serde_json::json!({ "note": "x" }),
                content_hash: "b".repeat(64),
                source: "test".into(),
                actor: "p-1".into(),
                created_at: "2026-01-01T00:00:00Z".into(),
                observed_at: "2026-01-01T00:00:00Z".into(),
                confidence: 0.9,
                sensitivity,
                purpose: "SEARCH".into(),
                retention: RetentionPolicy::for_duration(RetentionUnit::Days, 30),
                status: MemoryStatus::Active,
                derived_from: vec![],
                supersedes: None,
                embedding_ref: None,
            },
            score: 0.9,
        }
    }

    fn profile() -> AccessProfile {
        AccessProfile {
            tenant_id: "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7b02".into(),
            principal_id: "p-1".into(),
            allowed_namespaces: vec!["household".into(), "personal".into()],
            max_sensitivity: Sensitivity::Sensitive,
            private_allowed: true,
        }
    }

    fn filter(disclosure: DisclosureContext) -> DeterministicPrivacyFilter {
        DeterministicPrivacyFilter::new(
            profile(),
            PurposeLimiter::policy_for(ContextPurpose::Search),
            disclosure,
        )
    }

    #[test]
    fn ep016_unit_shared_room_excludes_private_memory() {
        let mut f = filter(DisclosureContext::SharedRoom);
        let candidates = vec![
            record("household", Sensitivity::Household),
            record("personal", Sensitivity::Personal),
            record("personal", Sensitivity::Sensitive),
        ];
        let out = f
            .filter(
                "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7b02",
                "p-1",
                ContextPurpose::Search,
                candidates,
            )
            .unwrap();
        let decisions: Vec<_> = out.iter().map(|f| f.decision).collect();
        assert_eq!(
            decisions,
            vec![
                PrivacyFilterDecision::Allow,
                PrivacyFilterDecision::Deny,
                PrivacyFilterDecision::Deny
            ]
        );
        assert!(out[1].reason.as_deref().unwrap().contains("shared-room"));
    }

    #[test]
    fn ep016_unit_private_channel_allows_authorized_private() {
        let mut f = filter(DisclosureContext::PrivateChannel);
        let candidates = vec![
            record("personal", Sensitivity::Personal),
            record("personal", Sensitivity::Sensitive),
        ];
        let out = f
            .filter(
                "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7b02",
                "p-1",
                ContextPurpose::Search,
                candidates,
            )
            .unwrap();
        assert!(
            out.iter()
                .all(|f| f.decision == PrivacyFilterDecision::Allow)
        );
    }

    #[test]
    fn ep016_unit_relevance_never_overrides_disclosure_privacy() {
        let mut f = filter(DisclosureContext::SharedRoom);
        let mut sensitive = record("personal", Sensitivity::Sensitive);
        sensitive.score = 1.0; // most relevant possible
        let out = f
            .filter(
                "0190e1c4-5c8a-7f40-8a1b-2c3d4e5f7b02",
                "p-1",
                ContextPurpose::Search,
                vec![sensitive],
            )
            .unwrap();
        assert_eq!(out[0].decision, PrivacyFilterDecision::Deny);
    }

    #[test]
    fn ep016_unit_principal_tenant_mismatch_fails_closed() {
        let mut f = filter(DisclosureContext::PrivateChannel);
        let err = f
            .filter("other-tenant", "p-1", ContextPurpose::Search, vec![])
            .unwrap_err();
        assert_eq!(err.code, nexus_context::ContextErrorCode::Authorization);
    }

    #[test]
    fn ep016_unit_private_routing_decision_records_boundary() {
        let shared = filter(DisclosureContext::SharedRoom);
        let decision = shared.routing_decision(true);
        assert!(decision.private_route);
        assert!(!decision.delivery_owned);
        let private = filter(DisclosureContext::PrivateChannel);
        let decision = private.routing_decision(true);
        assert!(!decision.private_route);
        assert!(!decision.delivery_owned);
    }
}
