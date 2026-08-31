//! EP-030 OpenWrt adapter core (SPEC-013; M3).
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
//! - OpenWrt is supported for embedded and consumer installations
//!   (behavior 2) and shares the canonical FirewallProvider contract
//!   with OPNsense (acceptance obligation 1).
//! - AUTOMATED CONTAINMENT IS LIMITED TO PREAUTHORIZED HIGH-CONFIDENCE
//!   REVERSIBLE RULES (behavior 5): `apply_containment` fails closed
//!   unless the proposal is preauthorized AND reversible AND approved.
//!   Destructive remediation, credential rotation, wipes, factory
//!   resets, and broad lockouts require human procedure (behavior 6)
//!   and are never automated here.
//! - QUARANTINE IS A PROPOSAL UNTIL APPROVED, APPLIED, AND VERIFIED:
//!   PROPOSED != APPROVED != APPLIED != VERIFIED.
//! - VERIFICATION BINDS TO THE EXACT RULE/DEVICE by independent
//!   readback (uci get); an unrelated rule never satisfies it.
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
use crate::transport::{OpenWrtRulePayload, OpenWrtTransport};

/// In-flight idempotency entry for one containment operation on one
/// proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    idempotency_key: String,
}

/// Real production OpenWrt adapter over a real OpenWrt transport.
///
/// `Send + Sync`: the transport trait object is required to be
/// shareable so in-flight idempotency can be proven with real
/// concurrent callers.
pub struct OpenWrtFirewallProvider {
    transport: Box<dyn OpenWrtTransport + Send + Sync>,
    tenant_id: TenantId,
    /// Proposal -> source network reference captured at propose time.
    /// The containment rule targets this network; an unknown proposal
    /// fails closed (never fabricates a source).
    sources: Mutex<HashMap<String, String>>,
    in_flight: Mutex<HashMap<String, InFlightEntry>>,
    observability: Mutex<SentinelObservability>,
}

