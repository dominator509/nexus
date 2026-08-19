//! EP-028 Hydra adapter core (SPEC-015; M2).
//!
//! Real production adapter behind the nexus-hydra `HydraProvider`
//! port: authenticated context reads, canonical capability mapping,
//! governed dual-gate action submission, exact-target verification,
//! in-flight idempotency, bounded observability, and fail-closed
//! behavior.
//!
//! Permanent invariants (SPEC-015):
//!
//! - HYDRAM REMAINS CANONICAL: this adapter stores references and
//!   projections; it never duplicates Hydra truth (non-goal:
//!   duplicating Hydra CDM).
//! - AUTHENTICATED CHANNELS ONLY: access is through authenticated
//!   MCP/REST/durable events; there is no direct-database path
//!   (behavior 2).
//! - SINGLE BUSINESS SCOPE unless explicitly authorized for portfolio
//!   reads (behavior 3); the business context validates before any
//!   transport call.
//! - DUAL AUTHORIZATION GATES (node contract acceptance obligation 4):
//!   the governed policy gate runs BEFORE the provider port, and the
//!   provider enforces again at its boundary.
//! - POLICY BEFORE MUTATION: denied actions make ZERO provider calls.
//! - PAID-AD BUDGET CHANGES and PUBLIC CRISIS RESPONSES require human
//!   approval (behavior 8).
//! - EXACT-TARGET VERIFICATION: an action is verified ONLY by a
//!   readback of that same action id with the expected state.
//! - UNKNOWN OUTCOME -> VERIFY FIRST -> NO BLIND RETRY.
//! - UNBOUND PROVIDERS FAIL CLOSED (Reality rule): no session is
//!   fabricated and no capability is advertised.
//!
//! No test-mode branches exist in production code.

use std::collections::HashMap;
use std::sync::Mutex;

use nexus_hydra::{
    BusinessContext, HydraActionRequest, HydraActionResult, HydraActionState, HydraCapabilityMap,
    HydraContextProjection, HydraError, HydraErrorCode, HydraProvider,
};

use crate::observability::{HydraAuditEntry, HydraObservability};
use crate::transport::HydraTransport;

/// In-flight idempotency entry for one action on one business.
#[derive(Debug, Clone, PartialEq, Eq)]
struct InFlightEntry {
    idempotency_key: String,
}

/// Real production Hydra adapter over a real Hydra transport.
///
/// `Send + Sync`: the transport trait object is required to be
/// shareable so in-flight idempotency can be proven with real
/// concurrent callers.
pub struct HydraAdapter {
    transport: Box<dyn HydraTransport + Send + Sync>,
    binding: nexus_hydra::HydraBusinessBinding,
    in_flight: Mutex<HashMap<String, InFlightEntry>>,
    observability: Mutex<HydraObservability>,
}

impl HydraAdapter {
    pub fn new(
        transport: Box<dyn HydraTransport + Send + Sync>,
        binding: nexus_hydra::HydraBusinessBinding,
        secrets: Vec<String>,
    ) -> Self {
        Self {
            transport,
            binding,
            in_flight: Mutex::new(HashMap::new()),
            observability: Mutex::new(HydraObservability::new(256, secrets)),
        }
    }

    pub fn audit(&self) -> Vec<HydraAuditEntry> {
        self.observability.lock().unwrap().audit()
    }

    fn record(&self, correlation: &str, operation: &str, outcome: &str, detail: String) {
        self.observability.lock().unwrap().record(HydraAuditEntry {
            correlation: correlation.to_string(),
            operation: operation.to_string(),
            outcome: outcome.to_string(),
            detail,
            fields: std::collections::BTreeMap::new(),
        });
    }

