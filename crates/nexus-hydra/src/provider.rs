//! EP-028 HydraProvider port (node contract public interface).
//!
//! Provider-neutral, versioned, and fail-closed: an unbound provider
//! returns Unavailable and never fabricates Hydra context. Provider
//! implementations live in connectors/hydra (M2+); M1 owns the port.

use crate::action::{HydraActionRequest, HydraActionState};
use crate::capability::HydraCapabilityMap;
use crate::context::HydraContextProjection;
use crate::error::{HydraError, HydraErrorCode};
use crate::model::{BusinessContext, HydraBusinessBinding};

/// Result of a governed action: the request, the binding it was
/// executed under, and the observed state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HydraActionResult {
    pub binding: HydraBusinessBinding,
    pub state: HydraActionState,
}

/// Provider-neutral Hydra port (SPEC-015 behavior 2: authenticated
/// MCP, REST, and durable events only; no direct database access).
pub trait HydraProvider {
    /// Read the authorized business context projection (single scope
    /// by default; portfolio only when explicitly authorized).
    fn read_context(
        &self,
        binding: &HydraBusinessBinding,
        context: &BusinessContext,
    ) -> Result<HydraContextProjection, HydraError>;

    /// The capabilities this provider actually advertises. Unbound and
    /// uncertified providers advertise nothing (fail closed).
    fn capabilities(&self) -> HydraCapabilityMap;

    /// Submit a governed action. The caller MUST have passed the
    /// policy gate; the provider enforces again (dual authorization
    /// gates, node contract acceptance obligation 4).
    fn submit_action(
        &self,
        binding: &HydraBusinessBinding,
        request: &HydraActionRequest,
    ) -> Result<HydraActionResult, HydraError>;
}

/// Fail-closed unbound provider. Every operation returns Unavailable;
/// it never fabricates context, capabilities, or action state.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundHydraProvider;

impl HydraProvider for UnboundHydraProvider {
    fn read_context(
        &self,
        _binding: &HydraBusinessBinding,
        _context: &BusinessContext,
    ) -> Result<HydraContextProjection, HydraError> {
        Err(HydraError::new(
            HydraErrorCode::Unavailable,
            "no Hydra provider bound",
            None,
            None,
            None,
            None,
        ))
    }

    fn capabilities(&self) -> HydraCapabilityMap {
        HydraCapabilityMap::new()
    }

    fn submit_action(
        &self,
        _binding: &HydraBusinessBinding,
        _request: &HydraActionRequest,
    ) -> Result<HydraActionResult, HydraError> {
        Err(HydraError::new(
            HydraErrorCode::Unavailable,
            "no Hydra provider bound",
            None,
            None,
            None,
            None,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::{HydraAccessChannel, HydraActionId, HydraActionKind, HydraBindingId};
    use nexus_domain::{ApprovalClass, BusinessId, PersonId, TenantId};
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

    fn binding() -> HydraBusinessBinding {
        HydraBusinessBinding::new(
            HydraBindingId::new("binding-1").unwrap(),
            tenant(),
            business(),
            BTreeSet::from([HydraAccessChannel::REST]),
        )
    }

    #[test]
    fn ep028_unit_unbound_provider_fails_closed() {
        let provider = UnboundHydraProvider;
        let ctx = BusinessContext::single(tenant(), person(), business());
        let err = provider.read_context(&binding(), &ctx).unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Unavailable);
        assert_eq!(provider.capabilities().kinds().count(), 0);
        let req = crate::action::HydraActionRequest::new(
            HydraActionId::new("action-1").unwrap(),
            tenant(),
            person(),
            business(),
            HydraActionKind::ReadContext,
            "idempotency-key-0001",
        )
        .with_approval_class(ApprovalClass::None);
        let err = provider.submit_action(&binding(), &req).unwrap_err();
        assert_eq!(err.code, HydraErrorCode::Unavailable);
    }
}