impl OpenWrtFirewallProvider {
    pub fn new(
        transport: Box<dyn OpenWrtTransport + Send + Sync>,
        tenant_id: TenantId,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        let username = username.into();
        let password = password.into();
        // Credentials are registered as redaction secrets so a
        // poisoned error can never leak them into the audit ring. The
        // transport holds the credentials for the login call.
        Self {
            transport,
            tenant_id,
            sources: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashMap::new()),
            observability: Mutex::new(SentinelObservability::new(256, vec![username, password])),
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

    fn rule_name(proposal_id: &str) -> String {
        format!("nexus-quarantine-{proposal_id}")
    }

    fn apply_containment_inner(
        &self,
        proposal: &QuarantineProposal,
    ) -> Result<QuarantineProposal, SentinelError> {
        let correlation = self.correlation();

        // Gate 1 (caller-side): the proposal must carry an IMMUTABLE
        // approval receipt binding the exact action (AUD-025).
        // Mutating `state` to `Approved` alone is forgeable state,
        // never authority - the receipt's action digest must match
        // THIS proposal and the approver's strength must meet the
        // required class. A proposal that is still PROPOSED is DATA,
        // never an executed rule.
        if !proposal.approval_binds() {
            self.record(
                &correlation,
                "APPLY_CONTAINMENT",
                "POLICY",
                "quarantine proposal lacks a matching immutable approval receipt".into(),
                std::collections::BTreeMap::from([(
                    "device".into(),
                    proposal.device_id.to_string(),
                )]),
            );
            return Err(SentinelError::new(
                SentinelErrorCode::Policy,
                "quarantine proposal lacks a matching immutable approval receipt",
                Some(correlation.clone()),
                None,
                Some(self.tenant_id.to_string()),
                Some(proposal.proposal_id.to_string()),
            ));
        }
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

        // Gate 3 (provider-side): login first (session state), then
        // the transport enforces the authenticated surface. Only NOW
        // does any provider mutation happen.
        let session = self.transport.login().map_err(|e| {
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
        })?;

        let name = Self::rule_name(proposal.proposal_id.as_str());
        let payload = OpenWrtRulePayload::containment_drop(&name, &source_net);
        let result = self.transport.add_rule(&session, &payload).map_err(|e| {
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

        let section = result?;

        // Reload the firewall so the new rule becomes active. Failure
        // leaves the staged rule; the adapter reports the provider
        // failure (no blind retry).
        self.transport.reload_firewall(&session).map_err(|e| {
            self.record(
                &correlation,
                "APPLY_CONTAINMENT",
                "EXTERNAL_PROVIDER",
                format!("rule {section} staged but reload failed: {}", e.message),
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
            format!("rule {section} applied"),
            std::collections::BTreeMap::from([
                ("device".into(), proposal.device_id.to_string()),
                ("target".into(), payload.target.clone()),
            ]),
        );

        Ok(QuarantineProposal {
            state: QuarantineState::Applied,
            ..proposal.clone().with_rule_ref(section)
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
        let section = proposal.rule_ref.clone().ok_or_else(|| {
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

        let session = self.transport.login().map_err(|e| {
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
        })?;

        // Disable the rule (reversible: the rule remains in the
        // config, disabled, so re-enablement is possible) and reload.
        let result = self
            .transport
            .toggle_rule(&session, &section, false)
            .map_err(|e| {
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

        self.transport.reload_firewall(&session).map_err(|e| {
            self.record(
                &correlation,
                "REVOKE_CONTAINMENT",
                "EXTERNAL_PROVIDER",
                format!("rule {section} disabled but reload failed: {}", e.message),
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
            format!("rule {section} disabled"),
            std::collections::BTreeMap::from([("device".into(), proposal.device_id.to_string())]),
        );

        Ok(QuarantineProposal {
            state: QuarantineState::Revoked,
            ..proposal.clone()
        })
    }
}

impl FirewallProvider for OpenWrtFirewallProvider {
    fn capabilities(&self) -> SentinelCapabilityMap {
        // Advertise only when the transport answers (reality rule).
        // An unbound or failing transport advertises nothing.
        let mut map = SentinelCapabilityMap::new();
        if self.transport.login().is_ok() {
            map.insert(SentinelCapabilityKind::ReadFirewallTelemetry);
            map.insert(SentinelCapabilityKind::Containment);
            map.insert(SentinelCapabilityKind::ProposeQuarantine);
        }
        map
    }

    fn read_telemetry(&self, tenant_id: &TenantId) -> Result<Vec<NetworkFinding>, SentinelError> {
        let correlation = self.correlation();
        let session = self.transport.login().map_err(|e| {
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
        let rules = self.transport.list_rules(&session).map_err(|e| {
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
            if !rule.name.starts_with("nexus-quarantine-") {
                continue;
            }
            let finding = NetworkFinding::new(
                nexus_sentinel::NetworkFindingId::new(format!("openwrt:{}", rule.section))
                    .map_err(|e| {
                        e.with_correlation(correlation.clone())
                            .with_tenant(self.tenant_id.to_string())
                    })?,
                tenant_id.clone(),
                nexus_sentinel::FindingKind::QuarantineProposed,
                nexus_sentinel::FindingSeverity::Medium,
                format!("openwrt:rule:{}", rule.section),
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
        observed_source: Option<&str>,
    ) -> Result<QuarantineProposal, SentinelError> {
        let correlation = self.correlation();
        // AUD-026: the containment rule MUST bind the OBSERVED network
        // identity (the device fingerprint's ip_ref), never the
        // display label. Without an observed source the proposal fails
        // closed - a label is not a network identity.
        let Some(observed_source) = observed_source.map(str::trim).filter(|s| !s.is_empty()) else {
            self.record(
                &correlation,
                "PROPOSE_CONTAINMENT",
                "NOT_FOUND",
                "no observed network identity for device".into(),
                std::collections::BTreeMap::from([("device".into(), device.device_id.to_string())]),
            );
            return Err(SentinelError::new(
                SentinelErrorCode::NotFound,
                "no observed network identity for device",
                Some(correlation.clone()),
                None,
                Some(self.tenant_id.to_string()),
                Some(device.device_id.to_string()),
            ));
        };
        // The proposal is DATA, not an executed rule. Capture the
        // device's OBSERVED source as the network identity for the
        // later containment rule.
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
        .with_source_net(observed_source)
        .with_business(business_id.cloned().unwrap_or_else(|| {
            nexus_domain::BusinessId::new("018f0f6f-9c1e-7b6e-8000-000000000003")
                .expect("static business id")
        }))
        .with_correlation(correlation.clone());

        self.sources.lock().unwrap().insert(
            proposal.proposal_id.as_str().to_string(),
            observed_source.to_string(),
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

        // Verification binds to the exact proposal: login, read the
        // firewall rule sections, and require the rule reference
        // (section) to exist, enabled, with target DROP. An unrelated
        // rule never satisfies verification (exact-target).
        let session = self.transport.login().map_err(|e| {
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
        let rules = self.transport.list_rules(&session).map_err(|e| {
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

        // AUD-026: verification proves the rule binds the OBSERVED
        // network identity (the fingerprint's ip_ref), not just a rule
        // section. The matching rule must carry the exact observed
        // source; a rule that is enabled DROP but binds a different
        // (or no) source is NOT verified.
        let expected_source = proposal.source_net.as_deref().unwrap_or("");
        let verified = rules
            .iter()
            .find(|r| Some(r.section.as_str()) == proposal.rule_ref.as_deref())
            .map(|r| {
                r.enabled
                    && r.target == "DROP"
                    && r.src_ip.as_deref() == Some(expected_source)
                    && !expected_source.is_empty()
            })
            .unwrap_or(false);

        let evidence = if verified {
            rules
                .iter()
                .find(|r| Some(r.section.as_str()) == proposal.rule_ref.as_deref())
                .map(|r| {
                    format!(
                        "openwrt:rule:{}:enabled:drop:source={}",
                        r.section,
                        r.src_ip.as_deref().unwrap_or("")
                    )
                })
                .unwrap_or_else(|| "openwrt:rule:none".to_string())
        } else {
            "openwrt:rule:none".to_string()
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
            "openwrt",
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
        toggle_calls: Arc<AtomicUsize>,
        reload_calls: Arc<AtomicUsize>,
        login_calls: Arc<AtomicUsize>,
        next_section: Arc<AtomicUsize>,
        rules: Arc<Mutex<Vec<crate::transport::OpenWrtRule>>>,
        fail_login: bool,
    }

    impl CountingTransport {
        fn add_rule_now(&self, name: &str) -> String {
            let section = format!(
                "cfg{}",
                self.next_section.fetch_add(1, Ordering::SeqCst) + 1
            );
            self.rules
                .lock()
                .unwrap()
                .push(crate::transport::OpenWrtRule {
                    section: section.clone(),
                    name: name.to_string(),
                    target: "DROP".into(),
                    src_ip: Some("192.0.2.10".into()),
                    enabled: true,
                });
            section
        }
    }

    impl OpenWrtTransport for CountingTransport {
        fn login(&self) -> Result<String, SentinelError> {
            self.login_calls.fetch_add(1, Ordering::SeqCst);
            if self.fail_login {
                return Err(SentinelError::new(
                    SentinelErrorCode::Authorization,
                    "fixture login denied",
                    None,
                    None,
                    None,
                    None,
                ));
            }
            Ok("session-1".into())
        }

        fn list_rules(
            &self,
            _session: &str,
        ) -> Result<Vec<crate::transport::OpenWrtRule>, SentinelError> {
            Ok(self.rules.lock().unwrap().clone())
        }

        fn add_rule(
            &self,
            _session: &str,
            payload: &OpenWrtRulePayload,
        ) -> Result<String, SentinelError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.add_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.add_rule_now(&payload.name))
        }

        fn toggle_rule(
            &self,
            _session: &str,
            section: &str,
            enabled: bool,
        ) -> Result<(), SentinelError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.toggle_calls.fetch_add(1, Ordering::SeqCst);
            let mut rules = self.rules.lock().unwrap();
            for r in rules.iter_mut() {
                if r.section == section {
                    r.enabled = enabled;
                }
            }
            Ok(())
        }

        fn reload_firewall(&self, _session: &str) -> Result<(), SentinelError> {
            self.reload_calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn approved_proposal(provider: &OpenWrtFirewallProvider, label: &str) -> QuarantineProposal {
        let d = device(label);
        let proposal = provider
            .propose_containment(&tenant(), None, &d, Some("192.0.2.10"))
            .unwrap();
        // AUD-025: approval is an immutable receipt binding the exact
        // action - never a bare state mutation. The helper must go
        // through the real approve() binding.
        proposal.approve(
            nexus_domain::ApprovalId::new("0190e1c4-5c8a-7f40-8a1b-2c3d4e5f6105").unwrap(),
            nexus_domain::PersonId::from_str("018f0f6f-9c1e-7b6e-8000-000000000002").unwrap(),
            nexus_domain::ApprovalClass::Human,
            "2026-08-20T00:00:00Z",
        )
    }

    #[test]
    fn ep030_unit_capabilities_fail_closed_when_transport_unavailable() {
        let t = CountingTransport::default();
        let provider = OpenWrtFirewallProvider::new(Box::new(t.clone()), tenant(), "root", "pass");
        let caps = provider.capabilities();
        assert!(caps.contains(SentinelCapabilityKind::Containment));

        let failing = CountingTransport {
            fail_login: true,
            ..Default::default()
        };
        let provider =
            OpenWrtFirewallProvider::new(Box::new(failing.clone()), tenant(), "root", "bad");
        assert!(provider.capabilities().is_empty());
    }

    #[test]
    fn ep030_unit_propose_is_data_not_containment() {
        let t = CountingTransport::default();
        let provider = OpenWrtFirewallProvider::new(Box::new(t.clone()), tenant(), "root", "pass");
        let proposal = provider
            .propose_containment(&tenant(), None, &device("thermostat"), Some("192.0.2.10"))
            .unwrap();
        assert_eq!(proposal.state, QuarantineState::Proposed);
        assert!(proposal.is_auto_applicable());
        // No provider calls happened at propose time.
        assert_eq!(t.calls.load(Ordering::SeqCst), 0);
        assert_eq!(t.login_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ep030_unit_apply_requires_approved_state_zero_calls_on_denial() {
        let t = CountingTransport::default();
        let provider = OpenWrtFirewallProvider::new(Box::new(t.clone()), tenant(), "root", "pass");
        let proposal = provider
            .propose_containment(&tenant(), None, &device("thermostat"), Some("192.0.2.10"))
            .unwrap();
        // Not approved: fails closed with ZERO provider calls.
        let err = provider.apply_containment(&proposal).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Policy);
        assert_eq!(t.calls.load(Ordering::SeqCst), 0);
        assert_eq!(t.login_calls.load(Ordering::SeqCst), 0);
        assert!(provider
            .audit()
            .iter()
            .any(|e| e.operation == "APPLY_CONTAINMENT" && e.outcome == "POLICY"));
    }

    #[test]
    fn ep030_unit_apply_approved_reversible_reaches_transport_once() {
        let t = CountingTransport::default();
        let provider = OpenWrtFirewallProvider::new(Box::new(t.clone()), tenant(), "root", "pass");
        let proposal = approved_proposal(&provider, "thermostat");
        let applied = provider.apply_containment(&proposal).unwrap();
        assert_eq!(applied.state, QuarantineState::Applied);
        assert!(applied.rule_ref.is_some());
        // login + add_rule + reload exactly once.
        assert_eq!(t.login_calls.load(Ordering::SeqCst), 1);
        assert_eq!(t.add_calls.load(Ordering::SeqCst), 1);
        assert_eq!(t.reload_calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn ep030_unit_verify_binds_exact_rule_and_device() {
        let t = CountingTransport::default();
        let provider = OpenWrtFirewallProvider::new(Box::new(t.clone()), tenant(), "root", "pass");
        let proposal = approved_proposal(&provider, "thermostat");
        let applied = provider.apply_containment(&proposal).unwrap();

        // Exact-target: verification reads back the exact rule and
        // finds it enabled with target DROP.
        let v = provider.verify_containment(&applied).unwrap();
        assert!(v.verified);
        assert_eq!(v.proposal_id, applied.proposal_id);
        assert_eq!(v.device_id, applied.device_id);

        // A proposal with no matching rule never verifies.
        let other = QuarantineProposal {
            state: QuarantineState::Applied,
            rule_ref: Some("cfg999".to_string()),
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
    fn ep030_unit_revoke_disables_and_reloads() {
        let t = CountingTransport::default();
        let provider = OpenWrtFirewallProvider::new(Box::new(t.clone()), tenant(), "root", "pass");
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
        let provider = OpenWrtFirewallProvider::new(Box::new(t.clone()), tenant(), "root", "pass");
        let proposal = provider
            .propose_containment(&tenant(), None, &device("thermostat"), Some("192.0.2.10"))
            .unwrap();
        let err = provider.revoke_containment(&proposal).unwrap_err();
        assert_eq!(err.code, SentinelErrorCode::Policy);
        assert_eq!(t.calls.load(Ordering::SeqCst), 0);
        assert_eq!(t.login_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ep030_unit_redaction_canary_zero_leakage() {
        let t = CountingTransport::default();
        let provider = OpenWrtFirewallProvider::new(
            Box::new(t.clone()),
            tenant(),
            "sekret-user",
            "sekret-pass",
        );
        let _ = provider.capabilities();
        provider.record(
            &provider.correlation(),
            "POISON",
            "ok",
            "user=sekret-user pass=sekret-pass".into(),
            std::collections::BTreeMap::from([
                ("u".into(), "sekret-user".into()),
                ("p".into(), "sekret-pass".into()),
            ]),
        );
        let audit = provider.audit();
        let joined = serde_json::to_string(&audit).unwrap();
        assert!(!joined.contains("sekret-user"));
        assert!(!joined.contains("sekret-pass"));
    }

    #[test]
    fn ep030_unit_read_telemetry_observes_containment_rules() {
        let t = CountingTransport::default();
        let provider = OpenWrtFirewallProvider::new(Box::new(t.clone()), tenant(), "root", "pass");
        // Apply one containment rule, then telemetry observes it.
        let proposal = approved_proposal(&provider, "thermostat");
        let _applied = provider.apply_containment(&proposal).unwrap();
        let findings = provider.read_telemetry(&tenant()).unwrap();
        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].kind,
            nexus_sentinel::FindingKind::QuarantineProposed
        );
    }
}