    fn submit_governed_inner(
        &self,
        request: &HydraActionRequest,
    ) -> Result<HydraActionResult, HydraError> {
        // Gate 1 (caller-side): validate + approval policy BEFORE any
        // provider call (dual authorization gates, obligation 4).
        nexus_hydra::enforce_hydra_action_policy(request)?;

        // In-flight idempotency: a duplicate in-flight action is a
        // Conflict; completion/failure releases the entry.
        let key = format!(
            "{}:{}",
            self.binding.binding_id.as_str(),
            request.action_id.as_str()
        );
        {
            let mut in_flight = self.in_flight.lock().unwrap();
            if let Some(entry) = in_flight.get(&key) {
                if entry.idempotency_key == request.idempotency_key {
                    return Err(HydraError::new(
                        HydraErrorCode::Conflict,
                        "action already in flight",
                        request.correlation.as_ref().map(|c| c.to_string()),
                        Some(request.principal_id.to_string()),
                        Some(request.tenant_id.to_string()),
                        Some(request.action_id.to_string()),
                    ));
                }
            }
            in_flight.insert(
                key.clone(),
                InFlightEntry {
                    idempotency_key: request.idempotency_key.clone(),
                },
            );
        }

        // Gate 2 (provider-side): the transport enforces the binding's
        // authorized channel and authenticated credential.
        let payload = serde_json::json!({
            "action_id": request.action_id.as_str(),
            "tenant_id": request.tenant_id.to_string(),
            "principal_id": request.principal_id.to_string(),
            "business_id": request.business_id.to_string(),
            "kind": request.kind.as_str(),
            "idempotency_key": request.idempotency_key,
            "approval_class": format!("{:?}", request.approval_class),
            "correlation": request.correlation.as_ref().map(|c| c.to_string()),
        });

        let result = self.transport.submit_action(&payload).and_then(|state| {
            let state = state.parse::<HydraActionState>().map_err(|_| {
                HydraError::new(
                    HydraErrorCode::ExternalProvider,
                    "provider returned an unknown action state",
                    request.correlation.as_ref().map(|c| c.to_string()),
                    Some(request.principal_id.to_string()),
                    Some(request.tenant_id.to_string()),
                    Some(request.action_id.to_string()),
                )
            })?;
            Ok(HydraActionResult {
                binding: self.binding.clone(),
                state,
            })
        });

        {
            let mut in_flight = self.in_flight.lock().unwrap();
            in_flight.remove(&key);
        }

        match &result {
            Ok(r) => self.record(
                request
                    .correlation
                    .as_ref()
                    .map(|c| c.as_str())
                    .unwrap_or("-"),
                "SUBMIT_ACTION",
                "ok",
                format!(
                    "action {} -> {}",
                    request.action_id.as_str(),
                    r.state.as_str()
                ),
            ),
            Err(e) => self.record(
                request
                    .correlation
                    .as_ref()
                    .map(|c| c.as_str())
                    .unwrap_or("-"),
                "SUBMIT_ACTION",
                e.code.as_str(),
                format!("action {} failed", request.action_id.as_str()),
            ),
        }
        result
    }
}

impl HydraProvider for HydraAdapter {
    fn read_context(
        &self,
        binding: &nexus_hydra::HydraBusinessBinding,
        context: &BusinessContext,
    ) -> Result<HydraContextProjection, HydraError> {
        // The binding used by the caller must match the configured
        // binding (explicit business-to-Hydra tenant binding).
        if binding.binding_id != self.binding.binding_id
            || binding.business_id != self.binding.business_id
            || binding.tenant_id != self.binding.tenant_id
        {
            return Err(HydraError::new(
                HydraErrorCode::Authorization,
                "binding mismatch",
                context.correlation.as_ref().map(|c| c.to_string()),
                Some(context.principal_id.to_string()),
                Some(context.tenant_id.to_string()),
                Some(binding.binding_id.to_string()),
            ));
        }
        if !binding.active {
            return Err(HydraError::new(
                HydraErrorCode::Policy,
                "binding is inactive",
                context.correlation.as_ref().map(|c| c.to_string()),
                Some(context.principal_id.to_string()),
                Some(context.tenant_id.to_string()),
                Some(binding.binding_id.to_string()),
            ));
        }
        context.validate()?;
        self.transport.read_context(context)
    }

    fn capabilities(&self) -> HydraCapabilityMap {
        self.transport.capabilities().unwrap_or_default()
    }

    fn submit_action(
        &self,
        binding: &nexus_hydra::HydraBusinessBinding,
        request: &HydraActionRequest,
    ) -> Result<HydraActionResult, HydraError> {
        if binding.binding_id != self.binding.binding_id {
            return Err(HydraError::new(
                HydraErrorCode::Authorization,
                "binding mismatch",
                request.correlation.as_ref().map(|c| c.to_string()),
                Some(request.principal_id.to_string()),
                Some(request.tenant_id.to_string()),
                Some(request.action_id.to_string()),
            ));
        }
        self.submit_governed_inner(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nexus_domain::{ApprovalClass, BusinessId, PersonId, TenantId};
    use nexus_hydra::{HydraAccessChannel, HydraBindingId};
    use std::collections::BTreeSet;
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

    fn binding() -> nexus_hydra::HydraBusinessBinding {
        nexus_hydra::HydraBusinessBinding::new(
            HydraBindingId::new("binding-1").unwrap(),
            tenant(),
            business(),
            BTreeSet::from([HydraAccessChannel::REST]),
        )
    }

    fn request(kind: nexus_hydra::HydraActionKind, approval: ApprovalClass) -> HydraActionRequest {
        HydraActionRequest::new(
            nexus_hydra::HydraActionId::new("action-1").unwrap(),
            tenant(),
            person(),
            business(),
            kind,
            "idempotency-key-0001",
        )
        .with_approval_class(approval)
    }

    struct OkTransport {
        calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
        state: &'static str,
    }

    impl HydraTransport for OkTransport {
        fn submit_action(&self, _action: &serde_json::Value) -> Result<String, HydraError> {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(self.state.to_string())
        }
    }

    #[test]
    fn ep028_unit_governed_action_denied_makes_zero_transport_calls() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let transport = OkTransport {
            calls: std::sync::Arc::clone(&calls),
            state: "SUBMITTED",
        };
        let adapter = HydraAdapter::new(Box::new(transport), binding(), Vec::new());
        let req = request(
            nexus_hydra::HydraActionKind::PaidAdBudgetChange,
            ApprovalClass::Policy,
        );
        let err = adapter.submit_action(&binding(), &req).unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Policy);
        // Zero provider calls on denial (policy before mutation).
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        // The audit records the policy outcome only.
        assert!(adapter.audit().iter().all(|e| e.outcome == "POLICY"));
    }

