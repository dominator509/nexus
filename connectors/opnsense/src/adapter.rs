//! EP-030 OPNsense adapter core (SPEC-013; M2).
//!
//! Real production adapter behind the nexus-sentinel
//! `FirewallProvider` port: capability advertisement only when the
//! transport answers, governed preauthorized reversible containment
//! (dual gates before any transport call), exact-target verification
//! by independent readback, in-flight idempotency, bounded
//! observability, and fail-closed behavior.
//!
//! Permanent invariants (SPEC-013):
//!
//! - OPNsense is the primary serious firewall (behavior 2) and shares
//!   the canonical FirewallProvider contract with OpenWrt (acceptance
//!   obligation 1).
//! - AUTOMATED CONTAINMENT IS LIMITED TO PREAUTHORIZED HIGH-CONFIDENCE
//!   REVERSIBLE RULES (behavior 5): `apply_containment` fails closed
//!   unless the proposal is preauthorized AND reversible AND approved.
//!   Destructive remediation, credential rotation, wipes, factory
//!   resets, and broad lockouts require human procedure (behavior 6)
//!   and are never automated here.
//! - QUARANTINE IS A PROPOSAL UNTIL APPROVED, APPLIED, AND VERIFIED:
//!   PROPOSED != APPROVED != APPLIED != VERIFIED.
//! - VERIFICATION BINDS TO THE EXACT RULE/DEVICE by independent
//!   readback (searchRule); an unrelated rule never satisfies it.
//! - POLICY BEFORE MUTATION: denied actions make ZERO provider calls.
//! - UNKNOWN OUTCOME -> VERIFY FIRST -> NO BLIND RETRY.
//! - UNBOUND PROVIDERS FAIL CLOSED (Reality rule): no session is
//!   fabricated and no capability is advertised.
//!
//! No test-mode branches exist in production code.

use std::collections::HashMap;
use std::sync::Mutex;

use nexus_domain::{BusinessId, TenantId};
use nexus_sentinel::{
    ContainmentVerification, FirewallProvider, NetworkDevice, NetworkFinding, QuarantineProposal,
    QuarantineState, SentinelCapabilityKind, SentinelCapabilityMap, SentinelError,
    SentinelErrorCode,
};

use crate::observability::{SentinelAuditEntry, SentinelObservability};
use crate::transport::{OpnsenseRulePayload, OpnsenseTransport};

/// In-flight idempotency entry for one containment operation on one
/// proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    idempotency_key: String,
}

/// Real production OPNsense adapter over a real OPNsense transport.
///
/// `Send + Sync`: the transport trait object is required to be
/// shareable so in-flight idempotency can be proven with real
/// concurrent callers.
pub struct OpnsenseFirewallProvider {
    transport: Box<dyn OpnsenseTransport + Send + Sync>,
    tenant_id: TenantId,
    /// Proposal -> source network reference captured at propose time.
    /// The containment rule targets this network; an unknown proposal
    /// fails closed (never fabricates a source).
    sources: Mutex<HashMap<String, String>>,
    in_flight: Mutex<HashMap<String, InFlightEntry>>,
    observability: Mutex<SentinelObservability>,
}

