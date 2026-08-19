//! EP-028 Hydra action request and governed submission (SPEC-015
//! behavior 8: paid-ad budget changes and public crisis responses
//! require human approval; node contract acceptance obligation 4: dual
//! authorization gates and end-to-end correlation are preserved).

use nexus_domain::{ApprovalClass, BusinessId, CorrelationId, PersonId, TenantId};
use serde::{Deserialize, Serialize};

use crate::error::HydraError;
use crate::vocabulary::{HydraActionId, HydraActionKind};

pub use crate::vocabulary::HydraActionState;

/// Canonical Hydra action request (SPEC-006 action-request shape).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydraActionRequest {
    pub action_id: HydraActionId,
    pub tenant_id: TenantId,
    pub principal_id: PersonId,
    pub business_id: BusinessId,
    pub kind: HydraActionKind,
    pub idempotency_key: String,
    pub approval_class: ApprovalClass,
    pub correlation: Option<CorrelationId>,
}

impl HydraActionRequest {
    pub fn new(
        action_id: HydraActionId,
        tenant_id: TenantId,
        principal_id: PersonId,
        business_id: BusinessId,
        kind: HydraActionKind,
        idempotency_key: impl Into<String>,
    ) -> Self {
        Self {
            action_id,
            tenant_id,
            principal_id,
            business_id,
            kind,
            idempotency_key: idempotency_key.into(),
            approval_class: ApprovalClass::None,
            correlation: None,
        }
    }

    pub fn with_approval_class(mut self, approval_class: ApprovalClass) -> Self {
        self.approval_class = approval_class;
        self
    }

    pub fn with_correlation(mut self, correlation: CorrelationId) -> Self {
        self.correlation = Some(correlation);
        self
    }

    pub fn validate(&self) -> Result<(), HydraError> {
        if self.idempotency_key.is_empty() {
            return Err(HydraError::validation("idempotency key required"));
        }
        if self.idempotency_key.len() < 16 {
            return Err(HydraError::validation(
                "idempotency key must be at least 16 characters",
            ));
        }
        Ok(())
    }
}

/// Action kinds that REQUIRE human approval (SPEC-015 behavior 8).
pub fn requires_human_approval(kind: HydraActionKind) -> bool {
    matches!(
        kind,
        HydraActionKind::PaidAdBudgetChange | HydraActionKind::PublicCrisisResponse
    )
}

/// Dual authorization gate: validate the request, then enforce the
/// approval policy BEFORE any provider invocation. A denied request
/// returns Policy and never reaches the provider port (tracking
/// provider tests prove zero calls).
pub fn enforce_hydra_action_policy(request: &HydraActionRequest) -> Result<(), HydraError> {
    request.validate()?;
    if requires_human_approval(request.kind) {
        let human = matches!(
            request.approval_class,
            ApprovalClass::Human | ApprovalClass::StrongHuman | ApprovalClass::FourEyes
        );
        if !human {
            return Err(HydraError::policy(format!(
                "{} requires human approval",
                request.kind
            )));
        }
    }
    Ok(())
}

/// The HydraProvider port method a governed action eventually invokes.
/// M1 owns the gate; the provider implementation is M2.
pub trait HydraActionSink {
    /// Submit an approved action to the provider boundary.
    fn submit(&self, request: &HydraActionRequest) -> Result<HydraActionState, HydraError>;
}

/// Governed action submission: policy gate FIRST, provider call only
/// after the gate passes. Returns the provider's state on success or a
/// typed error (Policy/Validation on the gate, provider error on the
/// call).
pub fn hydra_action_governed(
    sink: &dyn HydraActionSink,
    request: &HydraActionRequest,
) -> Result<HydraActionState, HydraError> {
    enforce_hydra_action_policy(request)?;
    sink.submit(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::HydraErrorCode;
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

    fn request(kind: HydraActionKind, approval: ApprovalClass) -> HydraActionRequest {
        HydraActionRequest::new(
            HydraActionId::new("action-1").unwrap(),
            tenant(),
            person(),
            business(),
            kind,
            "idempotency-key-0001",
        )
        .with_approval_class(approval)
    }

    struct TrackingSink {
        calls: AtomicUsize,
    }

    impl HydraActionSink for TrackingSink {
        fn submit(&self, _request: &HydraActionRequest) -> Result<HydraActionState, HydraError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(HydraActionState::Submitted)
        }
    }

    #[test]
    fn ep028_unit_paid_ad_budget_change_requires_human_approval() {
        assert!(requires_human_approval(HydraActionKind::PaidAdBudgetChange));
        assert!(requires_human_approval(
            HydraActionKind::PublicCrisisResponse
        ));
        assert!(!requires_human_approval(HydraActionKind::ReadContext));
    }

    #[test]
    fn ep028_unit_governed_action_denied_before_provider_zero_calls() {
        // Paid-ad budget change with only policy-class approval: the
        // gate must reject BEFORE the provider sink is invoked.
        let sink = TrackingSink {
            calls: AtomicUsize::new(0),
        };
        let req = request(HydraActionKind::PaidAdBudgetChange, ApprovalClass::Policy);
        let err = hydra_action_governed(&sink, &req).unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Policy);
        assert_eq!(sink.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ep028_unit_governed_action_human_approved_reaches_provider_once() {
        let sink = TrackingSink {
            calls: AtomicUsize::new(0),
        };
        let req = request(HydraActionKind::PaidAdBudgetChange, ApprovalClass::Human);
        let state = hydra_action_governed(&sink, &req).unwrap();
        assert_eq!(state, HydraActionState::Submitted);
        assert_eq!(sink.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn ep028_unit_governed_action_validation_denied_before_provider() {
        let sink = TrackingSink {
            calls: AtomicUsize::new(0),
        };
        let mut req = request(HydraActionKind::ReadContext, ApprovalClass::None);
        req.idempotency_key = "short".into();
        let err = hydra_action_governed(&sink, &req).unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Validation);
        assert_eq!(sink.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ep028_unit_action_request_serde_roundtrip() {
        let req = request(
            HydraActionKind::PublicCrisisResponse,
            ApprovalClass::FourEyes,
        );
        let json = serde_json::to_string(&req).unwrap();
        let back: HydraActionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, req);
        assert_eq!(back.approval_class, ApprovalClass::FourEyes);
    }
}