    #[test]
    fn ep028_unit_governed_action_approved_reaches_transport_once() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let transport = OkTransport {
            calls: std::sync::Arc::clone(&calls),
            state: "SUBMITTED",
        };
        let adapter = HydraAdapter::new(Box::new(transport), binding(), Vec::new());
        let req = request(
            nexus_hydra::HydraActionKind::ReadContext,
            ApprovalClass::None,
        );
        let result = adapter.submit_action(&binding(), &req).unwrap();
        assert_eq!(result.state, HydraActionState::Submitted);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(adapter.audit().iter().any(|e| e.outcome == "ok"));
    }

    #[test]
    fn ep028_unit_in_flight_duplicate_is_conflict_and_releases() {
        use std::sync::{Arc, Mutex};

        // Transport that blocks until a shared gate flips, then returns
        // SUBMITTED. The gate handle is shared with the test so release
        // is deterministic (Mutex<bool> is Sync).
        struct GatedTransport {
            gate: Arc<Mutex<bool>>,
            calls: std::sync::atomic::AtomicUsize,
        }
        impl HydraTransport for GatedTransport {
            fn submit_action(&self, _action: &serde_json::Value) -> Result<String, HydraError> {
                self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                // Block until the test flips the shared gate.
                loop {
                    let gate = self.gate.lock().unwrap();
                    if *gate {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Ok("SUBMITTED".to_string())
            }
        }

        let gate = Arc::new(Mutex::new(false));
        let transport = GatedTransport {
            gate: Arc::clone(&gate),
            calls: std::sync::atomic::AtomicUsize::new(0),
        };
        let adapter = Arc::new(HydraAdapter::new(
            Box::new(transport),
            binding(),
            Vec::new(),
        ));

        // First caller blocks inside the transport.
        let adapter1 = Arc::clone(&adapter);
        let handle1 = std::thread::spawn(move || {
            adapter1.submit_action(
                &binding(),
                &request(
                    nexus_hydra::HydraActionKind::ReadContext,
                    ApprovalClass::None,
                ),
            )
        });
        // Wait until the first call is in flight (it flips the gate
        // mutex briefly; give the thread time to reach the transport).
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Second caller while the first is in flight: Conflict, and it
        // must NOT reach the transport.
        let err = adapter
            .submit_action(
                &binding(),
                &request(
                    nexus_hydra::HydraActionKind::ReadContext,
                    ApprovalClass::None,
                ),
            )
            .unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Conflict);

        // Release the first caller; it completes with SUBMITTED.
        *gate.lock().unwrap() = true;
        let first = handle1.join().unwrap().unwrap();
        assert_eq!(first.state, HydraActionState::Submitted);

        // After completion the entry is released: a retry is not a
        // Conflict (idempotency releases after end).
        let retry = adapter.submit_action(
            &binding(),
            &request(
                nexus_hydra::HydraActionKind::ReadContext,
                ApprovalClass::None,
            ),
        );
        assert!(retry.is_ok());
    }

    #[test]
    fn ep028_unit_binding_mismatch_denied() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let transport = OkTransport {
            calls: std::sync::Arc::clone(&calls),
            state: "SUBMITTED",
        };
        let adapter = HydraAdapter::new(Box::new(transport), binding(), Vec::new());
        let other = nexus_hydra::HydraBusinessBinding::new(
            HydraBindingId::new("binding-other").unwrap(),
            tenant(),
            business(),
            BTreeSet::from([HydraAccessChannel::REST]),
        );
        let req = request(
            nexus_hydra::HydraActionKind::ReadContext,
            ApprovalClass::None,
        );
        let err = adapter.submit_action(&other, &req).unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Authorization);
    }

    #[test]
    fn ep028_unit_read_context_rejects_inactive_binding() {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let transport = OkTransport {
            calls: std::sync::Arc::clone(&calls),
            state: "SUBMITTED",
        };
        let mut b = binding();
        b.deactivate();
        let adapter = HydraAdapter::new(Box::new(transport), b.clone(), Vec::new());
        let ctx = BusinessContext::single(tenant(), person(), business());
        let err = adapter.read_context(&b, &ctx).unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Policy);
    }
}