impl OpnsenseFirewallProvider {
    pub fn new(
        transport: Box<dyn OpnsenseTransport + Send + Sync>,
        tenant_id: TenantId,
        api_key: impl Into<String>,
        api_secret: impl Into<String>,
    ) -> Self {
        let api_key = api_key.into();
        let api_secret = api_secret.into();
        // Credentials are registered as redaction secrets so a
        // poisoned error can never leak them into the audit ring. The
        // transport holds the credentials for the Basic auth header.
        Self {
            transport,
            tenant_id,
            sources: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashMap::new()),
            observability: Mutex::new(SentinelObservability::new(256, vec![api_key, api_secret])),
        }
    }

    pub fn audit(&self) -> Vec<SentinelAuditEntry> {
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
        self.observability
            .lock()
            .unwrap()
            .record(SentinelAuditEntry {
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

    fn rule_description(proposal_id: &str) -> String {
        format!("nexus-quarantine-{proposal_id}")
    }

    fn apply_containment_inner(
        &self,
        proposal: &QuarantineProposal,
    ) -> Result<QuarantineProposal, SentinelError> {
        let correlation = self.correlation();

        // Gate 1 (caller-side): the proposal must be APPROVED by a
        // human-governed approval decision before any containment can
        // be applied (SPEC-013 behavior 5/6). A proposal that is still
        // PROPOSED is DATA, never an executed rule.
        if proposal.state != QuarantineState::Approved {
            self.record(
                &correlation,
                "APPLY_CONTAINMENT",
                "POLICY",
                "quarantine proposal is not approved".into(),
                std::collections::BTreeMap::from([(
                    "device".into(),
                    proposal.device_id.to_string(),
                )]),
            );
            return Err(SentinelError::new(
                SentinelErrorCode::Policy,
                "quarantine proposal is not approved",
                Some(correlation.clone()),
                None,
                Some(self.tenant_id.to_string()),
                Some(proposal.proposal_id.to_string()),
            ));
        }

        // Gate 2 (policy): automated containment is limited to
        // preauthorized high-confidence reversible rules (SPEC-013
        // behavior 5). A non-reversible or non-preauthorized rule
        // requires human procedure and fails closed here.
        if !proposal.is_auto_applicable() {
            self.record(
                &correlation,
                "APPLY_CONTAINMENT",
                "POLICY",
                "containment rule is not preauthorized high-confidence reversible".into(),
                std::collections::BTreeMap::from([(
                    "device".into(),
                    proposal.device_id.to_string(),
                )]),
            );
            return Err(SentinelError::new(
                SentinelErrorCode::Policy,
                "containment rule is not preauthorized high-confidence reversible",
                Some(correlation.clone()),
                None,
                Some(self.tenant_id.to_string()),
                Some(proposal.proposal_id.to_string()),
            ));
        }

        // The source network must have been captured at propose time.
        // An unknown proposal fails closed (never fabricates a
        // source).
        let source_net = {
            let sources = self.sources.lock().unwrap();
            sources
                .get(proposal.proposal_id.as_str())
                .cloned()
                .ok_or_else(|| {
                    self.record(
                        &correlation,
                        "APPLY_CONTAINMENT",
                        "NOT_FOUND",
                        "no source network recorded for proposal".into(),
                        std::collections::BTreeMap::from([(
                            "device".into(),
                            proposal.device_id.to_string(),
                        )]),
                    );
                    SentinelError::new(
                        SentinelErrorCode::NotFound,
                        "no source network recorded for proposal",
                        Some(correlation.clone()),
                        None,
                        Some(self.tenant_id.to_string()),
                        Some(proposal.proposal_id.to_string()),
                    )
                })?
        };

        // In-flight idempotency: a duplicate in-flight apply is a
        // Conflict; completion/failure releases the entry.
        let key = proposal.proposal_id.as_str().to_string();
        {
            let mut in_flight = self.in_flight.lock().unwrap();
            if let Some(entry) = in_flight.get(&key) {
                if entry.idempotency_key == proposal.device_id.as_str() {
                    return Err(SentinelError::new(
                        SentinelErrorCode::Conflict,
                        "containment already in flight",
                        Some(correlation.clone()),
                        None,
                        Some(self.tenant_id.to_string()),
                        Some(proposal.proposal_id.to_string()),
                    ));
                }
            }
            in_flight.insert(
                key.clone(),
                InFlightEntry {
                    idempotency_key: proposal.device_id.as_str().to_string(),
                },
            );
        }

        // Gate 3 (provider-side): the transport enforces the
        // authenticated credential and the documented surface. Only
        // NOW does any provider call happen.
        let description = Self::rule_description(proposal.proposal_id.as_str());
        let payload = OpnsenseRulePayload::containment_block(&description, &source_net);
        let result = self.transport.add_rule(&payload).map_err(|e| {
            self.record(
                &correlation,
                "APPLY_CONTAINMENT",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(proposal.proposal_id.to_string())
        });

        // Release the in-flight entry after completion OR failure
        // (bounded retry: retry after completion is not a Conflict).
        {
            let mut in_flight = self.in_flight.lock().unwrap();
            in_flight.remove(&key);
        }

        let rule_uuid = result?;

        // Apply/reload the firewall so the new rule becomes active
        // (documented POST apply). Failure leaves the staged rule; the
        // adapter reports the provider failure (no blind retry).
        self.transport.apply().map_err(|e| {
            self.record(
                &correlation,
                "APPLY_CONTAINMENT",
                "EXTERNAL_PROVIDER",
                format!("rule {} staged but apply failed: {}", rule_uuid, e.message),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(proposal.proposal_id.to_string())
        })?;

        self.record(
            &correlation,
            "APPLY_CONTAINMENT",
            "ok",
            format!("rule {rule_uuid} applied"),
            std::collections::BTreeMap::from([
                ("device".into(), proposal.device_id.to_string()),
                ("action".into(), payload.action.clone()),
            ]),
        );

        Ok(QuarantineProposal {
            state: QuarantineState::Applied,
            ..proposal.clone().with_rule_ref(rule_uuid)
        })
    }

    fn revoke_containment_inner(
        &self,
        proposal: &QuarantineProposal,
    ) -> Result<QuarantineProposal, SentinelError> {
        let correlation = self.correlation();

        // Gate: only an APPLIED containment can be revoked; the rule
        // reference must exist. A proposal is data until applied.
        if proposal.state != QuarantineState::Applied {
            self.record(
                &correlation,
                "REVOKE_CONTAINMENT",
                "POLICY",
                "only an applied containment can be revoked".into(),
                std::collections::BTreeMap::from([(
                    "device".into(),
                    proposal.device_id.to_string(),
                )]),
            );
            return Err(SentinelError::new(
                SentinelErrorCode::Policy,
                "only an applied containment can be revoked",
                Some(correlation.clone()),
                None,
                Some(self.tenant_id.to_string()),
                Some(proposal.proposal_id.to_string()),
            ));
        }
        let rule_uuid = proposal.rule_ref.clone().ok_or_else(|| {
            self.record(
                &correlation,
                "REVOKE_CONTAINMENT",
                "POLICY",
                "applied containment has no rule reference".into(),
                std::collections::BTreeMap::from([(
                    "device".into(),
                    proposal.device_id.to_string(),
                )]),
            );
            SentinelError::new(
                SentinelErrorCode::Policy,
                "applied containment has no rule reference",
                Some(correlation.clone()),
                None,
                Some(self.tenant_id.to_string()),
                Some(proposal.proposal_id.to_string()),
            )
        })?;

        // In-flight idempotency for revoke.
        let key = format!("revoke:{}", proposal.proposal_id.as_str());
        {
            let mut in_flight = self.in_flight.lock().unwrap();
            if let Some(entry) = in_flight.get(&key) {
                if entry.idempotency_key == proposal.device_id.as_str() {
                    return Err(SentinelError::new(
                        SentinelErrorCode::Conflict,
                        "revocation already in flight",
                        Some(correlation.clone()),
                        None,
                        Some(self.tenant_id.to_string()),
                        Some(proposal.proposal_id.to_string()),
                    ));
                }
            }
            in_flight.insert(
                key.clone(),
                InFlightEntry {
                    idempotency_key: proposal.device_id.as_str().to_string(),
                },
            );
        }

        // Disable the rule (reversible: the rule remains in the
        // ruleset, disabled, so re-enablement is possible) and apply.
        let result = self.transport.toggle_rule(&rule_uuid, false).map_err(|e| {
            self.record(
                &correlation,
                "REVOKE_CONTAINMENT",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(proposal.proposal_id.to_string())
        });

        {
            let mut in_flight = self.in_flight.lock().unwrap();
            in_flight.remove(&key);
        }

        result?;

        self.transport.apply().map_err(|e| {
            self.record(
                &correlation,
                "REVOKE_CONTAINMENT",
                "EXTERNAL_PROVIDER",
                format!("rule {rule_uuid} disabled but apply failed: {}", e.message),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(proposal.proposal_id.to_string())
        })?;

        self.record(
            &correlation,
            "REVOKE_CONTAINMENT",
            "ok",
            format!("rule {rule_uuid} disabled"),
            std::collections::BTreeMap::from([("device".into(), proposal.device_id.to_string())]),
        );

        Ok(QuarantineProposal {
            state: QuarantineState::Revoked,
            ..proposal.clone()
        })
    }
}

impl FirewallProvider for OpnsenseFirewallProvider {
    fn capabilities(&self) -> SentinelCapabilityMap {
        // Advertise only when the transport answers (reality rule).
        // An unbound or failing transport advertises nothing.
        let mut map = SentinelCapabilityMap::new();
        if self.transport.search_rules("").is_ok() {
            map.insert(SentinelCapabilityKind::ReadFirewallTelemetry);
            map.insert(SentinelCapabilityKind::Containment);
            map.insert(SentinelCapabilityKind::ProposeQuarantine);
        }
        map
    }

    fn read_telemetry(&self, tenant_id: &TenantId) -> Result<Vec<NetworkFinding>, SentinelError> {
        let correlation = self.correlation();
        let rules = self
            .transport
            .search_rules("nexus-quarantine-")
            .map_err(|e| {
                self.record(
                    &correlation,
                    "READ_TELEMETRY",
                    "EXTERNAL_PROVIDER",
                    e.message.clone(),
                    std::collections::BTreeMap::new(),
                );
                e.with_correlation(correlation.clone())
                    .with_tenant(self.tenant_id.to_string())
            })?;
        let mut findings = Vec::new();
        for rule in rules {
            let finding = NetworkFinding::new(
                nexus_sentinel::NetworkFindingId::new(format!("opnsense:{}", rule.uuid)).map_err(
                    |e| {
                        e.with_correlation(correlation.clone())
                            .with_tenant(self.tenant_id.to_string())
                    },
                )?,
                tenant_id.clone(),
                nexus_sentinel::FindingKind::QuarantineProposed,
                nexus_sentinel::FindingSeverity::Medium,
                format!("opnsense:rule:{}", rule.uuid),
                String::new(),
            )
            .with_correlation(correlation.clone());
            findings.push(finding);
        }
        self.record(
            &correlation,
            "READ_TELEMETRY",
            "ok",
            format!("{} containment rules observed", findings.len()),
            std::collections::BTreeMap::new(),
        );
        Ok(findings)
    }

    fn propose_containment(
        &self,
        tenant_id: &TenantId,
        business_id: Option<&BusinessId>,
        device: &NetworkDevice,
    ) -> Result<QuarantineProposal, SentinelError> {
        let correlation = self.correlation();
        // The proposal is DATA, not an executed rule. Capture the
        // device's provider-neutral label as the source network
        // reference for the later containment rule.
        let proposal = QuarantineProposal::new(
            nexus_sentinel::QuarantineProposalId::new(format!(
                "{}-{}",
                device.device_id.as_str(),
                correlation
            ))
            .map_err(|e| {
                e.with_correlation(correlation.clone())
                    .with_tenant(self.tenant_id.to_string())
            })?,
            tenant_id.clone(),
            device.device_id.clone(),
            nexus_sentinel::NetworkSegment::Quarantine,
            nexus_sentinel::FirewallAction::Drop,
            true,
            true,
            nexus_domain::ApprovalClass::Human,
            String::new(),
        )
        .with_business(business_id.cloned().unwrap_or_else(|| {
            nexus_domain::BusinessId::new("018f0f6f-9c1e-7b6e-8000-000000000003")
                .expect("static business id")
        }))
        .with_correlation(correlation.clone());

        self.sources.lock().unwrap().insert(
            proposal.proposal_id.as_str().to_string(),
            device.label.clone(),
        );

        self.record(
            &correlation,
            "PROPOSE_CONTAINMENT",
            "ok",
            format!("proposal for device {}", device.device_id.as_str()),
            std::collections::BTreeMap::from([
                ("device".into(), device.device_id.to_string()),
                ("segment".into(), device.segment.to_string()),
            ]),
        );
        Ok(proposal)
    }

    fn apply_containment(
        &self,
        proposal: &QuarantineProposal,
    ) -> Result<QuarantineProposal, SentinelError> {
        self.apply_containment_inner(proposal)
    }

    fn verify_containment(
        &self,
        proposal: &QuarantineProposal,
    ) -> Result<ContainmentVerification, SentinelError> {
        let correlation = self.correlation();

        // Verification binds to the exact proposal: search the
        // observed rules by the canonical quarantine description and
        // require the rule reference to match. An unrelated rule never
        // satisfies verification (exact-target).
        let description = Self::rule_description(proposal.proposal_id.as_str());
        let rules = self.transport.search_rules(&description).map_err(|e| {
            self.record(
                &correlation,
                "VERIFY_CONTAINMENT",
                "EXTERNAL_PROVIDER",
                e.message.clone(),
                std::collections::BTreeMap::new(),
            );
            e.with_correlation(correlation.clone())
                .with_tenant(self.tenant_id.to_string())
                .with_resource(proposal.proposal_id.to_string())
        })?;

        let verified = rules
            .iter()
            .find(|r| Some(r.uuid.as_str()) == proposal.rule_ref.as_deref())
            .map(|r| r.enabled && r.action == "block")
            .unwrap_or(false);

        let evidence = if verified {
            rules
                .iter()
                .find(|r| Some(r.uuid.as_str()) == proposal.rule_ref.as_deref())
                .map(|r| format!("opnsense:rule:{}:enabled:block", r.uuid))
                .unwrap_or_else(|| "opnsense:rule:none".to_string())
        } else {
            "opnsense:rule:none".to_string()
        };

        self.record(
            &correlation,
            "VERIFY_CONTAINMENT",
            if verified { "ok" } else { "VERIFICATION" },
            evidence.clone(),
            std::collections::BTreeMap::from([("device".into(), proposal.device_id.to_string())]),
        );

        Ok(ContainmentVerification::new(
            proposal.proposal_id.clone(),
            proposal.device_id.clone(),
            verified,
            evidence,
            String::new(),
        ))
    }

    fn revoke_containment(
        &self,
        proposal: &QuarantineProposal,
    ) -> Result<QuarantineProposal, SentinelError> {
        self.revoke_containment_inner(proposal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_sentinel::{NetworkSegment, QuarantineProposalId};
    use std::str::FromStr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    fn tenant() -> TenantId {
        TenantId::from_str("018f0f6f-9c1e-7b6e-8000-000000000001").unwrap()
    }

    fn device(label: &str) -> NetworkDevice {
        NetworkDevice::new(
            nexus_sentinel::NetworkDeviceId::new(format!("dev-{label}")).unwrap(),
            tenant(),
            NetworkSegment::Iot,
            nexus_sentinel::TrustClass::Unknown,
            label,
            "opnsense",
            "2026-08-20T00:00:00Z",
            "2026-08-20T00:00:00Z",
        )
    }

    /// A counting transport used ONLY in unit tests (TESTING.md test
    /// zone): the peer is controlled, the adapter under test is real.
    #[derive(Clone, Default)]
    struct CountingTransport {
        calls: Arc<AtomicUsize>,
        add_calls: Arc<AtomicUsize>,
        apply_calls: Arc<AtomicUsize>,
        toggle_calls: Arc<AtomicUsize>,
        search_calls: Arc<AtomicUsize>,
        next_uuid: Arc<AtomicUsize>,
        rules: Arc<Mutex<Vec<crate::transport::OpnsenseRule>>>,
        fail_add: bool,
    }

    impl CountingTransport {
        fn add_rule_now(&self, description: &str) -> String {
            let uuid = format!("rule-{}", self.next_uuid.fetch_add(1, Ordering::SeqCst) + 1);
            self.rules
                .lock()
                .unwrap()
                .push(crate::transport::OpnsenseRule {
                    uuid: uuid.clone(),
                    description: description.to_string(),
                    enabled: true,
                    action: "block".into(),
                });
            uuid
        }
    }

    impl OpnsenseTransport for CountingTransport {
        fn search_rules(
            &self,
            phrase: &str,
        ) -> Result<Vec<crate::transport::OpnsenseRule>, SentinelError> {
            self.search_calls.fetch_add(1, Ordering::SeqCst);
            let rules = self.rules.lock().unwrap();
            Ok(rules
                .iter()
                .filter(|r| r.description.contains(phrase))
                .cloned()
                .collect())
        }

        fn add_rule(&self, payload: &OpnsenseRulePayload) -> Result<String, SentinelError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.add_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_add {
                return Err(SentinelError::new(
                    SentinelErrorCode::ExternalProvider,
                    "fixture add failed",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Ok(self.add_rule_now(&payload.description))
        }

        fn toggle_rule(&self, uuid: &str, enabled: bool) -> Result<(), SentinelError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.toggle_calls.fetch_add(1, Ordering::SeqCst);
            let mut rules = self.rules.lock().unwrap();
            for r in rules.iter_mut() {
                if r.uuid == uuid {
                    r.enabled = enabled;
                }
            }
            Ok(())
        }

        fn apply(&self) -> Result<(), SentinelError> {
            self.apply_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn approved_proposal(provider: &OpnsenseFirewallProvider, label: &str) -> QuarantineProposal {
        let d = device(label);
        let proposal = provider.propose_containment(&tenant(), None, &d).unwrap();
        QuarantineProposal {
            state: QuarantineState::Approved,
            ..proposal
        }
    }

    #[test]
    fn ep030_unit_capabilities_fail_closed_when_transport_unavailable() {
        let t = CountingTransport {
            fail_add: false,
            ..Default::default()
        };
        // A transport that answers advertises capabilities.
        let provider =
            OpnsenseFirewallProvider::new(Box::new(t.clone()), tenant(), "key", "secret");
        let caps = provider.capabilities();
        assert!(caps.contains(SentinelCapabilityKind::Containment));

        // An unbound transport advertises nothing.
        struct Unbound;
        impl OpnsenseTransport for Unbound {}
        let provider = OpnsenseFirewallProvider::new(Box::new(Unbound), tenant(), "key", "secret");
        assert!(provider.capabilities().is_empty());
    }

    #[test]
    fn ep030_unit_propose_is_data_not_containment() {
        let t = CountingTransport::default();
        let provider =
            OpnsenseFirewallProvider::new(Box::new(t.clone()), tenant(), "key", "secret");
        let proposal = provider
            .propose_containment(&tenant(), None, &device("thermostat"))
            .unwrap();
        assert_eq!(proposal.state, QuarantineState::Proposed);
        assert!(proposal.is_auto_applicable());
        // No provider calls happened at propose time.
        assert_eq!(t.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ep030_unit_apply_requires_approved_state_zero_calls_on_denial() {
        let t = CountingTransport::default();
        let provider =
            OpnsenseFirewallProvider::new(Box::new(t.clone()), tenant(), "key", "secret");
        let proposal = provider
            .propose_containment(&tenant(), None, &device("thermostat"))
            .unwrap();
        // Not approved: fails closed with ZERO provider calls.
        let err = provider.apply_containment(&proposal).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Policy);
        assert_eq!(t.calls.load(Ordering::SeqCst), 0);
        // The denial is audited with correlation.
        assert!(provider
            .audit()
            .iter()
            .any(|e| e.operation == "APPLY_CONTAINMENT" && e.outcome == "POLICY"));
    }

    #[test]
    fn ep030_unit_apply_requires_reversible_rule_zero_calls_on_denial() {
        let t = CountingTransport::default();
        let provider =
            OpnsenseFirewallProvider::new(Box::new(t.clone()), tenant(), "key", "secret");
        let d = device("thermostat");
        let proposed = provider.propose_containment(&tenant(), None, &d).unwrap();
        let proposal = QuarantineProposal {
            state: QuarantineState::Approved,
            ..proposed
        };
        // Force the proposal to be non-auto-applicable by constructing
        // a non-reversible variant through the model.
        let non_reversible = QuarantineProposal {
            state: QuarantineState::Approved,
            ..QuarantineProposal::new(
                QuarantineProposalId::new("q-nr").unwrap(),
                tenant(),
                d.device_id.clone(),
                NetworkSegment::Quarantine,
                nexus_sentinel::FirewallAction::Drop,
                false,
                false,
                nexus_domain::ApprovalClass::StrongHuman,
                String::new(),
            )
        };
        assert!(!non_reversible.is_auto_applicable());
        let err = provider.apply_containment(&non_reversible).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Policy);
        assert_eq!(t.calls.load(Ordering::SeqCst), 0);
        let _ = proposal;
    }

    #[test]
    fn ep030_unit_apply_approved_reversible_reaches_transport_once() {
        let t = CountingTransport::default();
        let provider =
            OpnsenseFirewallProvider::new(Box::new(t.clone()), tenant(), "key", "secret");
        let proposal = approved_proposal(&provider, "thermostat");
        let applied = provider.apply_containment(&proposal).unwrap();
        assert_eq!(applied.state, QuarantineState::Applied);
        assert!(applied.rule_ref.is_some());
        // addRule + apply exactly once.
        assert_eq!(t.add_calls.load(Ordering::SeqCst), 1);
        assert_eq!(t.apply_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn ep030_unit_verify_binds_exact_rule_and_device() {
        let t = CountingTransport::default();
        let provider =
            OpnsenseFirewallProvider::new(Box::new(t.clone()), tenant(), "key", "secret");
        let proposal = approved_proposal(&provider, "thermostat");
        let applied = provider.apply_containment(&proposal).unwrap();

        // Exact-target: verification reads back the exact rule and
        // finds it enabled with action block.
        let v = provider.verify_containment(&applied).unwrap();
        assert!(v.verified);
        assert_eq!(v.proposal_id, applied.proposal_id);
        assert_eq!(v.device_id, applied.device_id);

        // A proposal with no matching rule never verifies.
        let other = QuarantineProposal {
            state: QuarantineState::Applied,
            rule_ref: Some("rule-999".to_string()),
            ..QuarantineProposal::new(
                QuarantineProposalId::new("q-other").unwrap(),
                tenant(),
                nexus_sentinel::NetworkDeviceId::new("dev-other").unwrap(),
                NetworkSegment::Quarantine,
                nexus_sentinel::FirewallAction::Drop,
                true,
                true,
                nexus_domain::ApprovalClass::Human,
                String::new(),
            )
        };
        let v2 = provider.verify_containment(&other).unwrap();
        assert!(!v2.verified);
    }

    #[test]
    fn ep030_unit_revoke_disables_and_applies() {
        let t = CountingTransport::default();
        let provider =
            OpnsenseFirewallProvider::new(Box::new(t.clone()), tenant(), "key", "secret");
        let proposal = approved_proposal(&provider, "thermostat");
        let applied = provider.apply_containment(&proposal).unwrap();
        let revoked = provider.revoke_containment(&applied).unwrap();
        assert_eq!(revoked.state, QuarantineState::Revoked);
        assert_eq!(t.toggle_calls.load(Ordering::SeqCst), 1);
        // After revoke, verification fails (rule disabled).
        let v = provider.verify_containment(&revoked).unwrap();
        assert!(!v.verified);
    }

    #[test]
    fn ep030_unit_revoke_requires_applied_state() {
        let t = CountingTransport::default();
        let provider =
            OpnsenseFirewallProvider::new(Box::new(t.clone()), tenant(), "key", "secret");
        let proposal = provider
            .propose_containment(&tenant(), None, &device("thermostat"))
            .unwrap();
        let err = provider.revoke_containment(&proposal).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Policy);
        assert_eq!(t.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ep030_unit_redaction_canary_zero_leakage() {
        let t = CountingTransport::default();
        let provider = OpnsenseFirewallProvider::new(
            Box::new(t.clone()),
            tenant(),
            "sekret-api-key",
            "sekret-api-secret",
        );
        let _ = provider.capabilities();
        // Poison an audit entry with the credentials: they must be
        // redacted in the ring.
        provider.record(
            &provider.correlation(),
            "POISON",
            "ok",
            "key=sekret-api-key secret=sekret-api-secret".into(),
            std::collections::BTreeMap::from([
                ("k".into(), "sekret-api-key".into()),
                ("s".into(), "sekret-api-secret".into()),
            ]),
        );
        let audit = provider.audit();
        let joined = serde_json::to_string(&audit).unwrap();
        assert!(!joined.contains("sekret-api-key"));
        assert!(!joined.contains("sekret-api-secret"));
    }

    #[test]
    fn ep030_unit_inflight_duplicate_is_conflict_and_release_after_end() {
        // Use a transport that blocks inside add_rule to prove the
        // in-flight guard.
        use std::sync::mpsc;
        struct BlockingTransport {
            entered: Arc<AtomicUsize>,
            release: Arc<Mutex<Option<mpsc::Sender<()>>>>,
            calls: Arc<AtomicUsize>,
        }
        impl OpnsenseTransport for BlockingTransport {
            fn search_rules(
                &self,
                _phrase: &str,
            ) -> Result<Vec<crate::transport::OpnsenseRule>, SentinelError> {
                Ok(vec![])
            }
            fn add_rule(&self, _payload: &OpnsenseRulePayload) -> Result<String, SentinelError> {
                self.entered.fetch_add(1, Ordering::SeqCst);
                self.calls.fetch_add(1, Ordering::SeqCst);
                let (tx, rx) = mpsc::channel();
                *self.release.lock().unwrap() = Some(tx);
                rx.recv().map_err(|_| {
                    SentinelError::new(
                        SentinelErrorCode::ExternalProvider,
                        "released",
                        None,
                        None,
                        None,
                        None,
                    )
                })?;
                Ok("rule-1".into())
            }
            fn toggle_rule(&self, _uuid: &str, _enabled: bool) -> Result<(), SentinelError> {
                Ok(())
            }
            fn apply(&self) -> Result<(), SentinelError> {
                Ok(())
            }
        }

        let entered = Arc::new(AtomicUsize::new(0));
        let release: Arc<Mutex<Option<mpsc::Sender<()>>>> = Arc::new(Mutex::new(None));
        let calls = Arc::new(AtomicUsize::new(0));
        let transport = BlockingTransport {
            entered: entered.clone(),
            release: release.clone(),
            calls: calls.clone(),
        };
        let provider = std::sync::Arc::new(OpnsenseFirewallProvider::new(
            Box::new(transport),
            tenant(),
            "key",
            "secret",
        ));
        let proposal = approved_proposal(&provider, "thermostat");

        let p1 = proposal.clone();
        let p2 = proposal.clone();
        let provider1 = provider.clone();
        let h1 = std::thread::spawn(move || provider1.apply_containment(&p1));
        // Wait for the first call to enter the transport.
        while entered.load(Ordering::SeqCst) == 0 {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let provider2 = provider.clone();
        let h2 = std::thread::spawn(move || provider2.apply_containment(&p2));
        // The duplicate must be a Conflict while the first is in
        // flight.
        let second = h2.join().unwrap();
        assert_eq!(second.unwrap_err().code, SentinelErrorCode::Conflict);
        // Release the first; it completes and the in-flight entry is
        // removed (a retry after completion is NOT a Conflict).
        release.lock().unwrap().take().unwrap().send(()).unwrap();
        let first = h1.join().unwrap();
        assert!(first.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
